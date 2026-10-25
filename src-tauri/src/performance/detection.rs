//! Detecção do MangoHud — sem instalar nada e sem privilégios.
//!
//! O executável `mangohud` sozinho não garante que o overlay funcione: o que
//! carrega nos jogos é a camada Vulkan (manifesto em `implicit_layer.d`) e as
//! bibliotecas de 32/64 bits. Também verificamos o caso da Steam Flatpak, que
//! precisa da extensão `org.freedesktop.Platform.VulkanLayer.MangoHud`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LayerInfo {
    pub manifest: String,
    pub arch: String,
    pub library: String,
    pub library_exists: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MangoHudStatus {
    pub installed: bool,
    pub executable: Option<String>,
    pub version: Option<String>,
    pub layers: Vec<LayerInfo>,
    pub has_64bit: bool,
    pub has_32bit: bool,
    pub opengl_shim: bool,
    pub mangohudctl: Option<String>,
    pub mangoapp: Option<String>,
    pub gamescope: Option<String>,
    /// Logging com `output_folder`, `log_interval=0` e socket de controle.
    pub logging_supported: bool,
    pub flatpak_steam: bool,
    pub flatpak_layer: Vec<String>,
    pub notes: Vec<String>,
    pub install_hint: Option<InstallHint>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstallHint {
    pub distro: String,
    pub commands: Vec<String>,
}

/// Executa `mangohud --version` (substituível nos testes).
pub type VersionProbe = Box<dyn Fn(&Path) -> Option<String> + Send + Sync>;

pub struct Env {
    pub path_dirs: Vec<PathBuf>,
    pub layer_dirs: Vec<PathBuf>,
    pub lib_dirs: Vec<PathBuf>,
    pub flatpak_runtime_dirs: Vec<PathBuf>,
    pub flatpak_steam: bool,
    pub distro_id: Option<String>,
    pub distro_like: Option<String>,
    /// Executa `mangohud --version`; nos testes é substituído.
    pub run_version: VersionProbe,
}

impl Env {
    pub fn from_system(flatpak_steam: bool) -> Self {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default();
        let os = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
        let field = |k: &str| {
            os.lines()
                .find_map(|l| l.strip_prefix(&format!("{k}=")))
                .map(|v| v.trim_matches('"').to_owned())
        };
        Env {
            path_dirs: std::env::var_os("PATH")
                .map(|p| std::env::split_paths(&p).collect())
                .unwrap_or_default(),
            layer_dirs: vec![
                PathBuf::from("/usr/share/vulkan/implicit_layer.d"),
                PathBuf::from("/etc/vulkan/implicit_layer.d"),
                PathBuf::from("/usr/local/share/vulkan/implicit_layer.d"),
                home.join(".local/share/vulkan/implicit_layer.d"),
            ],
            lib_dirs: [
                "/usr/lib/mangohud",
                "/usr/lib32/mangohud",
                "/usr/lib64/mangohud",
                "/usr/lib/x86_64-linux-gnu/mangohud",
                "/usr/lib/i386-linux-gnu/mangohud",
                "/usr/local/lib/mangohud",
            ]
            .iter()
            .map(PathBuf::from)
            .collect(),
            flatpak_runtime_dirs: vec![
                home.join(
                    ".local/share/flatpak/runtime/org.freedesktop.Platform.VulkanLayer.MangoHud",
                ),
                PathBuf::from(
                    "/var/lib/flatpak/runtime/org.freedesktop.Platform.VulkanLayer.MangoHud",
                ),
            ],
            flatpak_steam,
            distro_id: field("ID"),
            distro_like: field("ID_LIKE"),
            run_version: Box::new(run_version),
        }
    }
}

/// `mangohud --version` com argumentos explícitos, sem shell e com tempo limite.
fn run_version(exe: &Path) -> Option<String> {
    let mut child = Command::new(exe)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(30))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let out = child.wait_with_output().ok()?;
    let v = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()?
        .trim()
        .to_owned();
    (!v.is_empty() && v.len() < 60 && v.chars().next()?.is_ascii_alphanumeric()).then_some(v)
}

