//! Comandos Tauri expostos ao front-end. Todos são somente leitura.

pub mod app;
pub mod fixes;
pub mod games;
pub mod performance;
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
}
