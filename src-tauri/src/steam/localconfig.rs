//! Edição **posicional** de `userdata/<conta>/config/localconfig.vdf`.
//!
//! Só a string `LaunchOptions` do jogo-alvo muda: o arquivo é lido com um
//! tokenizador que guarda posições e a troca é feita na fatia exata. Todo o
//! resto (outros jogos, ordem, indentação, comentários) fica byte a byte igual.
//! Quem decide *quando* escrever (Steam fechada, backup, verificação) é o
//! Fix Engine — este módulo só sabe ler e editar texto.

use std::path::{Path, PathBuf};

use super::userdata::most_recent_account;
use super::vdf;

const APPS_PATH: [&str; 5] = ["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"];

/// Arquivo da conta que entrou por último na Steam (a mesma usada pela interface).
pub fn path_for(root: &Path) -> Option<(u64, PathBuf)> {
    let account = most_recent_account(root)?;
    let p = root
        .join("userdata")
        .join(account.to_string())
        .join("config/localconfig.vdf");
    p.is_file().then_some((account, p))
}

/// Leitura estrita: um arquivo que não seja UTF-8 válido não é reescrito
/// (uma conversão com perdas corromperia o conteúdo).
pub fn read_file(path: &Path) -> Result<String, String> {
    const MAX: u64 = 32 * 1024 * 1024;
    let err = |e: std::io::Error| format!("Não foi possível ler {}: {e}", path.display());
    if std::fs::metadata(path).map_err(err)?.len() > MAX {
        return Err("localconfig.vdf é grande demais para ser editado com segurança.".into());
    }
    String::from_utf8(std::fs::read(path).map_err(err)?)
        .map_err(|_| "localconfig.vdf não está em UTF-8; o FRX não o reescreve.".into())
}

/// Valor atual de `LaunchOptions` (vazio ou ausente = `None`).
pub fn launch_options(text: &str, app_id: &str) -> Result<Option<String>, String> {
    let doc = vdf::parse(text).map_err(|e| e.to_string())?;
    let mut keys: Vec<&str> = APPS_PATH.to_vec();
    keys.push(app_id);
    Ok(doc
        .path(&keys)
        .and_then(|a| a.str_at("LaunchOptions"))
        .map(str::to_owned)
        .filter(|s| !s.trim().is_empty()))
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Str {
        start: usize,
        end: usize,
        value: String,
    },
    Open(usize),
    Close(usize),
}

fn lex(text: &str) -> Result<Vec<Tok>, String> {
    let b = text.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        match b[i] {
            c if c.is_ascii_whitespace() => i += 1,
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'{' => {
                out.push(Tok::Open(i));
                i += 1;
            }
            b'}' => {
                out.push(Tok::Close(i));
                i += 1;
            }
            b'[' => {
                while i < b.len() && b[i] != b']' && b[i] != b'\n' {
                    i += 1;
                }
                i += 1;
            }
            b'"' => {
                let start = i;
                i += 1;
                let mut v = Vec::new();
                loop {
                    match b.get(i) {
                        None => return Err("string sem fechamento".into()),
                        Some(b'"') => break,
                        Some(b'\\') => {
                            match b.get(i + 1) {
                                Some(b'"') => v.push(b'"'),
                                Some(b'\\') => v.push(b'\\'),
                                Some(b'n') => v.push(b'\n'),
                                Some(b't') => v.push(b'\t'),
                                Some(c) => {
                                    v.push(b'\\');
                                    v.push(*c);
                                }
                                None => return Err("escape incompleto".into()),
                            }
                            i += 2;
                        }
                        Some(c) => {
                            v.push(*c);
                            i += 1;
                        }
                    }
                }
                i += 1;
                out.push(Tok::Str {
                    start,
                    end: i,
                    value: String::from_utf8_lossy(&v).into_owned(),
                });
            }
            _ => {
                let start = i;
                while i < b.len()
                    && !b[i].is_ascii_whitespace()
                    && !matches!(b[i], b'{' | b'}' | b'"')
                {
                    i += 1;
                }
                out.push(Tok::Str {
                    start,
                    end: i,
                    value: text[start..i].to_owned(),
                });
            }
        }
    }
    Ok(out)
}

