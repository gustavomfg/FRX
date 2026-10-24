//! Leitura de `libraryfolders.vdf` (formatos novo e antigo).

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::models::LibraryStatus;
use super::vdf::{self, Vdf};

#[derive(Debug, Clone, PartialEq)]
pub struct LibraryEntry {
    pub path: PathBuf,
    pub label: Option<String>,
    pub app_ids: Vec<String>,
}

/// Lê as bibliotecas declaradas por uma raiz da Steam. A própria raiz é sempre incluída.
pub fn read_library_folders(root: &Path) -> (Vec<LibraryEntry>, Vec<String>) {
    let mut warnings = Vec::new();
    let mut entries = vec![LibraryEntry {
        path: root.to_path_buf(),
        label: None,
        app_ids: vec![],
    }];
    let candidates = [
        root.join("steamapps/libraryfolders.vdf"),
        root.join("config/libraryfolders.vdf"),
    ];
    let Some(file) = candidates.iter().find(|p| p.is_file()) else {
        return (entries, warnings);
    };
    let text = match read_text(file) {
        Ok(t) => t,
        Err(e) => {
            warnings.push(format!("Não foi possível ler {}: {e}", file.display()));
            return (entries, warnings);
        }
    };
    let doc = match vdf::parse(&text) {
        Ok(d) => d,
        Err(e) => {
            warnings.push(format!(
                "{} está malformado ({e}); usando só a biblioteca principal.",
                file.display()
            ));
            return (entries, warnings);
        }
    };
    let Some(folders) = doc.get("libraryfolders") else {
        warnings.push(format!("{} não contém 'libraryfolders'.", file.display()));
        return (entries, warnings);
    };
    for (key, value) in folders.entries() {
        // Apenas chaves numéricas representam bibliotecas.
        if !key.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let entry = match value {
            // Formato antigo: "1" "/caminho"
            Vdf::Str(p) => LibraryEntry {
                path: PathBuf::from(p),
                label: None,
                app_ids: vec![],
            },
            Vdf::Obj(_) => {
                let Some(p) = value.str_at("path") else {
                    continue;
                };
                let apps = value
                    .get("apps")
                    .map(|a| a.entries().iter().map(|(k, _)| k.clone()).collect())
                    .unwrap_or_default();
                let label = value
                    .str_at("label")
                    .filter(|l| !l.is_empty())
                    .map(str::to_owned);
                LibraryEntry {
                    path: PathBuf::from(p),
                    label,
                    app_ids: apps,
                }
            }
        };
        if entry.path.as_os_str().is_empty() {
            continue;
        }
        // A raiz também aparece como "0": mesclamos os AppIDs declarados.
        if let Some(existing) = entries.iter_mut().find(|e| same_path(&e.path, &entry.path)) {
            existing.app_ids.extend(entry.app_ids);
            if existing.label.is_none() {
                existing.label = entry.label;
            }
        } else {
            entries.push(entry);
        }
    }
    (entries, warnings)
}

pub fn same_path(a: &Path, b: &Path) -> bool {
    a == b || super::discovery::canonical(a) == super::discovery::canonical(b)
}

/// Verifica se a biblioteca pode ser lida agora (disco montado, permissão).
pub fn library_status(path: &Path) -> LibraryStatus {
    match std::fs::read_dir(path.join("steamapps")) {
        Ok(_) => LibraryStatus::Available,
        Err(e) if e.kind() == ErrorKind::PermissionDenied => LibraryStatus::Inaccessible,
        Err(_) => match std::fs::metadata(path) {
            Err(e) if e.kind() == ErrorKind::PermissionDenied => LibraryStatus::Inaccessible,
            _ => LibraryStatus::Unavailable,
        },
    }
}

/// Lê um arquivo de texto com limite de tamanho (arquivos da Steam são pequenos;
/// `localconfig.vdf` é o maior e raramente passa de alguns MB).
pub fn read_text(path: &Path) -> std::io::Result<String> {
    const MAX: u64 = 32 * 1024 * 1024;
    let meta = std::fs::metadata(path)?;
    if meta.len() > MAX {
        return Err(std::io::Error::other("arquivo grande demais"));
    }
    let bytes = std::fs::read(path)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(p: &Path, s: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, s).unwrap();
    }

    #[test]
    fn reads_new_format_with_multiple_disks() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Steam");
        write(
            &root.join("steamapps/libraryfolders.vdf"),
            &format!(
                r#""libraryfolders" {{
                    "0" {{ "path" "{}" "label" "" "apps" {{ "1493710" "1" }} }}
                    "1" {{ "path" "/mnt/Disco Extra/SteamLibrary" "label" "Jogos" "apps" {{ "200260" "1" "35140" "2" }} }}
                }}"#,
                root.display()
            ),
        );
        let (libs, warnings) = read_library_folders(&root);
        assert!(warnings.is_empty());
        assert_eq!(libs.len(), 2);
        assert_eq!(libs[0].app_ids, vec!["1493710"]);
        assert_eq!(libs[1].path, PathBuf::from("/mnt/Disco Extra/SteamLibrary"));
        assert_eq!(libs[1].label.as_deref(), Some("Jogos"));
        assert_eq!(libs[1].app_ids, vec!["200260", "35140"]);
    }

    #[test]
    fn reads_legacy_format() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Steam");
        write(
            &root.join("steamapps/libraryfolders.vdf"),
            r#""LibraryFolders" { "TimeNextStatsReport" "1" "1" "/games/lib" }"#,
        );
        let (libs, _) = read_library_folders(&root);
        assert_eq!(libs.len(), 2);
        assert_eq!(libs[1].path, PathBuf::from("/games/lib"));
    }

    #[test]
    fn malformed_file_falls_back_to_root_with_warning() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Steam");
        write(
            &root.join("steamapps/libraryfolders.vdf"),
            r#""libraryfolders" { "0" { "path" "#,
        );
        let (libs, warnings) = read_library_folders(&root);
        assert_eq!(libs.len(), 1);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn detects_unmounted_library() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            library_status(&tmp.path().join("nao-montado")),
            LibraryStatus::Unavailable
        );
        std::fs::create_dir_all(tmp.path().join("ok/steamapps")).unwrap();
        assert_eq!(
            library_status(&tmp.path().join("ok")),
            LibraryStatus::Available
        );
    }
}
