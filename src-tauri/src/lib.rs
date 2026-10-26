//! Camada nativa do FRX.
//!
//! Detecta jogos da Steam, pesquisa correções conhecidas em fontes públicas,
//! registra benchmarks reais com o MangoHud e aplica correções **revisadas**
//! do catálogo local pelo Fix Engine (prévia, backup, verificação, rollback).
//! A descoberta continua somente leitura e nenhum script de terceiros é
//! executado; só o Fix Engine escreve, e apenas o que o plano confirmado diz.

mod art;
mod cache;
mod commands;
mod fix_engine;
mod fixes;
mod net;
mod performance;
mod smart_packs;
mod steam;
mod system;
mod time;

use tauri::Manager;

use commands::AppCtx;

pub fn run() {
    configure_webkit();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                window.state::<AppCtx>().perf.capture.shutdown();
            }
        })
        .setup(|app| {
            let cache_dir = app.path().app_cache_dir()?;
            std::fs::create_dir_all(&cache_dir)?;
            let fetcher = net::ReqwestFetcher::new().map_err(|e| format!("cliente HTTP: {e}"))?;
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let engine =
                std::sync::Arc::new(fix_engine::FixEngine::new(fix_engine::data_root(&data_dir)));
            let perf = commands::performance::PerfCtx {
                store: performance::persistence::Store::new(data_dir.join("benchmarks")),
                data_dir,
                capture: Default::default(),
            };
            app.manage(AppCtx {
                cache: cache::Cache::new(cache_dir),
                fetcher,
                scan_lock: Default::default(),
                fixes: Default::default(),
                perf,
                engine,
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
            commands::performance::detect_mangohud,
            commands::performance::import_benchmark,
            commands::performance::list_benchmark_sessions,
            commands::performance::get_benchmark_session,
            commands::performance::compare_benchmark_sessions,
            commands::performance::update_benchmark_session,
            commands::performance::delete_benchmark_session,
            commands::performance::export_benchmark_session,
            commands::performance::get_capture_status,
            commands::performance::prepare_benchmark,
            commands::performance::start_capture,
            commands::performance::capture_logging,
            commands::performance::finish_capture,
            commands::performance::cancel_capture,
            commands::performance::reset_capture,
            commands::fix_engine::get_fix_engine_status,
            commands::fix_engine::list_fix_recipes,
            commands::fix_engine::get_fix_recipe,
            commands::fix_engine::analyze_fix_findings,
            commands::smart_packs::get_smart_packs,
            commands::smart_packs::compare_smart_packs,
            commands::smart_packs::plan_smart_pack_switch,
            commands::smart_packs::declare_benchmark_pack,
            commands::fix_engine::check_fix_compatibility,
            commands::fix_engine::prepare_fix_plan,
            commands::fix_engine::apply_fix_plan,
            commands::fix_engine::get_fix_operation,
            commands::fix_engine::list_fix_operations,
            commands::fix_engine::get_applied_fixes,
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
