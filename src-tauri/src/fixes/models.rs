//! Modelo unificado de descobertas (espelhado em `src/types/fixes.ts`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Compatibility,
    Performance,
    Graphics,
    Audio,
    Video,
    Physics,
    Launcher,
    Input,
    Network,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FindingKind {
    /// Script do UMU-Protonfixes para o jogo.
    Protonfix,
    /// Ajuste citado pela comunidade (variável, componente, opção de inicialização).
    Tweak,
    /// Relato de problema no tracker de compatibilidade.
    Report,
    /// Declaração de mantenedor: notas de versão ou comentário oficial.
    ReleaseNote,
}

/// Grau de confiança. `Verified` exige declaração verificável de mantenedores;
/// nunca é atribuído por popularidade, estrelas ou mera existência de arquivo.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    Verified,
    Curated,
    CommunityReported,
    Unverified,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Compatibility {
    Compatible,
    Incompatible,
    Unknown,
    AlreadyIncluded,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AnalysisState {
    Parsed,
    Partial,
    NotVerified,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderId {
    UmuProtonfixes,
    UmuDatabase,
    Github,
    Protondb,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum GpuReq {
    Nvidia,
    Amd,
    Intel,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ActionKind {
    Winetricks,
    Env,
    DllOverride,
    LaunchOption,
    ConfigFile,
    Registry,
    ProtonVersion,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FixAction {
    pub kind: ActionKind,
    pub label: String,
    pub value: String,
    pub conditional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FindingSource {
    pub provider: ProviderId,
    pub label: String,
    pub url: String,
    pub reference_id: Option<String>,
    pub snippet: Option<String>,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Requirements {
    pub gpu_vendor: Option<GpuReq>,
    pub proton_versions: Vec<String>,
    pub game_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Evidence {
    pub positive: u32,
    pub negative: u32,
}

/// Identity is established by the source itself, never by the search query.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DiscoveryScopeKind {
    VerifiedGame,
    GeneralTechnical,
    #[default]
    UnknownScope,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryScope {
    pub kind: DiscoveryScopeKind,
    pub app_id: Option<String>,
    pub basis: String,
}

impl DiscoveryScope {
    pub fn verified(app_id: &str, basis: impl Into<String>) -> Self {
        Self {
            kind: DiscoveryScopeKind::VerifiedGame,
            app_id: Some(app_id.to_owned()),
            basis: basis.into(),
        }
    }

    pub fn matches_game(&self, app_id: &str) -> bool {
        self.kind == DiscoveryScopeKind::VerifiedGame
            && self.app_id.as_deref() == Some(app_id)
            && crate::art::valid_app_id(app_id)
            && !self.basis.trim().is_empty()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PerformanceOutcome {
    Positive,
    Negative,
}

/// A public report is a contextual claim, never a local benchmark.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceReport {
    pub app_id: String,
    pub action_key: String,
    pub source_url: String,
    #[serde(default)]
    pub author: Option<String>,
    pub reported_at: Option<String>,
    pub statement: String,
    pub outcome: PerformanceOutcome,
    pub proton_version: Option<String>,
    pub gpu_vendor: Option<GpuReq>,
    pub confidence: Confidence,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceEvidence {
    pub positive: u32,
    pub negative: u32,
    #[serde(default)]
    pub reports: Vec<PerformanceReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FixFinding {
    #[serde(default)]
    pub scope: DiscoveryScope,
    #[serde(default)]
    pub performance_evidence: PerformanceEvidence,
    pub id: String,
    pub game_app_id: String,
    pub title: String,
    pub description: String,
    pub category: Category,
    pub kind: FindingKind,
    pub actions: Vec<FixAction>,
    pub sources: Vec<FindingSource>,
    pub requirements: Requirements,
    pub confidence: Confidence,
    pub compatibility: Compatibility,
    pub compatibility_note: String,
    pub experimental: bool,
    pub analysis: AnalysisState,
    pub evidence: Evidence,
    pub published_at: Option<String>,
    pub retrieved_at: String,
    /// Chaves normalizadas das ações (para agrupar fontes que citam o mesmo ajuste).
    #[serde(skip_serializing, default)]
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderState {
    Ok,
    Cached,
    Stale,
    NoData,
    Unavailable,
    RateLimited,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatus {
    pub id: ProviderId,
    pub name: String,
    pub state: ProviderState,
    pub message: Option<String>,
    pub fetched_at: Option<String>,
    pub reset_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub umu_id: Option<String>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReportContext {
    pub gpu_vendor: Option<GpuReq>,
    pub tool_name: Option<String>,
    pub tool_includes_protonfixes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FixReport {
    pub app_id: String,
    pub game_name: String,
    pub findings: Vec<FixFinding>,
    pub providers: Vec<ProviderStatus>,
    pub identity: Identity,
    pub context: ReportContext,
    pub retrieved_at: String,
    pub from_cache: bool,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: ProviderId,
    pub name: &'static str,
    pub enabled: bool,
    pub description: &'static str,
    pub reason: Option<&'static str>,
}
