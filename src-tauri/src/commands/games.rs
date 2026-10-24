use std::path::PathBuf;

use tauri::State;

use super::AppCtx;
use crate::art::{self, ArtKind, ArtResult};
use crate::cache::Entry;
use crate::steam::{self, discovery::DiscoveryEnv, models::ScanResult};

const LIBRARY_KEY: &str = "steam";

/// Varre as instalações da Steam (somente leitura) e guarda o resultado em cache.
#[tauri::command]
pub async fn scan_installed_games(ctx: State<'_, AppCtx>) -> Result<ScanResult, String> {
    let _guard = ctx.scan_lock.lock().await;
    let previous: Option<ScanResult> = ctx
        .cache
        .get::<ScanResult>("library", LIBRARY_KEY)
        .map(|e| e.data);
    let result = tauri::async_runtime::spawn_blocking(move || {
        steam::scan(&DiscoveryEnv::from_process(), previous.as_ref())
    })
    .await
    .map_err(|e| format!("falha na varredura: {e}"))?;
    if let Err(e) = ctx
        .cache
        .put("library", LIBRARY_KEY, &Entry::new(result.clone(), None))
    {
        eprintln!("[frx] não foi possível salvar o cache da biblioteca: {e}");
    }
    Ok(result)
}

/// Última varredura salva — permite mostrar a biblioteca antes da nova varredura terminar.
#[tauri::command]
pub fn get_installed_games(ctx: State<'_, AppCtx>) -> Option<ScanResult> {
    ctx.cache
        .get::<ScanResult>("library", LIBRARY_KEY)
        .map(|e| e.data)
}

fn steam_roots(ctx: &AppCtx) -> Vec<PathBuf> {
    match ctx.cache.get::<ScanResult>("library", LIBRARY_KEY) {
        Some(e) => e
            .data
            .steam_installs
            .iter()
            .map(|i| PathBuf::from(&i.path))
            .collect(),
        None => steam::discovery::discover(&DiscoveryEnv::from_process())
            .into_iter()
            .map(|i| PathBuf::from(i.path))
            .collect(),
    }
}

/// Caminho local de uma arte do jogo (capa, banner ou cabeçalho).
#[tauri::command]
pub async fn get_game_art(
    ctx: State<'_, AppCtx>,
    app_id: String,
    kind: String,
    offline: Option<bool>,
) -> Result<ArtResult, String> {
    let kind = ArtKind::parse(&kind).ok_or("tipo de arte inválido")?;
    let roots = steam_roots(&ctx);
    let dir = ctx.cache.ns_dir("art");
    Ok(art::resolve(
        &ctx.fetcher,
        &dir,
        &roots,
        &app_id,
        kind,
        offline.unwrap_or(false),
    )
    .await)
}
