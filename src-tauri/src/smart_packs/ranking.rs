//! Ranking Engine — ordenação explicável, por perfil, sem pontuação opaca.
//!
//! Três grupos, sempre separados:
//!
//! - **Ranking medido**: pacotes com resultado *conclusivo* contra o baseline
//!   (≥ 2 sessões comparáveis de cada lado). Ordem lexicográfica por classe
//!   do resultado e, dentro da classe, pela diferença da métrica principal.
//!   Diferenças entre pacotes dentro do limiar viram **empate técnico** (mesma
//!   posição).
//! - **Candidatos para testar**: seguros e compatíveis, sem dado conclusivo.
//!   A ordem é de *teste sugerido*, não de desempenho.
//! - **Não recomendados**: regressão conclusiva, incompatíveis, contestados
//!   ou que precisam de validação.
//!
//! Ausência de dado nunca conta como bom nem como ruim: só tira o pacote do
//! ranking medido. Um vencedor só existe quando o primeiro lugar melhorou a
//! métrica principal do perfil além da variação e não divide a posição.

use std::collections::{BTreeMap, HashMap};

use super::models::*;
use crate::fix_engine::dynamic::EvidenceLevel;

pub struct Input<'a> {
    pub profile: Profile,
    pub packs: &'a [SmartPack],
    pub baseline_id: &'a str,
    pub measured: &'a BTreeMap<String, Measurements>,
    pub vs: &'a HashMap<String, VsBaseline>,
    pub excluded: &'a [Excluded],
}

pub fn criteria(p: Profile) -> Vec<String> {
    let common = "Só entram no ranking medido pacotes com ≥ 2 sessões comparáveis de cada lado (pacote e baseline). Diferenças dentro do limiar contam como “sem diferença”.";
    let v: &[&str] = match p {
        Profile::Performance => &[
            "Métrica principal: FPS médio (mediana das sessões).",
            "Regressão no FPS médio → não recomendado.",
            "FPS médio igual e 1% low pior → não recomendado (estabilidade não é ignorada).",
            "FPS médio maior com 1% low menor → troca (sinal misto): fica abaixo das melhoras limpas e não vira vencedor.",
        ],
        Profile::Balanced => &[
            "Nenhuma regressão em FPS médio, 1% low ou frametime P99; qualquer uma → não recomendado.",
            "Ordem: melhoras em 1% low, depois frametime P99/picos, depois FPS médio.",
        ],
        Profile::Compatibility => &[
            "Não piorar a estabilidade medida (1% low, frametime P99, picos); FPS médio menor é aceito e informado.",
            "Ordem: evidência documental das receitas, risco, depois número de sessões que rodaram até o fim.",
            "Vencedor só quando as receitas têm evidência documentada ou confirmada; uma captura completa prova que o jogo rodou, não que o problema relatado sumiu.",
        ],
    };
    let mut out: Vec<String> = v.iter().map(|s| s.to_string()).collect();
    out.push(common.into());
    out
}

fn o(vs: &VsBaseline, m: &str) -> Outcome {
    vs.get(m).map(|d| d.outcome).unwrap_or(Outcome::NotMeasured)
}

fn fmt_pct(x: f64) -> String {
    format!("{}{:.1}%", if x >= 0.0 { "+" } else { "−" }, x.abs()).replace('.', ",")
}

fn describe(vs: &VsBaseline) -> Vec<String> {
    vs.deltas
        .iter()
        .map(|x| {
            let what = match (x.outcome, x.limited) {
                (Outcome::Improvement, false) => "melhora",
                (Outcome::Regression, false) => "piora",
                (Outcome::Improvement, true) => "indício de melhora",
                (Outcome::Regression, true) => "indício de piora",
                _ => "dentro da variação",
            };
            format!(
                "{}: {} ({what}; limiar ±{}%){}.",
                x.label,
                fmt_pct(x.delta_pct),
                format!("{:.1}", x.threshold_pct).replace('.', ","),
                if x.limited { ", indicativo" } else { "" }
            )
        })
        .collect()
}

