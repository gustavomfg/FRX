//! File I/O for system files, pinned to directory descriptors. Only a closed
//! allowlist of system files ([`SYSTEM_PATHS`]) can be opened, via
//! [`Target::open_system`]. No other path under `/etc` (or anywhere else on
//! the system) can be opened for writing.
//!
//! Every directory component is opened with `O_NOFOLLOW`, the file must be a
//! regular file with a single link, and writes are atomic renames inside the
//! pinned parent.

use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use super::{files::sha256_hex, models::BackupRef};

/// Arquivos de sistema são texto curto; nada perto disso é legítimo.
pub const MAX_SYSTEM: usize = 64 * 1024;
pub const SYSTEM_REFUSED: &str = "O FRX só altera arquivos de sistema permitidos: /etc/modprobe.d/frx-*.conf, /etc/systemd/system/frx-*.service e /etc/default/grub.";

/// Allowlist fechada de destinos de sistema: diretório, prefixo e sufixo
/// exigidos do nome. `None` no prefixo = arquivo exato (o nome inteiro está
/// em `suffix`). Arquivos criados pelo FRX levam `frx-` no nome, então nunca
/// colidem com arquivos de pacotes ou do usuário.
pub const SYSTEM_PATHS: &[(&str, Option<&str>, &str)] = &[
    ("etc/modprobe.d", Some("frx-"), ".conf"),
    ("etc/systemd/system", Some("frx-"), ".service"),
    ("etc/default", None, "grub"),
];

