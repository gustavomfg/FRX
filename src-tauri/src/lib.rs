//! Camada nativa do FRX.
//!
//! Esta versão detecta jogos da Steam e pesquisa correções conhecidas em
//! fontes públicas. É **somente leitura**: nenhuma correção é aplicada,
//! nenhum arquivo de jogo, prefixo ou configuração da Steam é modificado e
//! nenhum script de terceiros é executado.

mod art;
mod cache;
mod commands;
mod fixes;
mod net;
mod steam;
mod system;
mod time;

use tauri::Manager;

use commands::AppCtx;

pub fn run() {
    configure_webkit();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let cache_dir = app.path().app_cache_dir()?;
            std::fs::create_dir_all(&cache_dir)?;
            let fetcher = net::ReqwestFetcher::new().map_err(|e| format!("cliente HTTP: {e}"))?;
            app.manage(AppCtx {
                cache: cache::Cache::new(cache_dir),
                fetcher,
                scan_lock: Default::default(),
                fixes: Default::default(),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::app_info,
            commands::app::ping,
            commands::system::get_system_profile,
            commands::games::scan_installed_games,
            commands::games::get_installed_games,
            commands::games::get_game_art,
            commands::fixes::search_game_fixes,
            commands::fixes::get_cached_fix_reports,
            commands::fixes::get_fix_providers,
            commands::fixes::clear_fix_cache,
            commands::fixes::get_cache_size,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o FRX");
}

/// Ajustes de compatibilidade do WebKitGTK no Linux.
///
/// Em sessões Wayland com GPU NVIDIA, o renderizador DMA-BUF do WebKitGTK
/// pode deixar a janela em branco. Desativá-lo é o contorno recomendado e não
/// afeta outras GPUs de forma perceptível. O usuário pode sobrescrever
/// definindo a variável antes de abrir o app.
fn configure_webkit() {
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }
}
