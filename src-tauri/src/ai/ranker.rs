//! Ranker determinístico: compara o texto de cada candidato com o sistema do
//! usuário. Não diagnostica, não infere causa e não promete desempenho. O score
//! mede pertinência ao sistema, não se a correção funciona.
//!
//! As funções recebem texto já sanitizado, e o relógio vem de fora: a mesma
//! entrada dá sempre o mesmo resultado.

use super::driver::{self, DriverInfo};
use super::embedder::{cosine, EmbedError, Embedder};
use super::models::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Weights {
    pub gpu_match: f32,
    pub driver_proximity: f32,
    pub proton_match: f32,
    pub symptom_match: f32,
    pub source_trust: f32,
    pub recency: f32,
    pub penalty_stale: f32,
    pub penalty_other_gpu: f32,
}

/// Pesos iniciais, a ajustar com dados.
pub const WEIGHTS: Weights = Weights {
    gpu_match: 0.25,
    driver_proximity: 0.15,
    proton_match: 0.10,
    symptom_match: 0.20,
    source_trust: 0.15,
    recency: 0.10,
    penalty_stale: 0.20,
    penalty_other_gpu: 0.30,
};

pub const RECENCY_HALF_LIFE_DAYS: f32 = 365.0;
/// Mais de tantos majors atrás torna a menção obsoleta.
const STALE_MAJORS: u32 = 2;

fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || c == '.' || c == '-' || c == '_'))
        .map(|w| w.trim_matches(['.', '-', '_']).to_owned())
        .filter(|w| !w.is_empty())
        .collect()
}

// ---- GPU ----

#[derive(Debug, Clone, PartialEq)]
struct GpuMention {
    vendor: GpuVendor,
    /// Dígitos ou código do modelo, como "4070", "970" ou "a770".
    model: Option<String>,
}

fn leading_digits(w: &str) -> Option<String> {
    let d: String = w.chars().take_while(char::is_ascii_digit).collect();
    (d.len() >= 3 && d.len() <= 4).then_some(d)
}

/// Menções de hardware por palavras explícitas. "rx" e "arc" são palavras comuns
/// demais sozinhas, então só contam com um número de modelo em seguida.
fn gpu_mentions(text: &str) -> Vec<GpuMention> {
    let w = words(text);
    let mut out = Vec::new();
    let push = |m: GpuMention, out: &mut Vec<GpuMention>| {
        if !out.contains(&m) {
            out.push(m);
        }
    };
    for (i, word) in w.iter().enumerate() {
        let next = w.get(i + 1).map(String::as_str).unwrap_or("");
        match word.as_str() {
            "rtx" | "gtx" | "gt" => {
                if let Some(model) = leading_digits(next) {
                    push(
                        GpuMention {
                            vendor: GpuVendor::Nvidia,
                            model: Some(model),
                        },
                        &mut out,
                    );
                } else if word != "gt" {
                    push(
                        GpuMention {
                            vendor: GpuVendor::Nvidia,
                            model: None,
                        },
                        &mut out,
                    );
                }
            }
            "nvidia" | "geforce" => push(
                GpuMention {
                    vendor: GpuVendor::Nvidia,
                    model: None,
                },
                &mut out,
            ),
            "rx" => {
                if let Some(model) = leading_digits(next) {
                    push(
                        GpuMention {
                            vendor: GpuVendor::Amd,
                            model: Some(model),
                        },
                        &mut out,
                    );
                }
            }
            "amd" | "radeon" | "radv" | "amdgpu" => push(
                GpuMention {
                    vendor: GpuVendor::Amd,
                    model: None,
                },
                &mut out,
            ),
            "arc" => {
                if next.len() == 4 && next.starts_with(['a', 'b']) {
                    push(
                        GpuMention {
                            vendor: GpuVendor::Intel,
                            model: Some(next.to_owned()),
                        },
                        &mut out,
                    );
                }
            }
            "intel" | "iris" => push(
                GpuMention {
                    vendor: GpuVendor::Intel,
                    model: None,
                },
                &mut out,
            ),
            w if w.starts_with("rtx") || w.starts_with("gtx") => {
                if let Some(model) = leading_digits(&w[3..]) {
                    push(
                        GpuMention {
                            vendor: GpuVendor::Nvidia,
                            model: Some(model),
                        },
                        &mut out,
                    );
                }
            }
            _ => {}
        }
    }
    // Um modelo já diz o fabricante: a menção só de fabricante vira redundante.
    let with_model: Vec<GpuVendor> = out
        .iter()
        .filter(|m| m.model.is_some())
        .map(|m| m.vendor)
        .collect();
    out.retain(|m| m.model.is_some() || !with_model.contains(&m.vendor));
    out
}

