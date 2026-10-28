//! O que a instalação atual comprova sobre uma opção de inicialização: a
//! ferramenta Proton a reconhece? o VKD3D-Proton embutido conhece a flag? o
//! jogo traz o componente que ela atualiza, e numa versão mais antiga que a
//! oferecida pelo próprio Proton?
//!
//! Tudo aqui é leitura limitada de arquivos locais. Quando algo não pode ser
//! verificado (ferramenta sem script, pasta ausente), a resposta é `None` e
//! nada é bloqueado por suposição.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use super::dynamic::compatibility::{knowledge, Needs};
use super::models::{GameCtx, PlanStatus};

const SCRIPT_LIMIT: u64 = 2 * 1024 * 1024;
const DLL_LIMIT: u64 = 32 * 1024 * 1024;
const RSRC_LIMIT: u32 = 8 * 1024 * 1024;

/// Textos da ferramenta que declaram opções: o script `proton` e os módulos
/// do protonfixes embutido. `None` quando não é um Proton reconhecível.
struct Tool {
    scripts: Option<String>,
    /// Nomes curtos terminados em NUL na DLL: a tabela de opções do VKD3D-Proton.
    vkd3d: Option<HashSet<String>>,
}

fn read_limited(path: &Path, limit: u64) -> Option<Vec<u8>> {
    let mut f = File::open(path).ok()?;
    if f.metadata().ok()?.len() > limit {
        return None;
    }
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    Some(buf)
}

fn load_tool(root: &Path) -> Tool {
    let scripts = read_limited(&root.join("proton"), SCRIPT_LIMIT)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        // Sem `check_environment` não é o script de opções do Proton.
        .filter(|s| s.contains("check_environment("))
        .map(|mut s| {
            if let Ok(dir) = std::fs::read_dir(root.join("protonfixes")) {
                let mut extra = 0u64;
                for e in dir.flatten().take(64) {
                    let p = e.path();
                    if p.extension().is_some_and(|x| x == "py") && extra < SCRIPT_LIMIT {
                        if let Some(b) = read_limited(&p, SCRIPT_LIMIT) {
                            extra += b.len() as u64;
                            s.push_str(&String::from_utf8_lossy(&b));
                        }
                    }
                }
            }
            s
        });
    let vkd3d = [
        "files/lib/wine/vkd3d-proton/x86_64-windows",
        "files/lib/vkd3d/x86_64-windows",
        "files/lib64/vkd3d",
    ]
    .iter()
    .find_map(|d| read_limited(&root.join(d).join("d3d12core.dll"), DLL_LIMIT))
    .map(|dll| {
        dll.split(|b| *b == 0)
            .filter(|t| (3..=48).contains(&t.len()))
            .filter(|t| {
                t.iter()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_')
            })
            .map(|t| String::from_utf8_lossy(t).into_owned())
            .collect()
    });
    Tool { scripts, vkd3d }
}

/// Ferramenta lida, com o mtime do script `proton` na leitura.
type ToolCache = Mutex<HashMap<PathBuf, (Option<SystemTime>, Arc<Tool>)>>;

/// Uma leitura por ferramenta e versão (mtime do script), não por descoberta.
fn tool(root: &Path) -> Arc<Tool> {
    static CACHE: OnceLock<ToolCache> = OnceLock::new();
    let stamp = std::fs::metadata(root.join("proton"))
        .and_then(|m| m.modified())
        .ok();
    let mut map = CACHE
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some((s, t)) = map.get(root) {
        if *s == stamp {
            return t.clone();
        }
    }
    let t = Arc::new(load_tool(root));
    map.insert(root.to_owned(), (stamp, t.clone()));
    t
}

/// A ferramenta lê esta variável `PROTON_*`? `None` se não dá para saber.
pub fn tool_recognizes(root: &Path, key: &str) -> Option<bool> {
    if !key.starts_with("PROTON_") {
        return None;
    }
    let t = tool(root);
    let s = t.scripts.as_ref()?;
    Some(s.contains(&format!("\"{key}\"")) || s.contains(&format!("'{key}'")))
}