fn which(dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    dirs.iter().map(|d| d.join(name)).find(|p| p.is_file())
}

fn layers(dirs: &[PathBuf]) -> Vec<LayerInfo> {
    let mut out = Vec::new();
    for d in dirs {
        let Ok(rd) = std::fs::read_dir(d) else {
            continue;
        };
        let mut files: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().to_lowercase().starts_with("mangohud"))
            })
            .collect();
        files.sort();
        for f in files {
            let Ok(text) = std::fs::read_to_string(&f) else {
                continue;
            };
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
                continue;
            };
            let Some(layer) = v.get("layer") else {
                continue;
            };
            let name = layer.get("name").and_then(|x| x.as_str()).unwrap_or("");
            let lib = layer
                .get("library_path")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_owned();
            let arch = if name.contains("x86_64") || f.to_string_lossy().contains("x86_64") {
                "64 bits"
            } else if name.ends_with("_x86") || f.to_string_lossy().contains(".x86.") {
                "32 bits"
            } else {
                "desconhecida"
            };
            // Caminho relativo ou com $LIB é resolvido pelo carregador; não dá para checar aqui.
            let exists = !lib.starts_with('/') || lib.contains('$') || Path::new(&lib).is_file();
            out.push(LayerInfo {
                manifest: f.to_string_lossy().into_owned(),
                arch: arch.into(),
                library: lib,
                library_exists: exists,
            });
        }
    }
    out
}

/// Compara versões "0.8.4-1+" ≥ "0.6.0".
pub fn version_at_least(v: &str, major: u32, minor: u32) -> bool {
    let nums: Vec<u32> = v
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .take(2)
        .filter_map(|s| s.parse().ok())
        .collect();
    match nums.as_slice() {
        [a, b, ..] => (*a, *b) >= (major, minor),
        [a] => *a > major,
        _ => false,
    }
}

pub fn install_hint(id: Option<&str>, like: Option<&str>, flatpak: bool) -> InstallHint {
    let ids = format!("{} {}", id.unwrap_or(""), like.unwrap_or(""));
    let has = |k: &str| ids.split_whitespace().any(|x| x == k);
    let (distro, mut commands): (&str, Vec<String>) =
        if has("arch") || has("cachyos") || has("manjaro") || has("endeavouros") {
            (
                "Arch Linux e derivados",
                vec!["sudo pacman -S --needed mangohud lib32-mangohud".into()],
            )
        } else if has("fedora") || has("nobara") {
            (
                "Fedora",
                vec!["sudo dnf install mangohud mangohud.i686".into()],
            )
        } else if has("debian") || has("ubuntu") || has("pop") || has("linuxmint") {
            (
                "Debian/Ubuntu",
                vec!["sudo apt install mangohud mangohud:i386".into()],
            )
        } else if has("opensuse") || has("suse") || has("opensuse-tumbleweed") {
            (
                "openSUSE",
                vec!["sudo zypper install mangohud mangohud-32bit".into()],
            )
        } else {
            (
                "sua distribuição",
                vec![
                "Instale o pacote “mangohud” (e a versão de 32 bits) pelo gerenciador de pacotes."
                    .into(),
            ],
            )
        };
    if flatpak {
        commands
            .push("flatpak install flathub org.freedesktop.Platform.VulkanLayer.MangoHud".into());
    }
    InstallHint {
        distro: distro.into(),
        commands,
    }
}

