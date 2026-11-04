//! Comandos gerais do app (informações do sistema para a tela de ajustes).
//!
//! Somente leitura.

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    name: String,
    version: String,
    tauri_version: &'static str,
    os: &'static str,
    arch: &'static str,
    kernel: Option<String>,
    distro: Option<String>,
    session: Option<String>,
    desktop: Option<String>,
}

#[tauri::command]
pub fn app_info(app: tauri::AppHandle) -> AppInfo {
    let pkg = app.package_info();
    AppInfo {
        name: pkg.name.clone(),
        version: pkg.version.to_string(),
        tauri_version: tauri::VERSION,
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        kernel: read_trimmed("/proc/sys/kernel/osrelease"),
        distro: os_release_pretty_name(),
        session: std::env::var("XDG_SESSION_TYPE").ok(),
        desktop: std::env::var("XDG_CURRENT_DESKTOP").ok(),
    }
}

fn read_trimmed(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_owned())
}

fn os_release_pretty_name() -> Option<String> {
    let text = std::fs::read_to_string("/etc/os-release").ok()?;
    text.lines()
        .find_map(|l| l.strip_prefix("PRETTY_NAME="))
        .map(|v| v.trim_matches('"').to_owned())
}
