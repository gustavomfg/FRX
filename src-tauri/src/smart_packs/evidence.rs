//! Evidence Engine: reúne as medições locais de cada pacote.
//!
//! Metodologia (documentada em `docs/SMART-PACKS.md`):
//!
//! - Cada sessão do jogo pertence ao pacote das variáveis do FRX ativas na
//!   captura (`benchmark::session_config`). Sem registro, fica de fora até o
//!   usuário declarar a configuração.
//! - Referência de condições: a sessão de baseline mais recente com
//!   qualidade suficiente (ou, sem baseline, a sessão conhecida mais recente).
//!   Sessões com diferença **grave** de condições em relação a ela (jogo, GPU,
//!   resolução, preset, limite de FPS, VSync, modo de captura, duração > 4×)
//!   são excluídas, com o motivo. Diferenças menores (Proton, driver, kernel)
//!   viram notas.
//! - Por métrica e pacote: mediana das sessões (robusta a uma partida fora
//!   da curva), mínimo, máximo e dispersão (máx − mín) ÷ mediana.
//! - Pacote × baseline: diferença percentual entre medianas. Só conta como
//!   melhora ou regressão se passar do limiar:
//!   - com ≥ 2 sessões de cada lado: o maior entre o piso da métrica e a
//!     dispersão observada nos dois grupos;
//!   - com 1 sessão em algum lado (**limitado**): o maior entre o piso e a
//!     variação natural dentro da sessão (desvio do FPS por segundo, a mesma
//!     estimativa da comparação de sessões). Serve só como indicação.
//! - Pisos: FPS médio 3%, 1% low 5%, 0,1% low 8%, frametime P99 5%,
//!   picos/min 20%.
//!
//! Nenhuma porcentagem de confiança é inventada: o resultado é melhora,
//! regressão, sem diferença além da variação, ou inconclusivo.

use std::collections::{BTreeMap, HashMap};

use super::models::*;
use crate::performance::comparison::{comparability, noise_pct, pct};
use crate::performance::models::{QualityLevel, SessionMeta, SessionSource};

pub struct MetricDef {
    pub key: &'static str,
    pub label: &'static str,
    pub unit: &'static str,
    pub higher_better: bool,
    pub floor: f64,
}

pub const METRICS: &[MetricDef] = &[
    MetricDef {
        key: "fpsAvg",
        label: "FPS médio",
        unit: "FPS",
        higher_better: true,
        floor: 3.0,
    },
    MetricDef {
        key: "fpsLow1",
        label: "1% low",
        unit: "FPS",
        higher_better: true,
        floor: 5.0,
    },
    MetricDef {
        key: "fpsLow01",
        label: "0,1% low",
        unit: "FPS",
        higher_better: true,
        floor: 8.0,
    },
    MetricDef {
        key: "ftP99",
        label: "Frametime P99",
        unit: "ms",
        higher_better: false,
        floor: 5.0,
    },
    MetricDef {
        key: "spikesPerMin",
        label: "Picos por minuto",
        unit: "/min",
        higher_better: false,
        floor: 20.0,
    },
];

fn value(m: &SessionMeta, key: &str) -> Option<f64> {
    let s = &m.stats;
    match key {
        "fpsAvg" => s.fps.avg,
        "fpsLow1" => s.fps.low1,
        "fpsLow01" => s.fps.low01,
        "ftP99" => s.frametime.p99,
        "spikesPerMin" => s.stability.spikes_per_min,
        _ => None,
    }
    .filter(|v| v.is_finite() && *v >= 0.0)
}

pub fn stats(values: &[f64]) -> Option<MetricStats> {
    if values.is_empty() {
        return None;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    let n = v.len();
    let median = if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    };
    let (min, max) = (v[0], v[n - 1]);
    Some(MetricStats {
        n,
        median,
        min,
        max,
        spread_pct: (n >= 2 && median > 1e-9).then(|| (max - min) / median * 100.0),
    })
}

fn when(m: &SessionMeta) -> &str {
    m.captured_at.as_deref().unwrap_or(&m.created_at)
}

pub struct Grouped {
    pub measurements: BTreeMap<String, Measurements>,
    pub unassigned: Vec<SessionRef>,
    pub reference: Option<String>,
    pub notes: Vec<String>,
}

