//! Análise **estática** e limitada dos scripts do UMU-Protonfixes.
//!
//! Os arquivos são Python, mas nunca são executados, importados ou
//! interpretados: lemos o texto, separamos instruções respeitando strings e
//! parênteses, e reconhecemos apenas chamadas conhecidas da API `util.*`
//! com argumentos literais. Tudo o que não reconhecemos é sinalizado — a
//! interface então avisa que a descrição técnica não foi verificada.

use super::models::{ActionKind, AnalysisState, Category, FixAction, GpuReq};

#[derive(Debug, Clone, Default)]
pub struct PyAnalysis {
    pub module_doc: Option<String>,
    pub func_docs: Vec<String>,
    pub actions: Vec<FixAction>,
    /// Chaves normalizadas (ex.: `verb:dotnet40`, `env:PROTON_USE_WINED3D=1`).
    pub keys: Vec<String>,
    pub urls: Vec<String>,
    pub category: Option<Category>,
    pub gpu: Option<GpuReq>,
    pub unknown: Vec<String>,
    pub state: Option<AnalysisState>,
    pub downloads: bool,
}

#[derive(Debug, Clone)]
struct Stmt {
    indent: usize,
    text: String,
}

/// Divide o código em instruções lógicas; devolve também os comentários.
fn statements(src: &str) -> (Vec<Stmt>, Vec<String>) {
    let mut out = Vec::new();
    let mut comments = Vec::new();
    let mut cur = String::new();
    let mut indent = 0usize;
    let mut at_line_start = true;
    let mut depth = 0i32;
    let mut quote: Option<(char, bool)> = None; // (aspas, tripla)
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if let Some((q, triple)) = quote {
            cur.push(c);
            if c == '\\' && i + 1 < chars.len() {
                cur.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == q {
                if triple {
                    if i + 2 < chars.len() && chars[i + 1] == q && chars[i + 2] == q {
                        cur.push(q);
                        cur.push(q);
                        i += 3;
                        quote = None;
                        continue;
                    }
                } else {
                    quote = None;
                }
            } else if c == '\n' && !triple {
                quote = None; // string de uma linha sem fechamento: desiste dela
            }
            i += 1;
            continue;
        }
        if at_line_start {
            if c == ' ' {
                indent += 1;
                i += 1;
                continue;
            }
            if c == '\t' {
                indent += 4;
                i += 1;
                continue;
            }
            at_line_start = false;
        }
        match c {
            '#' => {
                let mut comment = String::new();
                while i < chars.len() && chars[i] != '\n' {
                    comment.push(chars[i]);
                    i += 1;
                }
                comments.push(comment.trim_start_matches('#').trim().to_owned());
                continue;
            }
            '"' | '\'' => {
                let triple = i + 2 < chars.len() && chars[i + 1] == c && chars[i + 2] == c;
                quote = Some((c, triple));
                if triple {
                    cur.push(c);
                    cur.push(c);
                    cur.push(c);
                    i += 3;
                    continue;
                }
                cur.push(c);
            }
            '(' | '[' | '{' => {
                depth += 1;
                cur.push(c);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                cur.push(c);
            }
            '\\' if i + 1 < chars.len() && chars[i + 1] == '\n' => {
                i += 2;
                continue;
            }
            '\n' => {
                if depth <= 0 {
                    let t = cur.trim().to_owned();
                    if !t.is_empty() {
                        out.push(Stmt { indent, text: t });
                    }
                    cur.clear();
                    indent = 0;
                    depth = 0;
                    at_line_start = true;
                } else {
                    cur.push(' ');
                }
            }
            _ => cur.push(c),
        }
        i += 1;
    }
    let t = cur.trim().to_owned();
    if !t.is_empty() {
        out.push(Stmt { indent, text: t });
    }
    (out, comments)
}