/// A flag existe na tabela de opções do VKD3D-Proton embutido?
pub fn vkd3d_recognizes(root: &Path, flag: &str) -> Option<bool> {
    let t = tool(root);
    Some(t.vkd3d.as_ref()?.contains(flag))
}

/// Primeiro arquivo com um dos nomes (sem diferenciar maiúsculas), até
/// quatro níveis de pasta e 20 000 entradas, sem seguir links.
pub fn find_game_file(install: &Path, names: &[&str]) -> Option<PathBuf> {
    fn walk(dir: &Path, depth: usize, names: &[&str], seen: &mut usize) -> Option<PathBuf> {
        let mut dirs = Vec::new();
        for e in std::fs::read_dir(dir).ok()?.flatten() {
            *seen += 1;
            if *seen > 20_000 {
                return None;
            }
            let Ok(kind) = e.file_type() else { continue };
            let name = e.file_name().to_string_lossy().to_ascii_lowercase();
            if kind.is_file() && names.contains(&name.as_str()) {
                return Some(e.path());
            }
            if kind.is_dir() && depth < 4 {
                dirs.push(e.path());
            }
        }
        dirs.into_iter()
            .find_map(|d| walk(&d, depth + 1, names, seen))
    }
    walk(install, 0, names, &mut 0)
}

/// Versão de arquivo (`VS_FIXEDFILEINFO`) lida só da seção de recursos.
pub fn pe_version(path: &Path) -> Option<[u16; 4]> {
    let mut f = File::open(path).ok()?;
    let mut head = [0u8; 4096];
    let n = f.read(&mut head).ok()?;
    let head = &head[..n];
    let u16_at = |b: &[u8], o: usize| b.get(o..o + 2).map(|x| u16::from_le_bytes([x[0], x[1]]));
    let u32_at = |b: &[u8], o: usize| {
        b.get(o..o + 4)
            .map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]))
    };
    let pe = u32_at(head, 0x3c)? as usize;
    if head.get(pe..pe + 4)? != b"PE\0\0" {
        return None;
    }
    let sections = u16_at(head, pe + 6)? as usize;
    let table = pe + 24 + u16_at(head, pe + 20)? as usize;
    for i in 0..sections.min(96) {
        let s = table + i * 40;
        if head.get(s..s + 5)? == b".rsrc" {
            let size = u32_at(head, s + 16)?.min(RSRC_LIMIT);
            let ptr = u32_at(head, s + 20)?;
            f.seek(SeekFrom::Start(ptr as u64)).ok()?;
            let mut rsrc = vec![0u8; size as usize];
            f.read_exact(&mut rsrc).ok()?;
            let at = rsrc
                .windows(4)
                .position(|w| w == [0xbd, 0x04, 0xef, 0xfe])?;
            let (ms, ls) = (u32_at(&rsrc, at + 8)?, u32_at(&rsrc, at + 12)?);
            return Some([(ms >> 16) as u16, ms as u16, (ls >> 16) as u16, ls as u16]);
        }
    }
    None
}

pub fn fmt_version(v: [u16; 4]) -> String {
    format!("{}.{}.{}.{}", v[0], v[1], v[2], v[3])
}

/// Versão mais recente que o próprio Proton baixaria, segundo o manifesto que
/// ele guarda em cache (`protonfixes/upscalers/manifest.json`).
fn manifest_latest(cache_root: &Path, entry: &str) -> Option<[u16; 4]> {
    let bytes = read_limited(
        &cache_root.join("protonfixes/upscalers/manifest.json"),
        SCRIPT_LIMIT,
    )?;
    let json: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    json.get(entry)?
        .as_array()?
        .iter()
        .filter_map(|x| x.get("version")?.as_str())
        .filter_map(|v| {
            let p: Vec<u16> = v
                .split('.')
                .map(|n| n.parse().ok())
                .collect::<Option<_>>()?;
            (p.len() == 4).then(|| [p[0], p[1], p[2], p[3]])
        })
        .max()
}

fn user_cache() -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".cache")))
}