/// Geração do modelo: "40" em RTX 4070, "9" em GTX 970, "7" em RX 7800, "a" em Arc A770.
fn generation(vendor: GpuVendor, model: &str) -> String {
    match (vendor, model.len()) {
        (GpuVendor::Nvidia, 4) => model[..2].to_owned(),
        _ => model[..1].to_owned(),
    }
}

/// 1.0 mesma GPU ou série; 0.5 mesmo fabricante; 0.0 não citada ou outro fabricante.
pub fn gpu_match(system: &SystemSnapshot, text: &str) -> f32 {
    let Some(gpu) = system.gpu() else {
        return 0.0;
    };
    let same_vendor: Vec<_> = gpu_mentions(text)
        .into_iter()
        .filter(|m| m.vendor == gpu.vendor)
        .collect();
    if same_vendor.is_empty() {
        return 0.0;
    }
    let own_model = gpu.name.as_deref().and_then(|n| {
        gpu_mentions(n)
            .into_iter()
            .find(|m| m.vendor == gpu.vendor)
            .and_then(|m| m.model)
    });
    if let Some(own) = own_model {
        let series = generation(gpu.vendor, &own);
        let same = same_vendor
            .iter()
            .filter_map(|m| m.model.as_deref())
            .any(|m| generation(gpu.vendor, m) == series);
        if same {
            return 1.0;
        }
    }
    0.5
}

const WORKS_PHRASES: &[&str] = &[
    "works on",
    "work on",
    "working on",
    "works with",
    "works fine on",
    "works great on",
    "works perfectly on",
    "runs on",
    "runs fine on",
    "fine on",
    "fixed on",
    "only on",
    "funciona em",
    "funciona na",
    "funciona no",
    "funciona com",
    "roda em",
    "roda na",
    "roda no",
];
const NEGATIONS: &[&str] = &[
    "not ", "n't ", "doesnt ", "dont ", "never ", "não ", "nao ", "nem ",
];

/// 1.0 quando o texto diz que funciona em outra GPU e essa é a única menção de
/// hardware. Negação e menção à GPU do usuário no mesmo texto não contam.
pub fn penalty_other_gpu(system: &SystemSnapshot, text: &str) -> f32 {
    let Some(gpu) = system.gpu() else {
        return 0.0;
    };
    let mentions = gpu_mentions(text);
    if mentions.is_empty() || mentions.iter().any(|m| m.vendor == gpu.vendor) {
        return 0.0;
    }
    let low = text.to_lowercase();
    for sentence in low.split(['.', '!', '?', '\n']) {
        if gpu_mentions(sentence).is_empty() {
            continue;
        }
        let affirmed = WORKS_PHRASES.iter().any(|p| {
            sentence.find(p).is_some_and(|i| {
                let before = &sentence[..i];
                !NEGATIONS.iter().any(|n| before.ends_with(n))
            })
        });
        if affirmed {
            return 1.0;
        }
    }
    0.0
}

// ---- Versões ----

fn first_number(text: &str) -> Option<u32> {
    let d: String = text.chars().take_while(char::is_ascii_digit).collect();
    d.parse().ok()
}