/// Valida um caminho absoluto de sistema contra a allowlist. Devolve o
/// diretório (relativo à raiz do sistema) e o nome do arquivo.
pub fn check_system_path(abs: &str) -> Result<(&'static str, String), String> {
    let rel = abs.strip_prefix('/').ok_or(SYSTEM_REFUSED)?;
    let (dir, name) = rel.rsplit_once('/').ok_or(SYSTEM_REFUSED)?;
    let ok_name = |n: &str| {
        !n.is_empty()
            && n.len() <= 64
            && n
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.')
            && !n.starts_with('.')
            && !n.contains("..")
    };
    for (allowed, prefix, suffix) in SYSTEM_PATHS {
        if dir != *allowed || !ok_name(name) {
            continue;
        }
        let matches = match prefix {
            None => name == *suffix,
            Some(p) => name
                .strip_prefix(p)
                .and_then(|n| n.strip_suffix(suffix))
                .is_some_and(|stem| !stem.is_empty() && !stem.contains('.')),
        };
        if matches {
            return Ok((allowed, name.to_owned()));
        }
    }
    Err(SYSTEM_REFUSED.into())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileIdentity {
    pub root_dev: u64,
    pub root_ino: u64,
    pub parent_dev: u64,
    pub parent_ino: u64,
}

pub struct Target {
    parent: File,
    name: CString,
    pub identity: FileIdentity,
    limit: usize,
}

fn directory_at(parent: &File, name: &std::ffi::OsStr) -> Result<File, String> {
    let name = CString::new(name.as_bytes()).map_err(|_| "Caminho inválido")?;
    // SAFETY: valid descriptor and NUL-terminated name; successful FD is owned.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(format!(
            "Diretório não acessível ou link simbólico: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

impl Target {
    /// Arquivo de sistema da allowlist ([`SYSTEM_PATHS`]). `system_root` é a
    /// raiz do sistema: `/` em produção, uma pasta temporária nos testes.
    /// `abs` é o caminho como o usuário o vê (`/etc/modprobe.d/frx-x.conf`).
    pub fn open_system(
        system_root: &Path,
        abs: &str,
        expected: Option<&FileIdentity>,
    ) -> Result<Self, String> {
        if !system_root.is_absolute() {
            return Err("A raiz do sistema deve ser absoluta.".into());
        }
        let (dir, name) = check_system_path(abs)?;
        Self::open_pinned(system_root, &format!("{dir}/{name}"), expected, MAX_SYSTEM)
    }

    /// Abre `root/rel` componente a componente com `O_NOFOLLOW`. Quem chama
    /// já validou `root` e `rel` para o seu tipo de destino.
    fn open_pinned(
        root: &Path,
        rel: &str,
        expected: Option<&FileIdentity>,
        limit: usize,
    ) -> Result<Self, String> {
        let mut dir = File::open("/").map_err(|e| e.to_string())?;
        for c in root.components() {
            match c {
                Component::RootDir => (),
                Component::Normal(name) => dir = directory_at(&dir, name)?,
                _ => {
                    return Err(
                        "A raiz deve ser absoluta, sem atalhos ou traversal.".into(),
                    )
                }
            }
        }
        let metadata = dir.metadata().map_err(|e| e.to_string())?;
        let (root_dev, root_ino) = (metadata.dev(), metadata.ino());
        let path = Path::new(rel);
        if let Some(parent) = path.parent() {
            for c in parent.components() {
                let Component::Normal(name) = c else {
                    return Err("Caminho inválido".into());
                };
                dir = directory_at(&dir, name)?;
            }
        }
        let meta = dir.metadata().map_err(|e| e.to_string())?;
        let identity = FileIdentity {
            root_dev,
            root_ino,
            parent_dev: meta.dev(),
            parent_ino: meta.ino(),
        };
        if expected.is_some_and(|e| *e != identity) {
            return Err("CONFLITO: o diretório do sistema mudou desde a prévia.".into());
        }
        let name = CString::new(path.file_name().ok_or("Destino sem nome")?.as_bytes())
            .map_err(|_| "Caminho inválido")?;
        let target = Self {
            parent: dir,
            name,
            identity,
            limit,
        };
        target.read()?; // refuses symlinks/non-files even when content isn't needed yet.
        Ok(target)
    }

    pub fn read(&self) -> Result<Option<Vec<u8>>, String> {
        let fd = unsafe {
            libc::openat(
                self.parent.as_raw_fd(),
                self.name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        };
        if fd < 0 {
            let e = std::io::Error::last_os_error();
            return if e.kind() == std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(format!("Arquivo não acessível ou link simbólico: {e}"))
            };
        }
        let file = unsafe { File::from_raw_fd(fd) };
        let meta = file.metadata().map_err(|e| e.to_string())?;
        if !meta.is_file() || meta.nlink() != 1 || meta.len() > self.limit as u64 {
            return Err("Destino especial, hard link ou arquivo maior que o limite.".into());
        }
        let mut bytes = Vec::new();
        file.take(self.limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > self.limit {
            return Err("Arquivo maior que o limite.".into());
        }
        Ok(Some(bytes))
    }

    pub fn sha(&self) -> Result<Option<String>, String> {
        Ok(self.read()?.as_deref().map(sha256_hex))
    }

    pub fn mode(&self) -> Result<Option<u32>, String> {
        let fd = unsafe {
            libc::openat(
                self.parent.as_raw_fd(),
                self.name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        };
        if fd < 0 {
            let e = std::io::Error::last_os_error();
            return if e.kind() == std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(e.to_string())
            };
        }
        let file = unsafe { File::from_raw_fd(fd) };
        Ok(Some(
            file.metadata()
                .map_err(|e| e.to_string())?
                .permissions()
                .mode()
                & 0o666,
        ))
    }

    pub fn write(&self, bytes: &[u8], before: Option<&str>, mode: u32) -> Result<(), String> {
        self.write_guarded(bytes, before, mode, || {}, || {})
    }

    fn write_guarded(
        &self,
        bytes: &[u8],
        before: Option<&str>,
        mode: u32,
        before_rename: impl FnOnce(),
        before_restore: impl FnOnce(),
    ) -> Result<(), String> {
        if self.sha()?.as_deref() != before {
            return Err("CONFLITO: o arquivo foi alterado por outro processo.".into());
        }
        if bytes.len() > self.limit {
            return Err("Conteúdo maior que o limite do destino.".into());
        }
        let current_mode = self.mode()?;
        if current_mode.is_some_and(|m| m & 0o222 == 0) {
            return Err("O arquivo de destino não permite escrita.".into());
        }
        let mode = current_mode.unwrap_or(mode) & 0o666;
        let temp = CString::new(format!(".frx-{}.tmp", super::journal::new_op_id()))
            .map_err(|e| e.to_string())?;
        let fd = unsafe {
            libc::openat(
                self.parent.as_raw_fd(),
                temp.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(format!(
                "Não foi possível preparar a escrita: {}",
                std::io::Error::last_os_error()
            ));
        }
        let mut file = unsafe { File::from_raw_fd(fd) };
        let mut preserve_displaced = false;
        let result = (|| {
            file.write_all(bytes).map_err(|e| e.to_string())?;
            file.set_permissions(std::fs::Permissions::from_mode(mode & 0o666))
                .map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            if self.sha()?.as_deref() != before {
                return Err("CONFLITO: destino mudou antes da gravação.".into());
            }
            before_rename();
            // NOREPLACE prevents clobbering a file created after the preview.
            let result = unsafe {
                libc::renameat2(
                    self.parent.as_raw_fd(),
                    temp.as_ptr(),
                    self.parent.as_raw_fd(),
                    self.name.as_ptr(),
                    if before.is_none() {
                        libc::RENAME_NOREPLACE
                    } else {
                        libc::RENAME_EXCHANGE
                    },
                )
            };
            if result != 0 {
                return Err(format!(
                    "Instalação falhou: {}",
                    std::io::Error::last_os_error()
                ));
            }
            if before.is_some() {
                preserve_displaced = true;
                let displaced = Self {
                    parent: self.parent.try_clone().map_err(|e| {
                        format!(
                            "RECUPERAÇÃO: arquivo deslocado preservado em {}: {e}",
                            temp.to_string_lossy()
                        )
                    })?,
                    name: temp.clone(),
                    identity: self.identity.clone(),
                    limit: self.limit,
                };
                if displaced.sha().ok().flatten().as_deref() != before {
                    // The displaced entry is retained until we can restore it;
                    // no concurrent content is silently discarded.
                    preserve_displaced = true;
                    if self
                        .sha()
                        .map_err(|e| {
                            format!(
                                "RECUPERAÇÃO: arquivo deslocado preservado em {}: {e}",
                                temp.to_string_lossy()
                            )
                        })?
                        .as_deref()
                        == Some(sha256_hex(bytes).as_str())
                    {
                        before_restore();
                        let restored = unsafe {
                            libc::renameat2(
                                self.parent.as_raw_fd(),
                                temp.as_ptr(),
                                self.parent.as_raw_fd(),
                                self.name.as_ptr(),
                                libc::RENAME_EXCHANGE,
                            )
                        };
                        if restored == 0
                            && displaced.sha().ok().flatten().as_deref()
                                == Some(sha256_hex(bytes).as_str())
                        {
                            preserve_displaced = false;
                            return Err("CONFLITO: o destino mudou na substituição atômica; a alteração externa foi preservada.".into());
                        }
                    }
                    return Err(format!("RECUPERAÇÃO: alteração concorrente preservada em {} no diretório do destino; não foi descartada.",temp.to_string_lossy()));
                }
                // Original matches the backed-up precondition. This removes only
                // the exchanged entry, through the already pinned directory.
                if unsafe { libc::unlinkat(self.parent.as_raw_fd(), temp.as_ptr(), 0) } != 0 {
                    return Err(format!(
                        "RECUPERAÇÃO: cópia deslocada preservada em {}; limpeza manual necessária.",
                        temp.to_string_lossy()
                    ));
                }
                preserve_displaced = false;
            }
            self.parent.sync_all().map_err(|e| e.to_string())?;
            Ok(())
        })();
        if result.is_err() && !preserve_displaced {
            unsafe { libc::unlinkat(self.parent.as_raw_fd(), temp.as_ptr(), 0) };
        }
        result
    }

    #[cfg(test)]
    pub fn write_with_race(
        &self,
        bytes: &[u8],
        before: Option<&str>,
        race: impl FnOnce(),
    ) -> Result<(), String> {
        self.write_guarded(bytes, before, 0o644, race, || {})
    }

    #[cfg(test)]
    pub fn write_with_two_races(
        &self,
        bytes: &[u8],
        before: Option<&str>,
        race: impl FnOnce(),
        restore_race: impl FnOnce(),
    ) -> Result<(), String> {
        self.write_guarded(bytes, before, 0o644, race, restore_race)
    }

    pub fn backup(&self, dir: &Path, name: &str) -> Result<BackupRef, String> {
        super::files::private_dir(dir)?;
        let path = dir.join(name);
        let Some(bytes) = self.read()? else {
            return Ok(BackupRef {
                path,
                existed: false,
                sha256: None,
                size: 0,
                mode: None,
            });
        };
        if super::files::free_space(dir).is_some_and(|n| n < bytes.len() as u64 * 2 + 1024 * 1024) {
            return Err("Espaço insuficiente para backup.".into());
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        let sha = sha256_hex(&bytes);
        if super::files::sha256_file(&path)?.as_deref() != Some(&sha) {
            return Err("Backup não confere.".into());
        }
        Ok(BackupRef {
            path,
            existed: true,
            sha256: Some(sha),
            size: bytes.len() as u64,
            mode: self.mode()?,
        })
    }

    pub fn restore(&self, backup: &BackupRef, expected: Option<&str>) -> Result<(), String> {
        if self.sha()?.as_deref() != expected {
            return Err("CONFLITO: alteração externa impede restaurar automaticamente.".into());
        }
        if backup.existed {
            let bytes = std::fs::read(&backup.path).map_err(|e| e.to_string())?;
            if backup.sha256.as_deref() != Some(sha256_hex(&bytes).as_str()) {
                return Err("Backup alterado ou inválido.".into());
            }
            self.write(&bytes, expected, backup.mode.unwrap_or(0o644))
        } else {
            let temp = CString::new(format!(".frx-remove-{}.tmp", super::journal::new_op_id()))
                .map_err(|e| e.to_string())?;
            let r = unsafe {
                libc::renameat2(
                    self.parent.as_raw_fd(),
                    self.name.as_ptr(),
                    self.parent.as_raw_fd(),
                    temp.as_ptr(),
                    libc::RENAME_NOREPLACE,
                )
            };
            if r != 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            let displaced = Self {
                parent: self.parent.try_clone().map_err(|e| {
                    format!(
                        "RECUPERAÇÃO: arquivo preservado em {}: {e}",
                        temp.to_string_lossy()
                    )
                })?,
                name: temp.clone(),
                identity: self.identity.clone(),
                limit: self.limit,
            };
            if displaced.sha().ok().flatten().as_deref() != expected {
                let restored = unsafe {
                    libc::renameat2(
                        self.parent.as_raw_fd(),
                        temp.as_ptr(),
                        self.parent.as_raw_fd(),
                        self.name.as_ptr(),
                        libc::RENAME_NOREPLACE,
                    )
                };
                return Err(if restored == 0 {
                    "CONFLITO: alteração externa preservada durante a remoção.".into()
                } else {
                    format!(
                        "RECUPERAÇÃO: alteração externa preservada em {} no diretório do destino.",
                        temp.to_string_lossy()
                    )
                });
            }
            if unsafe { libc::unlinkat(self.parent.as_raw_fd(), temp.as_ptr(), 0) } != 0 {
                return Err(format!(
                    "RECUPERAÇÃO: arquivo preservado em {}; limpeza necessária.",
                    temp.to_string_lossy()
                ));
            }
            self.parent.sync_all().map_err(|e| e.to_string())
        }
    }
}

#[cfg(test)]
mod system_tests {
    use super::*;

    /// Raiz de sistema fictícia com os três diretórios permitidos.
    fn sysroot() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        for d in ["etc/modprobe.d", "etc/systemd/system", "etc/default"] {
            std::fs::create_dir_all(tmp.path().join(d)).unwrap();
        }
        tmp
    }

    #[test]
    fn only_the_allowlisted_system_paths_are_accepted() {
        for ok in [
            "/etc/modprobe.d/frx-audio.conf",
            "/etc/modprobe.d/frx-snd-hda-intel.conf",
            "/etc/systemd/system/frx-iwlwifi-sleep.service",
            "/etc/default/grub",
        ] {
            assert!(check_system_path(ok).is_ok(), "{ok}");
        }
        for bad in [
            "/etc/passwd",
            "/etc/shadow",
            "/etc/sudoers.d/frx-x.conf",
            "/etc/modprobe.d/alsa.conf",          // sem frx-: arquivo de outro dono
            "/etc/modprobe.d/frx-.conf",          // sem nome
            "/etc/modprobe.d/frx-a.conf.bak",     // sufixo errado
            "/etc/modprobe.d/frx-a.b.conf",       // ponto extra
            "/etc/modprobe.d/../shadow",
            "/etc/modprobe.d/../modprobe.d/frx-a.conf",
            "/etc/modprobe.d//frx-a.conf",
            "/etc/modprobe.d/sub/frx-a.conf",
            "/etc/systemd/system/frx-a.timer",
            "/etc/systemd/system/multi-user.target.wants/frx-a.service",
            "/etc/default/grub.d",
            "/etc/default/frx-grub",
            "/usr/lib/modprobe.d/frx-a.conf",
            "etc/modprobe.d/frx-a.conf",          // relativo
            "/etc/modprobe.d/FRX-A.conf",         // maiúsculas
            "/etc/modprobe.d/frx-a b.conf",
            "/etc/modprobe.d/frx-a\nb.conf",
        ] {
            assert_eq!(check_system_path(bad).err().as_deref(), Some(SYSTEM_REFUSED), "{bad}");
        }
    }

    #[test]
    fn writes_restores_and_removes_inside_the_fake_system_root() {
        let root = sysroot();
        let t = Target::open_system(root.path(), "/etc/modprobe.d/frx-audio.conf", None).unwrap();
        assert_eq!(t.read().unwrap(), None);
        let backup_dir = root.path().join("backup");
        let b = t.backup(&backup_dir, "0").unwrap();
        assert!(!b.existed);
        t.write(b"options snd_hda_intel model=dell-headset-multi\n", None, 0o644).unwrap();
        let written = t.sha().unwrap();
        assert!(written.is_some());
        let mode = std::fs::metadata(root.path().join("etc/modprobe.d/frx-audio.conf"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o644);
        // O rollback de um arquivo criado pelo FRX o remove.
        t.restore(&b, written.as_deref()).unwrap();
        assert!(!root.path().join("etc/modprobe.d/frx-audio.conf").exists());
        // Nada além do destino foi criado no diretório.
        assert_eq!(std::fs::read_dir(root.path().join("etc/modprobe.d")).unwrap().count(), 0);
    }

    #[test]
    fn symlinks_and_hard_links_are_refused_in_system_paths() {
        let root = sysroot();
        let outside = tempfile::tempdir().unwrap();
        // Diretório permitido trocado por link simbólico.
        std::fs::remove_dir(root.path().join("etc/systemd/system")).unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("etc/systemd/system")).unwrap();
        let e = Target::open_system(root.path(), "/etc/systemd/system/frx-a.service", None)
            .err()
            .unwrap();
        assert!(e.contains("link simbólico"), "{e}");
        // Arquivo permitido que é link simbólico para fora.
        std::fs::write(outside.path().join("passwd"), "root:x:0:0").unwrap();
        std::os::unix::fs::symlink(outside.path().join("passwd"), root.path().join("etc/default/grub")).unwrap();
        let e = Target::open_system(root.path(), "/etc/default/grub", None).err().unwrap();
        assert!(e.contains("link simbólico"), "{e}");
        // Hard link: o conteúdo seria compartilhado com outro arquivo.
        std::fs::hard_link(
            outside.path().join("passwd"),
            root.path().join("etc/modprobe.d/frx-x.conf"),
        )
        .unwrap();
        let e = Target::open_system(root.path(), "/etc/modprobe.d/frx-x.conf", None)
            .err()
            .unwrap();
        assert!(e.contains("hard link"), "{e}");
        assert_eq!(std::fs::read_to_string(outside.path().join("passwd")).unwrap(), "root:x:0:0");
    }

    #[test]
    fn a_relative_system_root_is_refused() {
        assert!(Target::open_system(Path::new("tmp"), "/etc/default/grub", None).is_err());
    }

    #[test]
    fn identity_change_between_preview_and_write_is_a_conflict() {
        let root = sysroot();
        let t = Target::open_system(root.path(), "/etc/modprobe.d/frx-a.conf", None).unwrap();
        let id = t.identity.clone();
        // O diretório antigo continua existindo: o inode novo é garantidamente outro.
        std::fs::rename(root.path().join("etc/modprobe.d"), root.path().join("etc/old")).unwrap();
        std::fs::create_dir(root.path().join("etc/modprobe.d")).unwrap();
        let e = Target::open_system(root.path(), "/etc/modprobe.d/frx-a.conf", Some(&id))
            .err()
            .unwrap();
        assert!(e.starts_with("CONFLITO"), "{e}");
    }

    #[test]
    fn a_change_made_after_the_check_is_never_overwritten() {
        let root = sysroot();
        let file = root.path().join("etc/modprobe.d/frx-a.conf");
        std::fs::write(&file, "orig\n").unwrap();
        let t = Target::open_system(root.path(), "/etc/modprobe.d/frx-a.conf", None).unwrap();
        let before = sha256_hex(b"orig\n");
        let race_path = file.clone();
        let e = t
            .write_with_race(b"novo\n", Some(&before), move || {
                std::fs::write(&race_path, "outro\n").unwrap()
            })
            .unwrap_err();
        assert!(e.starts_with("CONFLITO"), "{e}");
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "outro\n", "a alteração externa fica");
    }

    #[test]
    fn a_race_during_restore_preserves_the_concurrent_content() {
        let root = sysroot();
        let dir = root.path().join("etc/modprobe.d");
        let file = dir.join("frx-a.conf");
        std::fs::write(&file, "orig\n").unwrap();
        let t = Target::open_system(root.path(), "/etc/modprobe.d/frx-a.conf", None).unwrap();
        let before = sha256_hex(b"orig\n");
        let (race_path, restore_path) = (file.clone(), file.clone());
        let e = t
            .write_with_two_races(
                b"novo\n",
                Some(&before),
                move || std::fs::write(&race_path, "outro\n").unwrap(),
                move || std::fs::write(&restore_path, "terceiro\n").unwrap(),
            )
            .unwrap_err();
        assert!(e.starts_with("RECUPERAÇÃO"), "{e}");
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "outro\n");
        let preserved = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .any(|entry| std::fs::read_to_string(entry.path()).is_ok_and(|s| s == "terceiro\n"));
        assert!(preserved, "o conteúdo concorrente não pode ser descartado");
    }
}
