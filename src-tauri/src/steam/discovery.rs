//! Localiza instalações da Steam a partir de caminhos conhecidos.
//!
//! Nada é varrido recursivamente: só testamos um punhado de diretórios
//! documentados (nativo, Flatpak, Snap, Debian) e usamos a configuração da
//! própria Steam para chegar às bibliotecas.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::models::{InstallKind, SteamInstall};

/// Ambiente usado pela descoberta — injetável nos testes.
#[derive(Debug, Clone, Default)]
pub struct DiscoveryEnv {
    pub home: Option<PathBuf>,
    pub xdg_data_home: Option<PathBuf>,
    /// `FRX_STEAM_ROOT`: força uma raiz específica (útil para diagnósticos).
    pub override_root: Option<PathBuf>,
}

impl DiscoveryEnv {
    pub fn from_process() -> Self {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let xdg_data_home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute());
        let override_root = std::env::var_os("FRX_STEAM_ROOT").map(PathBuf::from);
        Self {
            home,
            xdg_data_home,
            override_root,
        }
    }
}

pub fn candidate_roots(env: &DiscoveryEnv) -> Vec<(PathBuf, InstallKind)> {
    let mut out = Vec::new();
    if let Some(root) = &env.override_root {
        out.push((root.clone(), InstallKind::Custom));
    }
    if let Some(xdg) = &env.xdg_data_home {
        out.push((xdg.join("Steam"), InstallKind::Native));
    }
    if let Some(home) = &env.home {
        out.extend([
            (home.join(".steam/root"), InstallKind::Native),
            (home.join(".steam/steam"), InstallKind::Native),
            (home.join(".local/share/Steam"), InstallKind::Native),
            (home.join(".steam/debian-installation"), InstallKind::Native),
            (
                home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
                InstallKind::Flatpak,
            ),
            (
                home.join(".var/app/com.valvesoftware.Steam/data/Steam"),
                InstallKind::Flatpak,
            ),
            (
                home.join("snap/steam/common/.local/share/Steam"),
                InstallKind::Snap,
            ),
        ]);
    }
    out
}

/// Uma raiz é válida quando contém `steamapps/` ou a configuração de bibliotecas.
pub fn is_steam_root(p: &Path) -> bool {
    p.join("steamapps").is_dir() || p.join("config/libraryfolders.vdf").is_file()
}

/// Resolve links simbólicos para comparar caminhos equivalentes.
pub fn canonical(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

pub fn discover(env: &DiscoveryEnv) -> Vec<SteamInstall> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (path, kind) in candidate_roots(env) {
        if !is_steam_root(&path) {
            continue;
        }
        let real = canonical(&path);
        if !seen.insert(real.clone()) {
            continue;
        }
        // Um link para dentro de ~/.var/app continua sendo Flatpak.
        let kind = if real
            .to_string_lossy()
            .contains("/.var/app/com.valvesoftware.Steam/")
        {
            InstallKind::Flatpak
        } else {
            kind
        };
        out.push(SteamInstall {
            path: real.to_string_lossy().into_owned(),
            kind,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deduplicates_symlinked_roots_and_detects_flatpak() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let native = home.join(".local/share/Steam");
        std::fs::create_dir_all(native.join("steamapps")).unwrap();
        std::fs::create_dir_all(home.join(".steam")).unwrap();
        std::os::unix::fs::symlink(&native, home.join(".steam/root")).unwrap();
        std::os::unix::fs::symlink(&native, home.join(".steam/steam")).unwrap();
        let flat = home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam");
        std::fs::create_dir_all(flat.join("steamapps")).unwrap();

        let env = DiscoveryEnv {
            home: Some(home.to_path_buf()),
            ..Default::default()
        };
        let found = discover(&env);
        assert_eq!(found.len(), 2, "{found:?}");
        assert_eq!(
            found
                .iter()
                .filter(|i| i.kind == InstallKind::Flatpak)
                .count(),
            1
        );
    }

    #[test]
    fn returns_nothing_without_steam() {
        let tmp = tempfile::tempdir().unwrap();
        let env = DiscoveryEnv {
            home: Some(tmp.path().to_path_buf()),
            ..Default::default()
        };
        assert!(discover(&env).is_empty());
    }

    #[test]
    fn honours_override_root_with_spaces() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Meus Jogos/Steam Root");
        std::fs::create_dir_all(root.join("steamapps")).unwrap();
        let env = DiscoveryEnv {
            override_root: Some(root.clone()),
            ..Default::default()
        };
        let found = discover(&env);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, InstallKind::Custom);
        assert!(found[0].path.ends_with("Meus Jogos/Steam Root"));
    }
}
