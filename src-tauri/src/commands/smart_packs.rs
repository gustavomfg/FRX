//! Comandos do pacote do jogo. Composição e avaliação aqui; aplicar e
//! desfazer continuam nos comandos do Fix Engine (`prepare_fix_plan`,
//! `apply_fix_plan`, `rollback_fix`), com o `applyRecipeId` do pacote.

use tauri::State;

use super::fix_engine::game_ctx;
use super::AppCtx;
use crate::smart_packs::models::*;
use crate::smart_packs::{self};

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

/// Avalia automaticamente uma sessão (ex.: a que acabou de ser capturada)
/// contra o baseline do jogo: melhoria, regressão, sem diferença ou
/// inconclusivo, com a indicação de preliminar quando há poucas sessões.
#[tauri::command]
pub async fn evaluate_benchmark_session(
    ctx: State<'_, AppCtx>,
    session_id: String,
) -> Result<PackEvaluation, String> {
    let (engine, store) = (ctx.engine.clone(), ctx.perf.store.clone());
    blocking(move || smart_packs::evaluate_session(&engine, &store.list(), &session_id)).await
}

/// Complete Experience Pack do jogo. `without`: IDs de receitas que o
/// usuário desligou; o pacote é recomposto sem elas.
#[tauri::command]
pub async fn get_experience_pack(
    ctx: State<'_, AppCtx>,
    app_id: String,
    without: Vec<String>,
) -> Result<smart_packs::experience::ExperiencePack, String> {
    let (engine, cache) = (ctx.engine.clone(), ctx.cache.clone());
    blocking(move || {
        let game = game_ctx(&cache, &app_id)?;
        smart_packs::experience::build(
            &engine,
            &game,
            smart_packs::launch_line(&game).as_deref(),
            &without,
        )
    })
    .await
}
