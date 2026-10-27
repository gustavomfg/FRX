//! Fix Engine: aplica correções de forma transacional.
//!
//! Receitas vêm de dois caminhos: o catálogo local revisado (embutido) e o
//! gerador dinâmico (`dynamic`), que transforma descobertas com ações
//! reconhecidas em receitas `generated` sem cadastro manual.
//!
//! Fluxo: receita → verificação de compatibilidade →
//! plano (dry-run) → confirmação do usuário → backup verificado → gravação
//! atômica → verificação → operação registrada no diário. O rollback desfaz
//! só o que a operação fez e recusa sobrescrever mudanças de terceiros.
//!
//! O front-end nunca envia caminhos, valores ou comandos: só o ID da receita,
//! o ID do plano e a impressão digital que ele recebeu. Tudo é recalculado
//! aqui antes de qualquer escrita.

pub mod catalog;
pub mod dynamic;
pub mod files;
pub mod journal;
pub mod models;
pub mod planner;
pub mod security;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, TryLockError};

use crate::fixes::models::{ActionKind, FixFinding};
use crate::steam::launch_options::{remove_args, revert_env, LaunchLine, Revert};
use crate::steam::{localconfig, processes};
use crate::time::{now_iso, now_unix};
use catalog::Catalog;
use dynamic::{analyzer, registry::Registry, AssessmentView};
use files::{sha256_file, sha256_hex};
use journal::{new_op_id, Journal};
use models::*;
use planner::{launch_target_label, Computed};

/// Conteúdo novo de cada passo do rollback (`None` = nada a gravar).
type RollbackWrites = Vec<Option<Vec<u8>>>;

pub const PROGRESS_EVENT: &str = "fix-engine://progress";

pub struct FixEngine {
    root: PathBuf,
    journal: Journal,
    catalog: Catalog,
    registry: Registry,
    proc_root: PathBuf,
    plans: Mutex<HashMap<String, (FixPlan, Vec<Step>)>>,
    /// Uma operação de escrita por vez (aplicação ou rollback).
    exec: Mutex<()>,
}

