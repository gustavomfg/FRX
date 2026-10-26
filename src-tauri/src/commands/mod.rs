//! Comandos Tauri expostos ao front-end. Só o Fix Engine escreve, e apenas
//! depois de prévia, confirmação, backup e nova verificação no núcleo.

pub mod app;
pub mod fix_engine;
pub mod fixes;
pub mod games;
pub mod performance;
pub mod smart_packs;
pub mod system;

use crate::cache::Cache;
use crate::net::ReqwestFetcher;

/// Estado compartilhado entre os comandos.
pub struct AppCtx {
    pub cache: Cache,
    pub fetcher: ReqwestFetcher,
    /// Evita duas varreduras simultâneas da biblioteca.
    pub scan_lock: tokio::sync::Mutex<()>,
    pub fixes: crate::fixes::EngineState,
    pub perf: performance::PerfCtx,
    pub engine: std::sync::Arc<crate::fix_engine::FixEngine>,
}
