use super::models::*;
use crate::fix_engine::files::atomic_write;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
const LIMIT: u64 = 4 * 1024 * 1024;
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Inputs {
    pub symptoms: Vec<String>,
    pub log: Option<LogObservation>,
    pub knowledge: Vec<Evidence>,
}
pub struct Store {
    root: PathBuf,
}
impl Store {
    pub fn new(engine_root: &Path) -> Self {
        Self {
            root: engine_root.join("resolution"),
        }
    }
    fn filename(app: &str, kind: &str) -> Result<String, String> {
        if !crate::art::valid_app_id(app) {
            return Err("AppID inválido".into());
        }
        if !matches!(kind, "report" | "inputs") {
            return Err("Tipo de registro inválido".into());
        }
        Ok(format!("{app}-{kind}.json"))
    }
    /// Every ancestor is opened without following symlinks. The final owned
    /// descriptor remains live during IO, so replacing a parent path cannot
    /// redirect an operation into another directory.
    fn directory(&self, create: bool) -> Result<Option<File>, String> {
        let absolute = if self.root.is_absolute() {
            self.root.clone()
        } else {
            std::env::current_dir()
                .map_err(|e| e.to_string())?
                .join(&self.root)
        };
        let mut dir = File::open("/").map_err(|e| e.to_string())?;
        for component in absolute.components() {
            let name = match component {
                Component::RootDir => continue,
                Component::Normal(name) => name,
                _ => return Err("Diretório de estado contém componente ambíguo".into()),
            };
            let name = CString::new(name.as_bytes()).map_err(|_| "Diretório inválido")?;
            let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
            // SAFETY: dir is live; name is NUL-terminated and flags create no
            // files. A successful call returns a newly owned descriptor.
            let mut fd = unsafe { libc::openat(dir.as_raw_fd(), name.as_ptr(), flags) };
            if fd < 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() != std::io::ErrorKind::NotFound {
                    return Err(
                        "Diretório de estado indisponível ou com link simbólico; nada foi seguido."
                            .into(),
                    );
                }
                if !create {
                    return Ok(None);
                }
                // SAFETY: the parent descriptor is pinned and the name is a
                // single path component. Only missing directories are created.
                let made = unsafe { libc::mkdirat(dir.as_raw_fd(), name.as_ptr(), 0o700) };
                if made < 0
                    && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
                {
                    return Err("Não foi possível criar o diretório privado de estado".into());
                }
                // Reopen even after mkdirat: an intervening symlink is refused.
                // SAFETY: same descriptor/name guarantees as the first openat.
                fd = unsafe { libc::openat(dir.as_raw_fd(), name.as_ptr(), flags) };
                if fd < 0 {
                    return Err("O diretório de estado mudou durante a preparação".into());
                }
            }
            // SAFETY: this scope owns the successful openat descriptor.
            dir = unsafe { File::from_raw_fd(fd) };
        }
        if create {
            dir.set_permissions(std::fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
        }
        Ok(Some(dir))
    }
    fn pinned_path(dir: &File, name: &str) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", dir.as_raw_fd())).join(name)
    }
    fn load<T: DeserializeOwned>(&self, app: &str, kind: &str) -> Result<Option<T>, String> {
        let name = Self::filename(app, kind)?;
        let Some(dir) = self.directory(false)? else {
            return Ok(None);
        };
        let p = Self::pinned_path(&dir, &name);
        let f = match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(p)
        {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        let meta = f.metadata().map_err(|e| e.to_string())?;
        if !meta.is_file() || meta.len() > LIMIT {
            return Err("Relatório excede o limite ou não é arquivo regular".into());
        }
        let mut bytes = Vec::new();
        f.take(LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > LIMIT {
            return Err("Relatório excede o limite".into());
        }
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| format!("Relatório preservado, mas ilegível: {e}"))
    }
    fn save<T: Serialize>(&self, app: &str, kind: &str, data: &T) -> Result<(), String> {
        let name = Self::filename(app, kind)?;
        let bytes = serde_json::to_vec_pretty(data).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > LIMIT {
            return Err("Relatório excede o limite".into());
        }
        let dir = self.directory(true)?.ok_or("Diretório de estado ausente")?;
        let path = Self::pinned_path(&dir, &name);
        // atomic_write preserves an existing mode. Tighten it by descriptor
        // first, never through a symlink or a hardlink to an unrelated inode.
        match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)
        {
            Ok(file) => {
                let meta = file.metadata().map_err(|e| e.to_string())?;
                if !meta.is_file() || meta.nlink() != 1 {
                    return Err("Estado não é arquivo privado regular; registro preservado".into());
                }
                file.set_permissions(std::fs::Permissions::from_mode(0o600))
                    .map_err(|e| e.to_string())?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
        }
        // The descriptor must outlive atomic_write (temporary file, rename and
        // directory fsync all resolve beneath this pinned directory).
        atomic_write(&path, &bytes, 0o600)
    }
    pub fn report(&self, app: &str) -> Result<Option<Report>, String> {
        let r: Option<Report> = self.load(app, "report")?;
        if r.as_ref()
            .is_some_and(|r| r.app_id != app || r.schema != 1 || !report_scope(r, app))
        {
            return Err("Relatório de outro jogo ou formato não suportado; preservado.".into());
        }
        Ok(r)
    }
    pub fn save_report(&self, r: &Report) -> Result<(), String> {
        if !report_scope(r, &r.app_id) {
            return Err("Relatório contém dados de outro jogo".into());
        }
        self.save(&r.app_id, "report", r)
    }
    pub fn inputs(&self, app: &str) -> Result<Inputs, String> {
        let data: Inputs = self.load(app, "inputs")?.unwrap_or_default();
        super::valid_symptoms(&data.symptoms)?;
        if data.knowledge.iter().any(|e| !evidence_scope(e, app))
            || data.log.as_ref().is_some_and(|l| {
                l.app_id != app || l.evidence.iter().any(|e| !evidence_scope(e, app))
            })
        {
            return Err("Log persistido de outro jogo".into());
        }
        Ok(data)
    }
    pub fn save_inputs(&self, app: &str, i: &Inputs) -> Result<(), String> {
        self.save(app, "inputs", i)
    }
}