/// Classificação de um pacote medido conclusivamente. `Err` = não recomendado.
fn classify(p: Profile, vs: &VsBaseline) -> Result<(Outcome, Vec<String>), Vec<String>> {
    let (avg, low, p99, spikes) = (
        o(vs, "fpsAvg"),
        o(vs, "fpsLow1"),
        o(vs, "ftP99"),
        o(vs, "spikesPerMin"),
    );
    let r = |s: &str| vec![s.to_owned()];
    match p {
        Profile::Performance => match (avg, low) {
            (Outcome::Regression, _) => Err(r("FPS médio caiu além da variação medida.")),
            (Outcome::NoDifference, Outcome::Regression) => {
                Err(r("FPS médio igual e 1% low pior: mais quedas, sem ganho."))
            }
            (Outcome::Improvement, Outcome::Regression) => Ok((
                Outcome::Mixed,
                r("Troca: FPS médio maior, mas 1% low menor (mais quedas)."),
            )),
            (Outcome::Improvement, _) => Ok((
                Outcome::Improvement,
                r("FPS médio maior além da variação, sem piorar o 1% low."),
            )),
            _ => Ok((
                Outcome::NoDifference,
                r("FPS médio dentro da variação do baseline."),
            )),
        },
        Profile::Balanced => {
            if [avg, low, p99].contains(&Outcome::Regression) {
                return Err(r(
                    "Piorou FPS médio, 1% low ou frametime P99 além da variação.",
                ));
            }
            if [low, p99, spikes, avg].contains(&Outcome::Improvement) {
                Ok((
                    Outcome::Improvement,
                    r("Melhorou ao menos uma métrica sem piorar FPS médio, 1% low ou P99."),
                ))
            } else {
                Ok((
                    Outcome::NoDifference,
                    r("Todas as métricas dentro da variação do baseline."),
                ))
            }
        }
        Profile::Compatibility => {
            if [low, p99, spikes].contains(&Outcome::Regression) {
                return Err(r("Piorou a estabilidade medida (1% low, P99 ou picos)."));
            }
            let mut why = r("Rodou nas capturas sem piorar a estabilidade medida.");
            if avg == Outcome::Regression {
                why.push("O FPS médio caiu — aceito neste perfil, mas informado.".into());
            }
            Ok((Outcome::NoDifference, why))
        }
    }
}

fn class_rank(o: Outcome) -> u8 {
    match o {
        Outcome::Improvement => 0,
        Outcome::Mixed => 1,
        _ => 2,
    }
}

struct Row<'a> {
    pack: &'a SmartPack,
    outcome: Outcome,
    primary: f64,
    threshold: f64,
    reasons: Vec<String>,
    vs: Option<VsBaseline>,
}

fn primary_metric(p: Profile) -> &'static str {
    match p {
        Profile::Performance => "fpsAvg",
        Profile::Balanced => "fpsLow1",
        Profile::Compatibility => "ftP99",
    }
}

fn used(m: &BTreeMap<String, Measurements>, id: &str) -> usize {
    m.get(id).map(|x| x.used()).unwrap_or(0)
}

/// Ordem sugerida de teste: sem indício de regressão, já com dados (para
/// completar a repetição), evidência, risco, menos receitas, ID.
type TestOrder = (
    u8,
    u8,
    EvidenceLevel,
    crate::fix_engine::models::Risk,
    usize,
    String,
);

