//! Ferramentas de compatibilidade (Proton, GE-Proton, runtimes) e prefixos.
//!
//! Regras importantes:
//! - a ferramenta escolhida *para o jogo* (CompatToolMapping[appid]) é
//!   diferente do padrão global (CompatToolMapping["0"]) — nunca as confundimos;
//! - a versão do prefixo vem de `compatdata/<appid>/version` e descreve a
//!   última execução, não necessariamente a configuração atual;
//! - quando nada disso existe, devolvemos `None` em vez de adivinhar.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::libraries::read_text;
use super::manifests::AppManifest;
use super::models::{CompatTool, GameCompatibility, ToolKind, ToolOrigin, ToolSource};
use super::vdf;

/// AppIDs das ferramentas oficiais e seus nomes internos no Steam Play.
const OFFICIAL: &[(&str, &str)] = &[
    ("1493710", "proton_experimental"),
    ("2180100", "proton_hotfix"),
    ("3658110", "proton_10"),
    ("2805730", "proton_9"),
    ("2348590", "proton_8"),
    ("1887720", "proton_7"),
    ("1580130", "proton_63"),
    ("1420170", "proton_5"),
    ("1245040", "proton_513"),
    ("1113280", "proton_411"),
    ("1054830", "proton_42"),
    ("996510", "proton_316"),
    ("930400", "proton_37"),
];

const RUNTIMES: &[&str] = &["1070560", "1391110", "1628350", "4183110", "228980"];

/// Nome de exibição para nomes internos conhecidos (`proton_9` → `Proton 9.0`).
pub fn pretty_internal_name(name: &str) -> String {
    match name {
        "proton_experimental" => return "Proton Experimental".into(),
        "proton_hotfix" => return "Proton Hotfix".into(),
        _ => {}
    }
    if let Some(v) = name.strip_prefix("proton_") {
        if v.chars().all(|c| c.is_ascii_digit()) && !v.is_empty() {
            // proton_9 → 9.0, proton_10 → 10.0, proton_63 → 6.3, proton_513 → 5.13
            let major_only = v.len() == 1 || (v.len() == 2 && v.starts_with('1'));
            return if major_only {
                format!("Proton {v}.0")
            } else {
                format!("Proton {}.{}", &v[..1], &v[1..])
            };
        }
    }
    name.to_owned()
}

pub fn classify(internal: Option<&str>, display: &str, app_id: Option<&str>) -> ToolKind {
    let d = display.to_ascii_lowercase();
    let i = internal.unwrap_or("").to_ascii_lowercase();
    // "proton-cachyos (steam linux runtime)" é um Proton que *usa* o runtime, não o runtime.
    if app_id.is_some_and(|a| RUNTIMES.contains(&a))
        || d.starts_with("steam linux runtime")
        || d.starts_with("steamworks common")
    {
        ToolKind::Runtime
    } else if d.contains("ge-proton") || i.contains("ge-proton") || d.contains("proton-ge") {
        ToolKind::GeProton
    } else if i == "proton_experimental" || (app_id.is_some() && d.contains("experimental")) {
        ToolKind::Experimental
    } else if i == "proton_hotfix" || (app_id.is_some() && d.contains("hotfix")) {
        ToolKind::Hotfix
    } else if app_id.is_some_and(|a| OFFICIAL.iter().any(|(id, _)| *id == a))
        || i.starts_with("proton_")
    {
        ToolKind::Official
    } else if internal.is_some() {
        ToolKind::Custom
    } else {
        ToolKind::Unknown
    }
}

/// Versão anunciada pelo arquivo `version` (ex.: `1791989105 experimental-11.0-20261014`).
fn read_version_file(dir: &Path) -> Option<String> {
    let t = std::fs::read_to_string(dir.join("version")).ok()?;
    let line = t.lines().next()?.trim();
    let v = line.split_whitespace().last()?.to_owned();
    (!v.is_empty()).then_some(v)
}

/// Registro das ferramentas encontradas no disco.
#[derive(Debug, Default)]
pub struct ToolRegistry {
    pub tools: Vec<CompatTool>,
}

impl ToolRegistry {
    pub fn by_internal(&self, name: &str) -> Option<&CompatTool> {
        self.tools
            .iter()
            .find(|t| t.internal_name.as_deref() == Some(name))
    }

    /// A ferramenta cujo diretório contém o caminho informado.
    pub fn by_path(&self, p: &Path) -> Option<&CompatTool> {
        let p = super::discovery::canonical(p);
        self.tools
            .iter()
            .filter(|t| {
                t.install_path
                    .as_ref()
                    .is_some_and(|ip| p.starts_with(super::discovery::canonical(Path::new(ip))))
            })
            .max_by_key(|t| t.install_path.as_ref().map(|s| s.len()).unwrap_or(0))
    }

