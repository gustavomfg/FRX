//! Modelos dos Smart Packs (espelhados em `src/types/smartPacks.ts`).

use serde::{Deserialize, Serialize};

use crate::fix_engine::dynamic::{DynStatus, EvidenceLevel, GamePerformanceEvidence};
use crate::fix_engine::models::{PlanStatus, Purpose, RecipeAction, RecipeSource, Risk, Trust};
use crate::fixes::models::{ArtifactCandidate, GpuReq};

pub const PACK_SCHEMA: u32 = 1;
/// Versão do compositor: mudou → os pacotes são recompostos e registrados de novo.
pub const COMPOSER_VERSION: &str = "1.3.0";
/// Versão da metodologia de ranking (critérios e limites documentados).
pub const RANKING_VERSION: &str = "1.3.0";

/// Perfil de otimização: muda os critérios de ordenação, nunca os dados.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Profile {
    Performance,
    Balanced,
    Compatibility,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PackKind {
    /// Configuração de referência: nenhuma correção do FRX ativa.
    Baseline,
    /// Uma receita só.
    Single,
    /// Duas ou mais receitas compatíveis.
    Combination,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Goal {
    Reference,
    Compatibility,
    Performance,
    Balanced,
}

/// O que se sabe sobre o efeito de uma variável na medição.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PerfHint {
    /// Efeito no desempenho desconhecido: só a medição dirá.
    Unknown,
    /// Risco técnico potencial; não determina o desempenho deste jogo.
    Penalty,
    /// Limita o FPS: muda as condições do benchmark.
    FrameCap,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EnvPair {
    pub key: String,
    pub value: String,
}

/// Identidade do conteúdo efetivamente instalado; nunca usa chaves de env falsas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "camelCase")]
pub struct FileFixRef {
    pub target_path: String,
    pub sha256: String,
    pub artifact_id: Option<String>,
    pub artifact_version: Option<String>,
    pub artifact_sha256: Option<String>,
}

/// Receita aplicável que pode entrar em um pacote (unidade atômica: nunca é
/// dividida nem misturada com ações de outra receita).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Unit {
    #[serde(default)]
    pub hardware: crate::system::context::HardwareRequirements,
    #[serde(default)]
    pub file_actions: Vec<RecipeAction>,
    /// Escopo verificado da descoberta; ausente em dados legados não atribuídos.
    #[serde(default)]
    pub app_id: Option<String>,
    #[serde(default)]
    pub performance_evidence: GamePerformanceEvidence,
    pub recipe_id: String,
    pub version: String,
    /// SHA-256 da receita usada na composição.
    pub fingerprint: String,
    pub trust: Trust,
    pub title: String,
    pub env: Vec<EnvPair>,
    pub purpose: Purpose,
    pub risk: Risk,
    pub evidence: EvidenceLevel,
    pub evidence_note: String,
    pub finding_ids: Vec<String>,
    pub sources: Vec<RecipeSource>,
    pub proton_versions: Vec<String>,
    pub discovered_at: Option<String>,
    pub gpu: Option<GpuReq>,
    pub already_included: bool,
    pub perf: PerfHint,
    pub effects: Vec<String>,
    pub side_effects: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ConflictKind {
    SameFile,
    GameScope,
    SameKeyDifferentValue,
    MutuallyExclusive,
    Redundant,
    ProtonVersions,
    GpuMismatch,
    FrameLimiter,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Relation {
    /// Nenhum conflito conhecido — o que **não** prova que funcionam juntas.
    NoKnownConflict,
    Excludes,
}

/// Aresta do grafo de compatibilidade entre duas receitas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub a: String,
    pub b: String,
    pub relation: Relation,
    pub kind: Option<ConflictKind>,
    pub reason: String,
}

/// Estado operacional — execução e verificação, nunca ganho de FPS.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackState {
    Candidate,
    Ready,
    Conflict,
    Testing,
    Applied,
    Verified,
    Failed,
    PartiallyRecovered,
    RolledBack,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackMemberView {
    #[serde(default)]
    pub file_actions: Vec<RecipeAction>,
    pub recipe_id: String,
    pub version: String,
    pub fingerprint: String,
    pub trust: Trust,
    pub title: String,
    pub env: Vec<EnvPair>,
    pub evidence: EvidenceLevel,
    pub discovered_at: Option<String>,
    pub proton_versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SmartPack {
    #[serde(default)]
    pub artifacts: Vec<ArtifactCandidate>,
    #[serde(default)]
    pub file_fixes: Vec<FileFixRef>,
    #[serde(default)]
    pub performance_evidence: GamePerformanceEvidence,
    #[serde(default)]
    pub performance_warnings: Vec<String>,
    /// `pack.<appid>.<hash>` das variáveis ordenadas (estável).
    pub id: String,
    pub app_id: String,
    pub version: String,
    pub name: String,
    pub kind: PackKind,
    pub goal: Goal,
    pub goal_text: String,
    pub members: Vec<PackMemberView>,
    pub env: Vec<EnvPair>,
    /// Receita aberta no Fix Engine (`None` = baseline, que não se aplica).
    pub apply_recipe_id: Option<String>,
    /// Impressão digital da composição (receitas + versões + hashes).
    pub fingerprint: String,
    pub requirements: Vec<String>,
    pub compatibility: Vec<String>,
    /// Combinações descartadas por conflito envolvendo as receitas do pacote.
    pub conflicts_avoided: Vec<Edge>,
    pub sources: Vec<RecipeSource>,
    pub evidence: EvidenceLevel,
    pub evidence_note: String,
    pub risk: Risk,
    pub effects: Vec<String>,
    pub side_effects: Vec<String>,
    pub reversible: bool,
    pub state: PackState,
    pub state_detail: String,
    pub plan_status: Option<PlanStatus>,
    pub operation_id: Option<String>,
}

/// Configuração registrada em uma sessão do Performance Lab.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SnapshotSource {
    /// Gravado pelo FRX no fim da captura assistida, conferido no disco.
    Assisted,
    /// Informado pelo usuário para uma sessão importada ou antiga.
    Declared,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotRecipe {
    #[serde(default)]
    pub fingerprint: Option<String>,
    pub recipe_id: String,
    pub version: String,
}

/// Snapshot das condições do Fix Engine anexado a cada benchmark.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackSnapshot {
    #[serde(default)]
    pub file_fixes: Vec<FileFixRef>,
    /// Adicionado sem atribuir registros legados ao jogo atualmente aberto.
    #[serde(default)]
    pub app_id: Option<String>,
    /// Identidade da configuração registrada, incluindo jogo e versões.
    #[serde(default)]
    pub fingerprint: Option<String>,
    pub pack_id: String,
    pub env: Vec<EnvPair>,
    pub recipes: Vec<SnapshotRecipe>,
    pub operation_ids: Vec<String>,
    /// Todas as correções conferidas no disco no momento da captura.
    pub verified: bool,
    pub source: SnapshotSource,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MetricStats {
    pub n: usize,
    pub median: f64,
    pub min: f64,
    pub max: f64,
    /// (máx − mín) ÷ mediana, em %. `None` com uma sessão só.
    pub spread_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionRef {
    pub id: String,
    pub label: String,
    pub captured_at: Option<String>,
    pub source: SnapshotSource,
    pub verified: bool,
    pub duration_s: f64,
    pub avg_fps: Option<f64>,
    pub low1: Option<f64>,
    /// `None` = usada no ranking; `Some` = motivo da exclusão.
    pub excluded: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Measurements {
    pub pack_id: String,
    pub sessions: Vec<SessionRef>,
    pub avg: Option<MetricStats>,
    pub low1: Option<MetricStats>,
    pub low01: Option<MetricStats>,
    pub ft_p99: Option<MetricStats>,
    pub spikes_per_min: Option<MetricStats>,
}

impl Measurements {
    pub fn used(&self) -> usize {
        self.sessions
            .iter()
            .filter(|s| s.excluded.is_none())
            .count()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Improvement,
    Regression,
    NoDifference,
    Mixed,
    Inconclusive,
    NotMeasured,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MetricDelta {
    pub metric: String,
    pub label: String,
    pub delta_pct: f64,
    /// Percentual indisponível quando o valor de referência é zero.
    #[serde(default = "default_true")]
    pub percentage_available: bool,
    pub threshold_pct: f64,
    pub outcome: Outcome,
    /// Poucas sessões: tratar como indicativo.
    pub limited: bool,
    /// Medianas comparadas (referência e pacote), na unidade da métrica.
    #[serde(default)]
    pub before: f64,
    #[serde(default)]
    pub after: f64,
    #[serde(default)]
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VsBaseline {
    pub pack_id: String,
    pub deltas: Vec<MetricDelta>,
    pub limited: bool,
    pub notes: Vec<String>,
}

impl VsBaseline {
    pub fn get(&self, metric: &str) -> Option<&MetricDelta> {
        self.deltas.iter().find(|d| d.metric == metric)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Tier {
    /// Dados comparáveis suficientes para ordenar por desempenho observado.
    Measured,
    /// Seguro e compatível, sem evidência suficiente: ordem de teste.
    Candidate,
    /// Triagem preventiva: possível custo, sem recomendar testes obrigatórios.
    PerformanceRisk,
    NotRecommended,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RankEntry {
    pub pack_id: String,
    pub title: String,
    pub tier: Tier,
    /// Só no ranking medido; empates técnicos dividem a mesma posição.
    pub position: Option<u32>,
    pub outcome: Outcome,
    pub headline: String,
    pub reasons: Vec<String>,
    pub vs_baseline: Option<VsBaseline>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NextTest {
    pub pack_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Ranking {
    pub profile: Profile,
    pub version: String,
    pub entries: Vec<RankEntry>,
    /// Só quando os dados sustentam um vencedor.
    pub winner: Option<String>,
    pub summary: String,
    pub next_test: Option<NextTest>,
    pub criteria: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RankingSnapshot {
    /// IDs separados preventivamente; campo aditivo para leitores antigos.
    #[serde(default)]
    pub preventive_risks: Vec<String>,
    pub at: String,
    pub profile: Profile,
    pub version: String,
    pub reason: String,
    pub entries: Vec<(String, Option<u32>, Tier, Outcome)>,
    pub sessions: usize,
}

/// Correção que não entra em pacotes, com o motivo.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Excluded {
    #[serde(default)]
    pub performance_warnings: Vec<String>,
    pub id: String,
    pub title: String,
    pub status: Option<DynStatus>,
    pub reason: String,
}

/// Referência externa de Windows (modelo preparado; nenhuma fonte integrada).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WindowsReference {
    pub origin: String,
    pub url: Option<String>,
    pub game_version: Option<String>,
    pub cpu: Option<String>,
    pub gpu: Option<String>,
    pub driver: Option<String>,
    pub resolution: Option<String>,
    pub preset: Option<String>,
    pub avg_fps: Option<f64>,
    pub low1: Option<f64>,
    pub measured_at: Option<String>,
    pub conditions: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    /// Receitas consideradas na composição (as demais ficam fora, com motivo).
    pub max_units: usize,
    /// Pacotes oferecidos, incluindo o baseline.
    pub max_packs: usize,
    /// Receitas por pacote.
    pub max_pack_size: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_units: 8,
            max_packs: 12,
            max_pack_size: 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SmartPackReport {
    pub app_id: String,
    pub game_name: String,
    pub profile: Profile,
    pub generated_at: String,
    pub composer_version: String,
    pub rules_version: String,
    pub limits: Limits,
    /// Sessão cujas condições servem de referência para a comparabilidade.
    pub reference_session_id: Option<String>,
    pub baseline_id: String,
    pub packs: Vec<SmartPack>,
    pub units: Vec<Unit>,
    pub graph: Vec<Edge>,
    pub excluded: Vec<Excluded>,
    pub measurements: Vec<Measurements>,
    pub ranking: Ranking,
    pub history: Vec<RankingSnapshot>,
    /// Sessões do jogo sem configuração registrada (podem ser declaradas).
    pub unassigned_sessions: Vec<SessionRef>,
    /// Pacote equivalente à configuração atual do FRX neste jogo.
    pub current_pack_id: Option<String>,
    pub current_note: String,
    pub windows: Option<WindowsReference>,
    /// Última avaliação de cada pacote com sessão registrada.
    pub evaluations: Vec<PackEvaluation>,
    pub notes: Vec<String>,
}

/// Conclusão do teste de um pacote, a partir de uma sessão.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EvalVerdict {
    Improvement,
    Regression,
    NoDifference,
    Inconclusive,
    /// Condições diferentes da referência, qualidade insuficiente ou configuração desconhecida.
    NotComparable,
    /// Ainda não há baseline medido para comparar.
    NoBaseline,
    /// A sessão é do próprio baseline.
    Reference,
}

/// Avaliação automática de um pacote depois de uma captura.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackEvaluation {
    /// Contexto da sessão avaliada; evidência local a este hardware/software.
    #[serde(default)]
    pub conditions: crate::performance::models::Conditions,
    pub app_id: String,
    pub pack_id: String,
    pub pack_name: String,
    pub session_id: String,
    pub session_label: String,
    pub captured_at: Option<String>,
    pub verdict: EvalVerdict,
    /// Menos de 2 sessões comparáveis de algum lado: resultado preliminar.
    pub preliminary: bool,
    pub headline: String,
    pub summary: String,
    pub deltas: Vec<MetricDelta>,
    pub baseline_sessions: usize,
    pub pack_sessions: usize,
    pub limitations: Vec<String>,
    /// Próximo passo sugerido (repetir, capturar baseline…).
    pub next_step: Option<String>,
    /// O que este resultado muda no ranking.
    pub ranking_effect: String,
    pub baseline_id: String,
    /// Configuração do FRX ativa agora neste jogo (para oferecer restaurar).
    pub current_pack_id: Option<String>,
}

/// Comparação direta entre dois pacotes (mesma metodologia do ranking).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackComparison {
    pub a: String,
    pub b: String,
    pub deltas: Vec<MetricDelta>,
    pub limited: bool,
    pub summary: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SwitchStep {
    /// `rollback` (desfazer uma operação) ou `apply` (aplicar o pacote).
    pub kind: String,
    pub operation_id: Option<String>,
    pub recipe_id: Option<String>,
    pub title: String,
    pub detail: String,
}

/// Transição entre a configuração atual e outro pacote — sempre passo a
/// passo, cada passo com a própria prévia e confirmação.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SwitchPlan {
    pub to: String,
    pub from: Option<String>,
    pub keep: Vec<EnvPair>,
    pub remove: Vec<EnvPair>,
    pub add: Vec<EnvPair>,
    pub steps: Vec<SwitchStep>,
    pub possible: bool,
    pub notes: Vec<String>,
}

fn default_true() -> bool {
    true
}