fn session_ref(m: &SessionMeta, snap: Option<&PackSnapshot>) -> SessionRef {
    SessionRef {
        id: m.id.clone(),
        label: m.label.clone(),
        captured_at: m.captured_at.clone().or_else(|| Some(m.created_at.clone())),
        source: snap.map(|s| s.source).unwrap_or(SnapshotSource::Declared),
        verified: snap.is_some_and(|s| s.verified),
        duration_s: m.stats.duration_s,
        avg_fps: m.stats.fps.avg,
        low1: m.stats.fps.low1,
        excluded: None,
        notes: vec![],
    }
}

pub fn group(
    app_id: &str,
    sessions: &[SessionMeta],
    config: impl Fn(&SessionMeta) -> Option<PackSnapshot>,
    baseline_id: &str,
) -> Grouped {
    let mine: Vec<(&SessionMeta, Option<PackSnapshot>)> = sessions
        .iter()
        .filter(|m| m.app_id.as_deref() == Some(app_id))
        .map(|m| (m, config(m)))
        .collect();
    let mut notes = Vec::new();
    let mut unassigned = Vec::new();
    let usable = |m: &SessionMeta| {
        m.quality.level != QualityLevel::Unsuitable
            && m.source != SessionSource::Synthetic
            && !m.id.starts_with("s-fixture-")
            && !m.quality.issues.iter().any(|i| {
                matches!(
                    i.code.as_str(),
                    "synthetic" | "fixture" | "interrupted" | "pack-changed"
                )
            })
    };
    let latest = |filter: &dyn Fn(&PackSnapshot) -> bool| {
        mine.iter()
            .filter(|(m, s)| {
                usable(m)
                    && s.as_ref().is_some_and(|s| {
                        (s.verified || s.source == SnapshotSource::Declared) && filter(s)
                    })
            })
            .max_by(|a, b| when(a.0).cmp(when(b.0)))
            .map(|(m, _)| *m)
    };
    let reference = latest(&|s| s.pack_id == baseline_id).or_else(|| latest(&|_| true));
    if reference.is_some_and(|r| {
        mine.iter()
            .any(|(m, s)| m.id == r.id && s.as_ref().is_some_and(|s| s.pack_id != baseline_id))
    }) {
        notes.push("Ainda não há baseline medido; as condições de referência vêm da sessão conhecida mais recente.".into());
    }

    let mut groups: BTreeMap<String, Vec<(SessionRef, &SessionMeta)>> = BTreeMap::new();
    for (m, snap) in &mine {
        let mut r = session_ref(m, snap.as_ref());
        let Some(snap) = snap else {
            r.excluded = Some("Configuração do FRX não registrada (sessão importada ou anterior à v0.4). Declare a configuração para usá-la.".into());
            unassigned.push(r);
            continue;
        };
        if (m.source == SessionSource::Synthetic || m.id.starts_with("s-fixture-"))
            || m.quality
                .issues
                .iter()
                .any(|i| matches!(i.code.as_str(), "synthetic" | "fixture"))
        {
            r.excluded = Some(
                "Dados sintéticos identificados: não são uma medição real e não entram no ranking."
                    .into(),
            );
        } else if !usable(m) {
            r.excluded = Some("Captura interrompida, configuração alterada ou qualidade insuficiente. Repita a captura completa no mesmo trecho.".into());
        } else if snap.source == SnapshotSource::Assisted && !snap.verified {
            r.excluded = Some("Pacote não confirmado no disco durante a captura. Confira a operação, reabra o jogo e repita.".into());
        } else if let Some(reference) = reference.filter(|x| x.id != m.id) {
            let c = comparability(reference, m);
            let major: Vec<String> = c
                .differences
                .iter()
                .filter(|d| {
                    d.major || matches!(d.field.as_str(), "proton" | "driver" | "cpu" | "kernel")
                })
                .map(|d| d.label.clone())
                .collect();
            if !major.is_empty() {
                r.excluded = Some(format!(
                    "Condições diferentes da referência: {}.",
                    major.join(", ")
                ));
            }
            for d in c.differences.iter().filter(|d| {
                !d.major
                    && !matches!(
                        d.field.as_str(),
                        "fixes" | "proton" | "driver" | "cpu" | "kernel"
                    )
            }) {
                r.notes
                    .push(format!("{} diferente da referência.", d.label));
            }
            if m.conditions.resolution.is_none() || m.conditions.preset.is_none() {
                r.notes.push("Resolução ou preset não informados.".into());
            }
        }
        if snap.source == SnapshotSource::Declared {
            r.notes
                .push("Configuração declarada por você, não conferida pelo FRX.".into());
        } else if !snap.verified {
            r.notes
                .push("Alguma correção não estava confirmada no disco durante a captura.".into());
        }
        if m.stats.fps.lows_estimated {
            r.notes
                .push("1% e 0,1% low estimados (amostras periódicas).".into());
        }
        groups.entry(snap.pack_id.clone()).or_default().push((r, m));
    }
    let measurements = groups
        .into_iter()
        .map(|(id, list)| {
            let used: Vec<&SessionMeta> = list
                .iter()
                .filter(|(r, _)| r.excluded.is_none())
                .map(|(_, m)| *m)
                .collect();
            let metric =
                |k: &str| stats(&used.iter().filter_map(|m| value(m, k)).collect::<Vec<_>>());
            let m = Measurements {
                pack_id: id.clone(),
                avg: metric("fpsAvg"),
                low1: metric("fpsLow1"),
                low01: metric("fpsLow01"),
                ft_p99: metric("ftP99"),
                spikes_per_min: metric("spikesPerMin"),
                sessions: list.into_iter().map(|(r, _)| r).collect(),
            };
            (id, m)
        })
        .collect();
    Grouped {
        measurements,
        unassigned,
        reference: reference.map(|r| r.id.clone()),
        notes,
    }
}

