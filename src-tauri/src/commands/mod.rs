//! Comandos Tauri expostos ao front-end. Só o Fix Engine escreve, e apenas
//! depois de prévia, confirmação, backup e nova verificação no núcleo.

pub mod app;
/// Só em build de debug: laptop simulado para ver o fluxo de hardware.
#[cfg(debug_assertions)]
pub mod dev_laptop;
pub mod fix_engine;
pub mod hardware;
pub mod system;

use crate::cache::Cache;
use crate::fixes::http::EngineState;
use crate::net::ReqwestFetcher;

/// Estado compartilhado entre os comandos.
pub struct AppCtx {
    pub cache: Cache,
    pub fetcher: ReqwestFetcher,
    /// Bloqueios de limite de consultas da Arch Wiki.
    pub limits: EngineState,
    pub engine: std::sync::Arc<crate::fix_engine::FixEngine>,
}
