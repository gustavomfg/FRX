//! Estado operacional de cada pacote, derivado da prévia do Fix Engine e do
//! diário de operações. Não diz nada sobre ganho de FPS.
//!
//! | Estado | Quando |
//! |---|---|
//! | `candidate` | composição válida, prévia indisponível agora (ex.: jogo fora da biblioteca) |
//! | `ready` | a prévia permite aplicar (automático ou passo manual) |
//! | `conflict` | a prévia encontrou conflito (outra correção ativa, jogo aberto…) |
//! | `testing` | operação em andamento ou aguardando o passo manual na Steam |
//! | `applied` | o diário diz aplicada, mas o estado real não pôde ser lido agora |
//! | `verified` | aplicada e conferida no disco agora |
//! | `failed` | a última operação falhou ou a configuração foi alterada fora do FRX |
//! | `partially_recovered` | operação interrompida; precisa de recuperação |
//! | `rolled_back` | a última operação foi desfeita (pode ser aplicado de novo) |
//! | `unsupported` | incompatível com o sistema ou recusado pela validação |

use super::models::*;
use crate::fix_engine::models::{AppliedFix, Effective, FixPlan, OpState, Operation, PlanStatus};

pub fn op_env(op: &Operation) -> Vec<EnvPair> {
    let mut v: Vec<EnvPair> = op
        .env_writes()
        .map(|w| EnvPair {
            key: w.key.clone(),
            value: w.written.clone(),
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

fn set(p: &mut SmartPack, s: PackState, detail: impl Into<String>) {
    p.state = s;
    p.state_detail = detail.into();
}

pub fn derive(
    pack: &mut SmartPack,
    plan: Option<Result<FixPlan, String>>,
    ops: &[Operation],
    applied: &[AppliedFix],
    current_env: &[EnvPair],
) {
    if pack.kind == PackKind::Baseline {
        if current_env.is_empty() {
            set(
                pack,
                PackState::Verified,
                "Ativo agora: nenhuma correção do FRX aplicada neste jogo.",
            );
        } else {
            set(
                pack,
                PackState::Ready,
                "Para voltar ao baseline, desfaça as correções do FRX ativas (Restaurar).",
            );
        }
        return;
    }
    pack.plan_status = plan
        .as_ref()
        .and_then(|p| p.as_ref().ok())
        .map(|p| p.status);
    let latest = ops
        .iter()
        .filter(|o| op_env(o) == pack.env)
        .max_by(|a, b| a.updated_at.cmp(&b.updated_at).then(a.id.cmp(&b.id)));
    if let Some(op) = latest {
        pack.operation_id = Some(op.id.clone());
        match op.state {
            OpState::Applied => {
                let eff = applied
                    .iter()
                    .find(|a| a.operation_id == op.id)
                    .map(|a| a.effective);
                match eff {
                    Some(Effective::Active) => set(
                        pack,
                        PackState::Verified,
                        "Aplicado e conferido no disco agora.",
                    ),
                    Some(Effective::Drifted) => set(
                        pack,
                        PackState::Failed,
                        "Aplicado, mas a configuração foi alterada fora do FRX depois.",
                    ),
                    _ => set(
                        pack,
                        PackState::Applied,
                        "Aplicado segundo o diário; o estado real não pôde ser lido agora.",
                    ),
                }
                return;
            }
            OpState::RequiresManualStep => {
                set(
                    pack,
                    PackState::Testing,
                    "Aguardando o passo manual na Steam; ainda não conta como aplicado.",
                );
                return;
            }
            s if s.in_flight() => {
                set(pack, PackState::Testing, "Operação em andamento.");
                return;
            }
            OpState::RecoveryRequired => {
                set(
                    pack,
                    PackState::PartiallyRecovered,
                    "A última operação foi interrompida e precisa de recuperação no Histórico.",
                );
                return;
            }
            _ => {}
        }
    }
    match plan {
        None => set(pack, PackState::Candidate, "Prévia não calculada."),
        Some(Err(e)) => set(pack, PackState::Candidate, e),
        Some(Ok(p)) => match p.status {
            PlanStatus::Ready | PlanStatus::RequiresManualStep => {
                let detail = if p.status == PlanStatus::Ready {
                    "Pode ser aplicado agora, com prévia, backup e rollback."
                } else {
                    "Pode ser aplicado com um passo manual (a Steam está aberta)."
                };
                match latest.map(|o| o.state) {
                    Some(OpState::RolledBack) => set(
                        pack,
                        PackState::RolledBack,
                        format!("Desfeito antes. {detail}"),
                    ),
                    Some(OpState::Failed | OpState::Conflict) => set(
                        pack,
                        PackState::Failed,
                        format!(
                            "A última tentativa não foi concluída ({}). {detail}",
                            latest.and_then(|o| o.message.clone()).unwrap_or_default()
                        ),
                    ),
                    _ => set(pack, PackState::Ready, detail),
                }
            }
            PlanStatus::Conflict => set(
                pack,
                PackState::Conflict,
                p.reasons.first().cloned().unwrap_or_default(),
            ),
            PlanStatus::AlreadyApplied => {
                set(pack, PackState::Verified, "A configuração já está assim.")
            }
            PlanStatus::Incompatible | PlanStatus::AlreadyIncluded | PlanStatus::NotExecutable => {
                set(
                    pack,
                    PackState::Unsupported,
                    p.reasons.first().cloned().unwrap_or_default(),
                )
            }
        },
    }
}
