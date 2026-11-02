//! Versão de driver como o ranker a entende. Não substitui `system/`: lá fica o
//! que o sistema relata, aqui fica como comparar.
//!
//! `/sys/module/<driver>/version` de amdgpu, radeon, i915 ou xe é do módulo do
//! kernel, não do Mesa, e nunca é usado como tal. A versão do Mesa vem só de
//! leitura de arquivos, sem executar nada. Sem fonte confiável, é `Unknown`.

use std::path::Path;

use super::models::SystemSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverInfo {
    Nvidia { family: u32, raw: String },
    Mesa { major: u32, minor: u32, raw: String },
    Unknown { raw: String, reason: String },
}

fn unknown(raw: &str, reason: &str) -> DriverInfo {
    DriverInfo::Unknown {
        raw: raw.to_owned(),
        reason: reason.to_owned(),
    }
}

/// Primeiro grupo de 3 dígitos, como 580 em "580.65.06".
fn nvidia(raw: &str) -> Option<DriverInfo> {
    let first = raw.trim().split(['.', '-']).next()?;
    (first.len() == 3 && first.bytes().all(|b| b.is_ascii_digit())).then(|| DriverInfo::Nvidia {
        family: first.parse().unwrap_or(0),
        raw: raw.to_owned(),
    })
}

/// "25.3.1" ou "26.2.5-arch3.1". A época de pacote ("3:") já foi retirada.
fn mesa(raw: &str) -> Option<DriverInfo> {
    let mut parts = raw.trim().split(['.', '-', '+', '~']);
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next().and_then(|m| m.parse().ok()).unwrap_or(0);
    (major > 0 && major < 100).then(|| DriverInfo::Mesa {
        major,
        minor,
        raw: raw.to_owned(),
    })
}

#[derive(Clone, Copy)]
enum Family {
    Nvidia,
    Mesa,
}