/// Valor de um literal Python simples. `None` para expressões.
fn literal(s: &str) -> Option<String> {
    let s = s.trim();
    let body = s
        .strip_prefix('r')
        .or_else(|| s.strip_prefix('R'))
        .unwrap_or(s);
    for q in ["\"\"\"", "'''", "\"", "'"] {
        if body.len() >= 2 * q.len() && body.starts_with(q) && body.ends_with(q) {
            let inner = &body[q.len()..body.len() - q.len()];
            if inner.contains(q) {
                return None; // concatenação ou algo mais complexo
            }
            return Some(inner.replace("\\'", "'").replace("\\\"", "\""));
        }
    }
    if s == "True" || s == "False" || s == "None" || s.parse::<f64>().is_ok() {
        return Some(s.to_owned());
    }
    None
}

/// Separa argumentos no nível superior (respeitando strings e parênteses).
fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0;
    let mut quote: Option<char> = None;
    let mut prev = '\0';
    for c in s.chars() {
        if let Some(q) = quote {
            cur.push(c);
            if c == q && prev != '\\' {
                quote = None;
            }
            prev = c;
            continue;
        }
        match c {
            '\'' | '"' => {
                quote = Some(c);
                cur.push(c);
            }
            '(' | '[' | '{' => {
                depth += 1;
                cur.push(c);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                out.push(cur.trim().to_owned());
                cur.clear();
            }
            _ => cur.push(c),
        }
        prev = c;
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_owned());
    }
    out
}

/// `util.nome(args)` → (nome, args). Aceita só a chamada como instrução inteira.
fn util_call(text: &str) -> Option<(String, Vec<String>)> {
    let rest = text.strip_prefix("util.")?;
    let open = rest.find('(')?;
    let name = &rest[..open];
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') || !rest.ends_with(')') {
        return None;
    }
    Some((name.to_owned(), split_args(&rest[open + 1..rest.len() - 1])))
}

fn override_order(expr: &str) -> String {
    let e = expr.rsplit('.').next().unwrap_or(expr).trim();
    match e {
        "NATIVE" => "n".into(),
        "BUILTIN" => "b".into(),
        "NATIVE_BUILTIN" => "n,b".into(),
        "BUILTIN_NATIVE" => "b,n".into(),
        "DISABLED" => "desativada".into(),
        other => literal(other).unwrap_or_else(|| other.to_owned()),
    }
}

/// Descrição em português para os componentes de winetricks mais comuns.
pub fn verb_label(verb: &str) -> String {
    let v = verb.to_ascii_lowercase();
    let known: &[(&str, &str)] = &[
        ("dotnet", ".NET Framework"),
        ("vcrun", "Visual C++ Runtime"),
        ("d3dx9", "DirectX 9 (d3dx9)"),
        ("d3dx10", "DirectX 10 (d3dx10)"),
        ("d3dx11", "DirectX 11 (d3dx11)"),
        ("d3dcompiler", "Compilador de shaders D3D"),
        ("xact", "Áudio XACT"),
        ("xaudio", "XAudio"),
        ("faudio", "FAudio"),
        ("physx", "NVIDIA PhysX"),
        ("corefonts", "Fontes básicas da Microsoft"),
        ("mfc", "Microsoft Foundation Classes"),
        ("quartz", "DirectShow (quartz)"),
        ("wmp", "Windows Media Player"),
        ("mf", "Media Foundation"),
        ("lavfilters", "Codecs LAV Filters"),
        ("xna", "Microsoft XNA"),
        ("dsound", "DirectSound"),
        ("directmusic", "DirectMusic"),
        ("dinput", "DirectInput"),
        ("xliveless", "Remove Games for Windows Live"),
        ("gdiplus", "GDI+"),
        ("msxml", "MSXML"),
        ("vb6run", "Visual Basic 6 Runtime"),
        ("l3codecx", "Codec MP3 (l3codecx)"),
        ("devenum", "Enumerador de dispositivos DirectShow"),
    ];
    if let Some(winver) = [
        "win11", "win10", "win81", "win8", "win7", "vista", "winxp", "win2k", "win98",
    ]
    .iter()
    .find(|w| v == **w)
    {
        return format!("Define a versão do Windows do prefixo ({winver})");
    }
    known
        .iter()
        .find(|(k, _)| v.starts_with(k) && (*k != "mf" || v == "mf" || v.starts_with("mfc")))
        .map(|(_, l)| format!("Instala componente: {l}"))
        .unwrap_or_else(|| "Instala componente via winetricks".to_owned())
}