fn metric_stats<'a>(m: &'a Measurements, key: &str) -> Option<&'a MetricStats> {
    match key {
        "fpsAvg" => m.avg.as_ref(),
        "fpsLow1" => m.low1.as_ref(),
        "fpsLow01" => m.low01.as_ref(),
        "ftP99" => m.ft_p99.as_ref(),
        "spikesPerMin" => m.spikes_per_min.as_ref(),
        _ => None,
    }
}

/// Uma métrica, `pack` contra `base`.
pub fn delta(
    def: &MetricDef,
    base: &MetricStats,
    pack: &MetricStats,
    within_noise: Option<f64>,
) -> Option<MetricDelta> {
    let percentage = pct(base.median, pack.median);
    let d = percentage.unwrap_or(0.0);
    let limited = base.n < 2 || pack.n < 2;
    let threshold = if limited {
        def.floor.max(within_noise.unwrap_or(0.0))
    } else {
        def.floor
            .max(base.spread_pct.unwrap_or(0.0))
            .max(pack.spread_pct.unwrap_or(0.0))
    };
    let oriented = if def.higher_better { d } else { -d };
    let outcome = if percentage.is_none() {
        if (pack.median - base.median).abs() <= 1e-9 {
            Outcome::NoDifference
        } else if (pack.median > base.median) == def.higher_better {
            Outcome::Improvement
        } else {
            Outcome::Regression
        }
    } else if oriented.abs() <= threshold {
        Outcome::NoDifference
    } else if oriented > 0.0 {
        Outcome::Improvement
    } else {
        Outcome::Regression
    };
    Some(MetricDelta {
        metric: def.key.into(),
        label: def.label.into(),
        delta_pct: d,
        percentage_available: percentage.is_some(),
        threshold_pct: threshold,
        outcome,
        limited,
        before: base.median,
        after: pack.median,
        unit: def.unit.into(),
    })
}

/// Pacote × outro (normalmente o baseline), métrica por métrica.
pub fn compare(
    base: &Measurements,
    pack: &Measurements,
    metas: &HashMap<&str, &SessionMeta>,
) -> VsBaseline {
    // Variação natural dentro das sessões usadas (só para o FPS médio).
    let noise = |m: &Measurements| {
        m.sessions
            .iter()
            .filter(|s| s.excluded.is_none())
            .filter_map(|s| metas.get(s.id.as_str()).and_then(|x| noise_pct(&x.stats)))
            .fold(None, |acc: Option<f64>, v| {
                Some(acc.map_or(v, |a| a.max(v)))
            })
    };
    let within = match (noise(base), noise(pack)) {
        (Some(x), Some(y)) => Some((x * x + y * y).sqrt()),
        (x, y) => x.or(y),
    };
    let mut deltas = Vec::new();
    for def in METRICS {
        if let (Some(b), Some(p)) = (metric_stats(base, def.key), metric_stats(pack, def.key)) {
            let w = (def.key == "fpsAvg").then_some(within).flatten();
            if let Some(d) = delta(def, b, p, w) {
                deltas.push(d);
            }
        }
    }
    let limited = deltas.iter().any(|d| d.limited);
    let mut notes = Vec::new();
    if limited {
        notes.push(format!(
            "Poucas medições ({} × {} sessões): resultado indicativo. Repita as capturas para uma conclusão.",
            base.used(),
            pack.used()
        ));
    }
    VsBaseline {
        pack_id: pack.pack_id.clone(),
        deltas,
        limited,
        notes,
    }
}
