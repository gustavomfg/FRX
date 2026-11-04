//! Camada nativa do FRX (Hardware Fixer).
//!
//! Lê a identidade do laptop pelo DMI, pesquisa instruções na Arch Wiki e
//! aplica as reconhecidas pelo Fix Engine (prévia, backup, verificação,
//! desfazer). Só o Fix Engine escreve, e apenas o que o plano confirmado diz.
//! Fora de testes, ele não grava em `/etc`: prepara o arquivo e mostra o
//! comando `sudo` para o próprio usuário rodar.

mod cache;
mod commands;
mod fix_engine;
mod fixes;
mod net;
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
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let engine = fix_engine::FixEngine::new(fix_engine::data_root(&data_dir));
            app.manage(AppCtx {
                cache: cache::Cache::new(cache_dir),
                fetcher,
                limits: Default::default(),
                engine: std::sync::Arc::new(engine),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::app_info,
            commands::system::get_system_profile,
            commands::system::identify_laptop,
            commands::hardware::search_hardware_fixes,
            commands::hardware::prepare_system_fix,
            commands::hardware::apply_system_fix,
            commands::hardware::get_fix_providers,
            commands::hardware::get_cache_size,
            commands::hardware::clear_fix_cache,
            #[cfg(debug_assertions)]
            commands::dev_laptop::set_dev_laptop,
            commands::fix_engine::list_fix_operations,
            commands::fix_engine::prepare_fix_rollback,
            commands::fix_engine::rollback_fix,
            commands::fix_engine::verify_fix_operation,
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
