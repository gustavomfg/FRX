//! Game-file I/O pinned to directory descriptors. Archive names and network
//! data never become filesystem paths; the authorized destination is explicit.

use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use super::{files::sha256_hex, models::BackupRef, security};

pub const MAX_CONFIG: usize = 256 * 1024;

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
    pub fn open(root: &Path, rel: &str, expected: Option<&FileIdentity>) -> Result<Self, String> {
        security::check_game_root(root)?;
        security::check_rel_path(rel)?;
        let mut dir = File::open("/").map_err(|e| e.to_string())?;
        for c in root.components() {
            match c {
                Component::RootDir => (),
                Component::Normal(name) => dir = directory_at(&dir, name)?,
                _ => {
                    return Err(
                        "A raiz do jogo deve ser absoluta, sem atalhos ou traversal.".into(),
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
            return Err("CONFLITO: o diretório do jogo mudou desde a prévia.".into());
        }
        let name = CString::new(path.file_name().ok_or("Destino sem nome")?.as_bytes())
            .map_err(|_| "Caminho inválido")?;
        let target = Self {
            parent: dir,
            name,
            identity,
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
        if !meta.is_file() || meta.nlink() != 1 || meta.len() > MAX_CONFIG as u64 {
            return Err(
                "Destino especial, hard link ou arquivo maior que o limite de configuração.".into(),
            );
        }
        let mut bytes = Vec::new();
        file.take(MAX_CONFIG as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > MAX_CONFIG {
            return Err("Arquivo maior que o limite.".into());
        }
        Ok(Some(bytes))
    }

    pub fn sha(&self) -> Result<Option<String>, String> {
        Ok(self.read()?.as_deref().map(sha256_hex))
    }

    fn mode(&self) -> Result<Option<u32>, String> {
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
        if bytes.len() > MAX_CONFIG {
            return Err("Configuração maior que o limite.".into());
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