pub fn category_for_verb(verb: &str) -> Category {
    let v = verb.to_ascii_lowercase();
    if v.starts_with("physx") {
        Category::Physics
    } else if [
        "xact",
        "xaudio",
        "faudio",
        "dsound",
        "directmusic",
        "l3codecx",
        "directplay",
    ]
    .iter()
    .any(|p| v.starts_with(p))
    {
        Category::Audio
    } else if [
        "quartz",
        "wmp",
        "lavfilters",
        "devenum",
        "ffdshow",
        "cinepak",
        "xvid",
        "amstream",
    ]
    .iter()
    .any(|p| v.starts_with(p))
        || v == "mf"
    {
        Category::Video
    } else if ["d3dx", "d3dcompiler", "dxvk", "vkd3d", "d3d"]
        .iter()
        .any(|p| v.starts_with(p))
    {
        Category::Graphics
    } else if v.starts_with("dinput") || v.starts_with("xinput") {
        Category::Input
    } else {
        Category::Compatibility
    }
}

pub fn category_for_env(name: &str) -> Category {
    let n = name.to_ascii_uppercase();
    if [
        "DXVK_",
        "VKD3D_",
        "PROTON_USE_WINED3D",
        "PROTON_ENABLE_NVAPI",
        "PROTON_HIDE_NVIDIA",
        "__GL_",
        "RADV_",
        "MESA_",
        "PROTON_ENABLE_NGX",
        "PROTON_ENABLE_HDR",
        "ENABLE_HDR",
    ]
    .iter()
    .any(|p| n.starts_with(p))
    {
        Category::Graphics
    } else if ["PULSE_", "PIPEWIRE_", "SDL_AUDIO"]
        .iter()
        .any(|p| n.starts_with(p))
    {
        Category::Audio
    } else if [
        "PROTON_NO_ESYNC",
        "PROTON_NO_FSYNC",
        "PROTON_NO_NTSYNC",
        "WINE_CPU_TOPOLOGY",
        "PROTON_FORCE_LARGE_ADDRESS_AWARE",
    ]
    .iter()
    .any(|p| n.starts_with(p))
    {
        Category::Performance
    } else if n.starts_with("SDL_") {
        Category::Input
    } else {
        Category::Compatibility
    }
}

/// GPU para a qual um ajuste é específico, quando o nome deixa isso claro.
pub fn gpu_for_env(name: &str) -> Option<GpuReq> {
    let n = name.to_ascii_uppercase();
    if n.starts_with("RADV_") || n.starts_with("AMD_") || n.starts_with("ACO_") {
        Some(GpuReq::Amd)
    } else if n.starts_with("__GL_")
        || n.starts_with("__NV_")
        || n.contains("NVAPI")
        || n.contains("NVIDIA")
        || n == "PROTON_ENABLE_NGX_UPDATER"
    {
        Some(GpuReq::Nvidia)
    } else if n.starts_with("ANV_") || n.starts_with("INTEL_") {
        Some(GpuReq::Intel)
    } else {
        None
    }
}

fn first_doc(text: &str) -> Option<String> {
    literal(text)
        .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|s| !s.is_empty())
}

fn extract_urls(text: &str, out: &mut Vec<String>) {
    for word in text.split(|c: char| {
        c.is_whitespace() || c == '"' || c == '\'' || c == '<' || c == '>' || c == '(' || c == ')'
    }) {
        if word.starts_with("https://") && word.len() < 300 {
            let w = word.trim_end_matches(['.', ',', ';']).to_owned();
            if !out.contains(&w) {
                out.push(w);
            }
        }
    }
}

