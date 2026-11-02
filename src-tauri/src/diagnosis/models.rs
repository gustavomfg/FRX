//! Tipos do diagnóstico (espelhados em `src/types/diagnosis.ts`).

use serde::{Deserialize, Serialize};

use crate::fix_engine::availability::Availability;
use crate::fix_engine::dynamic::{AssessmentView, EvidenceLevel};
use crate::fix_engine::models::{Purpose, Recipe};
use crate::fixes::models::{FindingSource, FixFinding, GpuReq, ProviderStatus};
use crate::fixes::reports::Outcome;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Symptom {
    #[default]
    Unspecified,
    CrashOrWontStart,
    Performance,
    Graphics,
    Other,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosisContext {
    pub gpu_vendor: Option<GpuReq>,
    pub gpu_name: Option<String>,
    pub driver: Option<String>,
    pub proton_tool: Option<String>,
    pub uses_proton: Option<bool>,
    pub tool_includes_protonfixes: bool,
    /// Arquivos conhecidos encontrados na instalação, em minúsculas.
    pub game_files: Vec<String>,
    pub install_known: bool,
    /// Variáveis já presentes nas opções de inicialização do jogo.
    pub current_env: Vec<(String, String)>,
}

/// Melhoria do catálogo que o planejador já avaliou para este jogo.
#[derive(Debug, Clone)]
pub struct CatalogOption {
    pub recipe: Recipe,
    pub availability: Availability,
}

#[derive(Debug, Clone)]
pub struct DiagnosisInput {
    pub app_id: String,
    pub symptom: Symptom,
    pub context: DiagnosisContext,
    pub findings: Vec<FixFinding>,
    pub assessments: Vec<AssessmentView>,
    pub catalog: Vec<CatalogOption>,
    pub providers: Vec<ProviderStatus>,
    /// Vem de fora para o núcleo não depender do relógio.
    pub generated_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(tag = "type", content = "outcome", rename_all = "kebab-case")]
pub enum ClaimKind {
    /// O autor relata o que aconteceu ao usar a opção.
    ReportedResult(Outcome),
    /// Repositório curado, mantenedor ou documentação da ferramenta.
    Documented,
    /// A fonte cita a opção sem relatar resultado.
    MentionOnly,
}

/// Uma afirmação de uma fonte. O texto é só citação: nunca vira ação.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Claim {
    pub kind: ClaimKind,
    pub option: String,
    pub disabled: bool,
    pub combined_with: Vec<String>,
    pub quote: Option<String>,
    pub source: FindingSource,
    pub game_verified: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigItem {
    /// `env`, `dxvk`, `component`, `launch` ou `other`.
    pub kind: String,
    pub name: String,
    pub value: Option<String>,
}

/// Configuração normalizada: a identidade usada para deduplicar.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigKey {
    pub id: String,
    pub items: Vec<ConfigItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mapping {
    Mapped {
        recipe_id: String,
        availability: Availability,
    },
    Review {
        code: String,
        reason: String,
    },
    AlreadyIncluded {
        reason: String,
    },
    Incompatible {
        reason: String,
    },
    /// Relato sem nenhuma ação técnica: não vira sugestão.
    NoAction,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub key: ConfigKey,
    pub title: String,
    pub purpose: Option<Purpose>,
    pub claims: Vec<Claim>,
    pub mapping: Mapping,
    pub compatibility_note: String,
    pub gpu_vendor: Option<crate::fixes::models::GpuReq>,
    pub unattributed_reports: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum SuggestionState {
    Applicable,
    Temporary,
    AlreadyApplied,
    AlreadyIncluded,
    Review,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CompatStatus {
    Compatible,
    Unknown,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub key: ConfigKey,
    pub title: String,
    /// Melhoria de imagem, e não correção de problema.
    pub optional_improvement: bool,
    pub state: SuggestionState,
    pub confidence: EvidenceLevel,
    pub reason: String,
    pub compatibility: CompatStatus,
    pub compatibility_detail: String,
    /// Fabricante exigido pela variável ou pela fonte. `None` não prova que a
    /// configuração vale em qualquer GPU.
    pub gpu_vendor: Option<crate::fixes::models::GpuReq>,
    pub positive: u32,
    pub negative: u32,
    pub claims: Vec<Claim>,
    pub recipe_id: Option<String>,
    pub expected_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ExcludedKind {
    Incompatible,
    /// Relatos dizem que não resolveu ou piorou.
    Negative,
    /// Não serve para este sintoma.
    WrongPurpose,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Excluded {
    pub key: ConfigKey,
    pub title: String,
    pub kind: ExcludedKind,
    pub reason: String,
    pub claims: Vec<Claim>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    Suggestions,
    OnlyInReview,
    NoEvidence,
    SourcesUnavailable,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Origin {
    pub method: String,
    pub learned_components: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosisResult {
    pub app_id: String,
    pub symptom: Symptom,
    pub verdict: Verdict,
    pub message: String,
    pub context: DiagnosisContext,
    pub suggestions: Vec<Suggestion>,
    pub in_review: Vec<Suggestion>,
    pub excluded: Vec<Excluded>,
    pub providers: Vec<ProviderStatus>,
    pub limitations: Vec<String>,
    pub generated_at: String,
    pub origin: Origin,
    pub automatic_changes: bool,
}
