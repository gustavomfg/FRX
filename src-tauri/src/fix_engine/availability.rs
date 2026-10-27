//! Current applicability is obtained from the same planner as the actual
//! preview. Recipe-generation success alone never authorizes a product action.
use super::{
    dynamic::{Assessment, DynStatus},
    models::*,
};
use crate::{
    fixes::models::{ActionKind, FixFinding},
    time::now_iso,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AvailabilityState {
    Available,
    Temporary,
    Investigation,
    Incompatible,
    Applied,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AvailabilityReason {
    pub code: String,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Availability {
    pub state: AvailabilityState,
    pub reasons: Vec<AvailabilityReason>,
    pub checked_at: String,
    pub plan_status: Option<PlanStatus>,
    pub plan_fingerprint: Option<String>,
}
impl Default for Availability {
    fn default() -> Self {
        Self::reason(
            AvailabilityState::Investigation,
            "not-validated",
            "A aplicabilidade atual ainda não foi validada pelo planejador.",
        )
    }
}
impl Availability {
    pub fn reason(state: AvailabilityState, code: &str, message: &str) -> Self {
        Self {
            state,
            reasons: vec![AvailabilityReason {
                code: code.into(),
                message: message.into(),
            }],
            checked_at: now_iso(),
            plan_status: None,
            plan_fingerprint: None,
        }
    }
    pub fn can_compose(&self) -> bool {
        matches!(
            self.state,
            AvailabilityState::Available
                | AvailabilityState::Temporary
                | AvailabilityState::Applied
        )
    }
}
fn check_code(label: &str) -> &'static str {
    match label {
        "Jogo aberto" => "game-running",
        "Steam aberta" => "steam-running",
        "Conflitos" => "active-operation-conflict",
        "Operação pendente" => "pending-operation",
        "Jogo" => "game-scope",
        "GPU" => "hardware-incompatible",
        "Hardware e Proton" => "environment-incompatible",
        "Hardware não confirmado" => "hardware-unconfirmed",
        "Proton" => "proton-incompatible",
        "Receita revisada" => "recipe-trust",
        "Receita gerada" | "Smart Pack" => "recipe-validation",
        "Segurança" => "unsafe-action",
        "Artefato requer revisão" => "artifact-review",
        "Arquivo do jogo" => "file-target",
        "Configuração conhecida" | "Configuração DXVK" => "configuration-mismatch",
        "Arquivos conflitantes" => "file-conflict",
        "Permissão de escrita" => "destination-permission",
        "Backup e recuperação" => "recovery-storage-unavailable",
        "Configuração da Steam" => "steam-config-unavailable",
        "Opções atuais" => "launch-options-conflict",
        "Edição segura" => "automatic-edit-unsupported",
        "Protonfixes" => "already-included",
        "Histórico" | "Estado atual" => "already-applied",
        _ => "planner-blocked",
    }
}
/// A temporary blocker is a known operational prerequisite. A file error or
/// manual-only parser fallback cannot be called available by optimistic inference.
pub fn from_plan(plan: &FixPlan) -> Availability {
    let blocking: Vec<_> = plan
        .checks
        .iter()
        .filter(|c| {
            c.state == CheckState::Fail
                || (c.state == CheckState::Warn
                    && ["Steam aberta", "Edição segura"].contains(&c.label.as_str()))
        })
        .collect();
    let temporary_only = !blocking.is_empty()
        && blocking.iter().all(|c| {
            [
                "Jogo aberto",
                "Steam aberta",
                "Conflitos",
                "Operação pendente",
            ]
            .contains(&c.label.as_str())
        });
    let state = match plan.status {
        PlanStatus::Ready if plan.mode == Some(ExecMode::Automatic) && !plan.changes.is_empty() => {
            AvailabilityState::Available
        }
        PlanStatus::AlreadyApplied => AvailabilityState::Applied,
        PlanStatus::Incompatible => AvailabilityState::Incompatible,
        PlanStatus::Conflict | PlanStatus::RequiresManualStep if temporary_only => {
            AvailabilityState::Temporary
        }
        _ => AvailabilityState::Investigation,
    };
    let mut reasons: Vec<_> = blocking
        .iter()
        .map(|c| AvailabilityReason {
            code: check_code(&c.label).into(),
            message: if c.label == "Steam aberta" {
                "Feche a Steam normalmente e atualize a análise para o FRX aplicar automaticamente."
                    .into()
            } else {
                c.detail.clone()
            },
        })
        .collect();
    if reasons.is_empty() {
        reasons = plan
            .reasons
            .iter()
            .map(|r| AvailabilityReason {
                code: "planner-state".into(),
                message: r.clone(),
            })
            .collect();
    }
    Availability {
        state,
        reasons,
        checked_at: now_iso(),
        plan_status: Some(plan.status),
        plan_fingerprint: Some(plan.fingerprint.clone()),
    }
}
pub fn without_recipe(a: &Assessment, finding: Option<&FixFinding>) -> Availability {
    let state = if a.status == DynStatus::Incompatible {
        AvailabilityState::Incompatible
    } else {
        AvailabilityState::Investigation
    };
    let code = match a.status {
        DynStatus::Invalid => "source-or-game-scope",
        DynStatus::InsufficientData => "source-scope-unconfirmed",
        DynStatus::Incompatible => "environment-incompatible",
        DynStatus::AlreadyIncluded => "already-included",
        DynStatus::Contested => "conflicting-evidence",
        DynStatus::NoActions => "no-supported-action",
        _ => match finding {
            Some(f) if !f.artifacts.is_empty() => "artifact-review",
            Some(f) if f.actions.iter().any(|a| a.conditional) => "conditional-procedure",
            Some(f) if f.actions.iter().any(|a| a.kind == ActionKind::Winetricks) => {
                "dependency-unsupported"
            }
            Some(f) if f.actions.iter().any(|a| a.kind == ActionKind::Registry) => {
                "registry-unsupported"
            }
            Some(f) if f.actions.iter().any(|a| a.kind == ActionKind::DllOverride) => {
                "executable-component-review"
            }
            Some(f)
                if f.actions
                    .iter()
                    .any(|a| a.kind == ActionKind::ProtonVersion) =>
            {
                "proton-selection-unsupported"
            }
            Some(f) if f.actions.iter().any(|a| a.kind == ActionKind::ConfigFile) => {
                "configuration-incomplete"
            }
            _ if a.status == DynStatus::Blocked => "unsafe-action",
            _ => "recipe-not-generated",
        },
    };
    let mut result = Availability::reason(
        state,
        code,
        "Não há um plano automático completo para esta descoberta.",
    );
    if !a.reasons.is_empty() {
        result.reasons = a
            .reasons
            .iter()
            .map(|r| AvailabilityReason {
                code: code.into(),
                message: r.clone(),
            })
            .collect();
    }
    result
}