pub fn analyze(src: &str) -> PyAnalysis {
    let (stmts, comments) = statements(src);
    let mut a = PyAnalysis::default();
    let mut func: Option<String> = None;
    let mut body_indent: Option<usize> = None;
    let mut expecting_doc = false;
    let mut cats: Vec<Category> = Vec::new();

    for c in &comments {
        extract_urls(c, &mut a.urls);
    }

    for (idx, st) in stmts.iter().enumerate() {
        let t = st.text.as_str();
        if st.indent == 0 && !t.starts_with("def ") {
            func = None;
            body_indent = None;
        }
        if let Some(def) = t.strip_prefix("def ") {
            func = def.split('(').next().map(|n| n.trim().to_owned());
            body_indent = None;
            expecting_doc = true;
            continue;
        }
        if func.is_some() && body_indent.is_none() {
            body_indent = Some(st.indent);
        }
        let is_str = t.starts_with('"')
            || t.starts_with('\'')
            || t.starts_with("r\"")
            || t.starts_with("r'");
        if is_str {
            if let Some(doc) = first_doc(t) {
                extract_urls(&doc, &mut a.urls);
                if idx == 0 || (func.is_none() && a.module_doc.is_none() && a.actions.is_empty()) {
                    a.module_doc.get_or_insert(doc);
                } else if expecting_doc || func.is_some() {
                    a.func_docs.push(doc);
                }
            }
            continue;
        }
        expecting_doc = false;
        if t.starts_with("import ")
            || t.starts_with("from ")
            || t.starts_with("log.")
            || t.starts_with("@")
        {
            continue;
        }
        let first_word = t
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .next()
            .unwrap_or("");
        if [
            "if", "elif", "else", "for", "while", "try", "except", "finally", "with", "return",
            "pass", "break", "continue", "global",
        ]
        .contains(&first_word)
        {
            continue;
        }
        let in_entry = matches!(
            func.as_deref(),
            Some("main") | Some("early") | Some("main_with_id")
        );
        let conditional = !in_entry || body_indent.is_some_and(|b| st.indent > b);

        // os.environ['X'] = 'Y'
        if let Some(rest) = t.strip_prefix("os.environ[") {
            if let Some((key, val)) = rest.split_once("]") {
                if let (Some(k), Some(v)) = (
                    literal(key),
                    val.trim().strip_prefix('=').and_then(|v| literal(v.trim())),
                ) {
                    push_env(&mut a, &mut cats, &k, &v, conditional);
                    continue;
                }
            }
        }

        if let Some((name, args)) = util_call(t) {
            let lit = |i: usize| args.get(i).and_then(|s| literal(s));
            let raw = |i: usize| args.get(i).cloned().unwrap_or_default();
            match name.as_str() {
                "protontricks" => match lit(0) {
                    Some(verb) => {
                        cats.push(category_for_verb(&verb));
                        a.keys.push(format!("verb:{}", verb.to_ascii_lowercase()));
                        a.actions.push(FixAction {
                            kind: ActionKind::Winetricks,
                            label: verb_label(&verb),
                            value: verb,
                            conditional,
                        });
                    }
                    None => a.unknown.push(trunc(t)),
                },
                "set_environment" => match (lit(0), lit(1)) {
                    (Some(k), Some(v)) => push_env(&mut a, &mut cats, &k, &v, conditional),
                    _ => a.unknown.push(trunc(t)),
                },
                "del_environment" => match lit(0) {
                    Some(k) => a.actions.push(FixAction {
                        kind: ActionKind::Env,
                        label: "Remove variável de ambiente".into(),
                        value: k,
                        conditional,
                    }),
                    None => a.unknown.push(trunc(t)),
                },
                "winedll_override" | "wineexe_override" => match lit(0) {
                    Some(dll) => {
                        let order = override_order(&raw(1));
                        a.keys
                            .push(format!("dll:{}={}", dll.to_ascii_lowercase(), order));
                        a.actions.push(FixAction {
                            kind: ActionKind::DllOverride,
                            label: "Substituição de DLL no Wine".into(),
                            value: format!("{dll}={order}"),
                            conditional,
                        });
                    }
                    None => a.unknown.push(trunc(t)),
                },
                "append_argument" => match lit(0) {
                    Some(arg) => {
                        if arg.to_ascii_lowercase().contains("launcher") {
                            cats.push(Category::Launcher);
                        }
                        a.keys.push(format!("arg:{}", arg.trim()));
                        a.actions.push(FixAction {
                            kind: ActionKind::LaunchOption,
                            label: "Adiciona argumento de inicialização".into(),
                            value: arg,
                            conditional,
                        });
                    }
                    None => a.unknown.push(trunc(t)),
                },
                "replace_command" => {
                    cats.push(Category::Launcher);
                    let v = format!(
                        "{} → {}",
                        lit(0).unwrap_or_else(|| raw(0)),
                        lit(1).unwrap_or_else(|| raw(1))
                    );
                    a.actions.push(FixAction {
                        kind: ActionKind::LaunchOption,
                        label: "Substitui o executável ou argumento de inicialização".into(),
                        value: v,
                        conditional,
                    });
                }
                "regedit_add" => {
                    let v = args
                        .iter()
                        .take(4)
                        .map(|x| literal(x).unwrap_or_else(|| x.clone()))
                        .collect::<Vec<_>>()
                        .join(" · ");
                    a.actions.push(FixAction {
                        kind: ActionKind::Registry,
                        label: "Adiciona chave ao registro do prefixo".into(),
                        value: v,
                        conditional,
                    });
                }
                "set_ini_options" | "set_xml_options" | "set_dxvk_option"
                | "create_dosbox_conf" => {
                    if name == "set_dxvk_option" {
                        cats.push(Category::Graphics);
                    }
                    let v = args
                        .iter()
                        .filter_map(|x| literal(x))
                        .collect::<Vec<_>>()
                        .join(" · ");
                    a.actions.push(FixAction {
                        kind: ActionKind::ConfigFile,
                        label: "Altera arquivo de configuração do jogo".into(),
                        value: if v.is_empty() {
                            name.clone()
                        } else {
                            trunc(&v)
                        },
                        conditional,
                    });
                }
                "disable_nvapi" => {
                    cats.push(Category::Graphics);
                    a.gpu.get_or_insert(GpuReq::Nvidia);
                    a.keys.push("nvapi:off".into());
                    a.actions.push(FixAction {
                        kind: ActionKind::Other,
                        label: "Desativa NVAPI (recursos específicos da NVIDIA)".into(),
                        value: "disable_nvapi".into(),
                        conditional,
                    });
                }
                "patch_libcuda" => {
                    a.gpu = Some(GpuReq::Nvidia);
                    cats.push(Category::Graphics);
                    a.actions.push(FixAction {
                        kind: ActionKind::Other,
                        label: "Ajusta libcuda para CUDA/DLSS".into(),
                        value: "patch_libcuda".into(),
                        conditional,
                    });
                }
                "disable_esync" | "disable_fsync" | "disable_ntsync" => {
                    cats.push(Category::Performance);
                    let what = name.trim_start_matches("disable_");
                    a.keys.push(format!("sync:{what}:off"));
                    a.actions.push(FixAction {
                        kind: ActionKind::Other,
                        label: format!("Desativa {what}"),
                        value: name.clone(),
                        conditional,
                    });
                }
                "set_cpu_topology_limit" | "set_cpu_topology_nosmt" | "set_cpu_topology" => {
                    cats.push(Category::Performance);
                    a.actions.push(FixAction {
                        kind: ActionKind::Other,
                        label: "Limita a topologia de CPU vista pelo jogo".into(),
                        value: format!("{name}({})", args.join(", ")),
                        conditional,
                    });
                }
                "install_eac_runtime" | "install_battleye_runtime" => {
                    cats.push(Category::Network);
                    let which = if name.contains("eac") {
                        "Easy Anti-Cheat"
                    } else {
                        "BattlEye"
                    };
                    a.actions.push(FixAction {
                        kind: ActionKind::Other,
                        label: format!("Usa o runtime de {which} do Proton"),
                        value: name.clone(),
                        conditional,
                    });
                }
                "install_from_zip" | "install_all_from_tgz" => {
                    a.downloads = true;
                    a.actions.push(FixAction {
                        kind: ActionKind::Other,
                        label: "Baixa e extrai arquivos externos".into(),
                        value: lit(0).unwrap_or_else(|| raw(0)),
                        conditional,
                    });
                }
                "copy"
                | "copytree"
                | "move"
                | "rmtree"
                | "create_dos_device"
                | "import_saves_folder"
                | "set_game_drive" => {
                    a.actions.push(FixAction {
                        kind: ActionKind::ConfigFile,
                        label: "Operação de arquivos no prefixo".into(),
                        value: format!("{name}(…)"),
                        conditional,
                    });
                }
                // Consultas sem efeito colateral.
                "protonprefix"
                | "protondir"
                | "get_game_install_path"
                | "get_resolution"
                | "checkinstalled"
                | "get_steam_account_id"
                | "which"
                | "is_smt_enabled"
                | "get_cpu_count" => {}
                _ => a.unknown.push(trunc(t)),
            }
            continue;
        }

        // Atribuições locais simples (ex.: caminhos) não alteram nada sozinhas.
        if is_local_assignment(t) {
            continue;
        }
        a.unknown.push(trunc(t));
    }

    a.category = pick_category(&cats);
    a.state = Some(if a.actions.is_empty() && !a.unknown.is_empty() {
        AnalysisState::NotVerified
    } else if a.unknown.is_empty() {
        AnalysisState::Parsed
    } else {
        AnalysisState::Partial
    });
    a
}

