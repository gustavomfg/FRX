//! Operações de arquivo do Fix Engine: hash, escrita atômica do diário,
//! diretórios privados e espaço livre. Nada aqui segue links simbólicos no
//! destino nem usa diretórios temporários previsíveis. Os arquivos de sistema
//! (`/etc`) passam por `confined`, não por aqui.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

pub fn sha256_hex(bytes: &[u8]) -> String {
    let d = Sha256::digest(bytes);
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// Hash do arquivo, ou `None` se ele não existir.
pub fn sha256_file(p: &Path) -> Result<Option<String>, String> {
    match fs::read(p) {
        Ok(b) => Ok(Some(sha256_hex(&b))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Não foi possível ler {}: {e}", p.display())),
    }
}

/// Diretório privado (0700) — backups e diário não ficam legíveis por outros usuários.
pub fn private_dir(p: &Path) -> Result<(), String> {
    fs::create_dir_all(p).map_err(|e| format!("Não foi possível criar {}: {e}", p.display()))?;
    fs::set_permissions(p, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())
}

/// Bytes livres no sistema de arquivos de `p`.
pub fn free_space(p: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(p.as_os_str().as_bytes()).ok()?;
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: `c` é uma string C válida e `s` é uma estrutura inicializada.
    let r = unsafe { libc::statvfs(c.as_ptr(), &mut s) };
    (r == 0).then(|| s.f_bavail as u64 * s.f_frsize as u64)
}

fn refuse_symlink(p: &Path) -> Result<(), String> {
    match fs::symlink_metadata(p) {
        Ok(m) if m.file_type().is_symlink() => Err(format!(
            "{} é um link simbólico; o FRX não escreve através dele.",
            p.display()
        )),
        Ok(m) if !m.is_file() => Err(format!("{} não é um arquivo comum.", p.display())),
        _ => Ok(()),
    }
}

/// Escrita atômica: arquivo temporário exclusivo no mesmo diretório,
/// `fsync`, `rename` e `fsync` do diretório. Mantém o modo do arquivo atual
/// (ou usa `default_mode` quando ele ainda não existe).
pub fn atomic_write(target: &Path, bytes: &[u8], default_mode: u32) -> Result<(), String> {
    refuse_symlink(target)?;
    let dir = target.parent().ok_or("destino sem diretório")?;
    let mode = fs::metadata(target)
        .map(|m| m.permissions().mode() & 0o7777)
        .unwrap_or(default_mode);
    let name = target
        .file_name()
        .ok_or("destino sem nome")?
        .to_string_lossy();
    let tmp = tmp_path(dir, &name);
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)
        .map_err(|e| format!("Não foi possível criar o arquivo temporário: {e}"))?;
    let res = f
        .write_all(bytes)
        .and_then(|_| f.set_permissions(fs::Permissions::from_mode(mode)))
        .and_then(|_| f.sync_all())
        .and_then(|_| fs::rename(&tmp, target));
    if let Err(e) = res {
        let _ = fs::remove_file(&tmp);
        return Err(format!("Falha ao gravar {}: {e}", target.display()));
    }
    if let Ok(d) = File::open(dir) {
        let _ = d.sync_all();
    }
    Ok(())
}

fn tmp_path(dir: &Path, name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    dir.join(format!(".{name}.frx-{}-{nanos:x}.tmp", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_preserves_mode_and_refuses_symlinks() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("a.json");
        fs::write(&f, "old").unwrap();
        fs::set_permissions(&f, fs::Permissions::from_mode(0o640)).unwrap();
        atomic_write(&f, b"new", 0o644).unwrap();
        assert_eq!(fs::read_to_string(&f).unwrap(), "new");
        assert_eq!(
            fs::metadata(&f).unwrap().permissions().mode() & 0o777,
            0o640
        );
        // Nenhum temporário sobrando.
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 1);
        let link = tmp.path().join("link.json");
        std::os::unix::fs::symlink(&f, &link).unwrap();
        assert!(atomic_write(&link, b"x", 0o644).is_err());
        assert_eq!(fs::read_to_string(&f).unwrap(), "new");
    }
}