fn evidence_scope(e: &Evidence, app: &str) -> bool {
    e.app_id.as_deref().is_none_or(|id| id == app)
}
fn report_scope(r: &Report, app: &str) -> bool {
    r.plans
        .iter()
        .all(|p| p.app_id == app && p.sources.iter().all(|e| evidence_scope(e, app)))
        && r.packs.iter().all(|p| p.app_id == app)
        && r.knowledge.iter().all(|e| evidence_scope(e, app))
        && r.technologies
            .iter()
            .flat_map(|t| &t.evidence)
            .all(|e| evidence_scope(e, app))
        && r.problems
            .iter()
            .flat_map(|p| &p.evidence)
            .all(|e| evidence_scope(e, app))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn inputs() -> Inputs {
        Inputs {
            symptoms: vec!["startup".into()],
            ..Default::default()
        }
    }

    #[test]
    fn roundtrip_restart_and_private_permissions() {
        let temp = tempfile::tempdir().unwrap();
        let engine = temp.path().join("private").join("engine");
        let store = Store::new(&engine);
        assert!(store.report("123").unwrap().is_none());
        assert!(!engine.exists()); // A missing report does not create state.
        store.save_inputs("123", &inputs()).unwrap();
        let root = engine.join("resolution");
        let file = root.join("123-inputs.json");
        assert_eq!(std::fs::metadata(&root).unwrap().mode() & 0o777, 0o700);
        assert_eq!(std::fs::metadata(&file).unwrap().mode() & 0o777, 0o600);
        assert_eq!(
            Store::new(&engine).inputs("123").unwrap().symptoms,
            ["startup"]
        );
        // Existing permissive local state is tightened before another write.
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
        store.save_inputs("123", &inputs()).unwrap();
        assert_eq!(std::fs::metadata(file).unwrap().mode() & 0o777, 0o600);
    }

    #[test]
    fn rejects_symlinked_root_and_parent_without_changing_external_data() {
        for root_link in [true, false] {
            let temp = tempfile::tempdir().unwrap();
            let outside = temp.path().join("outside");
            std::fs::create_dir(&outside).unwrap();
            std::fs::set_permissions(&outside, std::fs::Permissions::from_mode(0o755)).unwrap();
            let sentinel = outside.join("123-inputs.json");
            std::fs::write(&sentinel, b"external preserved").unwrap();
            let engine = temp.path().join("engine");
            if root_link {
                std::fs::create_dir(&engine).unwrap();
                symlink(&outside, engine.join("resolution")).unwrap();
            } else {
                symlink(&outside, &engine).unwrap();
            }
            let store = Store::new(&engine);
            assert!(store.inputs("123").is_err());
            assert!(store.save_inputs("123", &inputs()).is_err());
            assert_eq!(std::fs::read(&sentinel).unwrap(), b"external preserved");
            assert_eq!(std::fs::metadata(&outside).unwrap().mode() & 0o777, 0o755);
            assert!(!outside.join("resolution").exists());
        }
    }

    #[test]
    fn rejects_leaf_symlinks_and_hardlinks_without_chmod_external_file() {
        for hard in [true, false] {
            let temp = tempfile::tempdir().unwrap();
            let outside = temp.path().join("outside.json");
            std::fs::write(&outside, b"external preserved").unwrap();
            std::fs::set_permissions(&outside, std::fs::Permissions::from_mode(0o644)).unwrap();
            let engine = temp.path().join("engine");
            let root = engine.join("resolution");
            std::fs::create_dir_all(&root).unwrap();
            let target = root.join("123-inputs.json");
            if hard {
                std::fs::hard_link(&outside, &target).unwrap();
            } else {
                symlink(&outside, &target).unwrap();
            }
            assert!(Store::new(&engine).save_inputs("123", &inputs()).is_err());
            assert_eq!(std::fs::read(&outside).unwrap(), b"external preserved");
            assert_eq!(std::fs::metadata(&outside).unwrap().mode() & 0o777, 0o644);
        }
    }

    #[test]
    fn pinned_directory_cannot_be_redirected_after_parent_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let engine = temp.path().join("engine");
        let moved = temp.path().join("moved-engine");
        let outside = temp.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        let store = Store::new(&engine);
        let descriptor = store.directory(true).unwrap().unwrap();
        std::fs::rename(&engine, &moved).unwrap();
        symlink(&outside, &engine).unwrap();
        let anchored = Store::pinned_path(&descriptor, "123-inputs.json");
        atomic_write(&anchored, b"{}", 0o600).unwrap();
        assert_eq!(
            std::fs::read(moved.join("resolution/123-inputs.json")).unwrap(),
            b"{}"
        );
        assert!(std::fs::read_dir(&outside).unwrap().next().is_none());
        assert!(store.save_inputs("123", &inputs()).is_err());
    }

    #[test]
    fn oversized_write_and_traversal_preserve_existing_state() {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path());
        store.save_inputs("123", &inputs()).unwrap();
        assert!(store
            .save("123", "inputs", &"x".repeat(LIMIT as usize))
            .is_err());
        assert!(store.save_inputs("../123", &inputs()).is_err());
        assert_eq!(store.inputs("123").unwrap().symptoms, ["startup"]);
    }
}