fn push_env(a: &mut PyAnalysis, cats: &mut Vec<Category>, k: &str, v: &str, conditional: bool) {
    cats.push(category_for_env(k));
    if let Some(g) = gpu_for_env(k) {
        a.gpu.get_or_insert(g);
    }
    a.keys.push(format!("env:{}={}", k.trim(), v.trim()));
    a.actions.push(FixAction {
        kind: ActionKind::Env,
        label: "Define variável de ambiente".into(),
        value: format!("{k}={v}"),
        conditional,
    });
}

fn is_local_assignment(t: &str) -> bool {
    match t.split_once('=') {
        Some((lhs, rhs)) => {
            let lhs = lhs.trim();
            !rhs.starts_with('=')
                && !lhs.is_empty()
                && lhs.chars().all(|c| {
                    c.is_ascii_alphanumeric()
                        || c == '_'
                        || c == ':'
                        || c == ' '
                        || c == '['
                        || c == ']'
                        || c == '.'
                })
                && !lhs.starts_with("os.")
                && !rhs.contains("subprocess")
                && !rhs.contains("open(")
        }
        None => false,
    }
}

fn pick_category(cats: &[Category]) -> Option<Category> {
    // Do mais específico para o mais genérico.
    let order = [
        Category::Physics,
        Category::Audio,
        Category::Video,
        Category::Graphics,
        Category::Input,
        Category::Launcher,
        Category::Network,
        Category::Performance,
        Category::Compatibility,
    ];
    order.into_iter().find(|c| cats.contains(c))
}

