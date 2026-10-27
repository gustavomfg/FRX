//! Comandos dos Smart Packs. Leitura e composição aqui; aplicar e desfazer
//! continuam nos comandos do Fix Engine (`prepare_fix_plan`, `apply_fix_plan`,
//! `rollback_fix`), com o `applyRecipeId` do pacote.

use tauri::State;

use super::fix_engine::game_ctx;
use super::AppCtx;
use crate::performance::models::SessionMeta;
use crate::smart_packs::models::*;
use crate::smart_packs::{self, benchmark, BuildInput};

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

/// Compõe, avalia e classifica os pacotes do jogo para o perfil escolhido.
#[tauri::command]
pub async fn get_smart_packs(
    ctx: State<'_, AppCtx>,
    app_id: String,
    profile: Profile,
) -> Result<SmartPackReport, String> {
    let (engine, cache, store) = (
        ctx.engine.clone(),
        ctx.cache.clone(),
        ctx.perf.store.clone(),
    );
    blocking(move || {
        let game = game_ctx(&cache, &app_id)?;
        let sessions = store.list();
        smart_packs::build(
            &engine,
            BuildInput {
                launch_line: smart_packs::launch_line(&game),
                game: &game,
                sessions: &sessions,
                profile,
                limits: Limits::default(),
            },
        )
    })
    .await
}

/// Pacote A × pacote B (mesma metodologia do ranking).
#[tauri::command]
pub async fn compare_smart_packs(
    ctx: State<'_, AppCtx>,
    app_id: String,
    a: String,
    b: String,
) -> Result<PackComparison, String> {
    if !crate::art::valid_app_id(&app_id) {
        return Err("AppID inválido".into());
    }
    let (engine, store) = (ctx.engine.clone(), ctx.perf.store.clone());
    blocking(move || smart_packs::compare_packs(&engine, &app_id, &store.list(), &a, &b)).await
}

/// Passos para trocar a configuração atual por outro pacote (cada passo
/// abre a própria prévia no Fix Engine).
#[tauri::command]
pub async fn plan_smart_pack_switch(
    ctx: State<'_, AppCtx>,
    app_id: String,
    pack_id: String,
) -> Result<SwitchPlan, String> {
    if !crate::art::valid_app_id(&app_id) {
        return Err("AppID inválido".into());
    }
    let engine = ctx.engine.clone();
    blocking(move || {
        let to = smart_packs::store_for(&engine)
            .pack(&app_id, &pack_id)
            .ok_or("Pacote não encontrado. Abra Smart Packs de novo.")?;
        Ok(smart_packs::switch_plan(&engine, &app_id, &to))
    })
    .await
}

/// Declara com qual pacote uma sessão sem registro foi capturada (ou limpa
/// a declaração com `packId = null`). O pacote precisa ser deste jogo.
#[tauri::command]
pub async fn declare_benchmark_pack(
    ctx: State<'_, AppCtx>,
    session_id: String,
    pack_id: Option<String>,
) -> Result<SessionMeta, String> {
    let (engine, store) = (ctx.engine.clone(), ctx.perf.store.clone());
    blocking(move || {
        let meta = store
            .list()
            .into_iter()
            .find(|m| m.id == session_id)
            .ok_or("Sessão não encontrada.")?;
        let app = meta
            .app_id
            .clone()
            .ok_or("Associe a sessão a um jogo antes.")?;
        let snap = match pack_id {
            None => None,
            Some(id) if id == benchmark::baseline_id(&app) => {
                Some(benchmark::declared(&app, vec![]))
            }
            Some(id) => {
                let p = smart_packs::store_for(&engine)
                    .pack(&app, &id)
                    .ok_or("Pacote não encontrado para este jogo.")?;
                Some(benchmark::declared_with_files(&app, p.env, p.file_fixes))
            }
        };
        store.set_smart_pack(&session_id, snap)
    })
    .await
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
