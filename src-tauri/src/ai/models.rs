//! Contratos do ranker. Conteúdo de fonte externa é sempre dado não confiável:
//! entra por `Candidate::sanitized` e nunca vira comando, caminho ou HTML.

use serde::{Deserialize, Serialize};

pub use crate::system::hardware::{GpuInfo, GpuVendor};
use crate::system::SystemProfile;

const MAX_TITLE: usize = 200;
const MAX_SNIPPET: usize = 1_000;
const MAX_RAW: usize = 8_000;
const MAX_SYMPTOM: usize = 500;

/// O que o ranker usa do `SystemProfile`. Mesmos nomes de campo em JSON: o
/// retorno de `get_system_profile` serve aqui direto. Campos a mais são ignorados.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SystemSnapshot {
    #[serde(default)]
    pub gpus: Vec<GpuInfo>,
    #[serde(default)]
    pub primary_gpu: Option<GpuInfo>,
    #[serde(default)]
    pub vulkan_drivers: Vec<String>,
    /// Versão do Mesa instalada no host, preenchida por `driver::read_host_mesa`.
    /// `SystemProfile` não a tem: lá, `driver_version` é a do módulo do kernel.
    #[serde(default)]
    pub mesa_version: Option<String>,
}

impl From<SystemProfile> for SystemSnapshot {
    fn from(p: SystemProfile) -> Self {
        Self {
            gpus: p.gpus,
            primary_gpu: p.primary_gpu,
            vulkan_drivers: p.vulkan_drivers,
            mesa_version: None,
        }
    }
}

impl SystemSnapshot {
    /// A GPU que provavelmente roda o jogo. Sem indicação, só a única GPU conta.
    pub fn gpu(&self) -> Option<&GpuInfo> {
        self.primary_gpu.as_ref().or(if self.gpus.len() == 1 {
            self.gpus.first()
        } else {
            None
        })
    }
}

/// Só o necessário para ranquear. Sem caminhos de instalação.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    pub name: String,
    /// Nome da ferramenta selecionada, como "GE-Proton10-34" ou "Proton 9.0-4".
    #[serde(default)]
    pub proton_tool: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResearchQuery {
    pub symptom: String,
    pub appid: u32,
    pub system: SystemSnapshot,
    pub game: GameInfo,
}

impl ResearchQuery {
    /// O sintoma como texto limitado e sem caracteres de controle.
    pub fn clean_symptom(&self) -> String {
        clean_line(&self.symptom, MAX_SYMPTOM)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    Umu,
    ReleaseNotes,
    GithubComment,
    Reddit,
    Blog,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub source: SourceKind,
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub raw_text: String,
    /// ISO 8601, como no restante do projeto. Sem data, a recência é zero.
    #[serde(default)]
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScoreFeatures {
    pub gpu_match: f32,
    /// `None` é "sem informação para comparar", nunca "sem match": não entra no score.
    pub driver_proximity: Option<f32>,
    pub proton_match: f32,
    pub symptom_match: f32,
    pub source_trust: f32,
    pub recency: f32,
    pub penalty_stale: f32,
    pub penalty_other_gpu: f32,
}

/// De que tipo é a fonte. Vem só de `SourceKind`: pertinência alta nunca muda
/// este valor, e ele não diz que a correção funciona neste computador.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ConfidenceLabel {
    Verified,
    Curated,
    CommunityReport,
    NotVerified,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScoredCandidate {
    pub candidate: Candidate,
    /// Pertinência ao sistema do usuário, em [0, 1]. Provisória; não é confiança.
    pub relevance: f32,
    pub features: ScoreFeatures,
    /// Tipo da fonte, independente de `relevance`.
    pub confidence: ConfidenceLabel,
    /// Quem calculou `symptom_match`: o modelo ou a alternativa léxica.
    pub symptom_method: String,
}

/// Linha com sintaxe de shell não entra: `; | && $(` e crase.
fn hostile_line(line: &str) -> bool {
    [";", "|", "&&", "$(", "`"].iter().any(|s| line.contains(s))
}

fn clean_line(text: &str, limit: usize) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(limit)
        .collect();
    redact_paths(cleaned.trim())
}

/// Troca o usuário em caminhos pessoais. O resto do texto fica literal.
pub fn redact_paths(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let found = ["/home/", "/var/home/", "/Users/"]
            .iter()
            .filter_map(|p| rest.find(p).map(|i| (i, p.len())))
            .min();
        let Some((start, len)) = found else {
            out.push_str(rest);
            return out.replace("$HOME", "~");
        };
        let after = &rest[start + len..];
        let end = after
            .find(|c: char| c == '/' || c == '\\' || c == '"' || c == '\'' || c.is_whitespace())
            .unwrap_or(after.len());
        out.push_str(&rest[..start + len]);
        out.push_str("<user>");
        rest = &after[end..];
    }
}

fn clean_block(text: &str, limit: usize) -> String {
    let kept: Vec<String> = text
        .lines()
        .filter(|l| !hostile_line(l))
        .map(|l| redact_paths(&l.chars().filter(|c| !c.is_control()).collect::<String>()))
        .collect();
    kept.join("\n").chars().take(limit).collect()
}

impl Candidate {
    /// `None` fora da allowlist HTTPS de `net`. Texto externo sai sem linhas com
    /// sintaxe de shell, sem caminhos pessoais e com tamanho limitado.
    pub fn sanitized(&self) -> Option<Candidate> {
        crate::net::is_allowed(&self.url).then(|| Candidate {
            source: self.source,
            url: self.url.clone(),
            title: clean_line(
                self.title
                    .lines()
                    .find(|l| !hostile_line(l))
                    .unwrap_or_default(),
                MAX_TITLE,
            ),
            snippet: clean_block(&self.snippet, MAX_SNIPPET),
            raw_text: clean_block(&self.raw_text, MAX_RAW),
            published_at: self.published_at.as_deref().map(|d| clean_line(d, 40)),
        })
    }
}
