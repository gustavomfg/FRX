//! Detecção de jogos instalados pela Steam.
//!
//! Fluxo: raízes da Steam → `libraryfolders.vdf` → manifestos de cada
//! biblioteca disponível → ferramenta de compatibilidade e prefixo de cada
//! jogo. Tudo somente leitura.

pub mod compatibility;
pub mod discovery;
pub mod libraries;
pub mod manifests;
pub mod models;
pub mod userdata;
pub mod vdf;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

use crate::time::{iso8601, now_iso};
use compatibility::ToolRegistry;
use discovery::{canonical, DiscoveryEnv};
use models::*;

pub fn scan(env: &DiscoveryEnv, previous: Option<&ScanResult>) -> ScanResult {
    let started = Instant::now();
    let installs = discovery::discover(env);
    let mut warnings = Vec::new();

    if installs.is_empty() {
        return ScanResult {
            status: ScanStatus::NoSteam,
            steam_installs: vec![],
            libraries: vec![],
            games: vec![],
            tools: vec![],
            global_default_tool: None,
            warnings,
            scanned_at: now_iso(),
            duration_ms: started.elapsed().as_millis() as u64,
        };
    }

    // 1. Bibliotecas de todas as instalações, sem duplicar caminhos equivalentes.
    struct Lib {
        path: PathBuf,
        status: LibraryStatus,
        is_primary: bool,
        label: Option<String>,
        declared: Vec<String>,
        install: String,
    }
    let mut libs: Vec<Lib> = Vec::new();
    let mut seen_libs = HashSet::new();
    for inst in &installs {
        let root = PathBuf::from(&inst.path);
        let (entries, w) = libraries::read_library_folders(&root);
        warnings.extend(w);
        for (i, e) in entries.into_iter().enumerate() {
            let real = canonical(&e.path);
            if !seen_libs.insert(real.clone()) {
                continue;
            }
            libs.push(Lib {
                status: libraries::library_status(&real),
                path: real,
                is_primary: i == 0,
                label: e.label,
                declared: e.app_ids,
                install: inst.path.clone(),
            });
        }
    }
    for l in &libs {
        match l.status {
            LibraryStatus::Unavailable => warnings.push(format!(
                "Biblioteca indisponível (disco desmontado?): {}",
                l.path.display()
            )),
            LibraryStatus::Inaccessible => warnings.push(format!(
                "Sem permissão para ler a biblioteca: {}",
                l.path.display()
            )),
            LibraryStatus::Available => {}
        }
    }

    // 2. Manifestos das bibliotecas disponíveis.
    let mut all: Vec<(manifests::AppManifest, PathBuf, usize)> = Vec::new();
    for (idx, l) in libs
        .iter()
        .enumerate()
        .filter(|(_, l)| l.status == LibraryStatus::Available)
    {
        let (ms, w) = manifests::read_manifests(&l.path);
        warnings.extend(w);
        for m in ms {
            let install = l
                .path
                .join("steamapps/common")
                .join(m.install_dir.as_deref().unwrap_or(""));
            all.push((m, install, idx));
        }
    }

    // 3. Ferramentas de compatibilidade.
    let tool_manifests: Vec<(manifests::AppManifest, PathBuf)> = all
        .iter()
        .filter(|(m, p, _)| compatibility::is_tool_manifest(m, p))
        .map(|(m, p, _)| (m.clone(), p.clone()))
        .collect();
    let mut tools = compatibility::tools_from_manifests(&tool_manifests);
    let roots: Vec<PathBuf> = installs.iter().map(|i| PathBuf::from(&i.path)).collect();
    let (custom, w) = compatibility::custom_tools(&compatibility::compat_tool_dirs(&roots));
    warnings.extend(w);
    tools.extend(custom);
    let registry = ToolRegistry { tools };

    let mut mapping = HashMap::new();
    let mut playtime = HashMap::new();
    for root in roots.iter().rev() {
        let (m, w) = compatibility::read_tool_mapping(root);
        warnings.extend(w);
        mapping.extend(m);
        playtime.extend(userdata::playtime_minutes(root));
    }

    // 4. Jogos.
    let available_libs: Vec<PathBuf> = libs
        .iter()
        .filter(|l| l.status == LibraryStatus::Available)
        .map(|l| l.path.clone())
        .collect();
    let mut games: HashMap<String, InstalledGame> = HashMap::new();
    for (m, install, idx) in &all {
        if compatibility::is_tool_manifest(m, install) {
            continue;
        }
        let lib = &libs[*idx];
        let mut search = vec![lib.path.clone()];
        search.extend(available_libs.iter().filter(|p| **p != lib.path).cloned());
        let prefix = compatibility::find_prefix(&m.app_id, &search);
        let has_dir = m.install_dir.is_some();
        let status = manifests::installation_status(m, has_dir.then_some(install.as_path()));
        let game = InstalledGame {
            app_id: m.app_id.clone(),
            name: m
                .name
                .clone()
                .unwrap_or_else(|| format!("App {}", m.app_id)),
            install_path: has_dir.then(|| install.to_string_lossy().into_owned()),
            library_path: lib.path.to_string_lossy().into_owned(),
            installation_status: status,
            state_flags: m.state_flags.map(manifests::flag_names).unwrap_or_default(),
            size_on_disk: m.size_on_disk.filter(|s| *s > 0),
            last_updated: m.last_updated.map(iso8601),
            last_played: m.last_played.map(iso8601),
            build_id: m.build_id.clone(),
            playtime_minutes: playtime.get(&m.app_id).copied(),
            compatibility: compatibility::resolve_game(
                &m.app_id,
                &mapping,
                &registry,
                prefix.as_deref(),
            ),
            source: "steam".into(),
            steam_install: lib.install.clone(),
        };
        // Mesmo AppID em duas bibliotecas: a instalação completa vence.
        match games.get(&m.app_id) {
            Some(existing)
                if rank(existing.installation_status) >= rank(game.installation_status) =>
            {
                warnings.push(format!(
                    "AppID {} aparece em mais de uma biblioteca; mantida {}",
                    m.app_id, existing.library_path
                ));
            }
            _ => {
                games.insert(m.app_id.clone(), game);
            }
        }
    }

    // 5. Jogos declarados em bibliotecas fora do ar: listados como indisponíveis.
    let previous_names: HashMap<&str, &InstalledGame> = previous
        .map(|p| p.games.iter().map(|g| (g.app_id.as_str(), g)).collect())
        .unwrap_or_default();
    let known_tools: HashSet<String> = tool_manifests
        .iter()
        .map(|(m, _)| m.app_id.clone())
        .collect();
    for l in libs.iter().filter(|l| l.status != LibraryStatus::Available) {
        for app in &l.declared {
            if games.contains_key(app) || known_tools.contains(app) || is_known_tool_id(app) {
                continue;
            }
            let prev = previous_names.get(app.as_str());
            games.insert(
                app.clone(),
                InstalledGame {
                    app_id: app.clone(),
                    name: prev
                        .map(|g| g.name.clone())
                        .unwrap_or_else(|| format!("App {app}")),
                    install_path: prev.and_then(|g| g.install_path.clone()),
                    library_path: l.path.to_string_lossy().into_owned(),
                    installation_status: InstallationStatus::Unavailable,
                    state_flags: vec![],
                    size_on_disk: prev.and_then(|g| g.size_on_disk),
                    last_updated: None,
                    last_played: prev.and_then(|g| g.last_played.clone()),
                    build_id: None,
                    playtime_minutes: playtime.get(app).copied(),
                    compatibility: compatibility::resolve_game(app, &mapping, &registry, None),
                    source: "steam".into(),
                    steam_install: l.install.clone(),
                },
            );
        }
    }

    let mut games: Vec<InstalledGame> = games.into_values().collect();
    games.sort_by_key(|g| g.name.to_lowercase());

    let libraries = libs
        .iter()
        .map(|l| {
            let p = l.path.to_string_lossy().into_owned();
            LibraryInfo {
                game_count: games.iter().filter(|g| g.library_path == p).count(),
                path: p,
                status: l.status,
                is_primary: l.is_primary,
                label: l.label.clone(),
                declared_app_ids: l.declared.clone(),
                steam_install: l.install.clone(),
            }
        })
        .collect();

    ScanResult {
        status: ScanStatus::Ok,
        steam_installs: installs,
        libraries,
        global_default_tool: compatibility::global_default(&mapping, &registry),
        tools: registry.tools,
        games,
        warnings,
        scanned_at: now_iso(),
        duration_ms: started.elapsed().as_millis() as u64,
    }
}

