//! Processos da Steam e de jogos, lidos de `/proc` — somente leitura.
//! O FRX nunca encerra, sinaliza ou interrompe processos.

use std::path::Path;

/// A Steam está aberta? (cliente nativo ou Flatpak, que aparece no mesmo `/proc`).
pub fn steam_running(proc_root: &Path) -> bool {
    pids(proc_root).any(|pid| {
        let base = proc_root.join(pid);
        let comm = std::fs::read_to_string(base.join("comm")).unwrap_or_default();
        let comm = comm.trim();
        if comm == "steam" || comm == "steamwebhelper" {
            return true;
        }
        let cmd = std::fs::read(base.join("cmdline")).unwrap_or_default();
        let first = cmd.split(|b| *b == 0).next().unwrap_or_default();
        let first = String::from_utf8_lossy(first);
        first.ends_with("/ubuntu12_32/steam") || first.ends_with("/steam.sh")
    })
}

/// Algum processo foi iniciado pela Steam para este AppID (`SteamAppId` no ambiente)?
pub fn game_running(proc_root: &Path, app_id: &str) -> bool {
    let needle = format!("SteamAppId={app_id}");
    pids(proc_root).any(|pid| {
        std::fs::read(proc_root.join(pid).join("environ"))
            .map(|env| env.split(|b| *b == 0).any(|kv| kv == needle.as_bytes()))
            .unwrap_or(false)
    })
}

fn pids(proc_root: &Path) -> impl Iterator<Item = String> {
    std::fs::read_dir(proc_root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.chars().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc_entry(root: &Path, pid: &str, comm: &str, cmdline: &[u8], environ: &[u8]) {
        let d = root.join(pid);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("comm"), comm).unwrap();
        std::fs::write(d.join("cmdline"), cmdline).unwrap();
        std::fs::write(d.join("environ"), environ).unwrap();
    }

    #[test]
    fn detects_steam_and_games_from_proc() {
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        proc_entry(r, "10", "bash\n", b"bash\0", b"HOME=/h\0");
        assert!(!steam_running(r));
        assert!(!game_running(r, "200260"));
        proc_entry(
            r,
            "20",
            "BatmanAC.exe\n",
            b"Z:\\game.exe\0",
            b"SteamAppId=200260\0HOME=/h\0",
        );
        assert!(game_running(r, "200260"));
        assert!(!game_running(r, "20026"), "AppID precisa casar inteiro");
        proc_entry(
            r,
            "30",
            "steam\n",
            b"/home/u/.local/share/Steam/ubuntu12_32/steam\0-silent\0",
            b"",
        );
        assert!(steam_running(r));
        std::fs::create_dir_all(r.join("self")).unwrap();
    }

    #[test]
    fn missing_proc_is_not_running() {
        assert!(!steam_running(Path::new("/nao/existe")));
    }
}
