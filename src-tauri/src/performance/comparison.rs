//! Comparação entre duas sessões ("antes" e "depois").
//!
//! O veredito é conservador: diferenças dentro da variação natural da
//! sessão viram "diferença pequena" ou "inconclusivo". Nunca atribuímos a
//! mudança a uma correção específica — só descrevemos o que foi medido.

use serde::{Deserialize, Serialize};

use super::models::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Better {
    Higher,
    Lower,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompareRow {
    pub key: String,
    pub label: String,
    pub unit: String,
    pub before: f64,
    pub after: f64,
    pub abs_diff: f64,
    /// `None` quando a base é zero ou próxima disso (porcentagem enganosa).
    pub pct_diff: Option<f64>,
    pub better: Better,
    /// "improved" | "worsened" | "same" — só para métricas com direção definida.
    pub direction: String,
    pub estimated: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum VerdictKind {
    Improvement,
    Regression,
    SmallDifference,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Verdict {
    pub kind: VerdictKind,
    pub avg_delta_pct: Option<f64>,
    pub low1_delta_pct: Option<f64>,
    /// Variação natural estimada (em % do FPS médio de referência).
    pub noise_pct: Option<f64>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Difference {
    pub field: String,
    pub label: String,
    pub before: Option<String>,
    pub after: Option<String>,
    /// `true` quando a diferença compromete uma comparação controlada.
    pub major: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Comparability {
    pub controlled: bool,
    pub differences: Vec<Difference>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonResult {
    pub before_id: String,
    pub after_id: String,
    pub rows: Vec<CompareRow>,
    pub verdict: Verdict,
    pub comparability: Comparability,
}

/// Número com vírgula decimal, como o resto da interface.
fn br(v: f64, digits: usize) -> String {
    format!("{v:.digits$}").replace('.', ",")
}

fn br_signed(v: f64) -> String {
    format!("{}{}", if v >= 0.0 { "+" } else { "−" }, br(v.abs(), 1))
}

/// Diferença percentual segura.
pub fn pct(before: f64, after: f64) -> Option<f64> {
    (before.abs() > 1e-9 && before.is_finite() && after.is_finite())
        .then(|| (after - before) / before.abs() * 100.0)
}

/// Equivalências de representação, sem inferir que modelos/versões distintos
/// são o mesmo ambiente. O logger OpenGL e o pci.ids podem grafar a mesma GPU
/// de formas diferentes; fabricantes ou séries semelhantes não bastam.
pub(crate) fn same_condition(field: &str, a: &str, b: &str) -> bool {
    fn normalize(field: &str, s: &str) -> String {
        let mut value = s
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        if field == "gpu" {
            if let Some(v) = value.strip_suffix("/pcie/sse2") {
                value = v.trim_end().to_owned();
            }
            if value.ends_with(']') {
                if let Some((_, marketing)) = value.rsplit_once('[') {
                    let marketing = marketing.trim_end_matches(']').trim();
                    if marketing.starts_with("geforce ") || marketing.starts_with("radeon ") {
                        value = marketing.to_owned();
                    }
                }
            }
            for prefix in ["nvidia ", "amd "] {
                if let Some(v) = value.strip_prefix(prefix) {
                    value = v.to_owned();
                    break;
                }
            }
        } else if field == "driver" {
            // MangoHud pode registrar só a versão do driver proprietário.
            // Não equipara versões Mesa, nouveau ou outros módulos a NVIDIA.
            if let Some(version) = value.strip_prefix("nvidia ") {
                if version.chars().all(|c| c.is_ascii_digit() || c == '.') {
                    value = version.to_owned();
                }
            }
        }
        value
    }
    normalize(field, a) == normalize(field, b)
}

fn row(
    key: &str,
    label: &str,
    unit: &str,
    before: Option<f64>,
    after: Option<f64>,
    better: Better,
    estimated: bool,
) -> Option<CompareRow> {
    let (b, a) = (before?, after?);
    let pct_diff = pct(b, a);
    let rel = pct_diff.unwrap_or(0.0);
    let direction = match better {
        Better::Neutral => "same",
        _ if rel.abs() < 1.0 => "same",
        Better::Higher if a > b => "improved",
        Better::Lower if a < b => "improved",
        _ => "worsened",
    };
    Some(CompareRow {
        key: key.into(),
        label: label.into(),
        unit: unit.into(),
        before: b,
        after: a,
        abs_diff: a - b,
        pct_diff,
        better,
        direction: direction.into(),
        estimated,
    })
}

/// Variação natural do FPS médio (em %), a partir do desvio por segundo.
/// Erro-padrão × 2 para compensar a autocorrelação entre segundos vizinhos.
pub(crate) fn noise_pct(s: &SessionStats) -> Option<f64> {
    let sd = s.stability.per_second_fps_stdev?;
    let avg = s.fps.avg?;
    let n = s.stability.seconds.max(1) as f64;
    Some(2.0 * sd / n.sqrt() / avg * 100.0)
}

pub fn compare(before: &SessionMeta, after: &SessionMeta) -> ComparisonResult {
    let (b, a) = (&before.stats, &after.stats);
    let est = b.fps.lows_estimated || a.fps.lows_estimated;
    let mut rows: Vec<CompareRow> = [
        row(
            "fpsAvg",
            "FPS médio",
            "FPS",
            b.fps.avg,
            a.fps.avg,
            Better::Higher,
            false,
        ),
        row(
            "fpsLow1",
            "1% low",
            "FPS",
            b.fps.low1,
            a.fps.low1,
            Better::Higher,
            est,
        ),
        row(
            "fpsLow01",
            "0,1% low",
            "FPS",
            b.fps.low01,
            a.fps.low01,
            Better::Higher,
            est,
        ),
        row(
            "fpsMedian",
            "FPS mediano",
            "FPS",
            b.fps.median,
            a.fps.median,
            Better::Higher,
            false,
        ),
        row(
            "ftAvg",
            "Frametime médio",
            "ms",
            b.frametime.avg,
            a.frametime.avg,
            Better::Lower,
            false,
        ),
        row(
            "ftP99",
            "Frametime P99",
            "ms",
            b.frametime.p99,
            a.frametime.p99,
            Better::Lower,
            est,
        ),
        row(
            "ftStdev",
            "Variação do frametime (desvio)",
            "ms",
            b.frametime.stdev,
            a.frametime.stdev,
            Better::Lower,
            false,
        ),
        row(
            "spikesPerMin",
            "Picos por minuto",
            "/min",
            b.stability.spikes_per_min,
            a.stability.spikes_per_min,
            Better::Lower,
            b.mode == CaptureMode::Periodic || a.mode == CaptureMode::Periodic,
        ),
    ]
    .into_iter()
    .flatten()
    .collect();
    let res = |s: &SessionStats, m: Metric| s.resources.get(&m).map(|r| r.avg);
    for (m, label, better) in [
        (Metric::CpuLoad, "Uso de CPU", Better::Neutral),
        (Metric::GpuLoad, "Uso de GPU", Better::Neutral),
        (Metric::Vram, "VRAM", Better::Neutral),
        (Metric::Ram, "RAM", Better::Neutral),
        (Metric::GpuTemp, "Temperatura da GPU", Better::Lower),
        (Metric::CpuTemp, "Temperatura da CPU", Better::Lower),
        (Metric::GpuPower, "Consumo da GPU", Better::Neutral),
        (Metric::CpuPower, "Consumo da CPU", Better::Neutral),
    ] {
        if let Some(r) = row(
            &format!("{m:?}"),
            label,
            m.unit(),
            res(b, m),
            res(a, m),
            better,
            false,
        ) {
            rows.push(r);
        }
    }

    let comparability = comparability(before, after);
    let verdict = verdict(before, after, &comparability);
    ComparisonResult {
        before_id: before.id.clone(),
        after_id: after.id.clone(),
        rows,
        verdict,
        comparability,
    }
}

fn verdict(before: &SessionMeta, after: &SessionMeta, comp: &Comparability) -> Verdict {
    let (b, a) = (&before.stats, &after.stats);
    let avg = match (b.fps.avg, a.fps.avg) {
        (Some(x), Some(y)) => pct(x, y),
        _ => None,
    };
    let low1 = match (b.fps.low1, a.fps.low1) {
        (Some(x), Some(y)) => pct(x, y),
        _ => None,
    };
    let noise = match (noise_pct(b), noise_pct(a)) {
        (Some(x), Some(y)) => Some((x * x + y * y).sqrt()),
        (Some(x), None) | (None, Some(x)) => Some(x),
        _ => None,
    };
    let mut reasons = Vec::new();
    let threshold = noise.unwrap_or(0.0).max(3.0);

    let unsuitable = [before, after]
        .iter()
        .any(|s| s.quality.level == QualityLevel::Unsuitable);
    let kind = if comp.differences.iter().any(|d| d.field == "appId") {
        reasons.push("Os AppIDs não identificam o mesmo jogo nas duas sessões. Os números continuam disponíveis, mas não representam melhoria ou regressão de um jogo.".into());
        VerdictKind::Inconclusive
    } else if unsuitable {
        reasons.push(
            "Ao menos uma sessão não tem qualidade suficiente para comparação (veja os avisos)."
                .into(),
        );
        VerdictKind::Inconclusive
    } else if let Some(d) = avg {
        let low_bad = low1.is_some_and(|l| l <= -threshold.max(5.0));
        let low_good = low1.is_some_and(|l| l >= threshold.max(5.0));
        if d.abs() < threshold {
            if low_bad {
                reasons.push(
                    "FPS médio parecido, mas o 1% low caiu: a sessão “depois” teve mais quedas."
                        .into(),
                );
                VerdictKind::Regression
            } else if low_good {
                reasons.push(
                    "FPS médio parecido, mas o 1% low subiu: menos quedas na sessão “depois”."
                        .into(),
                );
                VerdictKind::Improvement
            } else {
                reasons.push(format!(
                    "A diferença no FPS médio ({}%) está dentro da variação esperada (±{}%).",
                    br_signed(d),
                    br(threshold, 1)
                ));
                VerdictKind::SmallDifference
            }
        } else if d > 0.0 && low_bad {
            reasons.push("Sinais mistos: FPS médio maior, mas 1% low menor.".into());
            VerdictKind::Inconclusive
        } else if d < 0.0 && low_good {
            reasons.push("Sinais mistos: FPS médio menor, mas 1% low maior.".into());
            VerdictKind::Inconclusive
        } else if d > 0.0 {
            reasons.push(format!(
                "FPS médio {d:+.1}%, acima da variação esperada (±{threshold:.1}%)."
            ));
            VerdictKind::Improvement
        } else {
            reasons.push(format!(
                "FPS médio {d:+.1}%, além da variação esperada (±{threshold:.1}%)."
            ));
            VerdictKind::Regression
        }
    } else {
        reasons.push("Faltam dados de FPS em uma das sessões.".into());
        VerdictKind::Inconclusive
    };
    if !comp.controlled && kind != VerdictKind::Inconclusive {
        reasons.push("As condições dos testes não são iguais; trate o resultado como indicativo, não como prova.".into());
    }
    if noise.is_none() {
        reasons.push("Não foi possível estimar a variação natural (sessões curtas); usamos o limite mínimo de 3%.".into());
    }
    Verdict {
        kind,
        avg_delta_pct: avg,
        low1_delta_pct: low1,
        noise_pct: noise,
        reasons,
    }
}

pub(crate) fn comparability(before: &SessionMeta, after: &SessionMeta) -> Comparability {
    let mut differences = Vec::new();
    let mut notes = Vec::new();
    let (cb, ca) = (&before.conditions, &after.conditions);
    let mut field =
        |key: &str, label: &str, x: &Option<String>, y: &Option<String>, major: bool| {
            if let (Some(a), Some(b)) = (x, y) {
                if !same_condition(key, a, b) {
                    differences.push(Difference {
                        field: key.into(),
                        label: label.into(),
                        before: Some(a.clone()),
                        after: Some(b.clone()),
                        major,
                    });
                }
            }
        };
    field("appId", "Jogo (AppID)", &before.app_id, &after.app_id, true);
    field(
        "gameBuildId",
        "Versão do jogo (build Steam)",
        &cb.game_build_id,
        &ca.game_build_id,
        true,
    );
    field("gpu", "GPU", &cb.gpu, &ca.gpu, true);
    field("cpu", "CPU", &cb.cpu, &ca.cpu, true);
    field(
        "resolution",
        "Resolução",
        &cb.resolution,
        &ca.resolution,
        true,
    );
    field("preset", "Preset gráfico", &cb.preset, &ca.preset, true);
    field(
        "fpsLimit",
        "Limite de FPS",
        &cb.fps_limit,
        &ca.fps_limit,
        true,
    );
    field("proton", "Proton", &cb.proton, &ca.proton, false);
    field("driver", "Driver", &cb.driver, &ca.driver, false);
    field("kernel", "Kernel", &cb.kernel, &ca.kernel, false);
    field(
        "cpuScheduler",
        "Governador da CPU",
        &cb.cpu_scheduler,
        &ca.cpu_scheduler,
        false,
    );
    if [before, after].iter().any(|m| {
        m.app_id
            .as_deref()
            .is_none_or(|app| !crate::art::valid_app_id(app))
    }) && !differences.iter().any(|d| d.field == "appId")
    {
        differences.push(Difference {
            field: "appId".into(),
            label: "Jogo sem AppID confirmado".into(),
            before: before.app_id.clone(),
            after: after.app_id.clone(),
            major: true,
        });
        notes.push("Uma sessão sem AppID permanece no histórico sem ser atribuída automaticamente ao jogo aberto.".into());
    }
    if let (Some(x), Some(y)) = (cb.vsync, ca.vsync) {
        if x != y {
            differences.push(Difference {
                field: "vsync".into(),
                label: "VSync".into(),
                before: Some(on_off(x)),
                after: Some(on_off(y)),
                major: true,
            });
        }
    }
    // Correções do FRX: só registramos a diferença — sem atribuir causa.
    if let (Some(x), Some(y)) = (&before.active_fixes, &after.active_fixes) {
        let ids = |v: &[crate::fix_engine::models::ActiveFixRef]| {
            v.iter()
                .map(|f| format!("{}@{}", f.recipe_id, f.recipe_version))
                .collect::<Vec<_>>()
        };
        if ids(x) != ids(y) {
            let label = |v: &[crate::fix_engine::models::ActiveFixRef]| {
                if v.is_empty() {
                    "nenhuma".to_owned()
                } else {
                    v.iter()
                        .map(|f| format!("{} (v{})", f.title, f.recipe_version))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            };
            differences.push(Difference {
                field: "fixes".into(),
                label: "Correções do FRX ativas".into(),
                before: Some(label(x)),
                after: Some(label(y)),
                major: false,
            });
            notes.push("As sessões têm correções do FRX diferentes. Uma diferença de desempenho entre elas não prova que a correção foi a causa; repita as medições para confirmar.".into());
        }
    } else if before.active_fixes.is_some() != after.active_fixes.is_some() {
        notes.push("Uma das sessões não registra quais correções do FRX estavam ativas (importada ou anterior à v0.4).".into());
    }
    let (b, a) = (&before.stats, &after.stats);
    if b.mode != a.mode {
        differences.push(Difference {
            field: "mode".into(),
            label: "Modo de captura".into(),
            before: Some(mode_label(b.mode)),
            after: Some(mode_label(a.mode)),
            major: true,
        });
    }
    let ratio = if b.duration_s > 0.0 && a.duration_s > 0.0 {
        (b.duration_s / a.duration_s).max(a.duration_s / b.duration_s)
    } else {
        1.0
    };
    if ratio > 2.0 {
        differences.push(Difference {
            field: "duration".into(),
            label: "Duração".into(),
            before: Some(format!("{:.0} s", b.duration_s)),
            after: Some(format!("{:.0} s", a.duration_s)),
            major: ratio > 4.0,
        });
    }
    let missing = [
        cb.resolution.is_none() || ca.resolution.is_none(),
        cb.preset.is_none() || ca.preset.is_none(),
    ];
    if missing.iter().any(|m| *m) {
        notes.push("Resolução ou preset não informados em alguma sessão; não dá para confirmar que os testes usaram as mesmas configurações gráficas.".into());
    }
    notes.push("Para uma comparação justa, repita o mesmo trecho do jogo, com as mesmas configurações e a mesma duração.".into());
    let controlled = !differences.iter().any(|d| d.major);
    Comparability {
        controlled,
        differences,
        notes,
    }
}

fn on_off(b: bool) -> String {
    if b {
        "ligado".into()
    } else {
        "desligado".into()
    }
}

fn mode_label(m: CaptureMode) -> String {
    match m {
        CaptureMode::PerFrame => "por quadro".into(),
        CaptureMode::Periodic => "amostras periódicas".into(),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::metrics::analyze;
    use super::super::parser::{fixtures::per_frame, parse};
    use super::*;

    pub fn session(id: &str, fts: &[f64]) -> SessionMeta {
        let a = analyze(&parse(per_frame(fts, 90.0).as_bytes()).unwrap());
        SessionMeta {
            schema_version: SCHEMA_VERSION,
            analysis_version: ANALYSIS_VERSION,
            id: id.into(),
            app_id: Some("200260".into()),
            game_name: Some("Batman".into()),
            label: id.into(),
            notes: String::new(),
            created_at: "2026-10-24T00:00:00Z".into(),
            captured_at: None,
            source: SessionSource::Import,
            original_name: None,
            original_path: None,
            raw_hash: id.into(),
            raw_bytes: 0,
            format: LogFormat::MangohudV1,
            mangohud_version: None,
            conditions: Conditions {
                resolution: Some("1920x1080".into()),
                preset: Some("Alto".into()),
                ..Default::default()
            },
            stats: a.stats,
            quality: a.quality,
            fix_ids: vec![],
            active_fixes: None,
            smart_pack: None,
        }
    }

    /// Frametimes com leve oscilação determinística em torno de `base`.
    fn wobble(base: f64, n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| base * (1.0 + 0.03 * ((i as f64) / 7.0).sin()))
            .collect()
    }

    #[test]
    fn improvement_is_detected_with_correct_percentages() {
        let b = session("a", &wobble(20.0, 3000)); // ~50 FPS
        let a = session("b", &wobble(1000.0 / 112.0, 6720)); // ~112 FPS, mesma duração
        let r = compare(&b, &a);
        assert_eq!(r.verdict.kind, VerdictKind::Improvement, "{:?}", r.verdict);
        let avg = r.rows.iter().find(|x| x.key == "fpsAvg").unwrap();
        assert!((avg.pct_diff.unwrap() - 124.0).abs() < 1.0, "{:?}", avg);
        assert_eq!(avg.direction, "improved");
        let ft = r.rows.iter().find(|x| x.key == "ftAvg").unwrap();
        assert_eq!(ft.direction, "improved", "frametime menor é melhor");
        assert!(r.comparability.controlled);
    }

    #[test]
    fn regression_and_small_difference() {
        let b = session("a", &wobble(10.0, 6000));
        let worse = session("b", &wobble(12.5, 4800));
        assert_eq!(compare(&b, &worse).verdict.kind, VerdictKind::Regression);
        let same = session("c", &wobble(10.1, 5940));
        assert_eq!(
            compare(&b, &same).verdict.kind,
            VerdictKind::SmallDifference
        );
    }

    #[test]
    fn unsuitable_sessions_are_inconclusive_and_conditions_are_flagged() {
        let b = session("a", &wobble(10.0, 6000));
        let tiny = session("b", &wobble(5.0, 200));
        assert_eq!(compare(&b, &tiny).verdict.kind, VerdictKind::Inconclusive);

        let mut other = session("c", &wobble(8.0, 7500));
        other.conditions.resolution = Some("2560x1440".into());
        let r = compare(&b, &other);
        assert!(!r.comparability.controlled);
        assert!(r
            .comparability
            .differences
            .iter()
            .any(|d| d.field == "resolution"));
        assert!(r.verdict.reasons.iter().any(|x| x.contains("condições")));
    }

    #[test]
    fn different_active_fixes_are_listed_without_blocking() {
        use crate::fix_engine::models::ActiveFixRef;
        let fts = wobble(16.0, 2000);
        let mut a = session("a", &fts);
        let mut b = session("b", &fts);
        a.active_fixes = Some(vec![]);
        b.active_fixes = Some(vec![ActiveFixRef {
            recipe_id: "frx.compat.wined3d".into(),
            recipe_version: "1.0.0".into(),
            operation_id: "op-1".into(),
            title: "WineD3D".into(),
        }]);
        let c = compare(&a, &b).comparability;
        let d = c.differences.iter().find(|d| d.field == "fixes").unwrap();
        assert_eq!(d.before.as_deref(), Some("nenhuma"));
        assert_eq!(d.after.as_deref(), Some("WineD3D (v1.0.0)"));
        assert!(!d.major && c.controlled);
        assert!(c.notes.iter().any(|n| n.contains("não prova")));
        b.active_fixes = Some(vec![]);
        assert!(!compare(&a, &b)
            .comparability
            .differences
            .iter()
            .any(|d| d.field == "fixes"));
    }

    #[test]
    fn missing_metadata_does_not_block() {
        let mut b = session("a", &wobble(10.0, 6000));
        let mut a = session("b", &wobble(9.0, 6600));
        b.conditions = Conditions::default();
        a.conditions = Conditions::default();
        let r = compare(&b, &a);
        assert!(r.comparability.controlled);
        assert!(!r.comparability.notes.is_empty());
    }

    #[test]
    fn comparisons_never_claim_improvement_across_games_or_unknown_appids() {
        let b = session("synthetic-a", &wobble(20.0, 3000));
        let a = session("synthetic-b", &wobble(10.0, 6000));
        for app_id in [Some("480"), None, Some("invalid")] {
            let mut other = a.clone();
            other.app_id = app_id.map(str::to_owned);
            let result = compare(&b, &other);
            assert_eq!(result.verdict.kind, VerdictKind::Inconclusive);
            assert!(!result.comparability.controlled);
            assert!(result.rows.iter().any(|r| r.key == "fpsAvg"));
        }
        let mut unknown_before = b;
        let mut unknown_after = a;
        unknown_before.app_id = None;
        unknown_after.app_id = None;
        assert_eq!(
            compare(&unknown_before, &unknown_after).verdict.kind,
            VerdictKind::Inconclusive
        );
    }

    #[test]
    fn equivalent_gpu_formatting_does_not_hide_hardware_or_driver_changes() {
        assert!(same_condition(
            "gpu",
            "AD107 [GeForce RTX 4060]",
            "NVIDIA GeForce RTX 4060/PCIe/SSE2"
        ));
        assert!(same_condition(
            "gpu",
            "NAVI32 [Radeon RX 7700 XT]",
            "AMD Radeon RX 7700 XT"
        ));
        assert!(same_condition("driver", "nvidia 580.126.09", "580.126.09"));
        for (field, before, after) in [
            (
                "gpu",
                "NVIDIA GeForce RTX 4060",
                "NVIDIA GeForce RTX 4060 Ti",
            ),
            (
                "gpu",
                "NVIDIA GeForce RTX 4060",
                "NVIDIA GeForce RTX 4060 Laptop GPU",
            ),
            ("gpu", "AMD Radeon RX 7700 XT", "AMD Radeon RX 7800 XT"),
            ("driver", "nvidia 580.126.09", "580.126.10"),
            ("driver", "nouveau 580.126.09", "580.126.09"),
            ("driver", "Mesa 25.1", "nvidia 25.1"),
        ] {
            assert!(
                !same_condition(field, before, after),
                "{field}: {before} / {after}"
            );
        }
        let mut before = session("synthetic-before", &wobble(20.0, 3000));
        let mut after = session("synthetic-after", &wobble(10.0, 6000));
        before.conditions.gpu = Some("AD107 [GeForce RTX 4060]".into());
        after.conditions.gpu = Some("NVIDIA GeForce RTX 4060/PCIe/SSE2".into());
        assert!(comparability(&before, &after).controlled);
        before.conditions.game_build_id = Some("10001".into());
        after.conditions.game_build_id = Some("10002".into());
        assert!(!comparability(&before, &after).controlled);
        assert!(comparability(&before, &after)
            .differences
            .iter()
            .any(|d| d.field == "gameBuildId" && d.major));
    }

    #[test]
    fn percentage_is_safe() {
        assert_eq!(pct(0.0, 10.0), None);
        assert_eq!(pct(50.0, 112.0).map(|p| p.round()), Some(124.0));
        assert_eq!(pct(100.0, 87.0).map(|p| p.round()), Some(-13.0));
    }
}
