use crate::system::{self, LaptopIdentity, SystemProfile};

#[tauri::command]
pub async fn get_system_profile() -> Result<SystemProfile, String> {
    tauri::async_runtime::spawn_blocking(system::profile)
        .await
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaptopInfo {
    #[serde(flatten)]
    pub identity: LaptopIdentity,
    /// Identidade simulada pelo fixture de desenvolvimento.
    pub simulated: bool,
}

/// Identidade usada pelos comandos: o DMI real, ou o laptop de teste em debug.
pub(crate) fn effective_identity() -> LaptopInfo {
    #[cfg(debug_assertions)]
    if let Some(identity) = super::dev_laptop::current() {
        return LaptopInfo { identity, simulated: true };
    }
    LaptopInfo {
        identity: system::identify_laptop(),
        simulated: false,
    }
}

/// Fabricante, modelo e placa do laptop lidos do DMI (`/sys/devices/virtual/dmi/id`).
#[tauri::command]
pub async fn identify_laptop() -> Result<LaptopInfo, String> {
    tauri::async_runtime::spawn_blocking(effective_identity)
        .await
        .map_err(|e| e.to_string())
}
