//! Operações de arquivo do Fix Engine: hash, backup verificado, escrita
//! atômica preservando permissões e espaço livre. Nada aqui segue links
//! simbólicos no destino nem usa diretórios temporários previsíveis.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::models::BackupRef;

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

/// Copia `src` para `dir/name` e confere o hash da cópia. Se `src` não
/// existir, registra a ausência (o rollback então remove o arquivo criado).
pub fn backup(src: &Path, dir: &Path, name: &str) -> Result<BackupRef, String> {
    refuse_symlink(src)?;
    private_dir(dir)?;
    let dest = dir.join(name);
    let bytes = match fs::read(src) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(BackupRef {
                path: dest,
                existed: false,
                sha256: None,
                size: 0,
                mode: None,
            });
        }
        Err(e) => {
            return Err(format!(
                "Backup falhou: não foi possível ler {}: {e}",
                src.display()
            ))
        }
    };
    let mode = fs::metadata(src)
        .map(|m| m.permissions().mode() & 0o7777)
        .ok();
    if free_space(dir).is_some_and(|f| f < bytes.len() as u64 * 2 + 1024 * 1024) {
        return Err("Espaço insuficiente para o backup.".into());
    }
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&dest)
        .map_err(|e| format!("Backup falhou: {e}"))?;
    f.write_all(&bytes)
        .and_then(|_| f.sync_all())
        .map_err(|e| format!("Backup falhou: {e}"))?;
    let expected = sha256_hex(&bytes);
    if sha256_file(&dest)?.as_deref() != Some(expected.as_str()) {
        return Err("Backup falhou: a cópia não confere com o original.".into());
    }
    Ok(BackupRef {
        path: dest,
        existed: true,
        sha256: Some(expected),
        size: bytes.len() as u64,
        mode,
    })
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

/// Restaura o conteúdo do backup (ou remove o arquivo, se ele não existia).
pub fn restore(backup: &BackupRef, target: &Path) -> Result<(), String> {
    if !backup.existed {
        refuse_symlink(target)?;
        return match fs::remove_file(target) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        };
    }
    let bytes = fs::read(&backup.path).map_err(|e| format!("Backup ilegível: {e}"))?;
    if backup.sha256.as_deref() != Some(sha256_hex(&bytes).as_str()) {
        return Err("O backup foi alterado e não confere com o hash registrado.".into());
    }
    atomic_write(target, &bytes, backup.mode.unwrap_or(0o644))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_preserves_mode_and_refuses_symlinks() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("a.vdf");
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
        let link = tmp.path().join("link.vdf");
        std::os::unix::fs::symlink(&f, &link).unwrap();
        assert!(atomic_write(&link, b"x", 0o644).is_err());
        assert_eq!(fs::read_to_string(&f).unwrap(), "new");
    }

    #[test]
    fn backup_and_restore_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("cfg.ini");
        fs::write(&f, "a=1").unwrap();
        let b = backup(&f, &tmp.path().join("bk"), "0-cfg.ini").unwrap();
        assert!(b.existed && b.size == 3);
        assert_eq!(
            fs::metadata(tmp.path().join("bk"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        fs::write(&f, "a=2").unwrap();
        restore(&b, &f).unwrap();
        assert_eq!(fs::read_to_string(&f).unwrap(), "a=1");
        // Backup adulterado não é usado.
        fs::write(&b.path, "zzz").unwrap();
        assert!(restore(&b, &f).is_err());
        // Arquivo ausente: o rollback remove o que foi criado.
        let n = tmp.path().join("novo.ini");
        let b2 = backup(&n, &tmp.path().join("bk"), "1-novo.ini").unwrap();
        assert!(!b2.existed);
        fs::write(&n, "x").unwrap();
        restore(&b2, &n).unwrap();
        assert!(!n.exists());
        assert!(free_space(tmp.path()).unwrap() > 0);
    }
}
