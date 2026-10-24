//! Leitura de `appmanifest_<appid>.acf`.

use std::path::{Path, PathBuf};

use super::libraries::read_text;
use super::models::InstallationStatus;
use super::vdf;

#[derive(Debug, Clone, PartialEq)]
pub struct AppManifest {
    pub app_id: String,
    pub name: Option<String>,
    pub install_dir: Option<String>,
    pub state_flags: Option<u64>,
    pub size_on_disk: Option<u64>,
    pub last_updated: Option<u64>,
    pub last_played: Option<u64>,
    pub build_id: Option<String>,
    pub bytes_to_download: Option<u64>,
    pub bytes_downloaded: Option<u64>,
    pub manifest_path: PathBuf,
}

/// Bits de `StateFlags` (EAppState da Steam).
const FLAGS: &[(u64, &str)] = &[
    (1, "uninstalled"),
    (2, "update-required"),
    (4, "fully-installed"),
    (8, "encrypted"),
    (16, "locked"),
    (32, "files-missing"),
    (64, "app-running"),
    (128, "files-corrupt"),
    (256, "update-running"),
    (512, "update-paused"),
    (1024, "update-started"),
    (2048, "uninstalling"),
    (4096, "backup-running"),
    (65_536, "reconfiguring"),
    (131_072, "validating"),
    (262_144, "adding-files"),
    (524_288, "preallocating"),
    (1_048_576, "downloading"),
    (2_097_152, "staging"),
    (4_194_304, "committing"),
    (8_388_608, "update-stopping"),
];

pub fn flag_names(flags: u64) -> Vec<String> {
    FLAGS
        .iter()
        .filter(|(b, _)| flags & b != 0)
        .map(|(_, n)| (*n).to_owned())
        .collect()
}

pub fn parse_manifest(text: &str, manifest_path: &Path) -> Result<AppManifest, String> {
    let doc = vdf::parse(text).map_err(|e| e.to_string())?;
    let app = doc.get("AppState").ok_or("sem bloco AppState")?;
    let app_id = app
        .str_at("appid")
        .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
        .ok_or("appid ausente ou inválido")?;
    let nonempty = |k: &str| {
        app.str_at(k)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    Ok(AppManifest {
        app_id: app_id.to_owned(),
        name: nonempty("name"),
        install_dir: nonempty("installdir"),
        state_flags: app.u64_at("StateFlags"),
        size_on_disk: app.u64_at("SizeOnDisk"),
        last_updated: app.u64_at("LastUpdated").filter(|&t| t > 0),
        last_played: app.u64_at("LastPlayed").filter(|&t| t > 0),
        build_id: nonempty("buildid"),
        bytes_to_download: app.u64_at("BytesToDownload"),
        bytes_downloaded: app.u64_at("BytesDownloaded"),
        manifest_path: manifest_path.to_path_buf(),
    })
}

/// Lista e lê os manifestos de uma biblioteca (sem recursão).
pub fn read_manifests(library: &Path) -> (Vec<AppManifest>, Vec<String>) {
    let mut out = Vec::new();
    let mut warnings = Vec::new();
    let dir = library.join("steamapps");
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return (out, warnings);
    };
    let mut files: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("appmanifest_") && n.ends_with(".acf"))
        })
        .collect();
    files.sort();
    for f in files {
        match read_text(&f)
            .map_err(|e| e.to_string())
            .and_then(|t| parse_manifest(&t, &f))
        {
            Ok(m) => out.push(m),
            Err(e) => warnings.push(format!("Manifesto ignorado ({}): {e}", f.display())),
        }
    }
    (out, warnings)
}

/// Estado da instalação a partir dos flags e da presença dos arquivos.
pub fn installation_status(m: &AppManifest, install_path: Option<&Path>) -> InstallationStatus {
    let Some(flags) = m.state_flags else {
        return InstallationStatus::Unknown;
    };
    if let Some(p) = install_path {
        if !p.is_dir() {
            return InstallationStatus::Unavailable;
        }
    }
    let bad = 32 | 128; // arquivos ausentes ou corrompidos
    let busy = 256 | 512 | 1024 | 1_048_576 | 2_097_152 | 4_194_304 | 524_288;
    if flags & 4 != 0 && flags & bad == 0 && flags & busy == 0 {
        InstallationStatus::Installed
    } else if flags == 0 {
        InstallationStatus::Unknown
    } else {
        InstallationStatus::Incomplete
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACF: &str = r#""AppState"
{
	"appid"		"200260"
	"name"		"Batman: Arkham City GOTY"
	"StateFlags"		"4"
	"installdir"		"Batman Arkham City GOTY"
	"LastUpdated"		"1792709371"
	"LastPlayed"		"1792792536"
	"SizeOnDisk"		"19948606783"
	"buildid"		"3718300"
}"#;

    #[test]
    fn parses_real_world_manifest() {
        let m = parse_manifest(ACF, Path::new("/x/appmanifest_200260.acf")).unwrap();
        assert_eq!(m.app_id, "200260");
        assert_eq!(m.name.as_deref(), Some("Batman: Arkham City GOTY"));
        assert_eq!(m.state_flags, Some(4));
        assert_eq!(m.size_on_disk, Some(19_948_606_783));
        assert_eq!(m.build_id.as_deref(), Some("3718300"));
    }

    #[test]
    fn rejects_manifest_without_valid_appid() {
        assert!(parse_manifest(r#""AppState" { "name" "x" }"#, Path::new("a")).is_err());
        assert!(parse_manifest(r#""AppState" { "appid" "abc" }"#, Path::new("a")).is_err());
        assert!(parse_manifest("lixo {", Path::new("a")).is_err());
    }

    #[test]
    fn classifies_installation_states() {
        let tmp = tempfile::tempdir().unwrap();
        let present = tmp.path().join("game");
        std::fs::create_dir(&present).unwrap();
        let mut m = parse_manifest(ACF, Path::new("a")).unwrap();
        assert_eq!(
            installation_status(&m, Some(&present)),
            InstallationStatus::Installed
        );
        assert_eq!(
            installation_status(&m, Some(&tmp.path().join("sumiu"))),
            InstallationStatus::Unavailable
        );
        m.state_flags = Some(1026); // update-required + update-started
        assert_eq!(
            installation_status(&m, Some(&present)),
            InstallationStatus::Incomplete
        );
        m.state_flags = Some(4 | 32);
        assert_eq!(
            installation_status(&m, Some(&present)),
            InstallationStatus::Incomplete
        );
        m.state_flags = None;
        assert_eq!(
            installation_status(&m, Some(&present)),
            InstallationStatus::Unknown
        );
        assert_eq!(flag_names(6), vec!["update-required", "fully-installed"]);
    }
}
