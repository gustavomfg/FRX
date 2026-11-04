//! Fix Engine: correções de sistema de forma transacional.
//!
//! Fluxo: instrução reconhecida na Arch Wiki → prévia (dry-run) → confirmação
//! do usuário → backup verificado → gravação atômica (ou comando `sudo` para o
//! usuário rodar) → verificação no disco → operação registrada no diário. O
//! rollback desfaz só o que a operação fez e recusa sobrescrever mudanças de
//! terceiros.
//!
//! O front-end nunca envia caminhos, valores ou comandos: só o sintoma, o ID da
//! sugestão, o ID da ação e a impressão digital da prévia. Tudo é recalculado
//! aqui antes de qualquer escrita.

pub mod confined;
pub mod files;
pub mod journal;
pub mod models;
pub mod system;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, TryLockError};

use crate::time::now_iso;
use confined::Target;
use files::sha256_hex;
use journal::{new_op_id, Journal};
use models::*;
use system::SystemFixRequest;

pub struct FixEngine {
    root: PathBuf,
    journal: Journal,
    /// Prévias de correções de sistema, com o pedido já resolvido no núcleo.
    system_plans: Mutex<HashMap<String, (FixPlan, SystemFixRequest)>>,
    /// Raiz do sistema. Sempre `/` fora dos testes: com ela o FRX não grava
    /// em `/etc`, só prepara o arquivo e mostra o comando `sudo`.
    system_root: PathBuf,
    /// Testes: modo manual com raiz fictícia, para simular o `sudo install`.
    system_force_manual: bool,
    /// Uma operação de escrita por vez (aplicação ou rollback).
    exec: Mutex<()>,
}

impl FixEngine {
    pub fn new(root: PathBuf) -> Self {
        // Diário e backups ficam em pastas privadas (0700) desde o início.
        for d in [root.clone(), root.join("backups"), root.join("operations")] {
            if let Err(e) = files::private_dir(&d) {
                eprintln!("[frx] fix-engine: {e}");
            }
        }
        let journal = Journal::new(root.join("operations"));
        let engine = Self {
            root,
            journal,
            system_plans: Mutex::default(),
            system_root: PathBuf::from("/"),
            system_force_manual: false,
            exec: Mutex::new(()),
        };
        engine.recover();
        engine
    }

    /// Raiz de sistema fictícia: o núcleo grava nela sozinho (modo automático),
    /// para testar aplicação, verificação e rollback sem `sudo`.
    #[cfg(test)]
    pub fn with_system_root(mut self, root: PathBuf, manual: bool) -> Self {
        self.system_root = root;
        self.system_force_manual = manual;
        self
    }

    #[cfg(test)]
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn backups_dir(&self, op: &str) -> PathBuf {
        self.root.join("backups").join(op)
    }

    // ── Diário ───────────────────────────────────────────────────────────

    /// Todas as operações, mais recentes primeiro.
    pub fn operations(&self) -> Vec<Operation> {
        self.journal.list()
    }

    // ── Verificação ──────────────────────────────────────────────────────

    /// Confere no disco um passo manual ou uma operação em recuperação.
    pub fn verify(&self, op_id: &str) -> Result<Operation, String> {
        let _guard = self.exec.lock().map_err(|_| "estado indisponível")?;
        let op = self.journal.load(op_id)?;
        if op.manual_recovery_required {
            return Ok(op);
        }
        self.verify_system(op)
    }

    // ── Rollback ─────────────────────────────────────────────────────────

    pub fn prepare_rollback(&self, op_id: &str) -> Result<RollbackPlan, String> {
        let op = self.journal.load(op_id)?;
        Ok(self.rollback_plan(&op)?.0)
    }

    /// Prévia do rollback e, para cada passo, se ele precisa restaurar o backup.
    fn rollback_plan(&self, op: &Operation) -> Result<(RollbackPlan, Vec<bool>), String> {
        if op.manual_recovery_required {
            return Err(format!(
                "É necessária revisão manual do conteúdo externo preservado. {}",
                op.message
                    .as_deref()
                    .unwrap_or("Consulte os eventos da operação antes de restaurar arquivos.")
            ));
        }
        let pending_apply = op.state == OpState::RequiresManualStep
            && op
                .pending
                .as_ref()
                .is_some_and(|p| p.kind == PendingKind::Apply);
        if !matches!(op.state, OpState::Applied | OpState::RecoveryRequired) && !pending_apply {
            return Err("Só correções aplicadas (ou em recuperação) podem ser desfeitas.".into());
        }
        self.system_rollback_plan(op, pending_apply)
    }