    /// Resolve um nome interno; se não estiver no disco, cria uma referência.
    pub fn resolve(&self, internal: &str) -> CompatTool {
        self.by_internal(internal).cloned().unwrap_or_else(|| {
            let display = pretty_internal_name(internal);
            CompatTool {
                kind: classify(Some(internal), &display, None),
                internal_name: Some(internal.to_owned()),
                display_name: display,
                version: None,
                install_path: None,
                includes_protonfixes: false,
                origin: ToolOrigin::Referenced,
            }
        })
    }
}

/// Ferramentas instaladas como apps da Steam (Proton oficial, runtimes).
pub fn tools_from_manifests(manifests: &[(AppManifest, PathBuf)]) -> Vec<CompatTool> {
    manifests
        .iter()
        .filter_map(|(m, install)| {
            let is_tool = install.join("toolmanifest.vdf").is_file()
                || OFFICIAL.iter().any(|(id, _)| *id == m.app_id)
                || RUNTIMES.contains(&m.app_id.as_str());
            if !is_tool {
                return None;
            }
            let display = m
                .name
                .clone()
                .unwrap_or_else(|| format!("Ferramenta {}", m.app_id));
            let internal = OFFICIAL
                .iter()
                .find(|(id, _)| *id == m.app_id)
                .map(|(_, n)| (*n).to_owned());
            Some(CompatTool {
                kind: classify(internal.as_deref(), &display, Some(&m.app_id)),
                internal_name: internal,
                display_name: display,
                version: read_version_file(install),
                install_path: install
                    .is_dir()
                    .then(|| install.to_string_lossy().into_owned()),
                includes_protonfixes: install.join("protonfixes").is_dir(),
                origin: ToolOrigin::SteamApps,
            })
        })
        .collect()
}

pub fn is_tool_manifest(m: &AppManifest, install: &Path) -> bool {
    install.join("toolmanifest.vdf").is_file()
        || OFFICIAL.iter().any(|(id, _)| *id == m.app_id)
        || RUNTIMES.contains(&m.app_id.as_str())
}

/// Diretórios `compatibilitytools.d` conhecidos (por usuário e do sistema).
pub fn compat_tool_dirs(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = roots
        .iter()
        .map(|r| r.join("compatibilitytools.d"))
        .collect();
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        dirs.push(home.join(".steam/root/compatibilitytools.d"));
    }
    dirs.extend([
        PathBuf::from("/usr/share/steam/compatibilitytools.d"),
        PathBuf::from("/usr/local/share/steam/compatibilitytools.d"),
        PathBuf::from("/opt/steam/compatibilitytools.d"),
    ]);
    dirs
}

/// Lê `compatibilitytool.vdf` de cada subdiretório (um nível, sem recursão).
pub fn custom_tools(dirs: &[PathBuf]) -> (Vec<CompatTool>, Vec<String>) {
    let mut tools: Vec<CompatTool> = Vec::new();
    let mut warnings = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for dir in dirs {
        let real = super::discovery::canonical(dir);
        if !seen.insert(real.clone()) {
            continue;
        }
        let Ok(rd) = std::fs::read_dir(&real) else {
            continue;
        };
        let mut subdirs: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        subdirs.sort();
        for sub in subdirs {
            let file = sub.join("compatibilitytool.vdf");
            if !file.is_file() {
                continue;
            }
            let parsed = read_text(&file)
                .map_err(|e| e.to_string())
                .and_then(|t| vdf::parse(&t).map_err(|e| e.to_string()));
            let doc = match parsed {
                Ok(d) => d,
                Err(e) => {
                    warnings.push(format!("Ferramenta ignorada ({}): {e}", file.display()));
                    continue;
                }
            };
            let Some(list) = doc.path(&["compatibilitytools", "compat_tools"]) else {
                continue;
            };
            for (internal, info) in list.entries() {
                let display = info
                    .str_at("display_name")
                    .map(str::to_owned)
                    .unwrap_or_else(|| internal.clone());
                let install = match info.str_at("install_path") {
                    Some(".") | None => sub.clone(),
                    Some(p) if Path::new(p).is_absolute() => PathBuf::from(p),
                    Some(p) => sub.join(p),
                };
                if tools
                    .iter()
                    .any(|t| t.internal_name.as_deref() == Some(internal))
                {
                    continue; // primeira ocorrência vence, como na Steam
                }
                tools.push(CompatTool {
                    kind: classify(Some(internal), &display, None),
                    internal_name: Some(internal.clone()),
                    version: read_version_file(&install),
                    includes_protonfixes: install.join("protonfixes").is_dir(),
                    install_path: Some(install.to_string_lossy().into_owned()),
                    display_name: display,
                    origin: ToolOrigin::CompatibilityToolsD,
                });
            }
        }
    }
    (tools, warnings)
}