/// Pelo módulo do kernel. Sem módulo informado, pelo modelo. Módulo desconhecido
/// não recorre ao modelo: "nouveau" numa GeForce não é o driver da NVIDIA.
fn family_of(driver: Option<&str>, name: Option<&str>) -> Option<Family> {
    match driver.map(|d| d.trim().to_ascii_lowercase()).as_deref() {
        Some("nvidia" | "nvidia-drm" | "nvidia_drm") => Some(Family::Nvidia),
        Some("amdgpu" | "radeon" | "i915" | "xe") => Some(Family::Mesa),
        None | Some("") => {
            let low = name?.to_lowercase();
            let has = |w: &[&str]| {
                low.split(|c: char| !c.is_alphanumeric())
                    .any(|x| w.contains(&x))
            };
            if has(&["nvidia", "geforce", "rtx", "gtx"]) {
                Some(Family::Nvidia)
            } else if has(&["amd", "radeon", "intel", "arc"]) {
                Some(Family::Mesa)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// O driver do sistema, sem ler disco. Mesa só vem de `mesa_version`.
pub fn parse(system: &SystemSnapshot) -> DriverInfo {
    let Some(gpu) = system.gpu() else {
        return unknown("", "A GPU não foi identificada.");
    };
    let raw = gpu.driver_version.as_deref().unwrap_or("");
    match family_of(gpu.driver.as_deref(), gpu.name.as_deref()) {
        Some(Family::Nvidia) => nvidia(raw)
            .unwrap_or_else(|| unknown(raw, "A versão do driver NVIDIA não tem o formato esperado.")),
        Some(Family::Mesa) => system
            .mesa_version
            .as_deref()
            .and_then(mesa)
            .unwrap_or_else(|| {
                unknown(raw, "A versão do Mesa não pôde ser lida com confiança; a do módulo do kernel não serve.")
            }),
        None => unknown(raw, "O driver em uso não é reconhecido."),
    }
}

fn step_score(distance: u32) -> f32 {
    match distance {
        0 => 1.0,
        1 => 0.5,
        2 => 0.2,
        _ => 0.0,
    }
}

/// A NVIDIA numera em saltos: 575 e 580 são vizinhos. Distância em blocos de 10.
fn nvidia_distance(a: u32, b: u32) -> u32 {
    a.abs_diff(b).div_ceil(10)
}

/// `None` quando não há como comparar: vendors diferentes ou versão desconhecida.
pub fn proximity(a: &DriverInfo, b: &DriverInfo) -> Option<f32> {
    match (a, b) {
        (DriverInfo::Nvidia { family: x, .. }, DriverInfo::Nvidia { family: y, .. }) => {
            Some(step_score(nvidia_distance(*x, *y)))
        }
        (DriverInfo::Mesa { major: x, .. }, DriverInfo::Mesa { major: y, .. }) => {
            Some(step_score(x.abs_diff(*y)))
        }
        _ => None,
    }
}

/// Distância até uma versão mais antiga que a do usuário. Mais nova ou
/// incomparável dá `None`.
pub fn behind(own: &DriverInfo, other: &DriverInfo) -> Option<u32> {
    match (own, other) {
        (DriverInfo::Nvidia { family: x, .. }, DriverInfo::Nvidia { family: y, .. }) if x > y => {
            Some(nvidia_distance(*x, *y))
        }
        (DriverInfo::Mesa { major: x, .. }, DriverInfo::Mesa { major: y, .. }) if x > y => {
            Some(x - y)
        }
        _ => None,
    }
}

/// Desempate dentro da mesma major do Mesa. Não entra na proximidade.
pub fn minor_gap(own: &DriverInfo, other: &DriverInfo) -> Option<u32> {
    match (own, other) {
        (
            DriverInfo::Mesa {
                major: a, minor: x, ..
            },
            DriverInfo::Mesa {
                major: b, minor: y, ..
            },
        ) if a == b => Some(x.abs_diff(*y)),
        _ => None,
    }
}

/// Forma de versão NVIDIA completa, como 580.65.06 ou 535.183.01. Rejeita
/// endereços como 192.168.1.1 e versões de kernel como 6.10.4.
fn full_nvidia_shape(word: &str) -> bool {
    let parts: Vec<&str> = word.split('.').collect();
    matches!(parts.as_slice(), [a, b, c]
        if a.len() == 3 && (2..=3).contains(&b.len()) && c.len() == 2)
}

/// Versões citadas num texto. Um número solto, como "144.5 fps", não conta:
/// a NVIDIA exige a forma completa ou uma palavra de contexto logo antes, e o
/// Mesa exige o nome seguido de uma versão com ponto.
pub fn mentions(text: &str) -> Vec<DriverInfo> {
    const NVIDIA_WORDS: &[&str] = &["driver", "drivers", "nvidia", "geforce"];
    let low = text.to_lowercase();
    let words: Vec<&str> = low
        .split(|c: char| !(c.is_alphanumeric() || c == '.' || c == '-' || c == '_'))
        .map(|w| w.trim_matches(['.', '-', '_']))
        .filter(|w| !w.is_empty())
        .collect();
    let dotted_mesa = |w: &str| if w.contains('.') { mesa(w) } else { None };
    let mut out = Vec::new();
    for (i, word) in words.iter().enumerate() {
        if *word == "mesa" {
            out.extend(words.get(i + 1).and_then(|n| dotted_mesa(n)));
        } else if let Some(rest) = word.strip_prefix("mesa-") {
            out.extend(dotted_mesa(rest));
        } else if word.contains('.') && word.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
            let before = |n: usize| i.checked_sub(n).map(|k| words[k]);
            let context = before(1).is_some_and(|w| NVIDIA_WORDS.contains(&w))
                || (before(1) == Some("version")
                    && before(2).is_some_and(|w| NVIDIA_WORDS.contains(&w)));
            let two_parts = word.matches('.').count() == 1;
            if full_nvidia_shape(word) || (context && two_parts) {
                out.extend(nvidia(word));
            }
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MesaReading {
    pub version: Option<String>,
    pub sources: Vec<String>,
    pub reason: Option<String>,
}

fn after_epoch(version: &str) -> &str {
    version.split_once(':').map_or(version, |(_, v)| v)
}

/// Versão num arquivo `desc` do banco local do pacman.
fn from_pacman_desc(desc: &str) -> Option<String> {
    let field = |name: &str| -> Option<&str> {
        let mut lines = desc.lines();
        lines.find(|l| *l == name)?;
        lines.next().map(str::trim)
    };
    if field("%NAME%")? != "mesa" {
        return None;
    }
    Some(after_epoch(field("%VERSION%")?).to_owned())
}

/// Versão num `/var/lib/dpkg/status`: pacote Mesa conhecido e instalado de fato.
pub fn from_dpkg_status(status: &str) -> Option<String> {
    const PACKAGES: &[&str] = &[
        "libgl1-mesa-dri",
        "mesa-vulkan-drivers",
        "libglx-mesa0",
        "libegl-mesa0",
    ];
    status.split("\n\n").find_map(|stanza| {
        let field = |name: &str| {
            stanza
                .lines()
                .find_map(|l| l.strip_prefix(name)?.strip_prefix(": "))
                .map(str::trim)
        };
        (PACKAGES.contains(&field("Package")?) && field("Status")? == "install ok installed")
            .then(|| after_epoch(field("Version").unwrap_or("")).to_owned())
            .filter(|v| !v.is_empty())
    })
}

/// Versão no nome `libgallium-<versão>.so`. Mais de uma na pasta é ambíguo.
pub fn from_gallium_names<'a>(names: impl Iterator<Item = &'a str>) -> Option<String> {
    let found: Vec<&str> = names
        .filter_map(|n| n.strip_prefix("libgallium-")?.strip_suffix(".so"))
        .collect();
    match found.as_slice() {
        [one] => Some((*one).to_owned()),
        _ => None,
    }
}

fn file_names(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

const MAX_DB_BYTES: u64 = 64 * 1024 * 1024;

fn read_bounded(path: &Path) -> Option<String> {
    (std::fs::metadata(path).ok()?.len() <= MAX_DB_BYTES)
        .then(|| std::fs::read_to_string(path).ok())
        .flatten()
}

fn major_minor(version: &str) -> Option<(u32, u32)> {
    match mesa(version)? {
        DriverInfo::Mesa { major, minor, .. } => Some((major, minor)),
        _ => None,
    }
}

/// Lê a versão instalada no host a partir de `root`. Só abre e lista arquivos.
/// Fontes que discordam em major ou minor resultam em nenhuma versão.
pub fn read_host_mesa(root: &Path) -> MesaReading {
    let mut found: Vec<(&str, String)> = Vec::new();
    let pacman = root.join("var/lib/pacman/local");
    for name in file_names(&pacman)
        .iter()
        .filter(|n| n.starts_with("mesa-"))
    {
        if let Some(v) = read_bounded(&pacman.join(name).join("desc"))
            .as_deref()
            .and_then(from_pacman_desc)
        {
            found.push(("pacman", v));
        }
    }
    if let Some(v) = read_bounded(&root.join("var/lib/dpkg/status"))
        .as_deref()
        .and_then(from_dpkg_status)
    {
        found.push(("dpkg", v));
    }
    for dir in ["usr/lib64", "usr/lib/x86_64-linux-gnu", "usr/lib"] {
        let names = file_names(&root.join(dir));
        if let Some(v) = from_gallium_names(names.iter().map(String::as_str)) {
            found.push(("libgallium", v));
            break;
        }
    }
    let sources = found.iter().map(|(s, _)| (*s).to_owned()).collect();
    let versions: Vec<(u32, u32)> = found.iter().filter_map(|(_, v)| major_minor(v)).collect();
    if versions.is_empty() {
        return MesaReading {
            version: None,
            sources,
            reason: Some("Nenhuma fonte de versão do Mesa foi encontrada.".into()),
        };
    }
    if versions.len() != found.len() || versions.iter().any(|v| *v != versions[0]) {
        return MesaReading {
            version: None,
            sources,
            reason: Some("As fontes de versão do Mesa discordam.".into()),
        };
    }
    MesaReading {
        version: found.first().map(|(_, v)| v.clone()),
        sources,
        reason: None,
    }
}

#[cfg(test)]
mod ai_ranker_driver {
    use super::super::models::GpuInfo;
    use super::*;

    fn system(
        driver: Option<&str>,
        name: &str,
        version: Option<&str>,
        mesa: Option<&str>,
    ) -> SystemSnapshot {
        let gpu: GpuInfo = serde_json::from_value(serde_json::json!({
            "vendor": "other", "vendorId": "0", "deviceId": "0", "name": name,
            "driver": driver, "driverVersion": version, "bootVga": true
        }))
        .unwrap();
        SystemSnapshot {
            gpus: vec![gpu.clone()],
            primary_gpu: Some(gpu),
            vulkan_drivers: vec![],
            mesa_version: mesa.map(str::to_owned),
        }
    }

    fn nv(version: &str) -> DriverInfo {
        nvidia(version).unwrap()
    }

    fn m(version: &str) -> DriverInfo {
        mesa(version).unwrap()
    }

    #[test]
    fn parse_nvidia_family_from_version() {
        for driver in ["nvidia", "nvidia-drm", "nvidia_drm"] {
            let info = parse(&system(Some(driver), "x", Some("580.65.06"), None));
            assert!(
                matches!(info, DriverInfo::Nvidia { family: 580, .. }),
                "{driver}"
            );
        }
        let by_model = parse(&system(
            None,
            "NVIDIA GeForce RTX 4070",
            Some("575.64"),
            None,
        ));
        assert!(matches!(by_model, DriverInfo::Nvidia { family: 575, .. }));
        assert!(matches!(
            parse(&system(Some("nvidia"), "x", Some("bad"), None)),
            DriverInfo::Unknown { .. }
        ));
    }

    #[test]
    fn parse_mesa_from_dpkg_output() {
        let status = "Package: libfoo\nStatus: install ok installed\nVersion: 1.0\n\n\
Package: mesa-vulkan-drivers\nStatus: deinstall ok config-files\nVersion: 1:20.0.0-1\n\n\
Package: libgl1-mesa-dri\nStatus: install ok installed\nVersion: 1:25.0.7-0ubuntu0.24.04.2\n\n";
        let version = from_dpkg_status(status).unwrap();
        assert_eq!(
            version, "25.0.7-0ubuntu0.24.04.2",
            "ignora o pacote removido"
        );
        let info = parse(&system(Some("amdgpu"), "x", Some("6.10.4"), Some(&version)));
        assert!(matches!(
            info,
            DriverInfo::Mesa {
                major: 25,
                minor: 0,
                ..
            }
        ));
    }

    #[test]
    fn parse_mesa_from_pacman_desc_and_gallium_name() {
        let desc = "%NAME%\nmesa\n\n%VERSION%\n3:26.2.5-1\n\n%BASE%\nmesa\n";
        assert_eq!(from_pacman_desc(desc).as_deref(), Some("26.2.5-1"));
        assert_eq!(
            from_pacman_desc("%NAME%\nmesa-utils\n\n%VERSION%\n9.0-1\n"),
            None
        );
        let names = ["libc.so", "libgallium-26.2.5-arch3.1.so"];
        assert_eq!(
            from_gallium_names(names.into_iter()).as_deref(),
            Some("26.2.5-arch3.1")
        );
        let two = ["libgallium-26.2.5.so", "libgallium-25.0.1.so"];
        assert_eq!(from_gallium_names(two.into_iter()), None, "ambíguo");
    }

    #[test]
    fn parse_mesa_returns_unknown_when_no_source_is_confident() {
        // A versão do módulo do kernel nunca vira Mesa.
        for driver in ["amdgpu", "radeon", "i915", "xe"] {
            let info = parse(&system(Some(driver), "x", Some("6.10.4"), None));
            let DriverInfo::Unknown { reason, .. } = info else {
                panic!("{driver}: {info:?}");
            };
            assert!(reason.contains("kernel"));
        }
        let empty = tempfile::tempdir().unwrap();
        let reading = read_host_mesa(empty.path());
        assert_eq!(reading.version, None);
        assert!(reading.reason.unwrap().contains("Nenhuma fonte"));
    }

    #[test]
    fn host_sources_that_disagree_give_no_version() {
        let root = tempfile::tempdir().unwrap();
        let local = root.path().join("var/lib/pacman/local/mesa-3:26.2.5-1");
        std::fs::create_dir_all(&local).unwrap();
        std::fs::write(
            local.join("desc"),
            "%NAME%\nmesa\n\n%VERSION%\n3:26.2.5-1\n",
        )
        .unwrap();
        assert_eq!(
            read_host_mesa(root.path()).version.as_deref(),
            Some("26.2.5-1")
        );

        let lib = root.path().join("usr/lib");
        std::fs::create_dir_all(&lib).unwrap();
        std::fs::write(lib.join("libgallium-26.2.5-arch3.1.so"), "").unwrap();
        let agree = read_host_mesa(root.path());
        assert_eq!(agree.version.as_deref(), Some("26.2.5-1"));
        assert_eq!(agree.sources, ["pacman", "libgallium"]);

        std::fs::remove_file(lib.join("libgallium-26.2.5-arch3.1.so")).unwrap();
        std::fs::write(lib.join("libgallium-24.1.0.so"), "").unwrap();
        let conflict = read_host_mesa(root.path());
        assert_eq!(conflict.version, None);
        assert!(conflict.reason.unwrap().contains("discordam"));
    }

    #[test]
    fn unrecognized_drivers_and_missing_gpus_are_unknown() {
        assert!(matches!(
            parse(&SystemSnapshot::default()),
            DriverInfo::Unknown { .. }
        ));
        for driver in ["", "unknown", "nouveau", "vfio-pci"] {
            let name = if driver.is_empty() {
                "Placa"
            } else {
                "NVIDIA GeForce RTX 4070"
            };
            let info = parse(&system(Some(driver), name, Some("580.65"), None));
            assert!(matches!(info, DriverInfo::Unknown { .. }), "{driver}");
        }
    }

    #[test]
    fn proximity_nvidia_same_family_is_one() {
        assert_eq!(proximity(&nv("580.65.06"), &nv("580.65.06")), Some(1.0));
        assert_eq!(proximity(&nv("580.82"), &nv("580.65.06")), Some(1.0));
    }

    #[test]
    fn proximity_nvidia_adjacent_family_is_half() {
        assert_eq!(proximity(&nv("575.64"), &nv("580.65.06")), Some(0.5));
        assert_eq!(proximity(&nv("580.65.06"), &nv("575.64")), Some(0.5));
    }

    #[test]
    fn proximity_nvidia_two_families_apart_is_low() {
        assert_eq!(proximity(&nv("565.57"), &nv("580.65.06")), Some(0.2));
    }

    #[test]
    fn proximity_nvidia_far_apart_is_zero() {
        assert_eq!(proximity(&nv("535.183"), &nv("580.65.06")), Some(0.0));
    }

    #[test]
    fn proximity_mesa_same_major_is_one() {
        assert_eq!(
            proximity(&m("24.2.0"), &m("24.0.9")),
            Some(1.0),
            "sem distância em minor"
        );
    }

    #[test]
    fn proximity_mesa_one_major_apart_is_half() {
        assert_eq!(proximity(&m("23.3.0"), &m("24.2.0")), Some(0.5));
        assert_eq!(proximity(&m("22.3.0"), &m("24.2.0")), Some(0.2));
        assert_eq!(proximity(&m("21.3.0"), &m("24.2.0")), Some(0.0));
    }

    #[test]
    fn minor_only_breaks_ties_inside_the_same_major() {
        assert_eq!(minor_gap(&m("24.2.0"), &m("24.0.9")), Some(2));
        assert_eq!(minor_gap(&m("24.2.0"), &m("23.2.0")), None);
        assert_eq!(minor_gap(&nv("580.1"), &nv("580.2")), None);
    }

    #[test]
    fn proximity_cross_vendor_is_none() {
        let u = unknown("", "x");
        assert_eq!(proximity(&nv("580.65"), &m("24.2.0")), None);
        assert_eq!(proximity(&m("24.2.0"), &nv("580.65")), None);
        assert_eq!(proximity(&u, &nv("580.65")), None);
        assert_eq!(proximity(&m("24.2.0"), &u), None);
    }

    #[test]
    fn mentions_read_versions_and_ignore_loose_numbers() {
        assert_eq!(mentions("Using Mesa 25.1.7 here"), [m("25.1.7")]);
        assert_eq!(mentions("built with mesa-24.2.0"), [m("24.2.0")]);
        assert_eq!(mentions("driver 580.65"), [nv("580.65")]);
        assert_eq!(mentions("on 595.71.05"), [nv("595.71.05")]);
        assert!(mentions("I get 144.5 fps at 165.0 hz").is_empty());
        assert!(mentions("Proton 9.0-4 and kernel 6.10.4").is_empty());
        assert_eq!(mentions("driver version 535.183"), [nv("535.183")]);
        assert_eq!(mentions("on 535.183.01"), [nv("535.183.01")]);
        assert!(mentions("game version 123.45").is_empty());
        assert!(mentions("server at 192.168.1.1 and 192.168.1").is_empty());
        assert!(mentions("mesa 2 times").is_empty());
    }

    #[test]
    #[ignore = "lê o sistema real; só para conferência manual"]
    fn prints_the_host_reading() {
        println!("{:?}", read_host_mesa(Path::new("/")));
    }
}