impl FixEngine {
    pub fn new(root: PathBuf) -> Self {
        // Só em builds de desenvolvimento: permite validar o fluxo automático
        // contra uma Steam isolada enquanto a Steam real está aberta. O
        // binário de release sempre lê o /proc verdadeiro.
        #[cfg(debug_assertions)]
        let proc_root = std::env::var_os("FRX_DEV_PROC_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/proc"));
        #[cfg(not(debug_assertions))]
        let proc_root = PathBuf::from("/proc");
        Self::with(root, Catalog::bundled(), proc_root)
    }

    pub fn with(root: PathBuf, catalog: Catalog, proc_root: PathBuf) -> Self {
        // Diário e backups ficam em pastas privadas (0700) desde o início.
        for d in [
            root.clone(),
            root.join("backups"),
            root.join("operations"),
            root.join("generated"),
        ] {
            if let Err(e) = files::private_dir(&d) {
                eprintln!("[frx] fix-engine: {e}");
            }
        }
        let journal = Journal::new(root.join("operations"));
        let registry = Registry::new(root.join("generated"));
        let engine = Self {
            root,
            journal,
            catalog,
            registry,
            proc_root,
            plans: Mutex::default(),
            exec: Mutex::new(()),
        };
        engine.recover();
        engine
    }

    /// Pasta de dados do Fix Engine (`fix-engine/`).
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn backups_dir(&self, op: &str) -> PathBuf {
        self.root.join("backups").join(op)
    }

    // ── Catálogo e estado ────────────────────────────────────────────────

    pub fn recipes(&self, findings: &[FixFinding]) -> Vec<RecipeView> {
        self.catalog
            .recipes
            .iter()
            .map(|r| RecipeView {
                performance_warnings: r.performance_warnings(),
                executable: r.trust == Trust::Reviewed,
                integrity: true,
                related_findings: findings
                    .iter()
                    .filter(|f| related(r, f))
                    .map(|f| f.id.clone())
                    .collect(),
                recipe: r.clone(),
            })
            .collect()
    }

    pub fn recipe(&self, id: &str) -> Result<&Recipe, String> {
        self.catalog
            .get(id)
            .ok_or_else(|| "Receita não encontrada no catálogo local.".into())
    }

    fn analysis_ctx<'a>(&'a self, game: &'a GameCtx) -> analyzer::Ctx<'a> {
        analyzer::Ctx {
            app_id: &game.app_id,
            gpu: game.gpu,
            tool_name: game.tool_name.as_deref(),
            catalog: &self.catalog,
        }
    }

    /// Receita do catálogo ou gerada. Uma receita gerada é sempre refeita a
    /// partir das descobertas do cache e revalidada — nunca lida do registro.
    pub fn resolve(&self, id: &str, game: &GameCtx) -> Result<Recipe, String> {
        if id == format!("frx.capture.{}", game.app_id) {
            let data = self
                .root
                .parent()
                .ok_or("Pasta de dados do FRX indisponível")?;
            let config = data.join("mangohud").join(format!("{}.conf", game.app_id));
            let mut recipe = self.recipe("frx.diagnostics.proton-log")?.clone();
            recipe.id = id.to_owned();
            recipe.title = "Preparar MangoHud para este jogo (Vulkan)".into();
            recipe.summary = "Ativa a camada Vulkan e a configuração própria do FRX, preservando as opções existentes.".into();
            recipe.description = "Preparação da captura com prévia, backup e rollback. Feche a Steam e o jogo para aplicar pelo FRX. Não altera a configuração global do MangoHud.".into();
            recipe.scope.app_ids = vec![game.app_id.clone()];
            recipe.scope.requires_proton = false;
            recipe.actions = vec![
                RecipeAction::SetLaunchEnv {
                    key: "MANGOHUD".into(),
                    value: "1".into(),
                },
                RecipeAction::SetLaunchEnv {
                    key: "MANGOHUD_CONFIGFILE".into(),
                    value: config.to_string_lossy().into_owned(),
                },
            ];
            recipe.related_keys.clear();
            recipe.sources = vec![RecipeSource {
                label: "MangoHud — configuração da camada Vulkan".into(),
                url: "https://github.com/flightlessmango/MangoHud".into(),
            }];
            recipe.review = Some(Review { by: "FRX".into(), at: "2026-10-26".into(), notes: "Preparação interna restrita à configuração gerada pelo FRX; não vem de descobertas.".into() });
            recipe.effects = vec![
                "Habilita o MangoHud em jogos Vulkan e nas traduções Vulkan do Proton.".into(),
            ];
            recipe.side_effects = vec!["Jogos OpenGL podem precisar do wrapper mangohud; a configuração automática suporta Vulkan.".into()];
            return Ok(recipe);
        }
        if let Some(r) = self.catalog.get(id) {
            return Ok(r.clone());
        }
        if security::valid_pack_id(id) {
            return crate::smart_packs::resolve_pack(self, id, game);
        }
        if !security::valid_generated_id(id) {
            return Err("Receita não encontrada.".into());
        }
        analyzer::resolve(&game.findings, &self.analysis_ctx(game), id)
    }

    /// Analisa as descobertas do jogo (gerador dinâmico) e atualiza o registro.
    pub fn analyze(&self, game: &GameCtx) -> Result<Vec<AssessmentView>, String> {
        let ctx = self.analysis_ctx(game);
        let sys = dynamic::registry::System {
            tool: game.tool_name.as_deref(),
            gpu: game.gpu,
            driver: game.driver.as_deref(),
            catalog: &self.catalog.sha256,
        };
        let (entries, _) = self
            .registry
            .refresh(&game.app_id, &game.findings, &sys, |f| {
                analyzer::assess(f, &ctx)
            })?;
        let mut list: Vec<dynamic::Assessment> =
            entries.iter().map(|e| e.assessment.clone()).collect();
        analyzer::merge_duplicates(&mut list);
        Ok(entries
            .into_iter()
            .zip(list)
            .map(|(e, assessment)| AssessmentView {
                assessment,
                generated_at: e.generated_at,
                updated_at: e.updated_at,
                discovered_at: e.discovered_at,
                reevaluated: e.reevaluated,
            })
            .collect())
    }

    pub fn status(&self) -> EngineStatus {
        let ops = self.journal.list();
        EngineStatus {
            catalog_version: self.catalog.version.clone(),
            catalog_sha256: self.catalog.sha256.clone(),
            recipes: self.catalog.recipes.len(),
            reviewed: self
                .catalog
                .recipes
                .iter()
                .filter(|r| r.trust == Trust::Reviewed)
                .count(),
            issues: self.catalog.issues.clone(),
            recovery_required: ops
                .iter()
                .filter(|o| o.state == OpState::RecoveryRequired)
                .count(),
            pending_manual: ops
                .iter()
                .filter(|o| o.state == OpState::RequiresManualStep)
                .count(),
            busy: matches!(self.exec.try_lock(), Err(TryLockError::WouldBlock)),
            data_dir: self.root.display().to_string(),
        }
    }

    pub fn operations(&self, app_id: Option<&str>) -> Vec<Operation> {
        self.journal
            .list()
            .into_iter()
            .filter(|o| app_id.is_none_or(|a| o.app_id == a))
            .collect()
    }

    pub fn operation(&self, id: &str) -> Result<Operation, String> {
        self.journal.load(id)
    }

    /// Correções aplicadas, com o estado **real** relido do disco agora.
    pub fn applied(&self, app_id: Option<&str>) -> Vec<AppliedFix> {
        self.operations(app_id)
            .into_iter()
            .filter(|o| o.state == OpState::Applied)
            .map(|o| {
                let (effective, detail) = effective(&o);
                AppliedFix {
                    operation_id: o.id.clone(),
                    recipe_id: o.recipe_id.clone(),
                    recipe_version: o.recipe_version.clone(),
                    recipe_title: o.recipe_title.clone(),
                    app_id: o.app_id.clone(),
                    game_name: o.game_name.clone(),
                    applied_at: o.applied_at.clone(),
                    mode: o.mode,
                    effective,
                    detail,
                }
            })
            .collect()
    }

    /// Correções ativas de verdade no jogo (para o Performance Lab).
    pub fn active_for(&self, app_id: &str) -> Vec<ActiveFixRef> {
        let mut v: Vec<ActiveFixRef> = self
            .applied(Some(app_id))
            .into_iter()
            .filter(|a| a.effective == Effective::Active && !is_capture_setup(&a.recipe_id))
            .map(|a| ActiveFixRef {
                recipe_id: a.recipe_id,
                recipe_version: a.recipe_version,
                operation_id: a.operation_id,
                title: a.recipe_title,
            })
            .collect();
        v.sort_by(|a, b| a.recipe_id.cmp(&b.recipe_id));
        v
    }

    // ── Planejamento ─────────────────────────────────────────────────────

    fn compute(&self, recipe: &Recipe, game: &GameCtx) -> Computed {
        let others = self.journal.list();
        planner::compute(&self.catalog, recipe, game, &others, &self.proc_root)
    }

    /// Compatibilidade sem guardar plano.
    pub fn check(&self, recipe_id: &str, game: &GameCtx) -> Result<FixPlan, String> {
        let r = self.resolve(recipe_id, game)?;
        Ok(self.compute(&r, game).plan)
    }

    pub fn prepare(&self, recipe_id: &str, game: &GameCtx) -> Result<FixPlan, String> {
        let r = self.resolve(recipe_id, game)?;
        let Computed { plan, steps } = self.compute(&r, game);
        let mut plans = self.plans.lock().map_err(|_| "estado indisponível")?;
        let now = now_unix();
        plans.retain(|_, (p, _)| p.expires_at > now);
        plans.insert(plan.plan_id.clone(), (plan.clone(), steps));
        Ok(plan)
    }

    // ── Aplicação ────────────────────────────────────────────────────────

    /// Aplica um plano confirmado. O plano é recalculado e precisa bater com
    /// a impressão digital mostrada ao usuário.
    pub fn apply(
        &self,
        plan_id: &str,
        fingerprint: &str,
        game: &GameCtx,
        emit: &dyn Fn(&Operation),
    ) -> Result<Operation, String> {
        let _guard = match self.exec.try_lock() {
            Ok(g) => g,
            Err(TryLockError::WouldBlock) => {
                return Err("Outra correção está sendo aplicada ou desfeita. Aguarde.".into())
            }
            Err(TryLockError::Poisoned(_)) => return Err("estado indisponível".into()),
        };
        let (stored, _) = self
            .plans
            .lock()
            .map_err(|_| "estado indisponível")?
            .remove(plan_id)
            .ok_or("Prévia não encontrada ou já usada. Revise a correção de novo.")?;
        if stored.expires_at <= now_unix() {
            return Err("A prévia expirou. Revise a correção de novo.".into());
        }
        if stored.fingerprint != fingerprint || stored.app_id != game.app_id {
            return Err("A confirmação não corresponde à prévia mostrada.".into());
        }
        let recipe = self.resolve(&stored.recipe_id, game)?;
        let Computed { plan, steps } = self.compute(&recipe, game);
        if plan.fingerprint != stored.fingerprint {
            return Err("O estado do jogo ou da Steam mudou desde a prévia. Nada foi alterado; revise a correção de novo.".into());
        }
        if !plan.executable() {
            return Err(plan
                .reasons
                .first()
                .cloned()
                .unwrap_or_else(|| "Esta correção não pode ser aplicada agora.".into()));
        }
        let now = now_iso();
        let mut op = Operation {
            schema: JOURNAL_SCHEMA,
            id: new_op_id(),
            recipe_id: recipe.id.clone(),
            recipe_version: recipe.version.clone(),
            recipe_title: recipe.title.clone(),
            app_id: game.app_id.clone(),
            game_name: game.name.clone(),
            state: OpState::Checking,
            mode: plan.mode.unwrap_or(ExecMode::Automatic),
            steps: steps
                .into_iter()
                .map(|step| StepRecord {
                    step,
                    status: StepStatus::Pending,
                    backup: None,
                    after_sha: None,
                    rollback_backup: None,
                })
                .collect(),
            plan_fingerprint: plan.fingerprint.clone(),
            created_at: now.clone(),
            updated_at: now,
            applied_at: None,
            rolled_back_at: None,
            message: None,
            pending: None,
            events: vec![],
            recipe_trust: Some(recipe.trust),
            finding_ids: recipe
                .generated
                .as_ref()
                .map(|g| g.finding_ids.clone())
                .unwrap_or_default(),
        };
        if op.mode == ExecMode::Manual {
            op.pending = Some(Pending {
                kind: PendingKind::Apply,
                line: plan.manual_line.clone().unwrap_or_default(),
            });
            self.journal.transition(&mut op, OpState::RequiresManualStep, "Aguardando você colar a linha nas opções de inicialização do jogo na Steam. A correção só conta como aplicada depois que o FRX confirmar no arquivo.")?;
            emit(&op);
            return Ok(op);
        }
        self.journal.transition(
            &mut op,
            OpState::Checking,
            "Plano confirmado; conferindo o estado atual.",
        )?;
        emit(&op);
        self.run_apply(&mut op, emit);
        Ok(op)
    }

    fn run_apply(&self, op: &mut Operation, emit: &dyn Fn(&Operation)) {
        let result = self.try_apply(op, emit);
        if let Err(e) = result {
            let written = op
                .steps
                .iter()
                .any(|s| s.status == StepStatus::Written || s.status == StepStatus::Verified);
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
                        let _ = self.journal.transition(
                            op,
                            OpState::RecoveryRequired,
                            format!("{e} Não foi possível desfazer automaticamente: {u}"),
                        );
                    }
                }
            }
            emit(op);
        }
    }

    fn try_apply(&self, op: &mut Operation, emit: &dyn Fn(&Operation)) -> Result<(), String> {
        // 1. Backup de tudo antes de qualquer escrita.
        self.journal
            .transition(op, OpState::BackingUp, "Criando backup verificado.")?;
        emit(op);
        let dir = self.backups_dir(&op.id);
        for (i, rec) in op.steps.iter_mut().enumerate() {
            let target = rec.step.target().to_path_buf();
            let name = format!(
                "{i}-{}",
                target
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            );
            let b = files::backup(&target, &dir, &name)?;
            if let Step::LaunchOptions { file_sha, .. } = &rec.step {
                if b.sha256.as_deref() != Some(file_sha.as_str()) {
                    return Err("CONFLITO: localconfig.vdf mudou depois da prévia.".into());
                }
            }
            rec.backup = Some(b);
            rec.status = StepStatus::BackedUp;
        }
        self.journal.save(op)?;

        // 2. Escrita — conferindo de novo processos e conteúdo logo antes.
        self.journal
            .transition(op, OpState::Applying, "Gravando as alterações.")?;
        emit(op);
        if processes::game_running(&self.proc_root, &op.app_id) {
            return Err("CONFLITO: o jogo foi aberto durante a aplicação.".into());
        }
        let launch = op
            .steps
            .iter()
            .any(|s| matches!(s.step, Step::LaunchOptions { .. }));
        if launch && processes::steam_running(&self.proc_root) {
            return Err("CONFLITO: a Steam foi aberta durante a aplicação.".into());
        }
        for i in 0..op.steps.len() {
            let after_sha = write_step(&op.steps[i].step)?;
            op.steps[i].after_sha = Some(after_sha);
            op.steps[i].status = StepStatus::Written;
            self.journal.save(op)?;
        }

        // 3. Verificação do estado real.
        self.journal
            .transition(op, OpState::Verifying, "Conferindo o resultado no disco.")?;
        emit(op);
        for rec in &mut op.steps {
            verify_written(rec)?;
            rec.status = StepStatus::Verified;
        }
        op.applied_at = Some(now_iso());
        op.pending = None;
        self.journal.transition(
            op,
            OpState::Applied,
            "Aplicada e verificada. Vale a partir da próxima vez que o jogo abrir.",
        )?;
        emit(op);
        Ok(())
    }

    /// Restaura do backup o que foi gravado — só se ninguém mexeu depois.
    fn undo_written(&self, op: &mut Operation) -> Result<(), String> {
        for rec in op.steps.iter_mut().rev() {
            if !matches!(rec.status, StepStatus::Written | StepStatus::Verified) {
                continue;
            }
            let target = rec.step.target().to_path_buf();
            if sha256_file(&target)? != rec.after_sha {
                return Err(format!(
                    "{} foi alterado por outro programa.",
                    target.display()
                ));
            }
            files::restore(rec.backup.as_ref().ok_or("backup ausente")?, &target)?;
            rec.status = StepStatus::Reverted;
        }
        self.journal.save(op)
    }

    // ── Passos manuais e verificação ─────────────────────────────────────

    /// Confere no disco um passo manual ou uma operação em recuperação.
    pub fn verify(&self, op_id: &str, emit: &dyn Fn(&Operation)) -> Result<Operation, String> {
        let _guard = self.exec.lock().map_err(|_| "estado indisponível")?;
        let mut op = self.journal.load(op_id)?;
        let presence = presence(&op)?;
        match (op.state, op.pending.as_ref().map(|p| p.kind)) {
            (OpState::RequiresManualStep, Some(PendingKind::Apply))
            | (OpState::RecoveryRequired, _) => match presence {
                Presence::All => {
                    for s in &mut op.steps {
                        s.status = StepStatus::Verified;
                    }
                    op.applied_at.get_or_insert_with(now_iso);
                    op.pending = None;
                    self.journal.transition(&mut op, OpState::Applied, "Confirmado no localconfig.vdf: a correção está nas opções de inicialização.")?;
                }
                Presence::None if op.state == OpState::RecoveryRequired => {
                    self.journal.transition(
                        &mut op,
                        OpState::Failed,
                        "Conferido: a alteração não ficou gravada; nada está aplicado.",
                    )?;
                }
                _ => {
                    op.message = Some("Ainda não encontrei a linha no localconfig.vdf. A Steam só grava as opções no disco ao fechar — feche a Steam e confira de novo.".into());
                    self.journal.save(&op)?;
                }
            },
            (OpState::RequiresManualStep, Some(PendingKind::Rollback)) => match presence {
                Presence::None => {
                    for s in &mut op.steps {
                        s.status = StepStatus::Reverted;
                    }
                    op.rolled_back_at = Some(now_iso());
                    op.pending = None;
                    self.journal.transition(
                        &mut op,
                        OpState::RolledBack,
                        "Confirmado: a correção não está mais nas opções de inicialização.",
                    )?;
                }
                _ => {
                    op.message = Some("A correção ainda aparece no localconfig.vdf. Remova-a na Steam, feche a Steam e confira de novo.".into());
                    self.journal.save(&op)?;
                }
            },
            _ => return Err("Esta operação não tem nada a conferir.".into()),
        }
        emit(&op);
        Ok(op)
    }

    // ── Rollback ─────────────────────────────────────────────────────────

    pub fn prepare_rollback(&self, op_id: &str) -> Result<RollbackPlan, String> {
        let op = self.journal.load(op_id)?;
        Ok(self.rollback_plan(&op)?.0)
    }

    fn rollback_plan(&self, op: &Operation) -> Result<(RollbackPlan, RollbackWrites), String> {
        let pending_apply = op.state == OpState::RequiresManualStep
            && op
                .pending
                .as_ref()
                .is_some_and(|p| p.kind == PendingKind::Apply);
        if !matches!(op.state, OpState::Applied | OpState::RecoveryRequired) && !pending_apply {
            return Err("Só correções aplicadas (ou em recuperação) podem ser desfeitas.".into());
        }
        let steam_running = processes::steam_running(&self.proc_root);
        let game_running = processes::game_running(&self.proc_root, &op.app_id);
        let mut reasons = Vec::new();
        let mut changes = Vec::new();
        let mut writes: Vec<Option<Vec<u8>>> = Vec::new();
        let mut status = PlanStatus::Ready;
        let mut manual_line = None;
        let mut fp_parts: Vec<String> = vec![op.id.clone(), format!("{:?}", op.state)];
        for rec in &op.steps {
            let target = rec.step.target();
            fp_parts.push(sha256_file(target)?.unwrap_or_default());
            match &rec.step {
                Step::LaunchOptions {
                    config,
                    app_id,
                    env,
                    args,
                    ..
                } => {
                    let text = localconfig::read_file(config)?;
                    let cur = localconfig::launch_options(&text, app_id)?;
                    let mut line = cur.clone();
                    let mut conflict = false;
                    for w in env.iter().rev() {
                        match revert_env(
                            line.as_deref(),
                            &w.key,
                            &w.written,
                            w.previous.as_deref(),
                        )? {
                            Revert::Done(l) => line = Some(l),
                            Revert::Missing => reasons
                                .push(format!("{} já não está na linha; nada a remover.", w.key)),
                            Revert::Changed(v) => {
                                conflict = true;
                                reasons.push(format!("{} foi alterada para “{v}” depois da aplicação. O FRX não sobrescreve mudanças de outros programas.", w.key));
                            }
                        }
                    }
                    if !args.is_empty() {
                        match remove_args(line.as_deref(), args)? {
                            Revert::Done(l) => line = Some(l),
                            _ => reasons
                                .push("Os argumentos adicionados já não estão na linha.".into()),
                        }
                    }
                    if conflict {
                        status = PlanStatus::Conflict;
                        writes.push(None);
                        continue;
                    }
                    let new = line.unwrap_or_default();
                    if cur.as_deref().unwrap_or("") == new {
                        writes.push(None);
                        continue;
                    }
                    changes.push(PlannedChange {
                        kind: ChangeKind::LaunchOptions,
                        target: launch_target_label(&op.game_name),
                        before: cur.clone(),
                        after: Some(new.clone()),
                        summary: "remove só o que esta correção adicionou".into(),
                    });
                    manual_line = Some(new.clone());
                    match localconfig::set_launch_options(&text, app_id, &new) {
                        Ok(t) => writes.push(Some(t.into_bytes())),
                        Err(e) => {
                            // Sem edição posicional segura: só o modo manual.
                            if status == PlanStatus::Ready {
                                status = PlanStatus::RequiresManualStep;
                            }
                            reasons.push(format!(
                                "O FRX não consegue editar o localconfig.vdf com segurança ({e})."
                            ));
                            writes.push(None);
                        }
                    }
                }
                Step::GameFile {
                    target,
                    rel,
                    before_sha,
                    ..
                } => {
                    let cur = sha256_file(target)?;
                    if cur == rec.after_sha {
                        changes.push(PlannedChange {
                            kind: ChangeKind::GameFile,
                            target: format!("{}/{rel}", op.game_name),
                            before: None,
                            after: None,
                            summary: if rec.backup.as_ref().is_some_and(|b| b.existed) {
                                format!("restaura {rel} do backup")
                            } else {
                                format!("remove {rel}, criado pelo FRX")
                            },
                        });
                        writes.push(Some(vec![]));
                    } else if cur == *before_sha {
                        writes.push(None);
                    } else {
                        status = PlanStatus::Conflict;
                        reasons.push(format!("{rel} foi alterado depois da aplicação; o FRX não o substitui pelo backup antigo."));
                        writes.push(None);
                    }
                }
            }
        }
        if game_running {
            status = PlanStatus::Conflict;
            reasons.push("Feche o jogo antes de desfazer a correção.".into());
        }
        let only_launch = op
            .steps
            .iter()
            .all(|s| matches!(s.step, Step::LaunchOptions { .. }));
        if status == PlanStatus::Ready && steam_running && !changes.is_empty() {
            if only_launch {
                status = PlanStatus::RequiresManualStep;
                reasons.push("A Steam está aberta. Feche-a para o FRX desfazer sozinho, ou cole a linha abaixo nas opções de inicialização.".into());
            } else {
                status = PlanStatus::Conflict;
                reasons.push("Feche a Steam para desfazer esta correção.".into());
            }
        }
        if changes.is_empty() && status == PlanStatus::Ready {
            reasons.push(if pending_apply {
                "A correção ainda não foi colada na Steam; cancelar não altera nada.".into()
            } else {
                "Nada a desfazer no disco: as alterações já não estão lá.".into()
            });
        }
        fp_parts.push(format!("{steam_running}{game_running}{status:?}"));
        let mode = match status {
            PlanStatus::Ready => Some(ExecMode::Automatic),
            PlanStatus::RequiresManualStep => Some(ExecMode::Manual),
            _ => None,
        };
        let plan = RollbackPlan {
            operation_id: op.id.clone(),
            fingerprint: sha256_hex(fp_parts.join("|").as_bytes())[..32].to_owned(),
            status,
            mode,
            reasons,
            changes,
            manual_line: if mode == Some(ExecMode::Manual) {
                manual_line
            } else {
                None
            },
            steam_running,
            game_running,
        };
        Ok((plan, writes))
    }

    pub fn rollback(
        &self,
        op_id: &str,
        fingerprint: &str,
        emit: &dyn Fn(&Operation),
    ) -> Result<Operation, String> {
        let _guard = match self.exec.try_lock() {
            Ok(g) => g,
            Err(TryLockError::WouldBlock) => {
                return Err("Outra correção está sendo aplicada ou desfeita. Aguarde.".into())
            }
            Err(TryLockError::Poisoned(_)) => return Err("estado indisponível".into()),
        };
        let mut op = self.journal.load(op_id)?;
        let (plan, writes) = self.rollback_plan(&op)?;
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
                self.journal.transition(&mut op, OpState::RequiresManualStep, "Aguardando você colar a linha sem a correção nas opções de inicialização. O FRX confere depois.")?;
                emit(&op);
                return Ok(op);
            }
            _ => return Err(plan.reasons.join(" ")),
        }
        if op.state == OpState::RequiresManualStep && plan.changes.is_empty() {
            op.pending = None;
            self.journal.transition(
                &mut op,
                OpState::Failed,
                "Cancelada antes de ser aplicada; nada foi alterado.",
            )?;
            emit(&op);
            return Ok(op);
        }
        self.journal
            .transition(&mut op, OpState::RollingBack, "Desfazendo a correção.")?;
        emit(&op);
        let dir = self.backups_dir(&op.id);
        let result = (|| -> Result<(), String> {
            for (i, bytes) in writes.into_iter().enumerate() {
                let Some(bytes) = bytes else {
                    op.steps[i].status = StepStatus::Reverted;
                    continue;
                };
                let target = op.steps[i].step.target().to_path_buf();
                let name = format!(
                    "rollback-{i}-{}",
                    target
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                );
                op.steps[i].rollback_backup = Some(files::backup(&target, &dir, &name)?);
                self.journal.save(&op)?;
                match &op.steps[i].step {
                    Step::LaunchOptions { config, app_id, .. } => {
                        // Confere de novo logo antes de gravar.
                        if sha256_file(config)?
                            != op.steps[i]
                                .rollback_backup
                                .as_ref()
                                .and_then(|b| b.sha256.clone())
                        {
                            return Err("localconfig.vdf mudou durante o rollback.".into());
                        }
                        files::atomic_write(config, &bytes, 0o644)?;
                        let text = localconfig::read_file(config)?;
                        let expect = String::from_utf8_lossy(&bytes);
                        if text != expect || localconfig::launch_options(&text, app_id).is_err() {
                            return Err("A verificação do rollback falhou.".into());
                        }
                    }
                    Step::GameFile { target, .. } => {
                        files::restore(
                            op.steps[i].backup.as_ref().ok_or("backup ausente")?,
                            target,
                        )?;
                    }
                }
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
                self.journal.transition(
                    &mut op,
                    OpState::RecoveryRequired,
                    format!("Rollback interrompido: {e} Confira o estado antes de tentar de novo."),
                )?;
            }
        }
        emit(&op);
        Ok(op)
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
                    let cur = sha256_file(s.step.target()).ok().flatten();
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
        }
    }
}

