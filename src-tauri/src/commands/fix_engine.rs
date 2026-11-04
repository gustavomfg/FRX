//! Comandos do Fix Engine. A interface só envia identificadores (operação e
//! plano) e a impressão digital da prévia; caminhos, valores e passos são
//! sempre recalculados no núcleo.

use std::sync::Arc;

use tauri::State;

use super::AppCtx;
use crate::fix_engine::models::{Operation, RollbackPlan};
use crate::fix_engine::FixEngine;

pub(crate) async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

pub(crate) fn engine(ctx: &AppCtx) -> Arc<FixEngine> {
    ctx.engine.clone()
}

/// Operações registradas no diário, mais recentes primeiro.
#[tauri::command]
pub async fn list_fix_operations(ctx: State<'_, AppCtx>) -> Result<Vec<Operation>, String> {
    let e = engine(&ctx);
    blocking(move || Ok(e.operations())).await
}

/// Prévia do desfazer: o que será removido ou restaurado.
#[tauri::command]
pub async fn prepare_fix_rollback(
    ctx: State<'_, AppCtx>,
    operation_id: String,
) -> Result<RollbackPlan, String> {
    let e = engine(&ctx);
    blocking(move || e.prepare_rollback(&operation_id)).await
}

/// Desfaz a operação. Também cancela uma prévia que ainda não foi aplicada.
#[tauri::command]
pub async fn rollback_fix(
    ctx: State<'_, AppCtx>,
    operation_id: String,
    fingerprint: String,
) -> Result<Operation, String> {
    let e = engine(&ctx);
    blocking(move || e.rollback(&operation_id, &fingerprint)).await
}

/// Confere no disco um passo manual (linha colada no terminal) ou uma operação interrompida.
#[tauri::command]
pub async fn verify_fix_operation(
    ctx: State<'_, AppCtx>,
    operation_id: String,
) -> Result<Operation, String> {
    let e = engine(&ctx);
    blocking(move || e.verify(&operation_id)).await
}