fn rank(s: InstallationStatus) -> u8 {
    match s {
        InstallationStatus::Installed => 3,
        InstallationStatus::Incomplete => 2,
        InstallationStatus::Unknown => 1,
        InstallationStatus::Unavailable => 0,
    }
}

fn is_known_tool_id(app: &str) -> bool {
    [
        "1493710", "2180100", "3658110", "2805730", "2348590", "1887720", "1070560", "1391110",
        "1628350", "4183110", "228980",
    ]
    .contains(&app)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn write(p: &Path, s: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, s).unwrap();
    }

    fn manifest(lib: &Path, id: &str, name: &str, dir: &str, flags: u32) {
        write(
            &lib.join(format!("steamapps/appmanifest_{id}.acf")),
            &format!(
                r#""AppState" {{ "appid" "{id}" "name" "{name}" "installdir" "{dir}" "StateFlags" "{flags}" "SizeOnDisk" "1000" "LastPlayed" "1792792536" }}"#
            ),
        );
    }

    /// Instalação completa: duas bibliotecas (uma num "disco" extra com espaço
    /// no nome), um disco desmontado, um link simbólico duplicado, Proton
    /// oficial, ferramenta personalizada, manifestos quebrados e incompletos.
    fn build_fixture() -> (tempfile::TempDir, DiscoveryEnv) {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let root = home.join(".local/share/Steam");
        let extra = tmp.path().join("mnt/Disco Extra/SteamLibrary");
        let gone = tmp.path().join("mnt/Desmontado/SteamLibrary");
        let link = tmp.path().join("atalho-para-extra");
        std::fs::create_dir_all(home.join(".steam")).unwrap();
        std::fs::create_dir_all(root.join("steamapps/common")).unwrap();
        std::fs::create_dir_all(extra.join("steamapps/common")).unwrap();
        std::os::unix::fs::symlink(&root, home.join(".steam/root")).unwrap();
        std::os::unix::fs::symlink(&extra, &link).unwrap();
        write(
            &root.join("steamapps/libraryfolders.vdf"),
            &format!(
                r#""libraryfolders" {{
                    "0" {{ "path" "{r}" "apps" {{ "1493710" "1" "620" "1" }} }}
                    "1" {{ "path" "{e}" "apps" {{ "200260" "1" }} }}
                    "2" {{ "path" "{l}" "apps" {{ "200260" "1" }} }}
                    "3" {{ "path" "{g}" "apps" {{ "35140" "1" }} }}
                }}"#,
                r = root.display(),
                e = extra.display(),
                l = link.display(),
                g = gone.display()
            ),
        );
        // Proton Experimental (ferramenta, não jogo)
        manifest(
            &root,
            "1493710",
            "Proton Experimental",
            "Proton - Experimental",
            4,
        );
        write(
            &root.join("steamapps/common/Proton - Experimental/toolmanifest.vdf"),
            r#""manifest" { "version" "2" }"#,
        );
        write(
            &root.join("steamapps/common/Proton - Experimental/version"),
            "1791989105 experimental-11.0-20261014\n",
        );
        // Jogo nativo completo
        manifest(&root, "620", "Portal 2", "Portal 2", 4);
        std::fs::create_dir_all(root.join("steamapps/common/Portal 2")).unwrap();
        // Jogo baixando
        manifest(&root, "570", "Dota 2", "dota 2 beta", 1026);
        std::fs::create_dir_all(root.join("steamapps/common/dota 2 beta")).unwrap();
        // Manifesto apontando para pasta apagada
        manifest(&root, "440", "Team Fortress 2", "Team Fortress 2", 4);
        // Manifesto inválido
        write(
            &root.join("steamapps/appmanifest_999.acf"),
            "\"AppState\" { \"appid\" ",
        );
        // Jogo Windows no disco extra, com Proton por jogo e prefixo
        manifest(
            &extra,
            "200260",
            "Batman: Arkham City GOTY",
            "Batman Arkham City GOTY",
            4,
        );
        std::fs::create_dir_all(extra.join("steamapps/common/Batman Arkham City GOTY")).unwrap();
        std::fs::create_dir_all(extra.join("steamapps/compatdata/200260/pfx")).unwrap();
        write(
            &extra.join("steamapps/compatdata/200260/version"),
            "GE-Proton9-20\n",
        );
        // Ferramenta personalizada
        let ge = root.join("compatibilitytools.d/GE-Proton9-20");
        write(
            &ge.join("compatibilitytool.vdf"),
            r#""compatibilitytools" { "compat_tools" { "GE-Proton9-20" { "install_path" "." "display_name" "GE-Proton9-20" } } }"#,
        );
        std::fs::create_dir_all(ge.join("protonfixes/gamefixes-steam")).unwrap();
        write(
            &root.join("config/config.vdf"),
            r#""InstallConfigStore" { "Software" { "Valve" { "Steam" { "CompatToolMapping" {
                "0" { "name" "proton_experimental" }
                "200260" { "name" "GE-Proton9-20" }
            } } } } }"#,
        );
        let env = DiscoveryEnv {
            home: Some(home),
            ..Default::default()
        };
        (tmp, env)
    }

    #[test]
    fn full_scan_over_fixture() {
        let (_tmp, env) = build_fixture();
        let r = scan(&env, None);
        assert_eq!(r.status, ScanStatus::Ok);
        assert_eq!(
            r.steam_installs.len(),
            1,
            "o link .steam/root não pode duplicar a instalação"
        );
        assert_eq!(
            r.libraries.len(),
            3,
            "o link para o disco extra não pode duplicar a biblioteca"
        );
        assert_eq!(
            r.libraries
                .iter()
                .filter(|l| l.status == LibraryStatus::Unavailable)
                .count(),
            1
        );

        let by = |id: &str| {
            r.games
                .iter()
                .find(|g| g.app_id == id)
                .unwrap_or_else(|| panic!("{id} ausente: {:?}", r.games))
        };
        assert!(
            r.games.iter().all(|g| g.app_id != "1493710"),
            "ferramentas não são jogos"
        );
        assert!(
            r.games.iter().all(|g| g.app_id != "999"),
            "manifesto inválido é ignorado"
        );
        assert!(r.warnings.iter().any(|w| w.contains("appmanifest_999")));

        assert_eq!(by("620").installation_status, InstallationStatus::Installed);
        assert_eq!(by("620").compatibility.tool_source, ToolSource::None);
        assert_eq!(
            by("570").installation_status,
            InstallationStatus::Incomplete
        );
        assert_eq!(
            by("440").installation_status,
            InstallationStatus::Unavailable
        );
        assert_eq!(
            by("35140").installation_status,
            InstallationStatus::Unavailable
        );

        let batman = by("200260");
        assert!(batman.library_path.ends_with("Disco Extra/SteamLibrary"));
        assert_eq!(batman.compatibility.tool_source, ToolSource::PerGame);
        let tool = batman.compatibility.tool.as_ref().unwrap();
        assert_eq!(tool.kind, ToolKind::GeProton);
        assert!(tool.includes_protonfixes);
        assert_eq!(
            batman.compatibility.prefix_version.as_deref(),
            Some("GE-Proton9-20")
        );
        assert!(batman.compatibility.prefix_path.is_some());
        assert_eq!(batman.last_played.as_deref(), Some("2026-10-23T21:55:36Z"));

        let exp = r
            .tools
            .iter()
            .find(|t| t.display_name == "Proton Experimental")
            .unwrap();
        assert_eq!(exp.kind, ToolKind::Experimental);
        assert_eq!(exp.version.as_deref(), Some("experimental-11.0-20261014"));
        assert_eq!(
            r.global_default_tool.as_ref().unwrap().display_name,
            "Proton Experimental"
        );
    }

    #[test]
    fn unavailable_library_reuses_previous_names() {
        let (_tmp, env) = build_fixture();
        let mut prev = scan(&env, None);
        prev.games
            .iter_mut()
            .find(|g| g.app_id == "35140")
            .unwrap()
            .name = "Batman: Arkham Asylum".into();
        let r = scan(&env, Some(&prev));
        let g = r.games.iter().find(|g| g.app_id == "35140").unwrap();
        assert_eq!(g.name, "Batman: Arkham Asylum");
        assert_eq!(g.installation_status, InstallationStatus::Unavailable);
    }

    #[test]
    fn no_steam_is_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let r = scan(
            &DiscoveryEnv {
                home: Some(tmp.path().to_path_buf()),
                ..Default::default()
            },
            None,
        );
        assert_eq!(r.status, ScanStatus::NoSteam);
        assert!(r.games.is_empty());
    }
}

/// Diagnóstico manual na máquina do desenvolvedor:
/// `cargo test real_system_scan -- --ignored --nocapture`
#[cfg(test)]
mod real {
    #[test]
    #[ignore]
    fn real_system_scan() {
        let r = super::scan(&super::DiscoveryEnv::from_process(), None);
        println!("{}", serde_json::to_string_pretty(&r).unwrap());
        println!(
            "{}",
            serde_json::to_string_pretty(&crate::system::profile()).unwrap()
        );
    }
}