/// A melhor proximidade entre o driver do usuário e as versões citadas. `None`
/// quando nenhuma citação é comparável: sem bônus e sem penalidade.
pub fn driver_proximity(own: &DriverInfo, text: &str) -> Option<f32> {
    driver::mentions(text)
        .iter()
        .filter_map(|m| driver::proximity(own, m))
        .reduce(f32::max)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProtonFamily {
    Valve,
    Ge,
    Experimental,
}

/// Versões do Proton citadas, por família.
fn proton_versions(text: &str) -> Vec<(ProtonFamily, Option<u32>)> {
    let w = words(text);
    let mut out = Vec::new();
    for (i, word) in w.iter().enumerate() {
        if let Some(rest) = word
            .strip_prefix("ge-proton")
            .or_else(|| word.strip_prefix("proton-ge-"))
            .or_else(|| word.strip_prefix("protonge"))
        {
            out.push((ProtonFamily::Ge, first_number(rest.trim_start_matches('-'))));
        } else if word == "proton" {
            let next = w.get(i + 1).map(String::as_str).unwrap_or("");
            if next == "experimental" {
                out.push((ProtonFamily::Experimental, None));
            } else if next == "ge" {
                let after = w.get(i + 2).map(String::as_str).unwrap_or("");
                out.push((ProtonFamily::Ge, first_number(after)));
            } else if let Some(major) = first_number(next) {
                out.push((ProtonFamily::Valve, Some(major)));
            }
        } else if let Some(rest) = word.strip_prefix("proton-") {
            if rest == "experimental" {
                out.push((ProtonFamily::Experimental, None));
            } else if let Some(major) = first_number(rest) {
                out.push((ProtonFamily::Valve, Some(major)));
            }
        }
    }
    out
}

fn own_proton(game: &GameInfo) -> Option<(ProtonFamily, Option<u32>)> {
    proton_versions(game.proton_tool.as_deref()?)
        .into_iter()
        .next()
}

/// 1.0 mesmo major; 0.5 mesma família GE; 0.0 outro ou não citado.
pub fn proton_match(game: &GameInfo, text: &str) -> f32 {
    let Some((family, major)) = own_proton(game) else {
        return 0.0;
    };
    let mentions = proton_versions(text);
    let same_major = mentions.iter().any(|(f, m)| {
        if family == ProtonFamily::Experimental || *f == ProtonFamily::Experimental {
            *f == family
        } else {
            m.is_some() && *m == major
        }
    });
    if same_major {
        return 1.0;
    }
    if family == ProtonFamily::Ge && mentions.iter().any(|(f, _)| *f == ProtonFamily::Ge) {
        return 0.5;
    }
    0.0
}

/// 1.0 quando a versão mais nova citada, de Proton ou driver, está mais de dois
/// majors atrás da do usuário. Um texto que também cita a versão atual não é velho.
pub fn penalty_stale(own: &DriverInfo, game: &GameInfo, text: &str) -> f32 {
    let behind = |own: u32, newest: Option<u32>| newest.is_some_and(|n| own > n + STALE_MAJORS);
    if let Some((_, Some(own))) = own_proton(game) {
        let newest = proton_versions(text).iter().filter_map(|(_, m)| *m).max();
        if behind(own, newest) {
            return 1.0;
        }
    }
    // Cada citação comparável mede a distância; uma citação atual ou mais nova zera.
    let comparable: Vec<_> = driver::mentions(text)
        .into_iter()
        .filter(|m| driver::proximity(own, m).is_some())
        .collect();
    let stale = !comparable.is_empty()
        && comparable
            .iter()
            .all(|m| driver::behind(own, m).is_some_and(|d| d > STALE_MAJORS));
    if stale {
        return 1.0;
    }
    0.0
}

// ---- Fonte e data ----

pub fn source_trust(source: SourceKind) -> f32 {
    match source {
        SourceKind::Umu => 1.0,
        SourceKind::ReleaseNotes => 0.9,
        SourceKind::GithubComment => 0.7,
        SourceKind::Reddit => 0.5,
        SourceKind::Blog => 0.3,
    }
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Dia de `AAAA-MM-DD...`, em dias desde 1970. Formato inválido dá `None`.
fn parse_day(iso: &str) -> Option<i64> {
    let (y, m, d) = (iso.get(0..4)?, iso.get(5..7)?, iso.get(8..10)?);
    let (y, m, d): (i64, i64, i64) = (y.parse().ok()?, m.parse().ok()?, d.parse().ok()?);
    ((1..=12).contains(&m) && (1..=31).contains(&d)).then(|| days_from_civil(y, m, d))
}

/// Decaimento exponencial com meia-vida de 12 meses. Sem data, zero.
pub fn recency(published_at: Option<&str>, now_unix: u64) -> f32 {
    let Some(day) = published_at.and_then(parse_day) else {
        return 0.0;
    };
    let age = ((now_unix / 86_400) as i64 - day).max(0) as f32;
    0.5_f32.powf(age / RECENCY_HALF_LIFE_DAYS)
}

// ---- Score ----

/// Soma dos pesos positivos: um candidato perfeito e sem penalidade vale 1.0.
pub fn max_raw(w: &Weights) -> f32 {
    w.gpu_match + w.driver_proximity + w.proton_match + w.symptom_match + w.source_trust + w.recency
}

/// Soma ponderada, normalizada pelo máximo e limitada a [0, 1]. Sem informação de
/// driver, o peso sai da soma e do máximo: a feature não conta nem a favor nem contra.
pub fn score_of(f: &ScoreFeatures, w: &Weights) -> f32 {
    let max = match f.driver_proximity {
        Some(_) => max_raw(w),
        None => max_raw(w) - w.driver_proximity,
    };
    let raw = w.gpu_match * f.gpu_match
        + w.driver_proximity * f.driver_proximity.unwrap_or(0.0)
        + w.proton_match * f.proton_match
        + w.symptom_match * f.symptom_match
        + w.source_trust * f.source_trust
        + w.recency * f.recency
        - w.penalty_stale * f.penalty_stale
        - w.penalty_other_gpu * f.penalty_other_gpu;
    (raw / max).clamp(0.0, 1.0)
}

/// Confiança pelo tipo da fonte, sem olhar o score. `match` sem curinga: uma fonte
/// nova precisa decidir aqui. `Verified` exige declaração explícita de mantenedor,
/// que nenhum provedor expõe hoje, então nenhuma fonte recebe esse valor ainda.
pub fn confidence_of(source: SourceKind) -> ConfidenceLabel {
    match source {
        SourceKind::Umu | SourceKind::ReleaseNotes => ConfidenceLabel::Curated,
        SourceKind::GithubComment => ConfidenceLabel::CommunityReport,
        SourceKind::Reddit | SourceKind::Blog => ConfidenceLabel::NotVerified,
    }
}

/// Ranqueia do mais para o menos pertinente. Candidato de origem fora da
/// allowlist é descartado. Empates saem na ordem da URL, sem depender da entrada.
pub fn rank(
    query: &ResearchQuery,
    candidates: &[Candidate],
    embedder: &dyn Embedder,
    now_unix: u64,
) -> Result<Vec<ScoredCandidate>, EmbedError> {
    let clean: Vec<Candidate> = candidates.iter().filter_map(Candidate::sanitized).collect();
    let own_driver = driver::parse(&query.system);
    let symptom = query.clean_symptom();
    let mut texts: Vec<String> = vec![symptom];
    texts.extend(clean.iter().map(|c| format!("{}\n{}", c.title, c.snippet)));
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let vectors = embedder.embed(&refs)?;
    if vectors.len() != refs.len() {
        return Err(EmbedError::Invalid("quantidade de vetores".into()));
    }
    let mut out: Vec<(ScoredCandidate, u32)> = clean
        .into_iter()
        .zip(vectors.iter().skip(1))
        .map(|(candidate, vector)| {
            let text = format!(
                "{}\n{}\n{}",
                candidate.title, candidate.snippet, candidate.raw_text
            );
            let features = ScoreFeatures {
                gpu_match: gpu_match(&query.system, &text),
                driver_proximity: driver_proximity(&own_driver, &text),
                proton_match: proton_match(&query.game, &text),
                symptom_match: cosine(&vectors[0], vector).clamp(0.0, 1.0),
                source_trust: source_trust(candidate.source),
                recency: recency(candidate.published_at.as_deref(), now_unix),
                penalty_stale: penalty_stale(&own_driver, &query.game, &text),
                penalty_other_gpu: penalty_other_gpu(&query.system, &text),
            };
            // O minor do Mesa só desempata: fica fora do score.
            let gap = driver::mentions(&text)
                .iter()
                .filter_map(|m| driver::minor_gap(&own_driver, m))
                .min()
                .unwrap_or(u32::MAX);
            let scored = ScoredCandidate {
                confidence: confidence_of(candidate.source),
                relevance: score_of(&features, &WEIGHTS),
                features,
                candidate,
                symptom_method: embedder.id().to_owned(),
            };
            (scored, gap)
        })
        .collect();
    out.sort_by(|(a, gap_a), (b, gap_b)| {
        b.relevance
            .total_cmp(&a.relevance)
            .then_with(|| gap_a.cmp(gap_b))
            .then_with(|| a.candidate.url.cmp(&b.candidate.url))
    });
    Ok(out.into_iter().map(|(scored, _)| scored).collect())
}

#[cfg(test)]
mod ai_ranker_unit {
    use super::*;

    fn gpu(vendor: GpuVendor, name: &str, driver: Option<&str>) -> GpuInfo {
        GpuInfo {
            pci_address: None,
            gpu_family: None,
            vram_bytes: None,
            vendor,
            vendor_id: String::new(),
            device_id: String::new(),
            name: Some(name.into()),
            driver: None,
            driver_version: driver.map(str::to_owned),
            boot_vga: true,
        }
    }

    fn rtx() -> SystemSnapshot {
        let g = gpu(
            GpuVendor::Nvidia,
            "NVIDIA GeForce RTX 4070 SUPER",
            Some("580.82.07"),
        );
        SystemSnapshot {
            gpus: vec![g.clone()],
            primary_gpu: Some(g),
            vulkan_drivers: vec![],
            mesa_version: None,
        }
    }

    fn radeon() -> SystemSnapshot {
        // Como no sistema real: a versão do módulo do kernel não é a do Mesa.
        let mut g = gpu(GpuVendor::Amd, "AMD Radeon RX 7800 XT", Some("6.10.4"));
        g.driver = Some("amdgpu".into());
        SystemSnapshot {
            gpus: vec![g.clone()],
            primary_gpu: Some(g),
            vulkan_drivers: vec![],
            mesa_version: Some("25.3.1".into()),
        }
    }

    fn proton(tool: &str) -> GameInfo {
        GameInfo {
            name: "Jogo".into(),
            proton_tool: Some(tool.into()),
        }
    }

    #[test]
    fn gpu_match_same_series_same_vendor_other_and_absent() {
        let s = rtx();
        assert_eq!(gpu_match(&s, "Tested on an RTX 4070"), 1.0);
        assert_eq!(gpu_match(&s, "Tested on an RTX 4080"), 1.0, "mesma série");
        assert_eq!(gpu_match(&s, "Tested on my RTX4070 Ti"), 1.0);
        assert_eq!(gpu_match(&s, "Tested on an RTX 3060"), 0.5);
        assert_eq!(gpu_match(&s, "Happens with NVIDIA drivers"), 0.5);
        assert_eq!(gpu_match(&s, "Tested on an RX 7800 XT"), 0.0);
        assert_eq!(gpu_match(&s, "Tested yesterday"), 0.0);
    }

    #[test]
    fn gpu_match_without_a_known_gpu_is_zero() {
        assert_eq!(gpu_match(&SystemSnapshot::default(), "RTX 4070"), 0.0);
        let g = gpu(GpuVendor::Nvidia, "x", None);
        let two = SystemSnapshot {
            gpus: vec![g.clone(), g],
            ..Default::default()
        };
        assert_eq!(gpu_match(&two, "RTX 4070"), 0.0, "sem primária definida");
    }

    #[test]
    fn common_words_are_not_gpus() {
        assert!(gpu_mentions("The arc of the story, rx is short").is_empty());
        assert_eq!(
            gpu_mentions("Intel Arc A770")[0].model.as_deref(),
            Some("a770")
        );
    }

    #[test]
    fn critical_radeon_report_against_an_rtx_profile_is_penalized() {
        let s = rtx();
        assert_eq!(
            penalty_other_gpu(&s, "It works on my Radeon RX 7800 XT."),
            1.0
        );
        assert_eq!(penalty_other_gpu(&s, "Funciona na minha Radeon."), 1.0);
    }

    #[test]
    fn other_gpu_penalty_needs_an_affirmation_and_being_the_only_mention() {
        let s = rtx();
        assert_eq!(
            penalty_other_gpu(&s, "It does not work on Radeon."),
            0.0,
            "negação"
        );
        assert_eq!(
            penalty_other_gpu(&s, "Works on Radeon and on RTX 4070."),
            0.0,
            "cita a GPU do usuário"
        );
        assert_eq!(
            penalty_other_gpu(&s, "I have a Radeon RX 7800."),
            0.0,
            "sem afirmação"
        );
        assert_eq!(
            penalty_other_gpu(&s, "Works on my machine."),
            0.0,
            "sem hardware"
        );
        assert_eq!(
            penalty_other_gpu(&radeon(), "Works on my Radeon."),
            0.0,
            "mesma GPU"
        );
        assert_eq!(
            penalty_other_gpu(&SystemSnapshot::default(), "Works on Radeon."),
            0.0
        );
    }

    #[test]
    fn driver_proximity_uses_families_and_none_means_no_information() {
        let own = driver::parse(&rtx());
        assert_eq!(driver_proximity(&own, "on driver 580.65.06"), Some(1.0));
        assert_eq!(driver_proximity(&own, "on driver 575.64"), Some(0.5));
        assert_eq!(driver_proximity(&own, "on driver 565.57"), Some(0.2));
        assert_eq!(driver_proximity(&own, "on driver 535.54.03"), Some(0.0));
        assert_eq!(driver_proximity(&own, "no driver cited"), None);
        assert_eq!(
            driver_proximity(&own, "with Mesa 25.1.7"),
            None,
            "outro vendor"
        );
        let amd = driver::parse(&radeon());
        assert_eq!(driver_proximity(&amd, "with Mesa 25.1.7"), Some(1.0));
        assert_eq!(driver_proximity(&amd, "with Mesa 24.1"), Some(0.5));
        assert_eq!(
            driver_proximity(&amd, "driver 580.65"),
            None,
            "outro vendor"
        );
        let unknown = driver::parse(&SystemSnapshot::default());
        assert_eq!(driver_proximity(&unknown, "driver 580.65"), None);
    }

    #[test]
    fn the_best_of_several_mentions_counts() {
        let own = driver::parse(&rtx());
        assert_eq!(
            driver_proximity(&own, "driver 535.54 and later driver 580.65"),
            Some(1.0)
        );
    }

    #[test]
    fn proton_match_major_family_and_other() {
        let ge = proton("GE-Proton10-34");
        assert_eq!(proton_match(&ge, "Using GE-Proton10-4"), 1.0);
        assert_eq!(proton_match(&ge, "On Proton 10.0-3"), 1.0, "mesmo major");
        assert_eq!(proton_match(&ge, "Using GE-Proton9-20"), 0.5, "família GE");
        assert_eq!(proton_match(&ge, "On Proton 8.0-5"), 0.0);
        assert_eq!(proton_match(&ge, "no version"), 0.0);
        let valve = proton("Proton 9.0-4");
        assert_eq!(proton_match(&valve, "Proton 9.0"), 1.0);
        assert_eq!(proton_match(&valve, "Using GE-Proton8-1"), 0.0);
        assert_eq!(
            proton_match(&proton("Proton Experimental"), "Proton Experimental"),
            1.0
        );
        assert_eq!(proton_match(&GameInfo::default(), "Proton 9.0"), 0.0);
    }

    #[test]
    fn stale_penalty_uses_the_newest_mention() {
        let none = driver::parse(&SystemSnapshot::default());
        let g = proton("Proton 10.0-3");
        assert_eq!(penalty_stale(&none, &g, "Worked on Proton 7.0"), 1.0);
        assert_eq!(
            penalty_stale(&none, &g, "Worked on Proton 8.0"),
            0.0,
            "exatamente dois"
        );
        assert_eq!(
            penalty_stale(&none, &g, "Broke on Proton 5.0, works on Proton 10.0"),
            0.0
        );
        assert_eq!(penalty_stale(&none, &g, "no version"), 0.0);
    }

    #[test]
    fn stale_driver_is_measured_in_families_not_raw_numbers() {
        let own = driver::parse(&rtx());
        let g = GameInfo::default();
        assert_eq!(
            penalty_stale(&own, &g, "driver 575.10"),
            0.0,
            "famílias vizinhas"
        );
        assert_eq!(
            penalty_stale(&own, &g, "driver 560.10"),
            0.0,
            "duas famílias"
        );
        assert_eq!(penalty_stale(&own, &g, "driver 550.10"), 1.0);
        assert_eq!(
            penalty_stale(&own, &g, "driver 535.10, later driver 580.10"),
            0.0
        );
        assert_eq!(penalty_stale(&own, &g, "Mesa 20.0"), 0.0, "outro vendor");
        let amd = driver::parse(&radeon());
        assert_eq!(penalty_stale(&amd, &g, "Mesa 22.3"), 1.0);
        assert_eq!(penalty_stale(&amd, &g, "Mesa 23.3"), 0.0);
    }

    #[test]
    fn source_trust_table() {
        assert_eq!(source_trust(SourceKind::Umu), 1.0);
        assert_eq!(source_trust(SourceKind::ReleaseNotes), 0.9);
        assert_eq!(source_trust(SourceKind::GithubComment), 0.7);
        assert_eq!(source_trust(SourceKind::Reddit), 0.5);
        assert_eq!(source_trust(SourceKind::Blog), 0.3);
    }

    #[test]
    fn recency_halves_each_year_and_handles_bad_dates() {
        let now = days_from_civil(2026, 11, 1) as u64 * 86_400;
        assert_eq!(recency(Some("2026-11-01T00:00:00Z"), now), 1.0);
        let one_year = recency(Some("2025-11-01T00:00:00Z"), now);
        assert!((one_year - 0.5).abs() < 0.01, "{one_year}");
        assert_eq!(recency(Some("2030-01-01"), now), 1.0, "data futura");
        assert_eq!(recency(None, now), 0.0);
        assert_eq!(recency(Some("ontem"), now), 0.0);
    }

    #[test]
    fn days_from_civil_matches_the_project_clock() {
        for secs in [0u64, 86_400 * 19_000, 1_793_472_811] {
            let iso = crate::time::iso8601(secs);
            assert_eq!(parse_day(&iso), Some((secs / 86_400) as i64), "{iso}");
        }
    }

    #[test]
    fn score_is_normalized_and_clamped() {
        let w = &WEIGHTS;
        assert!((max_raw(w) - 0.95).abs() < 1e-6);
        let full = ScoreFeatures {
            gpu_match: 1.0,
            driver_proximity: Some(1.0),
            proton_match: 1.0,
            symptom_match: 1.0,
            source_trust: 1.0,
            recency: 1.0,
            ..Default::default()
        };
        assert!((score_of(&full, w) - 1.0).abs() < 1e-6);
        let none = ScoreFeatures {
            penalty_stale: 1.0,
            penalty_other_gpu: 1.0,
            ..Default::default()
        };
        assert_eq!(score_of(&none, w), 0.0);
        let half = ScoreFeatures {
            source_trust: 1.0,
            driver_proximity: Some(0.0),
            ..Default::default()
        };
        assert!((score_of(&half, w) - 0.15 / 0.95).abs() < 1e-6);
    }

    #[test]
    fn none_is_left_out_while_zero_counts_against() {
        let base = ScoreFeatures {
            gpu_match: 1.0,
            source_trust: 1.0,
            ..Default::default()
        };
        let none = score_of(&base, &WEIGHTS);
        let zero = score_of(
            &ScoreFeatures {
                driver_proximity: Some(0.0),
                ..base
            },
            &WEIGHTS,
        );
        let one = score_of(
            &ScoreFeatures {
                driver_proximity: Some(1.0),
                ..base
            },
            &WEIGHTS,
        );
        assert!((none - 0.40 / 0.80).abs() < 1e-6, "{none}");
        assert!((zero - 0.40 / 0.95).abs() < 1e-6, "{zero}");
        assert!(zero < none && none < one);
    }

    #[test]
    fn each_penalty_lowers_the_score_by_its_weight() {
        let base = ScoreFeatures {
            gpu_match: 1.0,
            source_trust: 1.0,
            symptom_match: 1.0,
            recency: 1.0,
            ..Default::default()
        };
        let plain = score_of(&base, &WEIGHTS);
        let denominator = 0.80;
        let stale = ScoreFeatures {
            penalty_stale: 1.0,
            ..base
        };
        let other = ScoreFeatures {
            penalty_other_gpu: 1.0,
            ..base
        };
        assert!((plain - score_of(&stale, &WEIGHTS) - 0.20 / denominator).abs() < 1e-6);
        assert!((plain - score_of(&other, &WEIGHTS) - 0.30 / denominator).abs() < 1e-6);
    }

    #[test]
    fn confidence_comes_only_from_the_source() {
        use ConfidenceLabel::*;
        assert_eq!(confidence_of(SourceKind::Umu), Curated);
        assert_eq!(confidence_of(SourceKind::ReleaseNotes), Curated);
        assert_eq!(confidence_of(SourceKind::GithubComment), CommunityReport);
        assert_eq!(confidence_of(SourceKind::Reddit), NotVerified);
        assert_eq!(confidence_of(SourceKind::Blog), NotVerified);
    }

    #[test]
    fn no_source_is_verified_until_a_provider_exposes_a_maintainer_statement() {
        for source in [
            SourceKind::Umu,
            SourceKind::ReleaseNotes,
            SourceKind::GithubComment,
            SourceKind::Reddit,
            SourceKind::Blog,
        ] {
            assert_ne!(confidence_of(source), ConfidenceLabel::Verified);
        }
    }
}
