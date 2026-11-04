//! Modelos do Fix Engine para correções de sistema: prévia, operação, passos
//! do diário e rollback. Todo passo é um arquivo da allowlist de `confined`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::confined::FileIdentity;

pub const JOURNAL_SCHEMA: u32 = 1;

/// Risco de uma ação, mostrado no cartão e na prévia.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Risk {
    Low,
    Moderate,
    High,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PlanStatus {
    Ready,
    RequiresManualStep,
    Conflict,
    Incompatible,
    AlreadyApplied,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ExecMode {
    /// O FRX grava o arquivo sozinho (só com uma raiz de sistema fictícia).
    Automatic,
    /// O FRX prepara o arquivo e mostra o comando `sudo` para o usuário rodar.
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
    /// Arquivo da allowlist de sistema (`/etc/modprobe.d`, `/etc/systemd/system`, `/etc/default/grub`).
    SystemFile,
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

/// Origem exibida no cartão: o trecho da Arch Wiki de onde veio a instrução.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceLink {
    pub label: String,
    pub url: String,
}

/// Prévia calculada pelo núcleo. A interface só devolve `planId` + `fingerprint`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FixPlan {
    pub plan_id: String,
    pub fingerprint: String,
    pub title: String,
    /// Nome do laptop para o qual a prévia foi feita.
    #[serde(alias = "gameName")]
    pub laptop: String,
    pub status: PlanStatus,
    pub mode: Option<ExecMode>,
    pub reasons: Vec<String>,
    pub checks: Vec<Check>,
    pub changes: Vec<PlannedChange>,
    /// Comando `sudo` (ou sequência) para o modo manual.
    pub manual_line: Option<String>,
    pub backup_note: Option<String>,
    pub risk: Risk,
    pub effects: Vec<String>,
    pub side_effects: Vec<String>,
    pub sources: Vec<SourceLink>,
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

/// Arquivo de sistema de um passo, já validado. Vai para o diário.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    /// Raiz do sistema: `/` em produção, pasta temporária nos testes.
    pub system_root: PathBuf,
    /// Caminho absoluto exibido (`/etc/modprobe.d/frx-x.conf`).
    pub path: String,
    pub identity: Option<FileIdentity>,
    pub before_sha: Option<String>,
    pub content: String,
    /// Comandos exibidos (nunca executados pelo FRX).
    #[serde(default)]
    pub post_apply: Vec<String>,
    #[serde(default)]
    pub pre_rollback: Vec<String>,
    #[serde(default)]
    pub post_rollback: Vec<String>,
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
    Writing,
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

/// Passo manual esperado do usuário (rodar o comando com `sudo`).
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
    /// Título da correção (antes `recipeTitle`).
    #[serde(default, alias = "recipeTitle")]
    pub title: String,
    /// Nome do laptop (antes `gameName`).
    #[serde(default, alias = "gameName")]
    pub laptop: String,
    /// Concurrent content retained outside the normal backup sequence needs
    /// manual review; presence checks alone must never dismiss that state.
    #[serde(default)]
    pub manual_recovery_required: bool,
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
}

impl Operation {
    /// A operação ocupa os arquivos dela (para detectar conflitos entre correções).
    pub fn holds_changes(&self) -> bool {
        matches!(self.state, OpState::Applied | OpState::RecoveryRequired)
            || (self.state == OpState::RequiresManualStep
                && self
                    .pending
                    .as_ref()
                    .is_some_and(|p| p.kind == PendingKind::Rollback))
    }
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
}