fn write_step(step: &Step) -> Result<String, String> {
    match step {
        Step::LaunchOptions {
            config,
            app_id,
            file_sha,
            after,
            ..
        } => {
            let text = localconfig::read_file(config)?;
            if sha256_hex(text.as_bytes()) != *file_sha {
                return Err("CONFLITO: localconfig.vdf foi alterado por outro programa.".into());
            }
            let new = localconfig::set_launch_options(&text, app_id, after)?;
            files::atomic_write(config, new.as_bytes(), 0o644)?;
            Ok(sha256_hex(new.as_bytes()))
        }
        Step::GameFile {
            root,
            rel,
            target,
            before_sha,
            content,
        } => {
            let again = security::resolve_in_root(root, rel)?;
            if again != *target {
                return Err("CONFLITO: o caminho do arquivo mudou.".into());
            }
            if sha256_file(target)? != *before_sha {
                return Err("CONFLITO: o arquivo do jogo foi alterado.".into());
            }
            files::atomic_write(target, content.as_bytes(), 0o644)?;
            Ok(sha256_hex(content.as_bytes()))
        }
    }
}

fn verify_written(rec: &StepRecord) -> Result<(), String> {
    let target = rec.step.target();
    if sha256_file(target)? != rec.after_sha {
        return Err("A verificação falhou: o arquivo não tem o conteúdo gravado.".into());
    }
    if let Step::LaunchOptions {
        config,
        app_id,
        after,
        ..
    } = &rec.step
    {
        let text = localconfig::read_file(config)?;
        let got = localconfig::launch_options(&text, app_id)?;
        if got.as_deref().unwrap_or("") != after.trim() {
            return Err("A verificação falhou: as opções lidas não são as esperadas.".into());
        }
    }
    Ok(())
}

