//! Experimental: usado só por `research_fix`, fora do caminho crítico. Ver
//! `docs/IA-SEARCH.md`.
//!
//! Reescrita de consulta: sintoma, hardware e jogo viram uma busca por provedor.
//!
//! Só formula buscas. Não diagnostica e não executa nada. Um termo só entra numa
//! consulta se veio do input, da tabela de palavras-chave aplicada ao sintoma ou
//! de uma palavra fixa de template (`TEMPLATE_WORDS`). `origin_error` confere isso,
//! para qualquer implementação do trait.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::driver::{self, DriverInfo};
use super::models::{GameInfo, GpuVendor, ResearchQuery, SystemSnapshot};

const MAX_SYMPTOM: usize = 500;
const MAX_NAME: usize = 80;
const MAX_QUERY: usize = 200;
const MAX_LITERALS: usize = 3;
pub const TEMPLATE_WORDS: &[&str] = &["umu", "linux"];
pub const REWRITER_ENV: &str = "FRX_QUERY_REWRITER";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind {
    Umu,
    Github,
    ReleaseNotes,
    Reddit,
    Blog,
}

impl ProviderKind {
    pub const ALL: [ProviderKind; 5] = [
        ProviderKind::Umu,
        ProviderKind::Github,
        ProviderKind::ReleaseNotes,
        ProviderKind::Reddit,
        ProviderKind::Blog,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            ProviderKind::Umu => "umu",
            ProviderKind::Github => "github",
            ProviderKind::ReleaseNotes => "release-notes",
            ProviderKind::Reddit => "reddit",
            ProviderKind::Blog => "blog",
        }
    }

    fn goal(self) -> &'static str {
        match self {
            ProviderKind::Umu => "Correções registradas para este AppID",
            ProviderKind::Github => "Relatos do sintoma neste jogo e nesta GPU",
            ProviderKind::ReleaseNotes => "Mudanças desta família de Proton sobre o sintoma",
            ProviderKind::Reddit => "Relatos de usuários no Linux",
            ProviderKind::Blog => "Artigos sobre o sintoma nesta GPU",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QueryInput {
    pub symptom: String,
    pub appid: u32,
    pub game: GameInfo,
    pub system: SystemSnapshot,
}

impl From<&ResearchQuery> for QueryInput {
    fn from(q: &ResearchQuery) -> Self {
        Self {
            symptom: q.clean_symptom(),
            appid: q.appid,
            game: q.game.clone(),
            system: q.system.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QueryIntent {
    pub goal: String,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RewrittenQuery {
    pub provider: ProviderKind,
    pub query: String,
    pub intent: QueryIntent,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryError {
    /// Sem sintoma aproveitável e sem AppID: nada a buscar sem inventar.
    NoSearchTerms,
    UnknownRewriter(String),
    ModelUnavailable(String),
    Model(String),
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryError::NoSearchTerms => {
                write!(f, "Não há sintoma nem AppID para formular a busca.")
            }
            QueryError::UnknownRewriter(name) => {
                write!(f, "Reescritor de consulta desconhecido: {name}.")
            }
            QueryError::ModelUnavailable(why) | QueryError::Model(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for QueryError {}

pub trait QueryRewriter {
    fn rewrite(&self, input: &QueryInput) -> Result<Vec<RewrittenQuery>, QueryError>;
    fn id(&self) -> &'static str;
}

/// Sem escolha explícita, sempre a determinística. Modelo ausente é erro, não
/// troca silenciosa de implementação.
pub fn select_rewriter(choice: Option<&str>) -> Result<Box<dyn QueryRewriter>, QueryError> {
    match choice.map(str::trim).filter(|c| !c.is_empty()) {
        None | Some("deterministic") => Ok(Box::new(Deterministic)),
        Some("qwen") => qwen_rewriter(),
        Some(other) => Err(QueryError::UnknownRewriter(
            safe_text(other, 40).to_string(),
        )),
    }
}

#[cfg(not(feature = "qwen-query"))]
fn qwen_rewriter() -> Result<Box<dyn QueryRewriter>, QueryError> {
    Err(QueryError::ModelUnavailable(
        "Este build não inclui a Qwen: compile com a feature qwen-query.".into(),
    ))
}

#[cfg(feature = "qwen-query")]
fn qwen_rewriter() -> Result<Box<dyn QueryRewriter>, QueryError> {
    Ok(Box::new(super::qwen::QwenLocal::load()?))
}

// ---- Texto ----

fn fold(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' => 'a',
            'é' | 'ê' => 'e',
            'í' => 'i',
            'ó' | 'ô' | 'õ' => 'o',
            'ú' | 'ü' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

/// Palavras normalizadas: minúsculas, sem acento, separadas por pontuação.
pub fn tokens(text: &str) -> Vec<String> {
    fold(text)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Mantém só o que serve num nome de busca. Tira símbolos de shell e de marca.
fn safe_text(text: &str, limit: usize) -> String {
    let kept: String = text
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | ':' | '\'' | '.'))
        .take(limit)
        .collect();
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ---- Sintoma ----

/// As 12 entidades do sintoma, nesta ordem. As 9 primeiras dizem qual é o
/// problema; as últimas 3 só dizem quando ele acontece.
const KEYWORDS: &[(&str, &[&str])] = &[
    (
        "crash",
        &[
            "crash",
            "crasha",
            "crashou",
            "crashes",
            "crashing",
            "crashed",
            "crashando",
            "fecha",
            "fecham",
            "fechou",
            "fechando",
            "segfault",
        ],
    ),
    (
        "freeze",
        &[
            "freeze",
            "freezes",
            "freezing",
            "frozen",
            "congela",
            "congelou",
            "congelando",
            "trava",
            "travou",
            "travando",
            "travado",
            "hang",
            "hangs",
        ],
    ),
    (
        "stutter",
        &[
            "stutter",
            "stutters",
            "stuttering",
            "microstutter",
            "engasga",
            "engasgando",
            "engasgos",
            "frametime",
            "frame time",
        ],
    ),
    (
        "audio",
        &["audio", "som", "sound", "crackling", "chiado", "mudo"],
    ),
    (
        "controller",
        &[
            "controller",
            "controllers",
            "controle",
            "controles",
            "gamepad",
            "joystick",
            "dualsense",
        ],
    ),
    ("dlss", &["dlss"]),
    ("fsr", &["fsr"]),
    (
        "raytracing",
        &[
            "raytracing",
            "ray tracing",
            "ray-tracing",
            "traçado de raios",
        ],
    ),
    ("physx", &["physx"]),
    (
        "launch",
        &[
            "launch",
            "launching",
            "start",
            "starts",
            "startup",
            "boot",
            "abre",
            "abrir",
            "inicia",
            "iniciar",
        ],
    ),
    ("menu", &["menu", "menus"]),
    ("gameplay", &["gameplay", "partida", "jogando"]),
];
const PROBLEM_KEYWORDS: usize = 9;

const STOPWORDS: &[&str] = &[
    "a", "o", "as", "os", "um", "uma", "de", "do", "da", "dos", "das", "em", "no", "na", "nos",
    "nas", "por", "para", "com", "sem", "que", "se", "e", "ou", "ao", "aos", "apos", "depois",
    "antes", "quando", "nao", "mais", "muito", "meu", "minha", "jogo", "the", "an", "of", "in",
    "on", "at", "to", "for", "with", "and", "or", "my", "game", "after", "before", "when", "not",
    "doesn", "does", "wont", "won", "very", "it", "is", "are", "esta", "estou", "fica", "ficou",
    "tem", "tenho", "isso", "this", "that",
];

fn canonical_keywords(symptom: &str) -> Vec<&'static str> {
    let padded = format!(" {} ", tokens(symptom).join(" "));
    KEYWORDS
        .iter()
        .filter(|(_, triggers)| {
            triggers
                .iter()
                .any(|t| padded.contains(&format!(" {} ", tokens(t).join(" "))))
        })
        .map(|(canonical, _)| *canonical)
        .collect()
}

fn is_trigger(word: &str) -> bool {
    KEYWORDS
        .iter()
        .flat_map(|(_, triggers)| triggers.iter())
        .any(|t| fold(t) == word)
}

/// Palavras do próprio sintoma, para o que a tabela não cobre. Uma palavra com
/// símbolo no meio fica de fora: caminho, variável ou opção não vão para a busca.
fn literal_terms(symptom: &str, game_name: &str) -> Vec<String> {
    let game = tokens(game_name);
    let mut out: Vec<String> = Vec::new();
    for raw in symptom.split_whitespace() {
        let word = raw.trim_matches([',', '.', '!', '?', ':', ';', '(', ')', '"', '\'']);
        let folded = fold(word);
        let usable = word.chars().all(char::is_alphanumeric)
            && word.chars().count() >= 3
            && !word.starts_with(|c: char| c.is_ascii_digit())
            && !STOPWORDS.contains(&folded.as_str())
            && !game.contains(&folded)
            && !is_trigger(&folded);
        let lower = word.to_lowercase();
        if usable && !out.contains(&lower) {
            out.push(lower);
        }
    }
    out.truncate(MAX_LITERALS);
    out
}

/// Palavras-chave da tabela. Sem nenhuma que diga o problema, entram as literais.
pub fn symptom_keywords(symptom: &str, game_name: &str) -> Vec<String> {
    let canonical = canonical_keywords(symptom);
    let names_problem = canonical.iter().any(|c| {
        KEYWORDS[..PROBLEM_KEYWORDS]
            .iter()
            .any(|(name, _)| name == c)
    });
    let mut out: Vec<String> = canonical.iter().map(|c| (*c).to_owned()).collect();
    if !names_problem {
        out.extend(literal_terms(symptom, game_name));
    }
    out
}

// ---- Hardware e Proton ----

/// "NVIDIA GeForce RTX 4060 Ti" vira "RTX 4060 Ti". Modelo não reconhecido fica
/// de fora da busca.
pub fn gpu_short(name: &str) -> Option<String> {
    let words: Vec<&str> = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let digit_led = |w: &&str| w.starts_with(|c: char| c.is_ascii_digit());
    for (i, word) in words.iter().enumerate() {
        let next = words.get(i + 1).copied();
        match fold(word).as_str() {
            "rtx" | "gtx" | "gt" | "rx" | "r5" | "r7" | "r9" | "radeon" => {
                if let Some(model) = next.filter(|w| digit_led(w)) {
                    let suffix = words.get(i + 2).copied().filter(|w| {
                        matches!(fold(w).as_str(), "ti" | "super" | "xt" | "xtx" | "ultra")
                    });
                    let mut out = format!("{word} {model}");
                    if let Some(s) = suffix {
                        out.push(' ');
                        out.push_str(s);
                    }
                    return Some(out);
                }
            }
            "arc" => {
                if let Some(model) = next.filter(|w| w.chars().any(|c| c.is_ascii_digit())) {
                    return Some(format!("{word} {model}"));
                }
            }
            "uhd" | "iris" => {
                let model = words[i + 1..].iter().take(2).find(|w| digit_led(w));
                if let Some(model) = model {
                    return Some(format!("{word} {model}"));
                }
            }
            _ => {}
        }
    }
    None
}

/// "GE-Proton9-20" vira "GE-Proton9"; "Proton 9.0-4" vira "Proton 9".
pub fn proton_family(tool: &str) -> Option<String> {
    let folded = fold(tool);
    if folded.contains("experimental") && folded.contains("proton") {
        return Some("Proton Experimental".into());
    }
    let digits = |s: &str| -> Option<String> {
        let d: String = s.chars().take_while(char::is_ascii_digit).collect();
        (!d.is_empty()).then_some(d)
    };
    if let Some(rest) = folded.trim().strip_prefix("ge-proton") {
        return Some(format!("GE-Proton{}", digits(rest)?));
    }
    let rest = folded.trim().strip_prefix("proton")?;
    let rest = rest.strip_prefix([' ', '_', '-'])?;
    Some(format!("Proton {}", digits(rest)?))
}

fn vendor_word(system: &SystemSnapshot) -> Option<&'static str> {
    match system.gpu()?.vendor {
        GpuVendor::Nvidia => Some("nvidia"),
        GpuVendor::Amd => Some("amd"),
        GpuVendor::Intel => Some("intel"),
        GpuVendor::Other => None,
    }
}

fn driver_family(system: &SystemSnapshot) -> Option<String> {
    match driver::parse(system) {
        DriverInfo::Nvidia { family, .. } => Some(family.to_string()),
        DriverInfo::Mesa { major, .. } => Some(format!("mesa {major}")),
        DriverInfo::Unknown { .. } => None,
    }
}

// ---- Origem ----

/// Todo termo que uma consulta pode usar para esta entrada.
fn allowed_terms(input: &QueryInput) -> Vec<String> {
    let mut allowed = tokens(&input.symptom);
    allowed.extend(tokens(&input.game.name));
    allowed.push(input.appid.to_string());
    if let Some(tool) = &input.game.proton_tool {
        allowed.extend(tokens(tool));
    }
    if let Some(gpu) = input.system.gpu() {
        allowed.extend(tokens(gpu.name.as_deref().unwrap_or("")));
        allowed.extend(tokens(gpu.driver_version.as_deref().unwrap_or("")));
    }
    allowed.extend(tokens(input.system.mesa_version.as_deref().unwrap_or("")));
    allowed.extend(
        canonical_keywords(&input.symptom)
            .iter()
            .map(|c| (*c).to_owned()),
    );
    allowed.extend(TEMPLATE_WORDS.iter().map(|w| (*w).to_owned()));
    allowed
}

/// `Some(motivo)` quando a consulta tem símbolo proibido ou termo sem origem.
pub fn origin_error(input: &QueryInput, query: &str) -> Option<String> {
    if query.trim().is_empty() {
        return Some("consulta vazia".into());
    }
    if query.chars().count() > MAX_QUERY {
        return Some("consulta longa demais".into());
    }
    if let Some(c) = query
        .chars()
        .find(|c| c.is_control() || ";|&$`<>/\\~=\"(){}[]*!#%".contains(*c))
    {
        return Some(format!("símbolo proibido: {c:?}"));
    }
    let allowed = allowed_terms(input);
    tokens(query)
        .into_iter()
        .find(|t| !allowed.contains(t))
        .map(|t| format!("termo sem origem no input: {t}"))
}

// ---- Implementação determinística ----

#[derive(Debug, Clone, Copy, Default)]
pub struct Deterministic;

fn join(parts: &[&str]) -> String {
    parts
        .iter()
        .filter(|p| !p.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}

impl QueryRewriter for Deterministic {
    /// Uma consulta por provedor. Faltando o que o template exige, o provedor
    /// fica sem consulta, em vez de receber um termo inventado.
    fn rewrite(&self, input: &QueryInput) -> Result<Vec<RewrittenQuery>, QueryError> {
        let symptom: String = input.symptom.chars().take(MAX_SYMPTOM).collect();
        let game = safe_text(&input.game.name, MAX_NAME);
        let keywords = symptom_keywords(&symptom, &game);
        let kw = keywords.join(" ");
        let gpu = input
            .system
            .gpu()
            .and_then(|g| g.name.as_deref())
            .and_then(gpu_short);
        let gpu_text = gpu.as_deref().unwrap_or("");
        let proton = input.game.proton_tool.as_deref().and_then(proton_family);
        let appid = input.appid.to_string();
        let has_appid = input.appid != 0;
        let has_symptom = !kw.is_empty();
        if !has_symptom && !has_appid {
            return Err(QueryError::NoSearchTerms);
        }

        let mut intent_words = keywords.clone();
        intent_words.extend(vendor_word(&input.system).map(str::to_owned));
        intent_words.extend(gpu.clone());
        intent_words.extend(driver_family(&input.system));
        intent_words.extend(proton.clone());

        let query_for = |provider| -> Option<String> {
            match provider {
                ProviderKind::Umu => has_appid.then(|| format!("umu {appid}")),
                // Sem sintoma, só o AppID ainda identifica algo útil.
                _ if !has_symptom => None,
                ProviderKind::Github => has_appid.then(|| join(&[&appid, &kw, gpu_text])),
                ProviderKind::ReleaseNotes => proton.as_deref().map(|p| join(&[p, &kw])),
                ProviderKind::Reddit => (!game.is_empty()).then(|| join(&[&game, &kw, "linux"])),
                ProviderKind::Blog => (!game.is_empty()).then(|| join(&[&game, &kw, gpu_text])),
            }
        };
        Ok(ProviderKind::ALL
            .into_iter()
            .filter_map(|provider| {
                query_for(provider).map(|query| RewrittenQuery {
                    provider,
                    query,
                    intent: QueryIntent {
                        goal: provider.goal().to_owned(),
                        keywords: intent_words.clone(),
                    },
                })
            })
            .collect())
    }

    fn id(&self) -> &'static str {
        "deterministic"
    }
}

#[cfg(test)]
mod ai_query {
    use super::super::models::GpuInfo;
    use super::*;

    fn gpu(vendor: &str, name: &str, driver: Option<&str>, version: &str) -> GpuInfo {
        serde_json::from_value(serde_json::json!({
            "vendor": vendor, "vendorId": "0", "deviceId": "0", "name": name,
            "driver": driver, "driverVersion": version, "bootVga": true
        }))
        .unwrap()
    }

    fn input(
        symptom: &str,
        appid: u32,
        game: &str,
        gpu_info: Option<GpuInfo>,
        tool: Option<&str>,
    ) -> QueryInput {
        QueryInput {
            symptom: symptom.into(),
            appid,
            game: GameInfo {
                name: game.into(),
                proton_tool: tool.map(str::to_owned),
            },
            system: SystemSnapshot {
                gpus: vec![],
                primary_gpu: gpu_info,
                vulkan_drivers: vec![],
                mesa_version: None,
            },
        }
    }

    fn rtx4060() -> GpuInfo {
        gpu(
            "nvidia",
            "NVIDIA GeForce RTX 4060",
            Some("nvidia"),
            "580.65.06",
        )
    }

    fn cyberpunk() -> QueryInput {
        input(
            "Cyberpunk fecha no menu após 10s",
            1091500,
            "Cyberpunk 2077",
            Some(rtx4060()),
            Some("GE-Proton9-20"),
        )
    }

    fn queries(i: &QueryInput) -> Vec<(ProviderKind, String)> {
        Deterministic
            .rewrite(i)
            .unwrap()
            .into_iter()
            .map(|r| (r.provider, r.query))
            .collect()
    }

    fn query_of(i: &QueryInput, provider: ProviderKind) -> Option<String> {
        queries(i)
            .into_iter()
            .find(|(p, _)| *p == provider)
            .map(|(_, q)| q)
    }

    #[test]
    fn cyberpunk_example_produces_the_shown_queries() {
        assert_eq!(
            queries(&cyberpunk()),
            [
                (ProviderKind::Umu, "umu 1091500".to_owned()),
                (
                    ProviderKind::Github,
                    "1091500 crash menu RTX 4060".to_owned()
                ),
                (
                    ProviderKind::ReleaseNotes,
                    "GE-Proton9 crash menu".to_owned()
                ),
                (
                    ProviderKind::Reddit,
                    "Cyberpunk 2077 crash menu linux".to_owned()
                ),
                (
                    ProviderKind::Blog,
                    "Cyberpunk 2077 crash menu RTX 4060".to_owned()
                ),
            ]
        );
    }

    #[test]
    fn each_of_the_twelve_entities_is_extracted() {
        for (symptom, expected) in [
            ("o jogo crasha", "crash"),
            ("congela do nada", "freeze"),
            ("frame time ruim", "stutter"),
            ("sem som", "audio"),
            ("controle não funciona", "controller"),
            ("DLSS borrado", "dlss"),
            ("FSR com artefatos", "fsr"),
            ("ray tracing escuro", "raytracing"),
            ("erro de PhysX", "physx"),
            ("não abre", "launch"),
            ("problema no menu", "menu"),
            ("problema durante gameplay", "gameplay"),
        ] {
            assert!(canonical_keywords(symptom).contains(&expected), "{symptom}");
        }
    }

    #[test]
    fn matching_uses_whole_words() {
        assert!(canonical_keywords("somente texto, sem nada").is_empty());
        assert!(canonical_keywords("restart").is_empty());
    }

    #[test]
    fn terms_outside_the_table_stay_literal_and_are_not_translated() {
        let keywords = symptom_keywords("queda de FPS no menu", "Jogo");
        assert_eq!(keywords, ["menu", "queda", "fps"]);
        assert_eq!(
            symptom_keywords("XeSS borrado", "Jogo"),
            ["xess", "borrado"]
        );
    }

    #[test]
    fn paths_variables_and_game_words_never_become_terms() {
        let keywords = symptom_keywords(
            "/home/gustavo/.steam DXVK_HUD=1 $HOME rm;ls Cyberpunk erro",
            "Cyberpunk 2077",
        );
        assert_eq!(keywords, ["erro"]);
    }

    #[test]
    fn gpu_short_keeps_only_the_recognized_model() {
        for (name, expected) in [
            ("NVIDIA GeForce RTX 4060 Ti", Some("RTX 4060 Ti")),
            ("AD107 [GeForce RTX 4060]", Some("RTX 4060")),
            ("AMD Radeon RX 7800 XT", Some("RX 7800 XT")),
            ("Intel Arc B580", Some("Arc B580")),
            ("Intel UHD Graphics 770", Some("UHD 770")),
            ("AMD Radeon 780M", Some("Radeon 780M")),
            ("Placa desconhecida", None),
        ] {
            assert_eq!(gpu_short(name).as_deref(), expected, "{name}");
        }
    }

    #[test]
    fn proton_family_comes_from_the_tool_name() {
        for (tool, expected) in [
            ("GE-Proton9-20", Some("GE-Proton9")),
            ("GE-Proton10-34", Some("GE-Proton10")),
            ("Proton 9.0-4", Some("Proton 9")),
            ("proton_9", Some("Proton 9")),
            ("Proton Experimental", Some("Proton Experimental")),
            ("proton_experimental", Some("Proton Experimental")),
            ("Wine custom", None),
            ("Proton", None),
        ] {
            assert_eq!(proton_family(tool).as_deref(), expected, "{tool}");
        }
    }

    #[test]
    fn missing_data_removes_the_query_instead_of_inventing() {
        let bare = input("trava no menu", 1091500, "", None, None);
        assert_eq!(
            queries(&bare),
            [
                (ProviderKind::Umu, "umu 1091500".to_owned()),
                (ProviderKind::Github, "1091500 freeze menu".to_owned()),
            ]
        );
        let no_appid = input("trava no menu", 0, "Jogo", None, None);
        assert_eq!(query_of(&no_appid, ProviderKind::Umu), None);
        assert_eq!(query_of(&no_appid, ProviderKind::Github), None);
        assert_eq!(
            query_of(&no_appid, ProviderKind::Reddit).as_deref(),
            Some("Jogo freeze menu linux")
        );
    }

    #[test]
    fn empty_symptom_keeps_only_the_appid_search() {
        let i = input(
            "  ",
            1091500,
            "Jogo",
            Some(rtx4060()),
            Some("GE-Proton9-20"),
        );
        assert_eq!(queries(&i), [(ProviderKind::Umu, "umu 1091500".to_owned())]);
        let none = input("?!", 0, "Jogo", None, None);
        assert_eq!(Deterministic.rewrite(&none), Err(QueryError::NoSearchTerms));
    }

    #[test]
    fn game_name_cannot_inject_symbols() {
        let i = input("crash", 1, "Jogo; rm -rf ~ $(x) ™", None, None);
        let reddit = query_of(&i, ProviderKind::Reddit).unwrap();
        assert_eq!(reddit, "Jogo rm -rf x crash linux");
        assert_eq!(origin_error(&i, &reddit), None);
    }

    #[test]
    fn deterministic_queries_always_pass_the_origin_check() {
        let cases = [
            cyberpunk(),
            input(
                "queda de FPS",
                1,
                "Baldur's Gate 3",
                Some(gpu(
                    "amd",
                    "AMD Radeon RX 7800 XT",
                    Some("amdgpu"),
                    "6.10.4",
                )),
                Some("Proton 9.0-4"),
            ),
            input(
                "sem som e controle falha",
                2,
                "Hades II",
                Some(gpu("intel", "Intel Arc B580", Some("xe"), "6.10.4")),
                Some("Proton Experimental"),
            ),
            input("não abre", 3, "NieR:Automata", None, None),
        ];
        for case in &cases {
            for rewritten in Deterministic.rewrite(case).unwrap() {
                assert_eq!(
                    origin_error(case, &rewritten.query),
                    None,
                    "{}",
                    rewritten.query
                );
            }
        }
    }

    #[test]
    fn origin_check_rejects_invented_terms_and_symbols() {
        let i = cyberpunk();
        assert_eq!(origin_error(&i, "Cyberpunk 2077 crash menu linux"), None);
        assert!(origin_error(&i, "Cyberpunk crash winetricks")
            .unwrap()
            .contains("winetricks"));
        assert!(origin_error(&i, "Cyberpunk freeze")
            .unwrap()
            .contains("freeze"));
        assert!(origin_error(&i, "RTX 4090 crash").unwrap().contains("4090"));
        for bad in ["crash; rm", "crash | sh", "crash $(x)", "/home/g crash", ""] {
            assert!(origin_error(&i, bad).is_some(), "{bad}");
        }
    }

    #[test]
    fn intent_carries_hardware_entities_without_changing_the_template() {
        let out = Deterministic.rewrite(&cyberpunk()).unwrap();
        let keywords = &out[1].intent.keywords;
        for expected in ["crash", "menu", "nvidia", "RTX 4060", "580", "GE-Proton9"] {
            assert!(keywords.iter().any(|k| k == expected), "{expected}");
        }
        assert!(!out[1].query.contains("580"));
    }

    #[test]
    fn selection_defaults_to_deterministic_and_rejects_unknown_names() {
        assert_eq!(select_rewriter(None).unwrap().id(), "deterministic");
        assert_eq!(select_rewriter(Some(" ")).unwrap().id(), "deterministic");
        assert_eq!(
            select_rewriter(Some("deterministic")).unwrap().id(),
            "deterministic"
        );
        assert!(matches!(
            select_rewriter(Some("gpt")),
            Err(QueryError::UnknownRewriter(_))
        ));
    }

    #[cfg(not(feature = "qwen-query"))]
    #[test]
    fn qwen_without_the_feature_is_a_clear_error_not_a_silent_swap() {
        let Err(e) = select_rewriter(Some("qwen")) else {
            panic!("não deveria selecionar a Qwen");
        };
        assert!(e.to_string().contains("qwen-query"));
    }
}
