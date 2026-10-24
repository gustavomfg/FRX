use crate::system::{self, SystemProfile};

#[tauri::command]
pub async fn get_system_profile() -> Result<SystemProfile, String> {
    tauri::async_runtime::spawn_blocking(system::profile)
        .await
        .map_err(|e| e.to_string())
}
