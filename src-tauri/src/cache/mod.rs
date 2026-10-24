//! Cache local em arquivos JSON (um arquivo por chave, escrita atômica).
//!
//! Estrutura: `<app_cache_dir>/<namespace>/<chave>.json`. Guarda apenas o
//! necessário — metadados de jogos e respostas públicas das fontes — nunca
//! tokens ou dados pessoais.

use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::time::now_unix;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry<T> {
    pub fetched_at: u64,
    #[serde(default)]
    pub etag: Option<String>,
    pub data: T,
}

impl<T> Entry<T> {
    pub fn new(data: T, etag: Option<String>) -> Self {
        Self {
            fetched_at: now_unix(),
            etag,
            data,
        }
    }

    pub fn age_secs(&self) -> u64 {
        now_unix().saturating_sub(self.fetched_at)
    }

    pub fn is_fresh(&self, max_age_secs: u64) -> bool {
        self.age_secs() <= max_age_secs
    }
}

#[derive(Debug, Clone)]
pub struct Cache {
    dir: PathBuf,
}

/// Namespaces permitidos — evita que uma chave vinda de fora escolha o diretório.
pub const NAMESPACES: &[&str] = &["library", "fixes", "providers", "art"];

pub fn sanitize(key: &str) -> String {
    let s: String = key
        .chars()
        .take(120)
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let s = s.trim_start_matches('.').to_owned();
    if s.is_empty() {
        "_".into()
    } else {
        s
    }
}

impl Cache {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn ns_dir(&self, ns: &str) -> PathBuf {
        debug_assert!(NAMESPACES.contains(&ns), "namespace desconhecido: {ns}");
        self.dir.join(sanitize(ns))
    }

    fn file(&self, ns: &str, key: &str) -> PathBuf {
        self.ns_dir(ns).join(format!("{}.json", sanitize(key)))
    }

    pub fn get<T: DeserializeOwned>(&self, ns: &str, key: &str) -> Option<Entry<T>> {
        let bytes = std::fs::read(self.file(ns, key)).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    pub fn put<T: Serialize>(&self, ns: &str, key: &str, entry: &Entry<T>) -> std::io::Result<()> {
        let path = self.file(ns, key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(
            &tmp,
            serde_json::to_vec(entry).map_err(std::io::Error::other)?,
        )?;
        std::fs::rename(tmp, path)
    }

    /// Lista as entradas de um namespace (ignora arquivos corrompidos).
    pub fn list<T: DeserializeOwned>(&self, ns: &str) -> Vec<(String, Entry<T>)> {
        let Ok(rd) = std::fs::read_dir(self.ns_dir(ns)) else {
            return vec![];
        };
        rd.flatten()
            .filter_map(|e| {
                let name = e
                    .file_name()
                    .to_string_lossy()
                    .strip_suffix(".json")?
                    .to_owned();
                let entry = serde_json::from_slice(&std::fs::read(e.path()).ok()?).ok()?;
                Some((name, entry))
            })
            .collect()
    }

    pub fn clear(&self, ns: Option<&str>) -> std::io::Result<()> {
        let targets: Vec<PathBuf> = match ns {
            Some(n) => vec![self.ns_dir(n)],
            None => NAMESPACES.iter().map(|n| self.ns_dir(n)).collect(),
        };
        for t in targets {
            match std::fs::remove_dir_all(&t) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
                _ => {}
            }
        }
        Ok(())
    }

    pub fn size_bytes(&self) -> u64 {
        fn walk(p: &Path) -> u64 {
            let Ok(rd) = std::fs::read_dir(p) else {
                return 0;
            };
            rd.flatten()
                .map(|e| match e.file_type() {
                    Ok(t) if t.is_dir() => walk(&e.path()),
                    Ok(t) if t.is_file() => e.metadata().map(|m| m.len()).unwrap_or(0),
                    _ => 0,
                })
                .sum()
        }
        walk(&self.dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_get_and_expiry() {
        let tmp = tempfile::tempdir().unwrap();
        let c = Cache::new(tmp.path());
        let mut e = Entry::new(vec![1, 2, 3], Some("\"abc\"".into()));
        c.put("fixes", "200260", &e).unwrap();
        let back: Entry<Vec<i32>> = c.get("fixes", "200260").unwrap();
        assert_eq!(back.data, vec![1, 2, 3]);
        assert_eq!(back.etag.as_deref(), Some("\"abc\""));
        assert!(back.is_fresh(60));
        e.fetched_at = now_unix() - 3600;
        assert!(!e.is_fresh(60));
        assert!(e.is_fresh(7200));
        assert_eq!(c.list::<Vec<i32>>("fixes").len(), 1);
        c.clear(Some("fixes")).unwrap();
        assert!(c.get::<Vec<i32>>("fixes", "200260").is_none());
    }

    #[test]
    fn keys_cannot_escape_the_cache_dir() {
        assert_eq!(sanitize("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(sanitize(""), "_");
        let tmp = tempfile::tempdir().unwrap();
        let c = Cache::new(tmp.path().join("c"));
        c.put("fixes", "../../fora", &Entry::new(1, None)).unwrap();
        assert!(!tmp.path().join("fora.json").exists());
    }

    #[test]
    fn corrupted_entries_are_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        let c = Cache::new(tmp.path());
        std::fs::create_dir_all(c.ns_dir("fixes")).unwrap();
        std::fs::write(c.ns_dir("fixes").join("x.json"), "{lixo").unwrap();
        assert!(c.get::<i32>("fixes", "x").is_none());
        assert!(c.list::<i32>("fixes").is_empty());
    }
}
