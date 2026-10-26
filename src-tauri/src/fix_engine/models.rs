//! Modelos do Fix Engine (espelhados em `src/types/fixEngine.ts`).
//!
//! Achado (`FixFinding`) → receita (revisada do catálogo local ou gerada pelo
//! `dynamic`) → plano → operação aplicada. Só receitas `reviewed` e
//! `generated` (validadas pelo núcleo) viram planos executáveis.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::dynamic::{provenance::Provenance, EvidenceLevel};
use crate::fixes::models::{Category, FixFinding, GpuReq};

pub const RECIPE_SCHEMA: u32 = 1;
pub const JOURNAL_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Trust {
    /// Visto em alguma fonte; nunca vira receita sozinho.
    Discovered,
    /// Escrito como receita, mas ainda não revisado: não executa.
    Candidate,
    /// Revisado e incluído no catálogo do FRX: pode virar plano.
    Reviewed,
    /// Formato ou ação que o FRX não sabe aplicar com segurança.
    Unsupported,
    /// Retirado do catálogo (problema conhecido).
    Revoked,
    /// Gerada pelo FRX a partir de uma descoberta, com ações do conjunto
    /// restrito e validação do núcleo. Não foi revisada por pessoas.
    Generated,
    /// Smart Pack: composição de receitas revisadas/geradas, aplicada como
    /// uma única operação (um backup, uma gravação, um rollback).
    Pack,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Purpose {
    Diagnostic,
    Compatibility,
    Performance,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Risk {
    Low,
    Moderate,
    High,
}

/// Ações **declarativas** — o conjunto é fechado. Não existe ação de comando,
/// script ou caminho absoluto: tudo o que uma receita pode fazer está aqui.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecipeAction {
    /// `KEY=valor` no começo das opções de inicialização (allowlist em `security`).
    SetLaunchEnv { key: String, value: String },
    /// Argumentos do jogo depois de `%command%`.
    AddLaunchArgs { args: Vec<String> },
    /// Arquivo de texto relativo à pasta de instalação do jogo.
    WriteGameFile { path: String, content: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Scope {
    /// Vazio = qualquer jogo.
    pub app_ids: Vec<String>,
    pub requires_proton: bool,
    pub gpu_vendor: Option<GpuReq>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RecipeSource {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub by: String,
    pub at: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Recipe {
    pub schema: u32,
    pub id: String,
    pub version: String,
    pub trust: Trust,
    pub title: String,
    pub summary: String,
    pub description: String,
    pub category: Category,
    pub purpose: Purpose,
    pub risk: Risk,
    pub scope: Scope,
    pub actions: Vec<RecipeAction>,
    #[serde(default)]
    pub already_included_by_protonfixes: bool,
    #[serde(default)]
    pub conflicts_with: Vec<String>,
    /// Chaves de achados relacionados (`env:KEY=valor`, `sync:esync:off`…).
    #[serde(default)]
    pub related_keys: Vec<String>,
    #[serde(default)]
    pub effects: Vec<String>,
    #[serde(default)]
    pub side_effects: Vec<String>,
    pub sources: Vec<RecipeSource>,
    pub review: Option<Review>,
    /// Só em receitas geradas; o catálogo recusa receitas com este campo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated: Option<GeneratedInfo>,
    /// Só em Smart Packs compostos; o catálogo recusa receitas com este campo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack: Option<PackInfo>,
}

/// Receita que compõe um Smart Pack, com a versão e a impressão digital
/// (SHA-256 da receita) usadas na composição.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackMember {
    pub recipe_id: String,
    pub version: String,
    pub fingerprint: String,
    pub trust: Trust,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackInfo {
    pub members: Vec<PackMember>,
    pub goal: String,
}

/// Origem de uma receita gerada: de qual descoberta, com que evidência e por
/// qual versão do gerador e das regras.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedInfo {
    pub generator_version: String,
    pub rules_version: String,
    pub interpreter: String,
    pub finding_ids: Vec<String>,
    pub evidence: EvidenceLevel,
    pub evidence_note: String,
    pub provenance: Vec<Provenance>,
    pub discovered_at: Option<String>,
}

impl Recipe {
    pub fn env_keys(&self) -> Vec<&str> {
        self.actions
            .iter()
            .filter_map(|a| match a {
                RecipeAction::SetLaunchEnv { key, .. } => Some(key.as_str()),
                _ => None,
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PlanStatus {
    Ready,
    RequiresManualStep,
    Conflict,
    Incompatible,
    AlreadyApplied,
    AlreadyIncluded,
    NotExecutable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ExecMode {
    /// O FRX grava o arquivo (Steam fechada).
    Automatic,
    /// O usuário cola a linha na Steam; o FRX só confere depois.
    Manual,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CheckState {
    Ok,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub label: String,
    pub state: CheckState,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeKind {
    LaunchOptions,
    GameFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlannedChange {
    pub kind: ChangeKind,
    /// Caminho mostrado ao usuário.
    pub target: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub summary: String,
}

/// Prévia calculada pelo núcleo. A interface só devolve `planId` + `fingerprint`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FixPlan {
    pub plan_id: String,
    pub fingerprint: String,
    pub recipe_id: String,
    pub recipe_version: String,
    pub recipe_title: String,
    pub app_id: String,
    pub game_name: String,
    pub status: PlanStatus,
    pub mode: Option<ExecMode>,
    pub reasons: Vec<String>,
    pub checks: Vec<Check>,
    pub changes: Vec<PlannedChange>,
    /// Linha para colar na Steam quando o modo é manual.
    pub manual_line: Option<String>,
    pub backup_note: Option<String>,
    pub steam_running: bool,
    pub game_running: bool,
    pub risk: Risk,
    pub purpose: Purpose,
    pub trust: Trust,
    pub effects: Vec<String>,
    pub side_effects: Vec<String>,
    pub sources: Vec<RecipeSource>,
    pub generated: Option<GeneratedInfo>,
    pub pack: Option<PackInfo>,
    pub created_at: String,
    pub expires_at: u64,
}

impl FixPlan {
    pub fn executable(&self) -> bool {
        matches!(
            self.status,
            PlanStatus::Ready | PlanStatus::RequiresManualStep
        )
    }
}

/// Mudança na variável feita pelo FRX — o bastante para desfazer só ela.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EnvWrite {
    pub key: String,
    pub written: String,
    pub previous: Option<String>,
}

/// Passo concreto e já validado de um plano (vai para o diário).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Step {
    #[serde(rename_all = "camelCase")]
    LaunchOptions {
        config: PathBuf,
        app_id: String,
        file_sha: String,
        before: Option<String>,
        after: String,
        env: Vec<EnvWrite>,
        args: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    GameFile {
        root: PathBuf,
        rel: String,
        target: PathBuf,
        before_sha: Option<String>,
        content: String,
    },
}

impl Step {
    pub fn target(&self) -> &std::path::Path {
        match self {
            Step::LaunchOptions { config, .. } => config,
            Step::GameFile { target, .. } => target,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OpState {
    Checking,
    BackingUp,
    Applying,
    Verifying,
    Applied,
    Failed,
    Conflict,
    RequiresManualStep,
    RollingBack,
    RolledBack,
    RecoveryRequired,
}

impl OpState {
    /// Estados intermediários: se o app fechar aqui, a operação precisa de recuperação.
    pub fn in_flight(self) -> bool {
        matches!(
            self,
            OpState::Checking
                | OpState::BackingUp
                | OpState::Applying
                | OpState::Verifying
                | OpState::RollingBack
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BackupRef {
    pub path: PathBuf,
    /// `false` = o arquivo não existia (rollback remove o que o FRX criou).
    pub existed: bool,
    pub sha256: Option<String>,
    pub size: u64,
    pub mode: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum StepStatus {
    Pending,
    BackedUp,
    Written,
    Verified,
    Reverted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StepRecord {
    pub step: Step,
    pub status: StepStatus,
    pub backup: Option<BackupRef>,
    pub after_sha: Option<String>,
    #[serde(default)]
    pub rollback_backup: Option<BackupRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpEvent {
    pub at: String,
    pub state: OpState,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PendingKind {
    Apply,
    Rollback,
}

/// Passo manual esperado do usuário (colar a linha na Steam).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
    pub kind: PendingKind,
    pub line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub schema: u32,
    pub id: String,
    pub recipe_id: String,
    pub recipe_version: String,
    pub recipe_title: String,
    pub app_id: String,
    pub game_name: String,
    pub state: OpState,
    pub mode: ExecMode,
    pub steps: Vec<StepRecord>,
    pub plan_fingerprint: String,
    pub created_at: String,
    pub updated_at: String,
    pub applied_at: Option<String>,
    pub rolled_back_at: Option<String>,
    pub message: Option<String>,
    pub pending: Option<Pending>,
    pub events: Vec<OpEvent>,
    /// Ausente em operações antigas (v0.4.0: sempre receitas do catálogo).
    #[serde(default)]
    pub recipe_trust: Option<Trust>,
    #[serde(default)]
    pub finding_ids: Vec<String>,
}

impl Operation {
    pub fn env_writes(&self) -> impl Iterator<Item = &EnvWrite> {
        self.steps.iter().flat_map(|s| match &s.step {
            Step::LaunchOptions { env, .. } => env.as_slice(),
            _ => &[],
        })
    }

    /// A operação ocupa as chaves/arquivos dela (para detectar conflitos entre receitas).
    pub fn holds_changes(&self) -> bool {
        matches!(self.state, OpState::Applied | OpState::RecoveryRequired)
            || (self.state == OpState::RequiresManualStep
                && self
                    .pending
                    .as_ref()
                    .is_some_and(|p| p.kind == PendingKind::Rollback))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Effective {
    /// A configuração real ainda tem o que o FRX gravou.
    Active,
    /// Algo removeu ou mudou a configuração depois da aplicação.
    Drifted,
    /// Não foi possível ler o estado real agora.
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppliedFix {
    pub operation_id: String,
    pub recipe_id: String,
    pub recipe_version: String,
    pub recipe_title: String,
    pub app_id: String,
    pub game_name: String,
    pub applied_at: Option<String>,
    pub mode: ExecMode,
    pub effective: Effective,
    pub detail: String,
}

/// Referência guardada nas sessões do Performance Lab.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveFixRef {
    pub recipe_id: String,
    pub recipe_version: String,
    pub operation_id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RollbackPlan {
    pub operation_id: String,
    pub fingerprint: String,
    pub status: PlanStatus,
    pub mode: Option<ExecMode>,
    pub reasons: Vec<String>,
    pub changes: Vec<PlannedChange>,
    pub manual_line: Option<String>,
    pub steam_running: bool,
    pub game_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecipeView {
    #[serde(flatten)]
    pub recipe: Recipe,
    pub executable: bool,
    pub integrity: bool,
    /// Achados da pesquisa (v0.2) que citam o mesmo ajuste.
    pub related_findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub catalog_version: String,
    pub catalog_sha256: String,
    pub recipes: usize,
    pub reviewed: usize,
    pub issues: Vec<String>,
    pub recovery_required: usize,
    pub pending_manual: usize,
    pub busy: bool,
    pub data_dir: String,
}

/// Contexto do jogo montado pelos comandos (biblioteca + hardware).
#[derive(Debug, Clone, Default)]
pub struct GameCtx {
    pub app_id: String,
    pub name: String,
    pub steam_root: Option<PathBuf>,
    pub install_path: Option<PathBuf>,
    pub uses_proton: Option<bool>,
    pub tool_name: Option<String>,
    pub tool_includes_protonfixes: bool,
    pub gpu: Option<GpuReq>,
    /// Versão do driver da GPU principal (para reavaliar receitas geradas).
    pub driver: Option<String>,
    /// Descobertas do cache do núcleo — nunca vindas da interface.
    pub findings: Vec<FixFinding>,
}
