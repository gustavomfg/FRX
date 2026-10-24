//! GPUs a partir de `/sys/class/drm/card*/device`.

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Other,
}

impl GpuVendor {
    pub fn from_pci(id: u16) -> Self {
        match id {
            0x10de => Self::Nvidia,
            0x1002 | 0x1022 => Self::Amd,
            0x8086 => Self::Intel,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub vendor: GpuVendor,
    pub vendor_id: String,
    pub device_id: String,
    pub name: Option<String>,
    /// Módulo do kernel em uso (nvidia, nouveau, amdgpu, radeon, i915, xe…).
    pub driver: Option<String>,
    pub driver_version: Option<String>,
    pub boot_vga: bool,
}

/// Banco `pci.ids` do sistema, quando presente (hwdata/pciutils).
#[derive(Default)]
pub struct PciIds {
    devices: HashMap<(u16, u16), String>,
}

impl PciIds {
    pub fn load() -> Self {
        for p in [
            "/usr/share/hwdata/pci.ids",
            "/usr/share/misc/pci.ids",
            "/usr/share/pci.ids",
        ] {
            if let Ok(text) = std::fs::read_to_string(p) {
                return Self::parse(&text, &[0x10de, 0x1002, 0x8086, 0x1022]);
            }
        }
        Self::default()
    }

    /// Lê apenas os fabricantes de interesse para manter a memória baixa.
    pub fn parse(text: &str, vendors: &[u16]) -> Self {
        let mut devices = HashMap::new();
        let mut current: Option<u16> = None;
        for line in text.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if !line.starts_with('\t') {
                current = u16::from_str_radix(line.get(..4).unwrap_or(""), 16)
                    .ok()
                    .filter(|v| vendors.contains(v));
                continue;
            }
            let Some(v) = current else { continue };
            if line.starts_with("\t\t") {
                continue; // subsistemas
            }
            let l = line.trim_start_matches('\t');
            if let (Some(id), Some(name)) = (
                l.get(..4).and_then(|s| u16::from_str_radix(s, 16).ok()),
                l.get(4..),
            ) {
                devices.insert((v, id), name.trim().to_owned());
            }
        }
        Self { devices }
    }

    pub fn device(&self, vendor: u16, device: u16) -> Option<&str> {
        self.devices.get(&(vendor, device)).map(String::as_str)
    }
}

fn read_hex(p: &Path) -> Option<u16> {
    let s = std::fs::read_to_string(p).ok()?;
    u16::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()
}

/// Nome amigável: o texto entre colchetes do pci.ids costuma ser o nome comercial.
fn friendly(name: &str) -> String {
    match (name.find('['), name.rfind(']')) {
        (Some(a), Some(b)) if b > a + 1 => name[a + 1..b].to_owned(),
        _ => name.to_owned(),
    }
}

fn driver_version(driver: &str) -> Option<String> {
    let from_file = |p: &str| {
        std::fs::read_to_string(p)
            .ok()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
    };
    match driver {
        "nvidia" => from_file("/sys/module/nvidia/version").or_else(|| {
            let t = std::fs::read_to_string("/proc/driver/nvidia/version").ok()?;
            t.split_whitespace()
                .skip_while(|w| *w != "Module")
                .nth(2)
                .map(str::to_owned)
        }),
        other => from_file(&format!("/sys/module/{other}/version")),
    }
}

pub fn gpus(drm: &Path, ids: &PciIds) -> Vec<GpuInfo> {
    let Ok(rd) = std::fs::read_dir(drm) else {
        return vec![];
    };
    let mut cards: Vec<_> = rd
        .flatten()
        .filter(|e| {
            let n = e.file_name();
            let n = n.to_string_lossy();
            n.starts_with("card") && n[4..].chars().all(|c| c.is_ascii_digit()) && n.len() > 4
        })
        .map(|e| e.path())
        .collect();
    cards.sort();
    let mut out: Vec<GpuInfo> = Vec::new();
    for card in cards {
        let dev = card.join("device");
        let (Some(vendor), Some(device)) =
            (read_hex(&dev.join("vendor")), read_hex(&dev.join("device")))
        else {
            continue;
        };
        let driver = std::fs::read_link(dev.join("driver"))
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()));
        let info = GpuInfo {
            vendor: GpuVendor::from_pci(vendor),
            vendor_id: format!("{vendor:04x}"),
            device_id: format!("{device:04x}"),
            name: ids.device(vendor, device).map(friendly),
            driver_version: driver.as_deref().and_then(driver_version),
            driver,
            boot_vga: std::fs::read_to_string(dev.join("boot_vga"))
                .map(|s| s.trim() == "1")
                .unwrap_or(false),
        };
        if !out.iter().any(|g| {
            g.vendor_id == info.vendor_id
                && g.device_id == info.device_id
                && g.boot_vga == info.boot_vga
        }) {
            out.push(info);
        }
    }
    out
}

/// A GPU mais provável para jogos: NVIDIA/AMD dedicada antes da integrada.
pub fn primary(gpus: &[GpuInfo]) -> Option<&GpuInfo> {
    gpus.iter()
        .find(|g| g.vendor == GpuVendor::Nvidia && g.driver.as_deref() == Some("nvidia"))
        .or_else(|| {
            gpus.iter()
                .find(|g| g.vendor == GpuVendor::Amd && !g.boot_vga)
        })
        .or_else(|| gpus.iter().find(|g| g.boot_vga))
        .or_else(|| gpus.first())
}

/// Drivers Vulkan instalados (nomes dos manifestos ICD).
pub fn vulkan_icds() -> Vec<String> {
    let mut out = Vec::new();
    for dir in ["/usr/share/vulkan/icd.d", "/etc/vulkan/icd.d"] {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let n = e
                    .file_name()
                    .to_string_lossy()
                    .trim_end_matches(".json")
                    .to_owned();
                if !out.contains(&n) {
                    out.push(n);
                }
            }
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pci_ids_subset() {
        let ids = PciIds::parse(
            "# c\n10de  NVIDIA Corporation\n\t2882  AD107 [GeForce RTX 4060]\n\t\t1043 1234  sub\n1234  Outro\n\t0001  X\n",
            &[0x10de],
        );
        assert_eq!(ids.device(0x10de, 0x2882), Some("AD107 [GeForce RTX 4060]"));
        assert_eq!(ids.device(0x1234, 0x0001), None);
        assert_eq!(friendly("AD107 [GeForce RTX 4060]"), "GeForce RTX 4060");
    }

    #[test]
    fn reads_hybrid_gpus_from_sysfs_fixture() {
        let tmp = tempfile::tempdir().unwrap();
        let mk = |card: &str, vendor: &str, device: &str, boot: &str| {
            let d = tmp.path().join(card).join("device");
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("vendor"), vendor).unwrap();
            std::fs::write(d.join("device"), device).unwrap();
            std::fs::write(d.join("boot_vga"), boot).unwrap();
        };
        mk("card0", "0x1002\n", "0x1636\n", "1\n");
        mk("card1", "0x10de\n", "0x2882\n", "0\n");
        std::fs::create_dir_all(tmp.path().join("card1-DP-1")).unwrap();
        let g = gpus(tmp.path(), &PciIds::default());
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].vendor, GpuVendor::Amd);
        assert!(g[0].boot_vga);
        // Sem driver ligado, a primária cai para a de boot.
        assert_eq!(primary(&g).unwrap().vendor, GpuVendor::Amd);
        let mut with_driver = g.clone();
        with_driver[1].driver = Some("nvidia".into());
        assert_eq!(primary(&with_driver).unwrap().vendor, GpuVendor::Nvidia);
    }
}
