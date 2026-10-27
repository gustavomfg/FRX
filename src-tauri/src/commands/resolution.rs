use super::{fix_engine::game_ctx, AppCtx};
use crate::resolution::{self, models::Report, persistence::Store};
use tauri::State;
fn compute(ctx: &AppCtx, app: &str, symptoms: Option<Vec<String>>) -> Result<Report, String> {
    let store = Store::new(ctx.engine.root());
    let previous = store.report(app)?;
    let mut inputs = store.inputs(app)?;
    if let Some(s) = symptoms {
        resolution::valid_symptoms(&s)?;
        inputs.symptoms = s;
    }
    let game = game_ctx(&ctx.cache, app)?;
    let mut report = resolution::diagnose(
        &ctx.engine,
        &game,
        &inputs.symptoms,
        inputs.log.as_ref(),
        inputs.knowledge.clone(),
        &ctx.perf.store.list(),
    )?;
    if let Some(previous) = previous.filter(|p| p.context_fingerprint == report.context_fingerprint)
    {
        for old in previous
            .plans
            .iter()
            .filter(|p| p.state == "verified-configuration")
        {
            if report.plans.iter().any(|p| {
                p.id == old.id
                    && p.operation_id == old.operation_id
                    && p.state == "applied-unverified"
            }) {
                // Persist the verification only after checking the same current
                // operation on disk again. Cached success is never authority.
                resolution::verify(&ctx.engine, &mut report, &old.id)?;
            }
        }
    }
    store.save_inputs(app, &inputs)?;
    store.save_report(&report)?;
    Ok(report)
}
#[tauri::command]
pub async fn get_resolution_report(
    ctx: State<'_, AppCtx>,
    app_id: String,
) -> Result<Option<Report>, String> {
    let _guard = ctx.resolution_lock.lock().await;
    if Store::new(ctx.engine.root()).report(&app_id)?.is_none() {
        return Ok(None);
    }
    compute(&ctx, &app_id, None).map(Some)
}
#[tauri::command]
pub async fn diagnose_game(
    ctx: State<'_, AppCtx>,
    app_id: String,
    symptoms: Vec<String>,
) -> Result<Report, String> {
    let _guard = ctx.resolution_lock.lock().await;
    compute(&ctx, &app_id, Some(symptoms))
}
#[tauri::command]
pub async fn research_game_resolutions(
    ctx: State<'_, AppCtx>,
    app_id: String,
    symptoms: Vec<String>,
) -> Result<Report, String> {
    let _guard = ctx.resolution_lock.lock().await;
    resolution::valid_symptoms(&symptoms)?;
    let game = game_ctx(&ctx.cache, &app_id)?;
    // Research uses the existing bounded HTTP/cache/rate-limit client. Findings
    // remain attributed by the existing Fix Discovery rather than fabricated here.
    super::fixes::search_game_fixes(
        ctx.clone(),
        super::fixes::SearchArgs {
            app_id: app_id.clone(),
            name: game.name.clone(),
            force: true,
            max_age_hours: 24,
        },
    )
    .await?;
    let http = crate::fixes::http::Ctx {
        fetcher: &ctx.fetcher,
        cache: &ctx.cache,
        state: &ctx.fixes,
        ttl_secs: 86400,
        force: true,
    };
    let knowledge = resolution::research::search(&http, &game, &symptoms).await;
    let store = Store::new(ctx.engine.root());
    let mut inputs = store.inputs(&app_id)?;
    inputs.knowledge = knowledge;
    store.save_inputs(&app_id, &inputs)?;
    compute(&ctx, &app_id, Some(symptoms))
}
#[tauri::command]
pub async fn import_resolution_log(
    ctx: State<'_, AppCtx>,
    app_id: String,
    path: String,
) -> Result<Report, String> {
    let _guard = ctx.resolution_lock.lock().await;
    let log = resolution::detection::import_log(&app_id, std::path::Path::new(&path))?;
    let store = Store::new(ctx.engine.root());
    let mut inputs = store.inputs(&app_id)?;
    inputs.log = Some(log);
    store.save_inputs(&app_id, &inputs)?;
    compute(&ctx, &app_id, None)
}
#[tauri::command]
pub async fn verify_resolution_plan(
    ctx: State<'_, AppCtx>,
    app_id: String,
    plan_id: String,
) -> Result<Report, String> {
    let _guard = ctx.resolution_lock.lock().await;
    let mut report = compute(&ctx, &app_id, None)?;
    resolution::verify(&ctx.engine, &mut report, &plan_id)?;
    Store::new(ctx.engine.root()).save_report(&report)?;
    Ok(report)
}