pub fn detect(env: &Env) -> MangoHudStatus {
    let exe = which(&env.path_dirs, "mangohud");
    let version = exe.as_deref().and_then(|e| (env.run_version)(e));
    let layers = layers(&env.layer_dirs);
    let has_64bit = layers
        .iter()
        .any(|l| l.arch == "64 bits" && l.library_exists)
        || env.lib_dirs.iter().any(|d| {
            !d.to_string_lossy().contains("32")
                && !d.to_string_lossy().contains("i386")
                && d.join("libMangoHud.so").is_file()
        });
    let has_32bit = layers
        .iter()
        .any(|l| l.arch == "32 bits" && l.library_exists)
        || env.lib_dirs.iter().any(|d| {
            (d.to_string_lossy().contains("32") || d.to_string_lossy().contains("i386"))
                && d.join("libMangoHud.so").is_file()
        });
    let opengl_shim = env.lib_dirs.iter().any(|d| {
        d.join("libMangoHud_opengl.so").is_file() || d.join("libMangoHud_shim.so").is_file()
    });
    let mut flatpak_layer: Vec<String> = Vec::new();
    for d in &env.flatpak_runtime_dirs {
        if let Ok(rd) = std::fs::read_dir(d.join("x86_64")) {
            for e in rd.flatten() {
                flatpak_layer.push(e.file_name().to_string_lossy().into_owned());
            }
        }
    }
    flatpak_layer.sort();
    flatpak_layer.dedup();

    let installed = has_64bit || exe.is_some();
    let logging_supported =
        installed && version.as_deref().is_none_or(|v| version_at_least(v, 0, 6));
    let mut notes = Vec::new();
    if installed && !has_32bit {
        notes.push(
            "Sem a biblioteca de 32 bits: jogos de 32 bits não mostram o overlay nem gravam logs."
                .to_owned(),
        );
    }
    if installed && layers.is_empty() {
        notes.push("Manifesto da camada Vulkan não encontrado; jogos Vulkan/DXVK podem não carregar o MangoHud.".to_owned());
    }
    if exe.is_some() && !opengl_shim {
        notes.push(
            "Biblioteca OpenGL do MangoHud não encontrada; jogos OpenGL podem não ser monitorados."
                .to_owned(),
        );
    }
    if env.flatpak_steam && flatpak_layer.is_empty() {
        notes.push("Steam Flatpak detectada sem a extensão VulkanLayer.MangoHud: jogos dessa Steam não terão o overlay.".to_owned());
    }
    if let Some(v) = &version {
        if !version_at_least(v, 0, 6) {
            notes.push(format!(
                "MangoHud {v} é antigo; o FRX precisa de 0.6 ou mais novo para gravar logs."
            ));
        }
    }
    notes.push("O mangohudctl controla o mangoapp (gamescope). Para jogos comuns, o FRX usa o socket de controle do próprio MangoHud.".into());

    let mangohudctl =
        which(&env.path_dirs, "mangohudctl").map(|p| p.to_string_lossy().into_owned());
    MangoHudStatus {
        installed,
        executable: exe.map(|p| p.to_string_lossy().into_owned()),
        version,
        layers,
        has_64bit,
        has_32bit,
        opengl_shim,
        mangohudctl,
        mangoapp: which(&env.path_dirs, "mangoapp").map(|p| p.to_string_lossy().into_owned()),
        gamescope: which(&env.path_dirs, "gamescope").map(|p| p.to_string_lossy().into_owned()),
        logging_supported,
        flatpak_steam: env.flatpak_steam,
        install_hint: (!installed || (env.flatpak_steam && flatpak_layer.is_empty()) || !has_32bit)
            .then(|| {
                install_hint(
                    env.distro_id.as_deref(),
                    env.distro_like.as_deref(),
                    env.flatpak_steam,
                )
            }),
        flatpak_layer,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(root: &Path, version: Option<&'static str>) -> Env {
        Env {
            path_dirs: vec![root.join("bin")],
            layer_dirs: vec![root.join("layers")],
            lib_dirs: vec![root.join("lib/mangohud"), root.join("lib32/mangohud")],
            flatpak_runtime_dirs: vec![root.join("flatpak")],
            flatpak_steam: false,
            distro_id: Some("cachyos".into()),
            distro_like: Some("arch".into()),
            run_version: Box::new(move |_| version.map(str::to_owned)),
        }
    }

    #[test]
    fn detects_full_installation() {
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        for d in ["bin", "layers", "lib/mangohud", "lib32/mangohud"] {
            std::fs::create_dir_all(r.join(d)).unwrap();
        }
        std::fs::write(r.join("bin/mangohud"), "#!/bin/sh").unwrap();
        std::fs::write(r.join("bin/mangohudctl"), "").unwrap();
        std::fs::write(r.join("lib/mangohud/libMangoHud.so"), "").unwrap();
        std::fs::write(r.join("lib/mangohud/libMangoHud_opengl.so"), "").unwrap();
        std::fs::write(r.join("lib32/mangohud/libMangoHud.so"), "").unwrap();
        let lib = r.join("lib/mangohud/libMangoHud.so");
        std::fs::write(
            r.join("layers/MangoHud.x86_64.json"),
            format!(
                r#"{{"layer":{{"name":"VK_LAYER_MANGOHUD_overlay_x86_64","library_path":"{}"}}}}"#,
                lib.display()
            ),
        )
        .unwrap();
        let s = detect(&env(r, Some("0.8.4-1+")));
        assert!(s.installed && s.logging_supported && s.has_64bit && s.has_32bit && s.opengl_shim);
        assert_eq!(s.version.as_deref(), Some("0.8.4-1+"));
        assert_eq!(s.layers.len(), 1);
        assert!(s.mangohudctl.is_some());
        assert!(s.install_hint.is_none());
    }

    #[test]
    fn missing_mangohud_gives_distro_specific_instructions() {
        let tmp = tempfile::tempdir().unwrap();
        let mut e = env(tmp.path(), None);
        e.flatpak_steam = true;
        let s = detect(&e);
        assert!(!s.installed && !s.logging_supported);
        let hint = s.install_hint.unwrap();
        assert_eq!(hint.distro, "Arch Linux e derivados");
        assert!(hint.commands[0].contains("lib32-mangohud"));
        assert!(hint
            .commands
            .iter()
            .any(|c| c.contains("VulkanLayer.MangoHud")));
        assert!(s.notes.iter().any(|n| n.contains("Flatpak")));
    }

    #[test]
    fn old_versions_and_missing_32bit_are_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        std::fs::create_dir_all(r.join("bin")).unwrap();
        std::fs::create_dir_all(r.join("lib/mangohud")).unwrap();
        std::fs::write(r.join("bin/mangohud"), "").unwrap();
        std::fs::write(r.join("lib/mangohud/libMangoHud.so"), "").unwrap();
        let s = detect(&env(r, Some("0.5.1")));
        assert!(s.installed && !s.logging_supported);
        assert!(!s.has_32bit);
        assert!(s.notes.iter().any(|n| n.contains("32 bits")));
    }

    #[test]
    fn version_comparison_and_hints() {
        assert!(version_at_least("0.8.4-1+", 0, 6));
        assert!(version_at_least("v0.7.0", 0, 6));
        assert!(!version_at_least("0.5.9", 0, 6));
        assert!(version_at_least("1", 0, 6));
        assert_eq!(
            install_hint(Some("ubuntu"), Some("debian"), false).distro,
            "Debian/Ubuntu"
        );
        assert_eq!(
            install_hint(Some("fedora"), None, false).commands[0],
            "sudo dnf install mangohud mangohud.i686"
        );
    }

    #[test]
    fn runs_version_without_a_shell_and_with_timeout() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let script = |name: &str, body: &str| {
            let p = tmp.path().join(name);
            std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
            p
        };
        assert_eq!(
            run_version(&script("ok", "echo 0.9.1-2")).as_deref(),
            Some("0.9.1-2")
        );
        assert_eq!(run_version(&script("lixo", "echo '--- nada'")), None);
        let started = std::time::Instant::now();
        assert_eq!(run_version(&script("trava", "sleep 10")), None);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "tempo limite respeitado"
        );
        assert_eq!(run_version(Path::new("/nao/existe")), None);
    }
}