enum Presence {
    All,
    Partial,
    None,
}

/// Quanto da operação está presente na configuração real.
fn presence(op: &Operation) -> Result<Presence, String> {
    let (mut found, mut total) = (0, 0);
    for rec in &op.steps {
        match &rec.step {
            Step::LaunchOptions {
                config,
                app_id,
                env,
                args,
                ..
            } => {
                let text = localconfig::read_file(config)?;
                let cur = localconfig::launch_options(&text, app_id)?.unwrap_or_default();
                let line = LaunchLine::parse(&cur).ok();
                let lead = line.as_ref().map(|l| l.leading_env()).unwrap_or_default();
                for w in env {
                    total += 1;
                    if lead.iter().any(|(_, k, v)| *k == w.key && *v == w.written) {
                        found += 1;
                    }
                }
                let tail: Vec<&str> = line
                    .as_ref()
                    .map(|l| {
                        l.words
                            .iter()
                            .skip(l.command.map_or(0, |c| c + 1))
                            .map(|w| w.value.as_str())
                            .collect()
                    })
                    .unwrap_or_default();
                for a in args {
                    total += 1;
                    if tail.contains(&a.as_str()) {
                        found += 1;
                    }
                }
            }
            Step::GameFile { target, .. } => {
                total += 1;
                if sha256_file(target)? == rec.after_sha && rec.after_sha.is_some() {
                    found += 1;
                }
            }
        }
    }
    Ok(match found {
        0 => Presence::None,
        n if n == total => Presence::All,
        _ => Presence::Partial,
    })
}