/// Por que esta variável não deve ser aplicada **neste** ambiente. `Ok`
/// também quando algo não pôde ser verificado: o resto do plano informa.
pub fn check_launch_env(
    game: &GameCtx,
    key: &str,
    value: &str,
) -> Result<(), (PlanStatus, String)> {
    check_launch_env_in(game, key, value, user_cache().as_deref())
}

pub fn check_launch_env_in(
    game: &GameCtx,
    key: &str,
    value: &str,
    cache_root: Option<&Path>,
) -> Result<(), (PlanStatus, String)> {
    let tool = game
        .tool_name
        .as_deref()
        .unwrap_or("a ferramenta selecionada");
    if let Some(root) = game.tool_path.as_deref() {
        if tool_recognizes(root, key) == Some(false) {
            return Err((
                PlanStatus::Incompatible,
                format!("{tool} não reconhece {key}: a opção não teria efeito nesta versão."),
            ));
        }
        if key == "VKD3D_CONFIG" {
            for flag in value.split(',') {
                if vkd3d_recognizes(root, flag) == Some(false) {
                    return Err((
                        PlanStatus::Incompatible,
                        format!("O VKD3D-Proton de {tool} não conhece a opção {flag}."),
                    ));
                }
            }
        }
    }
    let Some(k) = knowledge(key, value) else {
        return Ok(());
    };
    match k.needs {
        Needs::Nothing => Ok(()),
        Needs::Wayland => match std::env::var_os("WAYLAND_DISPLAY") {
            Some(_) => Ok(()),
            None => Err((
                PlanStatus::Incompatible,
                "A sessão atual não é Wayland; o Proton ignora esta opção fora do Wayland.".into(),
            )),
        },
        Needs::GameFile(names) => match game.install_path.as_deref() {
            Some(install) if find_game_file(install, names).is_none() => Err((
                PlanStatus::Incompatible,
                format!(
                    "O jogo não traz {}; a opção não tem alvo nesta instalação.",
                    names[0]
                ),
            )),
            _ => Ok(()),
        },
        Needs::Component { file, manifest } => {
            let Some(install) = game.install_path.as_deref() else {
                return Ok(());
            };
            let Some(found) = find_game_file(install, &[file]) else {
                return Err((
                    PlanStatus::Incompatible,
                    format!("O jogo não traz {file}; não há componente para atualizar."),
                ));
            };
            let bundled = pe_version(&found);
            let latest = cache_root.and_then(|c| manifest_latest(c, manifest));
            match (bundled, latest) {
                (Some(b), Some(l)) if b >= l => Err((
                    PlanStatus::AlreadyIncluded,
                    format!("O jogo já traz {file} {}, a versão mais recente oferecida pelo Proton; não há atualização.", fmt_version(b)),
                )),
                _ => Ok(()),
            }
        }
    }
}

/// O que foi conferido quando a opção passa, para a lista de verificações.
pub fn evidence(game: &GameCtx, key: &str, value: &str) -> Option<String> {
    let tool = game.tool_name.as_deref().unwrap_or("A ferramenta");
    let recognized = game
        .tool_path
        .as_deref()
        .and_then(|root| tool_recognizes(root, key))
        .filter(|ok| *ok)
        .map(|_| format!("{tool} reconhece {key}."));
    let parts: Vec<String> = recognized
        .into_iter()
        .chain(component_note(game, key, value))
        .collect();
    (!parts.is_empty()).then(|| parts.join(" "))
}

