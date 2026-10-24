//! Perfil do sistema obtido por interfaces do kernel (`/proc`, `/sys`) e
//! arquivos padrão (`/etc/os-release`, `pci.ids`). Sem comandos externos,
//! sem ferramentas exclusivas de fabricante e sem privilégios.

pub mod hardware;

use serde::Serialize;

pub use hardware::{GpuInfo, GpuVendor};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemProfile {
    pub distro: Option<String>,
    pub distro_id: Option<String>,
    pub kernel: Option<String>,
    pub arch: &'static str,
    pub session_type: Option<String>,
    pub desktop: Option<String>,
    pub cpu: Option<String>,
    pub cpu_threads: Option<usize>,
    pub ram_bytes: Option<u64>,
    pub gpus: Vec<GpuInfo>,
    /// GPU que provavelmente roda os jogos (dedicada ou a de boot).
    pub primary_gpu: Option<GpuInfo>,
    pub vulkan_drivers: Vec<String>,
}

pub fn profile() -> SystemProfile {
    let os = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let field = |k: &str| {
        os.lines()
            .find_map(|l| l.strip_prefix(&format!("{k}=")))
            .map(|v| v.trim().trim_matches('"').to_owned())
            .filter(|v| !v.is_empty())
    };
    let gpus = hardware::gpus(
        std::path::Path::new("/sys/class/drm"),
        &hardware::PciIds::load(),
    );
    SystemProfile {
        distro: field("PRETTY_NAME").or_else(|| field("NAME")),
        distro_id: field("ID"),
        kernel: read_trim("/proc/sys/kernel/osrelease"),
        arch: std::env::consts::ARCH,
        session_type: std::env::var("XDG_SESSION_TYPE")
            .ok()
            .filter(|s| !s.is_empty()),
        desktop: std::env::var("XDG_CURRENT_DESKTOP")
            .ok()
            .filter(|s| !s.is_empty()),
        cpu: cpu_model(),
        cpu_threads: std::thread::available_parallelism().ok().map(|n| n.get()),
        ram_bytes: mem_total(),
        primary_gpu: hardware::primary(&gpus).cloned(),
        gpus,
        vulkan_drivers: hardware::vulkan_icds(),
    }
}

fn read_trim(p: &str) -> Option<String> {
    std::fs::read_to_string(p)
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

fn cpu_model() -> Option<String> {
    let info = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    info.lines()
        .find(|l| l.starts_with("model name"))
        .and_then(|l| l.split_once(':'))
        .map(|(_, v)| v.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn mem_total() -> Option<u64> {
    let info = std::fs::read_to_string("/proc/meminfo").ok()?;
    let kb: u64 = info
        .lines()
        .find(|l| l.starts_with("MemTotal:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()?;
    Some(kb * 1024)
}
