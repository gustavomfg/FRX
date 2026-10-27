//! Comandos do Fix Engine. A interface só envia identificadores (receita,
//! jogo, plano, operação) e a impressão digital da prévia; caminhos, valores
//! e passos são sempre recalculados no núcleo.

use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use super::AppCtx;
use crate::cache::Cache;
use crate::fix_engine::dynamic::AssessmentView;
use crate::fix_engine::models::*;
use crate::fix_engine::{FixEngine, PROGRESS_EVENT};
use crate::fixes::compat::HardwareContext;
use crate::fixes::models::FixReport;
use crate::steam::{self, discovery::DiscoveryEnv, models::ScanResult};

/// Estado real do jogo, com uma varredura nova (a do cache pode estar velha),
/// e as descobertas da última pesquisa guardadas pelo próprio núcleo.
pub(crate) fn game_ctx(cache: &Cache, app_id: &str) -> Result<GameCtx, String> {
    if !crate::art::valid_app_id(app_id) {
        return Err("AppID inválido".into());
    }
    let scan: ScanResult = steam::scan(&DiscoveryEnv::from_process(), None);
    let game = scan
        .games
        .into_iter()
        .find(|g| g.app_id == app_id)
        .ok_or("Jogo não encontrado na biblioteca da Steam.")?;
    let profile = crate::system::profile();
    let mut conditions =
        super::performance::conditions_from_profile(&profile, game.compatibility.tool.as_ref());
    conditions.game_build_id = game.build_id.clone();
    let performance_conditions = Some(conditions);
    let hw = HardwareContext {
        gpus: profile.gpus,
        primary: profile.primary_gpu,
    };
    let tool = game.compatibility.tool.as_ref();
    Ok(GameCtx {
        performance_conditions,
        app_id: game.app_id.clone(),
        name: game.name.clone(),
        steam_root: Some(game.steam_install.clone().into()),
        install_path: game.install_path.clone().map(Into::into),
        uses_proton: game.compatibility.uses_proton,
        tool_name: tool.map(|t| crate::fixes::short_tool_name(&t.display_name)),
        tool_includes_protonfixes: tool.is_some_and(|t| t.includes_protonfixes),
        gpu: hw.primary_req(),
        driver: hw.primary.as_ref().and_then(|g| g.driver_version.clone()),
        findings: cache
            .get::<FixReport>("fixes", app_id)
            .map(|e| e.data)
            .filter(|r| r.app_id == app_id)
            .map(|r| r.findings)
            .unwrap_or_default(),
    })
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

fn engine(ctx: &AppCtx) -> Arc<FixEngine> {
    ctx.engine.clone()
}

fn emitter(app: AppHandle) -> impl Fn(&Operation) {
    move |op: &Operation| {
        let _ = app.emit(PROGRESS_EVENT, op);
    }
}

#[tauri::command]
pub fn get_fix_engine_status(ctx: State<'_, AppCtx>) -> EngineStatus {
    ctx.engine.status()
}

#[tauri::command]
pub fn cancel_fix_operation(ctx: State<'_, AppCtx>, operation_id: String) -> bool {
    ctx.engine.cancel_artifact_operation(&operation_id)
}

/// Catálogo local, com os achados da última pesquisa que citam o mesmo ajuste.
#[tauri::command]
pub fn list_fix_recipes(ctx: State<'_, AppCtx>, app_id: Option<String>) -> Vec<RecipeView> {
    let findings = app_id
        .filter(|a| crate::art::valid_app_id(a))
        .and_then(|a| {
            ctx.cache
                .get::<FixReport>("fixes", &a)
                .filter(|e| e.data.app_id == a)
        })
        .map(|e| e.data.findings)
        .unwrap_or_default();
    ctx.engine.recipes(&findings)
}

/// Gerador dinâmico: analisa as descobertas do jogo e devolve, para cada uma,
/// a categoria (aplicável, requer validação, informativa), o motivo e a
/// receita gerada quando houver. Não grava nada no jogo.
#[tauri::command]
pub async fn analyze_fix_findings(
    ctx: State<'_, AppCtx>,
    app_id: String,
) -> Result<Vec<AssessmentView>, String> {
    let (e, cache) = (engine(&ctx), ctx.cache.clone());
    blocking(move || e.analyze(&game_ctx(&cache, &app_id)?)).await
}

#[tauri::command]
pub fn get_fix_recipe(ctx: State<'_, AppCtx>, recipe_id: String) -> Result<Recipe, String> {
    ctx.engine.recipe(&recipe_id).cloned()
}

#[tauri::command]
pub async fn check_fix_compatibility(
    ctx: State<'_, AppCtx>,
    recipe_id: String,
    app_id: String,
) -> Result<FixPlan, String> {
    let (e, cache) = (engine(&ctx), ctx.cache.clone());
    blocking(move || e.check(&recipe_id, &game_ctx(&cache, &app_id)?)).await
}

/// Prévia (dry-run): o que mudaria, onde, e se dá para aplicar agora.
#[tauri::command]
pub async fn prepare_fix_plan(
    ctx: State<'_, AppCtx>,
    recipe_id: String,
    app_id: String,
    expected_fingerprint: Option<String>,
) -> Result<FixPlan, String> {
    let (e, cache) = (engine(&ctx), ctx.cache.clone());
    blocking(move || {
        let game = game_ctx(&cache, &app_id)?;
        let plan = e.prepare(&recipe_id, &game).map_err(|error| {
            if expected_fingerprint.is_some() {
                format!("A descoberta ou seus requisitos mudaram desde a listagem. Reavalie as correções deste jogo. Detalhe: {error}")
            } else { error }
        })?;
        validate_listing(&plan, expected_fingerprint.as_deref())?;
        Ok(plan)
    }).await
}

pub(crate) fn validate_listing(plan: &FixPlan, expected: Option<&str>) -> Result<(), String> {
    if expected.is_some_and(|expected| expected != plan.fingerprint) {
        return Err(format!("A fonte, o ambiente ou os arquivos mudaram desde a listagem; nenhuma alteração foi aplicada. Reavalie as correções para obter a prévia atual. {}", plan.reasons.join(" ")));
    }
    Ok(())
}

#[tauri::command]
pub async fn apply_fix_plan(
    app: AppHandle,
    ctx: State<'_, AppCtx>,
    plan_id: String,
    fingerprint: String,
    app_id: String,
) -> Result<Operation, String> {
    let (e, cache) = (engine(&ctx), ctx.cache.clone());
    blocking(move || {
        e.apply(
            &plan_id,
            &fingerprint,
            &game_ctx(&cache, &app_id)?,
            &emitter(app),
        )
    })
    .await
}

#[tauri::command]
pub fn get_fix_operation(
    ctx: State<'_, AppCtx>,
    operation_id: String,
) -> Result<Operation, String> {
    ctx.engine.operation(&operation_id)
}

#[tauri::command]
pub async fn list_fix_operations(
    ctx: State<'_, AppCtx>,
    app_id: Option<String>,
) -> Result<Vec<Operation>, String> {
    let e = engine(&ctx);
    blocking(move || Ok(e.operations(app_id.as_deref()))).await
}

#[tauri::command]
pub async fn get_applied_fixes(
    ctx: State<'_, AppCtx>,
    app_id: Option<String>,
) -> Result<Vec<AppliedFix>, String> {
    let e = engine(&ctx);
    blocking(move || Ok(e.applied(app_id.as_deref()))).await
}

#[tauri::command]
pub async fn prepare_fix_rollback(
    ctx: State<'_, AppCtx>,
    operation_id: String,
) -> Result<RollbackPlan, String> {
    let e = engine(&ctx);
    blocking(move || e.prepare_rollback(&operation_id)).await
}

#[tauri::command]
pub async fn rollback_fix(
    app: AppHandle,
    ctx: State<'_, AppCtx>,
    operation_id: String,
    fingerprint: String,
) -> Result<Operation, String> {
    let e = engine(&ctx);
    blocking(move || e.rollback(&operation_id, &fingerprint, &emitter(app))).await
}

/// Confere no disco um passo manual (linha colada na Steam) ou uma operação interrompida.
#[tauri::command]
pub async fn verify_fix_operation(
    app: AppHandle,
    ctx: State<'_, AppCtx>,
    operation_id: String,
) -> Result<Operation, String> {
    let e = engine(&ctx);
    blocking(move || e.verify(&operation_id, &emitter(app))).await
}
