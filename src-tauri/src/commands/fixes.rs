use serde::Deserialize;
use tauri::State;

use super::AppCtx;
use crate::fixes::{self, compat::HardwareContext, models::*, SearchRequest};
use crate::steam::models::ScanResult;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchArgs {
    pub app_id: String,
    pub name: String,
    pub force: bool,
    pub max_age_hours: u64,
}

fn ttl(hours: u64) -> u64 {
    hours.clamp(1, 24 * 30) * 3600
}

/// Pesquisa correções conhecidas para um jogo. Somente leitura.
#[tauri::command]
pub async fn search_game_fixes(
    ctx: State<'_, AppCtx>,
    args: SearchArgs,
) -> Result<FixReport, String> {
    // Ferramenta do jogo (para saber se a correção já vem embutida) e hardware.
    let tool = ctx
        .cache
        .get::<ScanResult>("library", "steam")
        .and_then(|e| e.data.games.into_iter().find(|g| g.app_id == args.app_id))
        .and_then(|g| g.compatibility.tool);
    let profile = tauri::async_runtime::spawn_blocking(crate::system::profile)
        .await
        .map_err(|e| e.to_string())?;
    let hw = HardwareContext {
        gpus: profile.gpus,
        primary: profile.primary_gpu,
    };
    let req = SearchRequest {
        app_id: args.app_id,
        name: args.name,
        force: args.force,
        ttl_secs: ttl(args.max_age_hours),
    };
    fixes::search(&ctx.fetcher, &ctx.cache, &ctx.fixes, req, tool, hw).await
}

/// Relatórios já salvos — funcionam sem conexão.
#[tauri::command]
pub fn get_cached_fix_reports(ctx: State<'_, AppCtx>, max_age_hours: u64) -> Vec<FixReport> {
    fixes::cached_reports(&ctx.cache, ttl(max_age_hours))
}

#[tauri::command]
pub fn get_fix_providers() -> Vec<ProviderInfo> {
    fixes::providers::catalog()
}

/// Limpa relatórios e respostas das fontes (mantém capas e biblioteca).
#[tauri::command]
pub fn clear_fix_cache(ctx: State<'_, AppCtx>) -> Result<(), String> {
    ctx.cache
        .clear(Some("fixes"))
        .and_then(|_| ctx.cache.clear(Some("providers")))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_cache_size(ctx: State<'_, AppCtx>) -> u64 {
    ctx.cache.size_bytes()
}