/// `CompatToolMapping` de `config/config.vdf`: appid → nome interno.
pub fn read_tool_mapping(root: &Path) -> (HashMap<String, String>, Option<String>) {
    let mut map = HashMap::new();
    let Ok(text) = read_text(&root.join("config/config.vdf")) else {
        return (map, None);
    };
    let Ok(doc) = vdf::parse(&text) else {
        return (map, Some("config.vdf malformado".into()));
    };
    let Some(mapping) = doc.path(&[
        "InstallConfigStore",
        "Software",
        "Valve",
        "Steam",
        "CompatToolMapping",
    ]) else {
        return (map, None);
    };
    for (app, entry) in mapping.entries() {
        if let Some(name) = entry
            .str_at("name")
            .map(str::trim)
            .filter(|n| !n.is_empty())
        {
            map.insert(app.clone(), name.to_owned());
        }
    }
    (map, None)
}

/// Lê `compatdata/<appid>` procurando primeiro na biblioteca do jogo.
pub fn find_prefix(app_id: &str, libraries: &[PathBuf]) -> Option<PathBuf> {
    libraries
        .iter()
        .map(|l| l.join("steamapps/compatdata").join(app_id))
        .find(|p| p.is_dir())
}

pub fn resolve_game(
    app_id: &str,
    mapping: &HashMap<String, String>,
    registry: &ToolRegistry,
    prefix_dir: Option<&Path>,
) -> GameCompatibility {
    let per_game = mapping.get(app_id).map(|n| registry.resolve(n));
    let (prefix_path, prefix_version, last_used_tool) = match prefix_dir {
        Some(dir) => {
            let pfx = dir.join("pfx");
            let version = std::fs::read_to_string(dir.join("version"))
                .ok()
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty());
            // `config_info`: 1ª linha = versão, demais = caminhos dentro da ferramenta usada.
            let last = std::fs::read_to_string(dir.join("config_info"))
                .ok()
                .and_then(|info| {
                    info.lines()
                        .skip(1)
                        .find(|l| l.starts_with('/'))
                        .and_then(|l| registry.by_path(Path::new(l.trim())).cloned())
                });
            (
                pfx.is_dir().then(|| pfx.to_string_lossy().into_owned()),
                version,
                last,
            )
        }
        None => (None, None, None),
    };
    let has_prefix = prefix_path.is_some();
    let (tool, tool_source) = if let Some(t) = per_game {
        (Some(t), ToolSource::PerGame)
    } else if let Some(t) = last_used_tool.clone() {
        (Some(t), ToolSource::LastUsed)
    } else if let Some(global) = mapping.get("0").filter(|_| has_prefix) {
        (Some(registry.resolve(global)), ToolSource::GlobalDefault)
    } else {
        (None, ToolSource::None)
    };
    let uses_proton = if tool_source == ToolSource::PerGame || has_prefix {
        Some(true)
    } else {
        None
    };
    GameCompatibility {
        tool,
        tool_source,
        last_used_tool,
        prefix_path,
        prefix_version,
        uses_proton,
    }
}