    pub fn rollback(
        &self,
        op_id: &str,
        fingerprint: &str,
    ) -> Result<Operation, String> {
        let _guard = match self.exec.try_lock() {
            Ok(g) => g,
            Err(TryLockError::WouldBlock) => {
                return Err("Outra correção está sendo aplicada ou desfeita. Aguarde.".into())
            }
            Err(TryLockError::Poisoned(_)) => return Err("estado indisponível".into()),
        };
        let mut op = self.journal.load(op_id)?;
        let (plan, restores) = self.rollback_plan(&op)?;
        if plan.fingerprint != fingerprint {
            return Err(
                "O estado mudou desde a prévia do rollback. Nada foi alterado; revise de novo."
                    .into(),
            );
        }
        match plan.status {
            PlanStatus::Ready => {}
            PlanStatus::RequiresManualStep => {
                op.pending = Some(Pending {
                    kind: PendingKind::Rollback,
                    line: plan.manual_line.unwrap_or_default(),
                });
                self.journal.transition(
                    &mut op,
                    OpState::RequiresManualStep,
                    "Aguardando você rodar os comandos com sudo para desfazer. O FRX confere o arquivo depois.",
                )?;
                return Ok(op);
            }
            _ => return Err(plan.reasons.join(" ")),
        }
        // Cancelar uma prévia que nunca foi rodada: nada a restaurar.
        if op.state == OpState::RequiresManualStep && plan.changes.is_empty() {
            op.pending = None;
            if journal::valid_op_id(&op.id) {
                let _ = std::fs::remove_dir_all(system::staged_dir(&self.root, &op.id));
            }
            self.journal.transition(
                &mut op,
                OpState::Failed,
                "Cancelada antes de ser aplicada; nada foi alterado.",
            )?;
            return Ok(op);
        }
        self.journal
            .transition(&mut op, OpState::RollingBack, "Desfazendo a correção.")?;
        let dir = self.backups_dir(&op.id);
        // A retry after a crash must not collide with an earlier, preserved
        // rollback backup: each attempt gets new names.
        let rollback_attempt = new_op_id();
        let result = (|| -> Result<(), String> {
            for (i, restore) in restores.into_iter().enumerate() {
                if !restore {
                    op.steps[i].status = StepStatus::Reverted;
                    continue;
                }
                let name = format!("rollback-{rollback_attempt}-{i}");
                let opened = open_step(&op.steps[i].step)?;
                op.steps[i].rollback_backup = Some(opened.backup(&dir, &name)?);
                self.journal.save(&op)?;
                opened.restore(
                    op.steps[i].backup.as_ref().ok_or("backup ausente")?,
                    op.steps[i].after_sha.as_deref(),
                )?;
                op.steps[i].status = StepStatus::Reverted;
                self.journal.save(&op)?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                op.rolled_back_at = Some(now_iso());
                op.pending = None;
                self.journal.transition(
                    &mut op,
                    OpState::RolledBack,
                    "Desfeita e verificada. Só o que esta correção adicionou foi removido.",
                )?;
            }
            Err(e) => {
                op.manual_recovery_required |= e.contains("RECUPERAÇÃO:");
                self.journal.transition(
                    &mut op,
                    OpState::RecoveryRequired,
                    format!("Rollback interrompido: {e} Confira o estado antes de tentar de novo."),
                )?;
            }
        }
        Ok(op)
    }

    // ── Aplicação automática (só com raiz fictícia) ──────────────────────

    /// Grava os passos com backup, confere o resultado e restaura o backup se algo falhar.
    fn run_apply(&self, op: &mut Operation) {
        if let Err(e) = self.try_apply(op) {
            if e.starts_with("RECUPERAÇÃO:") {
                op.manual_recovery_required = true;
                let _ = self.journal.transition(op, OpState::RecoveryRequired, e);
                return;
            }
            let written = op.steps.iter().any(|s| {
                matches!(
                    s.status,
                    StepStatus::Writing | StepStatus::Written | StepStatus::Verified
                )
            });
            if !written {
                let state = if e.starts_with("CONFLITO:") {
                    OpState::Conflict
                } else {
                    OpState::Failed
                };
                let msg = format!(
                    "{} Nenhum arquivo foi alterado.",
                    e.trim_start_matches("CONFLITO:").trim()
                );
                let _ = self.journal.transition(op, state, msg);
            } else {
                match self.undo_written(op) {
                    Ok(()) => {
                        let _ = self.journal.transition(
                            op,
                            OpState::Failed,
                            format!("{e} As alterações foram desfeitas a partir do backup."),
                        );
                    }
                    Err(u) => {
                        op.manual_recovery_required |= u.contains("RECUPERAÇÃO:");
                        let _ = self.journal.transition(
                            op,
                            OpState::RecoveryRequired,
                            format!("{e} Não foi possível desfazer automaticamente: {u}"),
                        );
                    }
                }
            }
        }
    }

    fn try_apply(&self, op: &mut Operation) -> Result<(), String> {
        // 1. Backup de tudo antes de qualquer escrita.
        self.journal
            .transition(op, OpState::BackingUp, "Criando backup verificado.")?;
        let dir = self.backups_dir(&op.id);
        for (i, rec) in op.steps.iter_mut().enumerate() {
            let name = format!("{i}-{}", file_name(&rec.step.path));
            let b = open_step(&rec.step)?.backup(&dir, &name)?;
            if b.sha256 != rec.step.before_sha {
                return Err(format!("CONFLITO: {} mudou desde a prévia.", rec.step.path));
            }
            rec.backup = Some(b);
            rec.status = StepStatus::BackedUp;
        }
        self.journal.save(op)?;

        // 2. Escrita, com a conferência do conteúdo logo antes de cada passo.
        self.journal
            .transition(op, OpState::Applying, "Gravando as alterações.")?;
        for i in 0..op.steps.len() {
            op.steps[i].after_sha = Some(sha256_hex(op.steps[i].step.content.as_bytes()));
            op.steps[i].status = StepStatus::Writing;
            self.journal.save(op)?;
            write_step(&op.steps[i].step)?;
            op.steps[i].status = StepStatus::Written;
            self.journal.save(op)?;
        }

        // 3. Verificação do estado real.
        self.journal
            .transition(op, OpState::Verifying, "Conferindo o resultado no disco.")?;
        for rec in &mut op.steps {
            verify_written(rec)?;
            rec.status = StepStatus::Verified;
        }
        op.applied_at = Some(now_iso());
        op.pending = None;
        self.journal
            .transition(op, OpState::Applied, "Aplicada e verificada no disco.")?;
        Ok(())
    }

    /// Restaura do backup o que foi gravado — só se ninguém mexeu depois.
    fn undo_written(&self, op: &mut Operation) -> Result<(), String> {
        for rec in op.steps.iter_mut().rev() {
            if !matches!(
                rec.status,
                StepStatus::Writing | StepStatus::Written | StepStatus::Verified
            ) {
                continue;
            }
            let path = rec.step.path.clone();
            if step_sha(&rec.step)? != rec.after_sha {
                if rec.status == StepStatus::Writing
                    && step_sha(&rec.step)? == rec.backup.as_ref().and_then(|b| b.sha256.clone())
                {
                    rec.status = StepStatus::Reverted;
                    continue;
                }
                return Err(format!("{path} foi alterado por outro programa."));
            }
            open_step(&rec.step)?.restore(
                rec.backup.as_ref().ok_or("backup ausente")?,
                rec.after_sha.as_deref(),
            )?;
            rec.status = StepStatus::Reverted;
        }
        self.journal.save(op)
    }

    // ── Recuperação ──────────────────────────────────────────────────────

    /// Ao abrir: operações que ficaram no meio do caminho nunca contam como concluídas.
    fn recover(&self) {
        for mut op in self
            .journal
            .list()
            .into_iter()
            .filter(|o| o.state.in_flight())
        {
            let untouched = op.state != OpState::RollingBack
                && op
                    .steps
                    .iter()
                    .all(|s| matches!(s.status, StepStatus::Pending | StepStatus::BackedUp))
                && op.steps.iter().all(|s| {
                    let cur = step_sha(&s.step).ok().flatten();
                    match &s.backup {
                        Some(b) => cur == b.sha256,
                        None => true,
                    }
                });
            let (state, msg) = if untouched {
                (
                    OpState::Failed,
                    "O FRX foi fechado antes de gravar qualquer alteração. Nada mudou.",
                )
            } else {
                (OpState::RecoveryRequired, "O FRX foi fechado no meio da operação. Confira o estado real ou desfaça a correção.")
            };
            let _ = self.journal.transition(&mut op, state, msg);
            if journal::valid_op_id(&op.id) {
                let _ = std::fs::remove_dir_all(system::staged_dir(&self.root, &op.id));
            }
        }
    }
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or("arquivo")
}

/// Abre o arquivo de sistema de um passo, confinado à allowlist.
fn open_step(step: &Step) -> Result<Target, String> {
    Target::open_system(&step.system_root, &step.path, step.identity.as_ref())
}

fn step_sha(step: &Step) -> Result<Option<String>, String> {
    open_step(step)?.sha()
}

/// Grava o conteúdo do passo, se o arquivo ainda for o que a prévia viu.
fn write_step(step: &Step) -> Result<(), String> {
    let opened = open_step(step)?;
    if opened.sha()? != step.before_sha {
        return Err(format!("CONFLITO: {} foi alterado por outro programa.", step.path));
    }
    opened.write(step.content.as_bytes(), step.before_sha.as_deref(), 0o644)
}

fn verify_written(rec: &StepRecord) -> Result<(), String> {
    if step_sha(&rec.step)? != rec.after_sha {
        return Err("A verificação falhou: o arquivo não tem o conteúdo gravado.".into());
    }
    Ok(())
}

pub fn data_root(app_data: &Path) -> PathBuf {
    app_data.join("fix-engine")
}