pub fn rank(i: Input<'_>) -> Ranking {
    let pm = primary_metric(i.profile);
    let base_n = used(i.measured, i.baseline_id);
    let mut measured: Vec<Row> = Vec::new();
    let mut candidates: Vec<(RankEntry, TestOrder)> = Vec::new();
    let mut not_rec: Vec<RankEntry> = Vec::new();

    for p in i.packs.iter().filter(|p| p.kind != PackKind::Baseline) {
        let vs = i.vs.get(&p.id).cloned();
        if p.state == PackState::Unsupported {
            not_rec.push(RankEntry {
                pack_id: p.id.clone(),
                title: p.name.clone(),
                tier: Tier::NotRecommended,
                position: None,
                outcome: Outcome::NotMeasured,
                headline: "Incompatível com o sistema atual".into(),
                reasons: vec![p.state_detail.clone()],
                vs_baseline: vs,
            });
            continue;
        }
        match vs
            .as_ref()
            .filter(|v| !v.limited && v.get("fpsAvg").is_some())
        {
            Some(v) => match classify(i.profile, v) {
                Ok((outcome, mut reasons)) => {
                    reasons.extend(describe(v));
                    measured.push(Row {
                        pack: p,
                        outcome,
                        primary: v
                            .get(pm)
                            .map(|x| {
                                if pm == "ftP99" {
                                    -x.delta_pct
                                } else {
                                    x.delta_pct
                                }
                            })
                            .unwrap_or(0.0),
                        threshold: v.get(pm).map(|x| x.threshold_pct).unwrap_or(0.0),
                        reasons,
                        vs: Some(v.clone()),
                    })
                }
                Err(mut reasons) => {
                    reasons.extend(describe(v));
                    not_rec.push(RankEntry {
                        pack_id: p.id.clone(),
                        title: p.name.clone(),
                        tier: Tier::NotRecommended,
                        position: None,
                        outcome: Outcome::Regression,
                        headline: "Regressão medida".into(),
                        reasons,
                        vs_baseline: Some(v.clone()),
                    });
                }
            },
            None => {
                let n = used(i.measured, &p.id);
                let indicated = vs.as_ref().and_then(|v| v.get("fpsAvg")).map(|x| x.outcome);
                let regress = vs
                    .as_ref()
                    .is_some_and(|v| v.deltas.iter().any(|x| x.outcome == Outcome::Regression));
                let (headline, mut reasons) = if n == 0 {
                    (
                        if matches!(
                            p.evidence,
                            EvidenceLevel::Documented | EvidenceLevel::Corroborated
                        ) {
                            "Candidato promissor — ainda não testado".to_owned()
                        } else {
                            "Experimental — ainda não testado".to_owned()
                        },
                        vec![format!(
                            "Sem benchmark com este pacote. Evidência: {}",
                            p.evidence_note
                        )],
                    )
                } else if base_n == 0 {
                    (
                        "Medido, mas sem baseline para comparar".to_owned(),
                        vec![format!("{n} sessão(ões) com este pacote; meça o baseline para saber se ele ajuda.")],
                    )
                } else if regress {
                    (
                        "Regressão indicativa — repita para confirmar".to_owned(),
                        vec![],
                    )
                } else {
                    (
                        match indicated {
                            Some(Outcome::Improvement) => {
                                "Melhora indicativa — repita para confirmar"
                            }
                            _ => "Resultado inconclusivo",
                        }
                        .to_owned(),
                        vec![],
                    )
                };
                if let Some(v) = &vs {
                    reasons.extend(v.notes.clone());
                    reasons.extend(describe(v));
                }
                let key = (
                    u8::from(regress),
                    u8::from(n == 0),
                    p.evidence,
                    p.risk,
                    p.members.len(),
                    p.id.clone(),
                );
                candidates.push((
                    RankEntry {
                        pack_id: p.id.clone(),
                        title: p.name.clone(),
                        tier: Tier::Candidate,
                        position: None,
                        outcome: if n == 0 {
                            Outcome::NotMeasured
                        } else {
                            Outcome::Inconclusive
                        },
                        headline,
                        reasons,
                        vs_baseline: vs,
                    },
                    key,
                ));
            }
        }
    }

    // Ordem do ranking medido.
    match i.profile {
        Profile::Compatibility => measured.sort_by(|a, b| {
            a.pack
                .evidence
                .cmp(&b.pack.evidence)
                .then(a.pack.risk.cmp(&b.pack.risk))
                .then(used(i.measured, &b.pack.id).cmp(&used(i.measured, &a.pack.id)))
                .then(a.pack.id.cmp(&b.pack.id))
        }),
        _ => measured.sort_by(|a, b| {
            class_rank(a.outcome)
                .cmp(&class_rank(b.outcome))
                .then(b.primary.total_cmp(&a.primary))
                .then(a.pack.id.cmp(&b.pack.id))
        }),
    }
    let mut entries: Vec<RankEntry> = Vec::new();
    let mut pos = 0u32;
    let mut prev: Option<(Outcome, f64, f64, EvidenceLevel)> = None;
    for r in &measured {
        let tie = prev.is_some_and(|(o, p, t, e)| match i.profile {
            Profile::Compatibility => e == r.pack.evidence,
            _ => o == r.outcome && (p - r.primary).abs() <= t.max(r.threshold),
        });
        if !tie {
            pos += 1;
        }
        prev = Some((r.outcome, r.primary, r.threshold, r.pack.evidence));
        entries.push(RankEntry {
            pack_id: r.pack.id.clone(),
            title: r.pack.name.clone(),
            tier: Tier::Measured,
            position: Some(pos),
            outcome: r.outcome,
            headline: match r.outcome {
                Outcome::Improvement => "Melhor que o baseline nas medições".into(),
                Outcome::Mixed => "Troca: mais FPS médio, mais quedas".into(),
                _ => "Sem diferença além da variação".into(),
            },
            reasons: r.reasons.clone(),
            vs_baseline: r.vs.clone(),
        });
    }
    // Empates na mesma posição ficam marcados.
    let positions: Vec<Option<u32>> = entries.iter().map(|e| e.position).collect();
    for e in &mut entries {
        if positions.iter().filter(|p| **p == e.position).count() > 1 {
            e.reasons.insert(0, "Empate técnico: a diferença para outro pacote nesta posição está dentro da variação medida.".into());
        }
    }

    let top = entries.first();
    let winner = top
        .filter(|t| {
            t.position == Some(1) && positions.iter().filter(|p| **p == Some(1)).count() == 1
        })
        .filter(|t| match i.profile {
            Profile::Compatibility => i.packs.iter().find(|p| p.id == t.pack_id).is_some_and(|p| {
                matches!(
                    p.evidence,
                    EvidenceLevel::Documented | EvidenceLevel::Corroborated
                )
            }),
            _ => t.outcome == Outcome::Improvement,
        })
        .map(|t| t.pack_id.clone());

    // Baseline como referência (posição 0 = referência, não compete).
    let baseline_entry = RankEntry {
        pack_id: i.baseline_id.to_owned(),
        title: "Baseline — sem correções do FRX".into(),
        tier: if base_n >= 2 {
            Tier::Measured
        } else {
            Tier::Candidate
        },
        position: None,
        outcome: if base_n == 0 {
            Outcome::NotMeasured
        } else {
            Outcome::NoDifference
        },
        headline: match base_n {
            0 => "Referência ainda não medida".into(),
            1 => "Referência com 1 sessão — capture mais uma".into(),
            n => format!("Referência medida ({n} sessões)"),
        },
        reasons: vec![
            "Configuração de referência: os outros pacotes são comparados com ela.".into(),
        ],
        vs_baseline: None,
    };

    candidates.sort_by(|a, b| a.1.cmp(&b.1));
    let next_test = if base_n < 2 {
        Some(NextTest {
            pack_id: i.baseline_id.to_owned(),
            reason: if base_n == 0 {
                "Meça o baseline primeiro: uma captura sem nenhuma correção do FRX. Sem ele, nenhum pacote pode ser comparado.".into()
            } else {
                "Repita o baseline (1 de 2 sessões): com uma sessão só não dá para separar diferença real da variação entre partidas.".into()
            },
        })
    } else {
        candidates.first().map(|(c, _)| NextTest {
            pack_id: c.pack_id.clone(),
            reason: if c.outcome == Outcome::NotMeasured {
                format!("Teste “{}”: aplique, capture no mesmo trecho e com as mesmas configurações do baseline.", c.title)
            } else {
                format!("Repita “{}” para chegar a 2 sessões comparáveis e confirmar o resultado.", c.title)
            },
        })
    };
    let summary = match (&winner, entries.is_empty()) {
        (Some(w), _) => format!(
            "#1 — {} para o perfil escolhido: {}.",
            match i.profile {
                Profile::Performance => "Melhor desempenho medido",
                Profile::Balanced => "Melhor equilíbrio medido",
                Profile::Compatibility => "Mais compatível pelas evidências",
            },
            i.packs
                .iter()
                .find(|p| &p.id == w)
                .map(|p| p.name.as_str())
                .unwrap_or(w)
        ),
        (None, true) => "Ainda não há dados suficientes para recomendar um vencedor.".into(),
        (None, false) => {
            if entries
                .iter()
                .any(|e| e.outcome == Outcome::Mixed && e.position == Some(1))
            {
                "Sem vencedor claro: o primeiro colocado troca FPS médio por mais quedas. Veja o perfil Equilibrado se o 1% low importa.".into()
            } else {
                "Resultado inconclusivo: nenhum pacote se separou do baseline (ou dos outros) além da variação medida.".into()
            }
        }
    };

    let mut all = vec![baseline_entry];
    all.extend(entries);
    all.extend(candidates.into_iter().map(|(e, _)| e));
    all.extend(not_rec);
    all.extend(i.excluded.iter().map(|x| RankEntry {
        pack_id: x.id.clone(),
        title: x.title.clone(),
        tier: Tier::NotRecommended,
        position: None,
        outcome: Outcome::NotMeasured,
        headline: "Fora dos pacotes".into(),
        reasons: vec![x.reason.clone()],
        vs_baseline: None,
    }));
    Ranking {
        profile: i.profile,
        version: RANKING_VERSION.into(),
        entries: all,
        winner,
        summary,
        next_test,
        criteria: criteria(i.profile),
    }
}