/// Ferramenta global padrão, se configurada.
pub fn global_default(
    mapping: &HashMap<String, String>,
    registry: &ToolRegistry,
) -> Option<CompatTool> {
    mapping.get("0").map(|n| registry.resolve(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prettifies_official_names() {
        assert_eq!(pretty_internal_name("proton_9"), "Proton 9.0");
        assert_eq!(pretty_internal_name("proton_10"), "Proton 10.0");
        assert_eq!(pretty_internal_name("proton_63"), "Proton 6.3");
        assert_eq!(pretty_internal_name("proton_513"), "Proton 5.13");
        assert_eq!(
            pretty_internal_name("proton_experimental"),
            "Proton Experimental"
        );
        assert_eq!(pretty_internal_name("GE-Proton9-20"), "GE-Proton9-20");
    }

    #[test]
    fn classifies_tools() {
        assert_eq!(
            classify(Some("GE-Proton9-20"), "GE-Proton9-20", None),
            ToolKind::GeProton
        );
        assert_eq!(
            classify(Some("proton_experimental"), "Proton Experimental", None),
            ToolKind::Experimental
        );
        assert_eq!(
            classify(None, "Proton Hotfix", Some("2180100")),
            ToolKind::Hotfix
        );
        assert_eq!(
            classify(None, "Proton 9.0", Some("2805730")),
            ToolKind::Official
        );
        assert_eq!(
            classify(None, "Steam Linux Runtime 3.0 (sniper)", Some("1628350")),
            ToolKind::Runtime
        );
        assert_eq!(
            classify(
                Some("proton-cachyos-slr"),
                "proton-cachyos-11.0 (steam linux runtime)",
                None
            ),
            ToolKind::Custom
        );
    }

    fn fixture() -> (tempfile::TempDir, ToolRegistry, HashMap<String, String>) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        // Ferramenta personalizada com protonfixes embutido
        let ctd = root.join("compatibilitytools.d/GE-Proton9-20");
        std::fs::create_dir_all(ctd.join("protonfixes")).unwrap();
        std::fs::write(
            ctd.join("compatibilitytool.vdf"),
            r#""compatibilitytools" { "compat_tools" { "GE-Proton9-20" { "install_path" "." "display_name" "GE-Proton9-20" } } }"#,
        )
        .unwrap();
        std::fs::write(ctd.join("version"), "1700000000 GE-Proton9-20\n").unwrap();
        // Ferramenta quebrada não deve derrubar a leitura
        let broken = root.join("compatibilitytools.d/quebrada");
        std::fs::create_dir_all(&broken).unwrap();
        std::fs::write(broken.join("compatibilitytool.vdf"), "{{{").unwrap();
        let (tools, warnings) = custom_tools(&[root.join("compatibilitytools.d")]);
        assert_eq!(warnings.len(), 1);
        let reg = ToolRegistry { tools };
        std::fs::create_dir_all(root.join("config")).unwrap();
        std::fs::write(
            root.join("config/config.vdf"),
            r#""InstallConfigStore" { "Software" { "Valve" { "Steam" { "CompatToolMapping" {
                "0" { "name" "proton_experimental" "config" "" "priority" "75" }
                "200260" { "name" "GE-Proton9-20" "config" "" "priority" "250" }
                "35140" { "name" "" }
            } } } } }"#,
        )
        .unwrap();
        let (map, err) = read_tool_mapping(root);
        assert!(err.is_none());
        (tmp, reg, map)
    }

    #[test]
    fn per_game_tool_wins_over_global() {
        let (_tmp, reg, map) = fixture();
        let c = resolve_game("200260", &map, &reg, None);
        assert_eq!(c.tool_source, ToolSource::PerGame);
        let tool = c.tool.unwrap();
        assert_eq!(tool.kind, ToolKind::GeProton);
        assert!(tool.includes_protonfixes);
        assert_eq!(tool.version.as_deref(), Some("GE-Proton9-20"));
    }

    #[test]
    fn global_default_only_applies_with_prefix_and_never_invents_versions() {
        let (tmp, reg, map) = fixture();
        // Sem prefixo e sem mapeamento: desconhecido.
        let none = resolve_game("35140", &map, &reg, None);
        assert_eq!(none.tool_source, ToolSource::None);
        assert!(none.tool.is_none() && none.uses_proton.is_none());
        // Com prefixo, sem config_info: padrão global, marcado como tal.
        let pfx = tmp.path().join("compatdata/35140");
        std::fs::create_dir_all(pfx.join("pfx")).unwrap();
        std::fs::write(pfx.join("version"), "9.0-4\n").unwrap();
        let g = resolve_game("35140", &map, &reg, Some(&pfx));
        assert_eq!(g.tool_source, ToolSource::GlobalDefault);
        assert_eq!(g.tool.unwrap().display_name, "Proton Experimental");
        assert_eq!(g.prefix_version.as_deref(), Some("9.0-4"));
        assert_eq!(g.uses_proton, Some(true));
    }

    #[test]
    fn last_used_tool_comes_from_config_info() {
        let (tmp, reg, map) = fixture();
        let tool_dir = reg
            .by_internal("GE-Proton9-20")
            .unwrap()
            .install_path
            .clone()
            .unwrap();
        let pfx = tmp.path().join("compatdata/999");
        std::fs::create_dir_all(pfx.join("pfx")).unwrap();
        std::fs::write(
            pfx.join("config_info"),
            format!("GE-Proton9-20\n{tool_dir}/files/share/fonts/\n"),
        )
        .unwrap();
        let c = resolve_game("999", &map, &reg, Some(&pfx));
        assert_eq!(c.tool_source, ToolSource::LastUsed);
        assert_eq!(
            c.last_used_tool.unwrap().internal_name.as_deref(),
            Some("GE-Proton9-20")
        );
    }
}