fn effective(op: &Operation) -> (Effective, String) {
    match presence(op) {
        Ok(Presence::All) => (Effective::Active, "Conferido agora no disco.".into()),
        Ok(_) => (Effective::Drifted, "A configuração foi alterada fora do FRX depois da aplicação; a correção não está mais completa.".into()),
        Err(e) => (Effective::Unknown, e),
    }
}

/// Achado e receita falam do mesmo ajuste? Compara as ações do achado
/// (`KEY=valor`, linhas de inicialização) com as chaves da receita.
pub fn related(r: &Recipe, f: &FixFinding) -> bool {
    let mut keys: Vec<String> = f.keys.clone();
    for a in &f.actions {
        match a.kind {
            ActionKind::Env => keys.push(format!("env:{}", a.value.trim())),
            ActionKind::LaunchOption => {
                if let Ok(l) = LaunchLine::parse(&a.value) {
                    keys.extend(
                        l.leading_env()
                            .into_iter()
                            .map(|(_, k, v)| format!("env:{k}={v}")),
                    );
                }
            }
            _ => {}
        }
    }
    keys.iter().any(|k| r.related_keys.contains(k))
}

pub fn data_root(app_data: &Path) -> PathBuf {
    app_data.join("fix-engine")
}

/// Preparação técnica do MangoHud; não é uma correção nem integra Smart Packs.
pub fn is_capture_setup(id: &str) -> bool {
    id.strip_prefix("frx.capture.")
        .is_some_and(crate::art::valid_app_id)
}