/// O que o FRX conferiu, para a prévia e para a análise.
pub fn component_note(game: &GameCtx, key: &str, value: &str) -> Option<String> {
    let Needs::Component { file, manifest } = knowledge(key, value)?.needs else {
        return None;
    };
    let found = find_game_file(game.install_path.as_deref()?, &[file])?;
    let bundled = pe_version(&found)
        .map(fmt_version)
        .unwrap_or_else(|| "versão não identificada".into());
    Some(match user_cache().and_then(|c| manifest_latest(&c, manifest)) {
        Some(l) => format!("O jogo traz {file} {bundled}; o Proton oferece {}.", fmt_version(l)),
        None => format!("O jogo traz {file} {bundled}; o Proton confere a versão disponível na primeira execução."),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pe_with_version(v: [u16; 4]) -> Vec<u8> {
        let mut b = vec![0u8; 0x400];
        b[0x3c] = 0x80;
        b[0x80..0x84].copy_from_slice(b"PE\0\0");
        b[0x86] = 1; // uma seção
        b[0x94] = 0xf0; // tamanho do cabeçalho opcional
        let s = 0x80 + 24 + 0xf0;
        b[s..s + 5].copy_from_slice(b".rsrc");
        b[s + 16..s + 20].copy_from_slice(&64u32.to_le_bytes());
        b[s + 20..s + 24].copy_from_slice(&0x300u32.to_le_bytes());
        let r = 0x300 + 8;
        b[r..r + 4].copy_from_slice(&[0xbd, 0x04, 0xef, 0xfe]);
        b[r + 8..r + 12].copy_from_slice(&(((v[0] as u32) << 16) | v[1] as u32).to_le_bytes());
        b[r + 12..r + 16].copy_from_slice(&(((v[2] as u32) << 16) | v[3] as u32).to_le_bytes());
        b
    }

    #[test]
    fn tool_component_and_session_decide_whether_an_option_has_effect() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = tmp.path().join("tool");
        std::fs::create_dir_all(tool.join("files/lib/wine/vkd3d-proton/x86_64-windows")).unwrap();
        std::fs::write(
            tool.join("proton"),
            "self.check_environment(\"PROTON_DLSS_UPGRADE\", \"dlss\")\n",
        )
        .unwrap();
        std::fs::write(
            tool.join("files/lib/wine/vkd3d-proton/x86_64-windows/d3d12core.dll"),
            b"x\0descriptor_heap\0nodxr\0",
        )
        .unwrap();
        let game_dir = tmp.path().join("game/bin64");
        std::fs::create_dir_all(&game_dir).unwrap();
        std::fs::write(
            game_dir.join("nvngx_dlss.dll"),
            pe_with_version([310, 9, 1, 0]),
        )
        .unwrap();
        let cache = tmp.path().join("cache/protonfixes/upscalers");
        std::fs::create_dir_all(&cache).unwrap();
        let manifest = |v: &str| {
            std::fs::write(
                cache.join("manifest.json"),
                format!(r#"{{"dlss":[{{"version":"310.9.1.0"}},{{"version":"{v}"}}]}}"#),
            )
            .unwrap()
        };
        let game = GameCtx {
            tool_name: Some("proton-x".into()),
            tool_path: Some(tool.clone()),
            install_path: Some(tmp.path().join("game")),
            ..GameCtx::default()
        };
        let root = Some(tmp.path().join("cache"));
        let check = |k: &str, v: &str| check_launch_env_in(&game, k, v, root.as_deref());

        manifest("310.9.16.0");
        assert_eq!(
            pe_version(&game_dir.join("nvngx_dlss.dll")),
            Some([310, 9, 1, 0])
        );
        assert!(check("PROTON_DLSS_UPGRADE", "1").is_ok());
        // Opção que o script da ferramenta não lê: sem efeito nesta versão.
        assert_eq!(
            check("PROTON_ENABLE_HDR", "1").unwrap_err().0,
            PlanStatus::Incompatible
        );
        assert!(check("VKD3D_CONFIG", "descriptor_heap,nodxr").is_ok());
        assert!(check("VKD3D_CONFIG", "no_upload_hvv").is_err());
        // O jogo já está na versão mais recente do manifesto do Proton.
        manifest("310.9.1.0");
        assert_eq!(
            check("PROTON_DLSS_UPGRADE", "1").unwrap_err().0,
            PlanStatus::AlreadyIncluded
        );
        // Sem o componente no jogo, não há o que atualizar.
        std::fs::remove_file(game_dir.join("nvngx_dlss.dll")).unwrap();
        assert_eq!(
            check("PROTON_DLSS_UPGRADE", "1").unwrap_err().0,
            PlanStatus::Incompatible
        );
    }
}
