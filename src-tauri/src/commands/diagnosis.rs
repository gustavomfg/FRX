//! Diagnóstico por jogo. Só é chamado pelo botão Diagnosticar. Consulta as
//! fontes atuais e o Fix Engine e devolve sugestões; não aplica nada.

use tauri::State;

use super::fix_engine::game_ctx;
use super::fixes::{search_game_fixes, SearchArgs};
use super::AppCtx;
use crate::diagnosis::{self, models::*};
use crate::smart_packs;

#[tauri::command]
pub async fn diagnose_game(
    ctx: State<'_, AppCtx>,
    app_id: String,
    name: String,
    symptom: Symptom,
    max_age_hours: u64,
) -> Result<DiagnosisResult, String> {
    let report = search_game_fixes(
        ctx.clone(),
        SearchArgs {
            app_id: app_id.clone(),
            name,
            force: false,
            max_age_hours,
        },
    )
    .await?;
    let (engine, cache) = (ctx.engine.clone(), ctx.cache.clone());
    tauri::async_runtime::spawn_blocking(move || {
        let game = game_ctx(&cache, &app_id)?;
        let gpu_name = crate::system::profile().primary_gpu.and_then(|g| g.name);
        let line = smart_packs::launch_line(&game);
        let input = diagnosis::assemble::input(
            &engine,
            &game,
            symptom,
            gpu_name,
            line.as_deref(),
            report.providers,
        )?;
        Ok(diagnosis::diagnose(input))
    })
    .await
    .map_err(|e| e.to_string())?
}