fn trunc(s: &str) -> String {
    let s: String = s.chars().take(140).collect();
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzes_typical_fix_with_two_docstrings() {
        let src = r#""""Game fix for Project Torque"""
import os

from protonfixes import util


def early() -> None:
    os.environ['PROTON_DLL_COPY'] = '*'


def main() -> None:
    """Game needs dotnet35 to launch"""
    """https://bugs.winehq.org/show_bug.cgi?id=57666"""
    util.protontricks('dotnet35sp1')
    util.protontricks("dotnet40")
    util.protontricks('win10')
    util.set_environment('PROTON_USE_XALIA', '0')
"#;
        let a = analyze(src);
        assert_eq!(a.module_doc.as_deref(), Some("Game fix for Project Torque"));
        assert_eq!(a.func_docs[0], "Game needs dotnet35 to launch");
        assert!(a.urls.iter().any(|u| u.contains("winehq.org")));
        assert_eq!(a.actions.len(), 5);
        assert!(a.keys.contains(&"verb:dotnet40".to_owned()));
        assert!(a.keys.contains(&"env:PROTON_USE_XALIA=0".to_owned()));
        assert_eq!(a.state, Some(AnalysisState::Parsed));
        assert!(a.actions.iter().all(|x| !x.conditional));
        assert_eq!(a.category, Some(Category::Compatibility));
    }

    #[test]
    fn marks_conditionals_and_unknown_code_as_partial() {
        let src = r#""""Game fix for Fable III"""
import os
import shutil

from protonfixes import util


def main() -> None:
    # https://www.reddit.com/r/SteamDeck/comments/vuagy2/
    util.protontricks('xliveless')
    dirpath = os.path.join(
        util.protonprefix(),
        'drive_c',
    )
    if os.path.exists(dirpath):
        shutil.rmtree(dirpath)
        util.winedll_override('d3d9', OverrideOrder.NATIVE_BUILTIN)
"#;
        let a = analyze(src);
        assert_eq!(a.actions.len(), 2);
        assert!(!a.actions[0].conditional);
        assert!(
            a.actions[1].conditional,
            "chamada dentro do if é condicional"
        );
        assert_eq!(a.actions[1].value, "d3d9=n,b");
        assert_eq!(a.unknown.len(), 1, "{:?}", a.unknown);
        assert_eq!(a.state, Some(AnalysisState::Partial));
        assert!(a.urls[0].starts_with("https://www.reddit.com"));
    }

    #[test]
    fn detects_gpu_specific_and_physics_actions() {
        let a = analyze("from protonfixes import util\n\ndef main():\n    util.disable_nvapi()\n    util.protontricks('physx')\n");
        assert_eq!(a.gpu, Some(GpuReq::Nvidia));
        assert_eq!(a.category, Some(Category::Physics));
        let b = analyze("def main():\n    util.set_environment('RADV_PERFTEST', 'gpl')\n");
        assert_eq!(b.gpu, Some(GpuReq::Amd));
        assert_eq!(b.category, Some(Category::Graphics));
    }

    #[test]
    fn never_trusts_dynamic_arguments() {
        let a = analyze(
            "def main():\n    util.protontricks(VERB + 'x')\n    exec(open('evil').read())\n",
        );
        assert!(a.actions.is_empty());
        assert_eq!(a.unknown.len(), 2);
        assert_eq!(a.state, Some(AnalysisState::NotVerified));
    }

    #[test]
    fn handles_garbage_without_panicking() {
        for s in [
            "",
            "\"\"\"sem fim",
            "def (((",
            "util.",
            "'''",
            "\u{0}\u{1}((((\n))))",
        ] {
            let _ = analyze(s);
        }
    }

    #[test]
    fn labels_common_verbs() {
        assert_eq!(verb_label("dotnet40"), "Instala componente: .NET Framework");
        assert_eq!(
            verb_label("win10"),
            "Define a versão do Windows do prefixo (win10)"
        );
        assert_eq!(
            verb_label("mfc42"),
            "Instala componente: Microsoft Foundation Classes"
        );
        assert_eq!(verb_label("mf"), "Instala componente: Media Foundation");
        assert_eq!(verb_label("zzz"), "Instala componente via winetricks");
        assert_eq!(category_for_verb("xact_x64"), Category::Audio);
    }
}