/// Bloco encontrado: posição da chave, do `{` e do `}` correspondente (índices de token).
struct Block {
    open: usize,
    close: usize,
}

/// Procura `key { … }` entre os tokens `from..to` (filhos diretos), sem diferenciar maiúsculas.
fn find_child(toks: &[Tok], from: usize, to: usize, key: &str) -> Option<Block> {
    let mut i = from;
    while i < to {
        let Tok::Str { value, .. } = &toks[i] else {
            return None;
        };
        match toks.get(i + 1) {
            Some(Tok::Open(_)) => {
                let close = matching(toks, i + 1)?;
                if value.eq_ignore_ascii_case(key) {
                    return Some(Block { open: i + 1, close });
                }
                i = close + 1;
            }
            Some(Tok::Str { .. }) => i += 2,
            _ => return None,
        }
    }
    None
}

fn matching(toks: &[Tok], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (j, t) in toks.iter().enumerate().skip(open) {
        match t {
            Tok::Open(_) => depth += 1,
            Tok::Close(_) => {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
            }
            _ => {}
        }
    }
    None
}

pub fn escape(value: &str) -> Result<String, String> {
    if value.chars().any(|c| c.is_control() && c != '\t') {
        return Err("O valor tem quebras de linha ou caracteres de controle.".into());
    }
    Ok(format!(
        "\"{}\"",
        value.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}

fn indent_before(text: &str, pos: usize) -> String {
    let line_start = text[..pos].rfind('\n').map_or(0, |n| n + 1);
    text[line_start..pos]
        .chars()
        .take_while(|c| *c == '\t' || *c == ' ')
        .collect()
}

/// Troca (ou cria) `LaunchOptions` do jogo. Devolve o texto novo.
pub fn set_launch_options(text: &str, app_id: &str, value: &str) -> Result<String, String> {
    let quoted = escape(value)?;
    let toks = lex(text).map_err(|e| format!("localconfig.vdf ilegível: {e}"))?;
    let mut from = 0;
    let mut to = toks.len();
    let mut apps: Option<Block> = None;
    for (n, key) in APPS_PATH.iter().enumerate() {
        let b = find_child(&toks, from, to, key)
            .ok_or_else(|| format!("Estrutura inesperada no localconfig.vdf (sem “{key}”)."))?;
        from = b.open + 1;
        to = b.close;
        if n == APPS_PATH.len() - 1 {
            apps = Some(b);
        }
    }
    let apps = apps.expect("caminho percorrido");
    let pos = |t: &Tok| match t {
        Tok::Str { start, .. } => *start,
        Tok::Open(p) | Tok::Close(p) => *p,
    };
    let mut out = text.to_owned();
    match find_child(&toks, apps.open + 1, apps.close, app_id) {
        Some(app) => {
            // Procura LaunchOptions entre os filhos diretos do jogo.
            let mut i = app.open + 1;
            while i < app.close {
                match (&toks[i], toks.get(i + 1)) {
                    (Tok::Str { value: k, .. }, Some(Tok::Str { start, end, .. }))
                        if k.eq_ignore_ascii_case("LaunchOptions") =>
                    {
                        out.replace_range(*start..*end, &quoted);
                        return Ok(out);
                    }
                    (Tok::Str { .. }, Some(Tok::Open(_))) => {
                        i = matching(&toks, i + 1).ok_or("bloco sem fechamento")? + 1
                    }
                    (Tok::Str { .. }, Some(Tok::Str { .. })) => i += 2,
                    _ => return Err("Estrutura inesperada no bloco do jogo.".into()),
                }
            }
            let close = pos(&toks[app.close]);
            let ind = indent_before(text, close);
            out.insert_str(close, &format!("\t\"LaunchOptions\"\t\t{quoted}\n{ind}"));
        }
        None => {
            let close = pos(&toks[apps.close]);
            let ind = indent_before(text, close);
            out.insert_str(close, &format!("\t\"{app_id}\"\n{ind}\t{{\n{ind}\t\t\"LaunchOptions\"\t\t{quoted}\n{ind}\t}}\n{ind}"));
        }
    }
    Ok(out)
}

#[cfg(test)]
pub mod fixtures {
    /// Trecho no formato real do Steam (tabs, chaves em maiúsculas variadas).
    pub const LOCALCONFIG: &str = "\"UserLocalConfigStore\"\n{\n\t\"Software\"\n\t{\n\t\t\"Valve\"\n\t\t{\n\t\t\t\"Steam\"\n\t\t\t{\n\t\t\t\t\"apps\"\n\t\t\t\t{\n\t\t\t\t\t\"200260\"\n\t\t\t\t\t{\n\t\t\t\t\t\t\"LastPlayed\"\t\t\"1792792536\"\n\t\t\t\t\t\t\"Playtime\"\t\t\"46\"\n\t\t\t\t\t\t\"LaunchOptions\"\t\t\"PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command%\"\n\t\t\t\t\t\t\"cloud\"\n\t\t\t\t\t\t{\n\t\t\t\t\t\t\t\"last_sync_state\"\t\t\"synchronized\"\n\t\t\t\t\t\t}\n\t\t\t\t\t}\n\t\t\t\t\t\"620\"\n\t\t\t\t\t{\n\t\t\t\t\t\t\"Playtime\"\t\t\"10\"\n\t\t\t\t\t}\n\t\t\t\t}\n\t\t\t}\n\t\t}\n\t}\n\t\"friends\"\n\t{\n\t\t\"x\"\t\t\"y\"\n\t}\n}\n";
}

#[cfg(test)]
mod tests {
    use super::fixtures::LOCALCONFIG;
    use super::*;

    #[test]
    fn reads_current_value() {
        assert_eq!(
            launch_options(LOCALCONFIG, "200260").unwrap().as_deref(),
            Some("PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command%")
        );
        assert_eq!(launch_options(LOCALCONFIG, "620").unwrap(), None);
        assert_eq!(launch_options(LOCALCONFIG, "999").unwrap(), None);
    }

    #[test]
    fn replaces_only_the_target_value() {
        let new = set_launch_options(
            LOCALCONFIG,
            "200260",
            "PROTON_LOG=1 PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command%",
        )
        .unwrap();
        assert_eq!(
            launch_options(&new, "200260").unwrap().as_deref(),
            Some("PROTON_LOG=1 PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command%")
        );
        // Só a fatia do valor mudou.
        let diff_start = LOCALCONFIG
            .bytes()
            .zip(new.bytes())
            .position(|(a, b)| a != b)
            .unwrap();
        let value_start = LOCALCONFIG.find("\"LaunchOptions\"\t\t\"").unwrap() + 18;
        assert!(diff_start >= value_start);
        assert!(new.ends_with(&LOCALCONFIG[LOCALCONFIG.find("\n\t\t\t\t\t\t\"cloud\"").unwrap()..]));
        // Ida e volta restaura o arquivo byte a byte.
        let back = set_launch_options(
            &new,
            "200260",
            "PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command%",
        )
        .unwrap();
        assert_eq!(back, LOCALCONFIG);
    }

    #[test]
    fn inserts_key_or_block_when_missing() {
        let a = set_launch_options(LOCALCONFIG, "620", "PROTON_LOG=1 %command%").unwrap();
        assert_eq!(
            launch_options(&a, "620").unwrap().as_deref(),
            Some("PROTON_LOG=1 %command%")
        );
        assert_eq!(
            launch_options(&a, "200260").unwrap().as_deref(),
            Some("PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command%")
        );
        let b = set_launch_options(LOCALCONFIG, "1174180", "%command% -vulkan").unwrap();
        assert_eq!(
            launch_options(&b, "1174180").unwrap().as_deref(),
            Some("%command% -vulkan")
        );
        assert!(vdf::parse(&b).is_ok());
    }

    #[test]
    fn escapes_quotes_and_refuses_control_chars() {
        let v = "PROTON_LOG=1 %command% -arg \"com aspas\" C:\\\\x";
        let a = set_launch_options(LOCALCONFIG, "200260", v).unwrap();
        assert_eq!(launch_options(&a, "200260").unwrap().as_deref(), Some(v));
        assert!(set_launch_options(LOCALCONFIG, "200260", "a\nb").is_err());
    }

    #[test]
    fn refuses_unknown_structure() {
        assert!(set_launch_options("\"Outro\" { }", "1", "x").is_err());
        assert!(set_launch_options("\"UserLocalConfigStore\" { \"Software\" {", "1", "x").is_err());
    }
}
