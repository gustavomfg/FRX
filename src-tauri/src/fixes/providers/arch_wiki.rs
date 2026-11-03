//! Arch Wiki — páginas de laptops por modelo, pela API MediaWiki pública
//! (`https://wiki.archlinux.org/api.php`).
//!
//! Fluxo (no máximo duas requisições, ambas com cache):
//! 1. `list=search` com o nome do laptop (`display_name` do DMI);
//! 2. se um título casa com o modelo, `action=parse&prop=wikitext` da página.
//!
//! Da página saem as seções cujo título bate com o sintoma e a linha da
//! tabela de hardware ("Working?"). Sempre entra também a página geral do
//! sintoma, marcada como "Geral". O texto é mostrado como citação com link,
//! **nunca executado**. Conteúdo da Arch Wiki: GFDL 1.3 ou posterior — a
//! interface mostra a atribuição; o cache local guarda a resposta da API só
//! para evitar consultas repetidas.

use reqwest::Url;
use serde::{Deserialize, Serialize};

use super::super::http::{Ctx, Outcome};
use super::super::models::{ProviderId, ProviderInfo, ProviderState, ProviderStatus};
use super::{summarize, LaptopQuery};
use crate::fix_engine::system::{valid_kernel_param, valid_module, valid_module_option, SystemAction};
use crate::net::{Fetcher, HttpRequest};

const NAME: &str = "Arch Wiki";
const API: &str = "https://wiki.archlinux.org/api.php";
const WIKI: &str = "https://wiki.archlinux.org/title/";
const ACCEPT: &str = "application/json";
/// Balde de limite de consultas da fonte.
const RESOURCE: &str = "arch-wiki";
const MAX_SECTIONS: usize = 3;
const EXCERPT_CHARS: usize = 600;

pub fn info() -> ProviderInfo {
    ProviderInfo {
        id: ProviderId::ArchWiki,
        name: NAME,
        enabled: true,
        description: "Páginas de laptops por fabricante e modelo (Wi-Fi, áudio, touchpad, suspensão, firmware). Conteúdo GFDL 1.3+.",
        reason: None,
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum HardwareSymptom {
    Wifi,
    Audio,
    Touchpad,
    Suspend,
    Firmware,
}

impl HardwareSymptom {
    /// Termos (minúsculos) procurados nos títulos de seção e na tabela de hardware.
    fn keywords(self) -> &'static [&'static str] {
        match self {
            Self::Wifi => &["wi-fi", "wifi", "wireless", "wlan", "802.11"],
            Self::Audio => &["audio", "sound", "speaker", "microphone", "headphone"],
            Self::Touchpad => &["touchpad", "trackpad", "clickpad", "trackpoint"],
            Self::Suspend => &["suspend", "sleep", "hibernat", "s0ix", "lid", "power management"],
            Self::Firmware => &["firmware", "bios", "uefi", "fwupd"],
        }
    }

    fn general_page(self) -> (&'static str, &'static str) {
        match self {
            Self::Wifi => (
                "Network configuration/Wireless",
                "Guia geral de redes sem fio: drivers, firmware e solução de problemas.",
            ),
            Self::Audio => (
                "Advanced Linux Sound Architecture/Troubleshooting",
                "Solução de problemas de áudio no ALSA: sem som, microfone, placas não detectadas.",
            ),
            Self::Touchpad => (
                "Libinput",
                "Configuração do libinput, o driver de entrada usado por touchpads modernos.",
            ),
            Self::Suspend => (
                "Power management/Suspend and hibernate",
                "Suspensão e hibernação: modos de sono, parâmetros do kernel e problemas comuns.",
            ),
            Self::Firmware => (
                "Fwupd",
                "Atualização de firmware pelo fwupd e LVFS.",
            ),
        }
    }

    fn matches(self, text: &str) -> bool {
        let t = text.to_lowercase();
        self.keywords().iter().any(|k| t.contains(k))
    }
}

/// A quem a sugestão se aplica: à página do modelo detectado ou ao caso geral.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Fit {
    Model,
    General,
}

/// Valor da coluna "Working?" na tabela de hardware da página do modelo.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Support {
    Works,
    Partial,
    Broken,
    Unknown,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WikiSuggestion {
    pub id: String,
    pub title: String,
    pub page: String,
    pub section: Option<String>,
    pub url: String,
    /// Texto simples extraído do wikitext, para citação. Nunca é executado.
    pub excerpt: Option<String>,
    pub fit: Fit,
    /// Instruções da seção que o Fix Engine sabe aplicar (sempre com prévia).
    pub actions: Vec<WikiAction>,
}

/// Instrução reconhecida no texto da wiki, convertida numa ação tipada.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WikiAction {
    /// Hash da ação: estável entre buscas, para o núcleo reencontrá-la.
    pub id: String,
    pub action: SystemAction,
    /// Trecho literal reconhecido (`options iwlwifi disable_11ax=Y`).
    pub snippet: String,
    /// Frase da wiki em volta da instrução, em texto simples.
    pub context: String,
    /// A frase impõe condições ("só em Intel", "se…"): o usuário precisa ler.
    pub conditional: bool,
    /// Checagem leve do sistema, preenchida pelo comando de busca (o provedor
    /// não lê o `/etc`). `None` = sem checagem prévia para este tipo de ação.
    pub check: Option<crate::fix_engine::system::QuickCheck>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WikiSearch {
    pub status: ProviderStatus,
    /// Página da Arch Wiki identificada como a do modelo, se houver.
    pub model_page: Option<String>,
    /// Linha da tabela de hardware do modelo correspondente ao sintoma.
    pub support: Option<Support>,
    pub suggestions: Vec<WikiSuggestion>,
}

pub async fn search<F: Fetcher>(
    ctx: &Ctx<'_, F>,
    q: &LaptopQuery,
    symptom: HardwareSymptom,
) -> WikiSearch {
    let mut outcomes: Vec<Outcome> = Vec::new();
    let mut suggestions = Vec::new();
    let mut model_page = None;
    let mut support = None;

    let identity = q.identity.as_ref();
    let name = identity.and_then(|i| i.display_name.as_deref());
    let vendor = identity.and_then(|i| i.vendor.as_deref());

    if let Some(name) = name {
        let so = ctx
            .get(
                HttpRequest::new(search_url(name, vendor), ACCEPT, 512 * 1024),
                Some(RESOURCE),
                ctx.ttl_secs,
            )
            .await;
        let titles = so.body.as_deref().map(parse_search).unwrap_or_default();
        outcomes.push(so);

        if let Some(title) = best_title(name, &titles) {
            let po = ctx
                .get(
                    HttpRequest::new(page_url(&title), ACCEPT, 2 * 1024 * 1024),
                    Some(RESOURCE),
                    ctx.ttl_secs,
                )
                .await;
            if let Some(wikitext) = po.body.as_deref().and_then(parse_wikitext) {
                support = table_support(&wikitext, symptom);
                let mut sections: Vec<_> = sections(&wikitext)
                    .into_iter()
                    .filter(|s| symptom.matches(&s.heading))
                    .map(|s| {
                        let actions = actions(&s.body);
                        (s, actions)
                    })
                    .collect();
                // Seções com instrução aplicável primeiro; a ordem da página no resto.
                sections.sort_by_key(|(_, a)| a.is_empty());
                sections.truncate(MAX_SECTIONS);
                if sections.is_empty() {
                    suggestions.push(WikiSuggestion {
                        id: format!("arch-wiki:{title}"),
                        title: format!("Página do seu modelo: {title}"),
                        page: title.clone(),
                        section: None,
                        url: article_url(&title, None),
                        excerpt: None,
                        fit: Fit::Model,
                        actions: vec![],
                    });
                }
                for (s, actions) in sections {
                    suggestions.push(WikiSuggestion {
                        id: format!("arch-wiki:{title}#{}", s.heading),
                        title: s.heading.clone(),
                        page: title.clone(),
                        url: article_url(&title, Some(&s.heading)),
                        excerpt: excerpt(&s.body),
                        section: Some(s.heading),
                        fit: Fit::Model,
                        actions,
                    });
                }
                model_page = Some(title);
            }
            outcomes.push(po);
        }
    }

    let found = suggestions.len();
    let (page, summary) = symptom.general_page();
    suggestions.push(WikiSuggestion {
        id: format!("arch-wiki:{page}"),
        title: page.to_owned(),
        page: page.to_owned(),
        section: None,
        url: article_url(page, None),
        excerpt: Some(summary.to_owned()),
        fit: Fit::General,
        actions: vec![],
    });

    let refs: Vec<&Outcome> = outcomes.iter().collect();
    let mut status = summarize(ProviderId::ArchWiki, NAME, &refs, found);
    if name.is_none() {
        status.state = ProviderState::NoData;
        status.message = Some("Modelo do laptop não identificado pelo DMI; mostrando só a página geral.".into());
    } else if model_page.is_none() && status.state == ProviderState::NoData {
        status.message = Some("Nenhuma página da Arch Wiki para este modelo; mostrando a página geral.".into());
    }
    WikiSearch {
        status,
        model_page,
        support,
        suggestions,
    }
}

fn search_url(name: &str, vendor: Option<&str>) -> String {
    // O fabricante ajuda a busca quando o nome não o contém ("XPS 13" → "Dell XPS 13").
    let query = match vendor {
        Some(v) if !name.to_lowercase().contains(&v.to_lowercase()) => format!("{v} {name}"),
        _ => name.to_owned(),
    };
    Url::parse_with_params(
        API,
        &[
            ("action", "query"),
            ("list", "search"),
            ("srsearch", query.as_str()),
            ("srnamespace", "0"),
            ("srlimit", "10"),
            ("srprop", ""),
            ("format", "json"),
            ("formatversion", "2"),
        ],
    )
    .map(|u| u.to_string())
    .unwrap_or_default()
}

fn page_url(title: &str) -> String {
    Url::parse_with_params(
        API,
        &[
            ("action", "parse"),
            ("page", title),
            ("prop", "wikitext"),
            ("redirects", "1"),
            ("format", "json"),
            ("formatversion", "2"),
        ],
    )
    .map(|u| u.to_string())
    .unwrap_or_default()
}

fn article_url(title: &str, section: Option<&str>) -> String {
    let enc = |s: &str| {
        s.replace(' ', "_")
            .chars()
            .map(|c| match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '-' | '.' | '(' | ')' | '/' | ':' => {
                    c.to_string()
                }
                _ => c
                    .to_string()
                    .bytes()
                    .map(|b| format!("%{b:02X}"))
                    .collect(),
            })
            .collect::<String>()
    };
    match section {
        Some(s) => format!("{WIKI}{}#{}", enc(title), enc(s)),
        None => format!("{WIKI}{}", enc(title)),
    }
}

fn parse_search(body: &str) -> Vec<String> {
    #[derive(Deserialize)]
    struct Hit {
        title: String,
    }
    #[derive(Deserialize)]
    struct Query {
        search: Vec<Hit>,
    }
    #[derive(Deserialize)]
    struct Resp {
        query: Query,
    }
    serde_json::from_str::<Resp>(body)
        .map(|r| r.query.search.into_iter().map(|h| h.title).collect())
        .unwrap_or_default()
}

fn parse_wikitext(body: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Parse {
        wikitext: String,
    }
    #[derive(Deserialize)]
    struct Resp {
        parse: Parse,
    }
    serde_json::from_str::<Resp>(body)
        .ok()
        .map(|r| r.parse.wikitext)
}

fn tokens(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Escolhe o título que corresponde ao modelo. Todos os tokens com dígitos
/// ("x1", "9", "9310") precisam aparecer — "Gen 9" não pode virar "Gen 10" —
/// e pelo menos 60% dos demais também. Na dúvida, nenhum: a página geral entra.
fn best_title(name: &str, titles: &[String]) -> Option<String> {
    let want = tokens(name);
    if want.is_empty() {
        return None;
    }
    titles
        .iter()
        .filter_map(|t| {
            let have = tokens(t);
            let digits_ok = want
                .iter()
                .filter(|w| w.chars().any(|c| c.is_ascii_digit()))
                .all(|w| have.contains(w));
            let hits = want.iter().filter(|w| have.contains(w)).count();
            let ratio = hits as f32 / want.len() as f32;
            (digits_ok && ratio >= 0.6).then_some((ratio, have.len(), t))
        })
        // Maior cobertura; no empate, o título mais curto (menos coisa extra).
        .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
        .map(|(_, _, t)| t.clone())
}

#[derive(Debug, PartialEq)]
struct Section {
    heading: String,
    body: String,
}

/// Divide o wikitext em seções de nível 2 e 3 (`== X ==`, `=== Y ===`).
fn sections(wikitext: &str) -> Vec<Section> {
    let mut out: Vec<Section> = Vec::new();
    for line in wikitext.lines() {
        let t = line.trim();
        let level = t.chars().take_while(|&c| c == '=').count();
        if (2..=3).contains(&level) && t.ends_with(&"=".repeat(level)) && t.len() > level * 2 {
            let heading = t[level..t.len() - level].trim().to_owned();
            out.push(Section {
                heading: plain(&heading),
                body: String::new(),
            });
        } else if let Some(s) = out.last_mut() {
            s.body.push_str(line);
            s.body.push('\n');
        }
    }
    out
}

/// Lê a coluna "Working?" da linha da tabela de hardware que bate com o sintoma.
fn table_support(wikitext: &str, symptom: HardwareSymptom) -> Option<Support> {
    let start = wikitext.find("{|")?;
    let end = start + wikitext[start..].find("|}")?;
    wikitext[start..end]
        .lines()
        .filter_map(|l| l.trim().strip_prefix('|'))
        .filter(|l| !l.starts_with('-') && !l.starts_with('}'))
        .find_map(|row| {
            let cells: Vec<&str> = row.split("||").map(str::trim).collect();
            let first = cells.first()?;
            if !symptom.matches(first) {
                return None;
            }
            let last = cells.last()?.to_lowercase();
            Some(if last.starts_with("{{yes") || last.starts_with("{{g|") {
                Support::Works
            } else if last.starts_with("{{no") || last.starts_with("{{r|") {
                Support::Broken
            } else if last.starts_with("{{y|") || last.contains("partial") || last.contains("modified") {
                Support::Partial
            } else {
                Support::Unknown
            })
        })
}

fn excerpt(body: &str) -> Option<String> {
    let mut lines = Vec::new();
    let mut table = 0usize;
    for raw in body.lines() {
        let t = raw.trim_end();
        if t.trim_start().starts_with("{|") {
            table += 1;
            continue;
        }
        if table > 0 {
            if t.trim_start().starts_with("|}") {
                table -= 1;
            }
            continue;
        }
        // Linhas que começam com espaço são blocos de código no wikitext.
        let code = t.starts_with(' ') && !t.trim().is_empty();
        let mut s = plain(t.trim());
        if s.is_empty() {
            if lines.last().is_some_and(|l: &String| !l.is_empty()) {
                lines.push(String::new());
            }
            continue;
        }
        if code {
            s = format!("  {s}");
        } else if let Some(rest) = s.strip_prefix(['*', '#']) {
            s = format!("• {}", rest.trim_start_matches(['*', '#']).trim());
        } else if let Some(rest) = s.strip_prefix(':') {
            s = rest.trim_start_matches(':').trim().to_owned();
        }
        lines.push(s);
    }
    let text = lines.join("\n").trim().to_owned();
    if text.is_empty() {
        return None;
    }
    if text.chars().count() <= EXCERPT_CHARS {
        return Some(text);
    }
    let cut: String = text.chars().take(EXCERPT_CHARS).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(a, _)| a);
    Some(format!("{}…", cut.trim_end()))
}

// ── Instruções aplicáveis ────────────────────────────────────────────────

/// Parâmetros genéricos de exibição que aparecem nos exemplos de
/// `GRUB_CMDLINE_LINUX_DEFAULT` e não são a correção em si.
const GENERIC_PARAMS: &[&str] = &["quiet", "splash", "rhgb", "nowatchdog"];
const MAX_ACTIONS: usize = 4;

/// Reconhece, numa seção, só instruções inequívocas:
/// - `options <módulo> <chave=valor>…` (bloco de código ou `{{hc|/etc/modprobe.d/…}}`);
/// - `{{ic|param=valor}}` numa frase que fala de "kernel parameter";
/// - um bloco logo depois de uma frase "…kernel parameter(s):";
/// - `GRUB_CMDLINE_LINUX_DEFAULT="…"` (sem os parâmetros genéricos).
///
/// Qualquer token fora da gramática estrita descarta a instrução inteira.
fn actions(body: &str) -> Vec<WikiAction> {
    let mut out: Vec<WikiAction> = Vec::new();
    let mut prose = String::new();
    let mut expect_params = false;
    let push = |out: &mut Vec<WikiAction>, action: SystemAction, snippet: String, context: &str| {
        if action.validate().is_err() || out.iter().any(|a| a.action == action) || out.len() >= MAX_ACTIONS {
            return;
        }
        let context = context.to_owned();
        out.push(WikiAction {
            id: action.key(),
            conditional: is_conditional(&context),
            action,
            snippet,
            context,
            check: None,
        });
    };
    let mut lines = CodeLines { in_block: false };
    for raw in body.lines() {
        let code = lines.classify(raw);
        let text = code.as_deref().unwrap_or(raw.trim());
        if text.is_empty() {
            continue;
        }

        if let Some(rest) = text.strip_prefix("options ") {
            let mut words = rest.split_whitespace();
            if let Some(module) = words.next().filter(|m| valid_module(m)) {
                let options: Vec<String> = words.map(str::to_owned).collect();
                if !options.is_empty() && options.iter().all(|o| valid_module_option(o)) {
                    let action = SystemAction::AddModprobeOption {
                        module: module.to_owned(),
                        options,
                    };
                    push(&mut out, action, text.to_owned(), &prose);
                }
            }
            continue;
        }

        if let Some(cmdline) = text
            .strip_prefix("GRUB_CMDLINE_LINUX_DEFAULT=\"")
            .and_then(|t| t.strip_suffix('"'))
        {
            let params: Vec<String> = cmdline
                .split_whitespace()
                .filter(|p| !GENERIC_PARAMS.contains(p) && !p.starts_with("loglevel="))
                .map(str::to_owned)
                .collect();
            if !params.is_empty() && params.iter().all(|p| valid_kernel_param(p)) {
                push(&mut out, SystemAction::AddKernelParameters { params }, text.to_owned(), &prose);
            }
            continue;
        }

        if code.is_some() {
            // Bloco logo depois de "…kernel parameter:" com um único parâmetro por linha.
            if expect_params && text.contains('=') && valid_kernel_param(text) {
                push(&mut out, SystemAction::AddKernelParameters { params: vec![text.to_owned()] }, text.to_owned(), &prose);
            }
            continue;
        }

        // Linha de texto: vira o contexto das próximas instruções.
        prose = plain(raw.trim());
        let mentions = raw.to_lowercase().contains("kernel param");
        expect_params = mentions && prose.trim_end().ends_with(':') && instructs(&prose);
        if mentions {
            for param in inline_code(raw) {
                // Em texto corrido, só `chave=valor` numa frase que manda aplicar:
                // `{{ic|lspci}}` ou "o parâmetro X não funciona" não são instruções.
                if param.contains('=')
                    && valid_kernel_param(&param)
                    && instructs(sentence_with(&prose, &param))
                {
                    push(&mut out, SystemAction::AddKernelParameters { params: vec![param.clone()] }, param, &prose);
                }
            }
        }
    }
    out
}

/// Separa o wikitext em linhas de código e de texto. Código é: linha que
/// começa com espaço, ou conteúdo de `{{bc|…}}`/`{{hc|arquivo|…}}`, na mesma
/// linha ou em várias, até o `}}` de fechamento.
struct CodeLines {
    in_block: bool,
}

impl CodeLines {
    /// `Some(conteúdo)` para código (pode ser vazio), `None` para texto.
    fn classify(&mut self, raw: &str) -> Option<String> {
        let t = raw.trim();
        let strip = |x: &str| x.trim_start_matches("1=").trim_start_matches("2=").trim_end_matches("}}").trim().to_owned();
        if self.in_block {
            if t.ends_with("}}") {
                self.in_block = false;
            }
            return Some(strip(t));
        }
        let opened = if let Some(rest) = t.strip_prefix("{{bc|") {
            Some(rest)
        } else {
            t.strip_prefix("{{hc|").map(|rest| rest.split_once('|').map_or("", |(_, r)| r))
        };
        if let Some(inner) = opened {
            self.in_block = !t.ends_with("}}");
            return Some(strip(inner));
        }
        (raw.starts_with(' ') && !t.is_empty()).then(|| strip(t))
    }
}

/// A frase do texto que contém `needle` (ou o texto inteiro).
fn sentence_with<'a>(text: &'a str, needle: &str) -> &'a str {
    text.split(". ").find(|s| s.contains(needle)).unwrap_or(text)
}

/// A frase manda aplicar algo, sem negação.
fn instructs(sentence: &str) -> bool {
    let s = format!(" {} ", sentence.to_lowercase());
    let verb = [" add ", " adding ", " append", " put ", " putting ", " set ", " setting ", " use ", " using ", " try ", " pass ", " passing ", " boot with "]
        .iter()
        .any(|v| s.contains(v));
    let negated = [" not ", "n't ", " never ", " avoid ", " remove ", " removing ", " instead of "]
        .iter()
        .any(|n| s.contains(n));
    verb && !negated
}

/// Argumentos de `{{ic|…}}` de uma linha (com o `1=` removido).
fn inline_code(raw: &str) -> Vec<String> {
    raw.split("{{ic|")
        .skip(1)
        .filter_map(|r| r.split_once("}}").map(|(a, _)| a))
        .map(|a| a.trim_start_matches("1=").trim().to_owned())
        .collect()
}

/// Frases com condição: o FRX não sabe avaliá-las, então avisa.
fn is_conditional(context: &str) -> bool {
    let c = format!(" {} ", context.to_lowercase());
    [
        " only ", " if ", " on intel", " on amd", " may ", " might ", " unless ",
        " some users", " workaround", " experimental", " instead of fixing",
    ]
    .iter()
    .any(|w| c.contains(w))
}

/// Converte wikitext inline em texto simples.
fn plain(s: &str) -> String {
    let s = strip_refs(s);
    let s = templates(&s);
    let s = links(&s);
    let s = strip_tags(&s);
    s.replace("'''", "").replace("''", "").trim().to_owned()
}

fn strip_refs(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("<ref") {
        out.push_str(&rest[..i]);
        let after = &rest[i..];
        if let Some(j) = after.find("/>").filter(|&j| !after[..j].contains('>')) {
            rest = &after[j + 2..];
        } else if let Some(j) = after.find("</ref>") {
            rest = &after[j + 6..];
        } else {
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

/// `{{ic|x}}`, `{{Pkg|x}}`, `{{AUR|x}}` → `x`; `{{Note|x}}` → `Nota: x`;
/// demais modelos somem. Modelos aninhados são resolvidos de dentro para fora.
fn templates(s: &str) -> String {
    let mut s = s.to_owned();
    for _ in 0..64 {
        let Some(open) = s.rfind("{{") else { break };
        let Some(len) = s[open..].find("}}") else { break };
        let inner = &s[open + 2..open + len];
        let mut parts = inner.splitn(2, '|');
        let name = parts.next().unwrap_or("").trim().to_lowercase();
        let arg = parts.next().unwrap_or("").trim();
        let arg = arg.strip_prefix("1=").unwrap_or(arg);
        let replacement = match name.as_str() {
            "ic" | "pkg" | "aur" | "grp" | "man" | "bc" | "hc" => {
                arg.rsplit('|').next().unwrap_or(arg).to_owned()
            }
            "note" => format!("Nota: {arg}"),
            "tip" => format!("Dica: {arg}"),
            "warning" => format!("Aviso: {arg}"),
            _ => String::new(),
        };
        s.replace_range(open..open + len + 2, &replacement);
    }
    s
}

/// `[[Página|texto]]` → `texto`; `[[Página]]` → `Página`; `[url texto]` → `texto`.
fn links(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('[') {
        out.push_str(&rest[..i]);
        let after = &rest[i..];
        if let Some(inner) = after.strip_prefix("[[") {
            let Some(j) = inner.find("]]") else {
                out.push_str(after);
                rest = "";
                break;
            };
            let link = &inner[..j];
            if !link.to_lowercase().starts_with("category:") {
                out.push_str(link.rsplit('|').next().unwrap_or(link));
            }
            rest = &inner[j + 2..];
        } else if let Some(j) = after.find(']') {
            let inner = &after[1..j];
            match inner.split_once(' ') {
                Some((url, text)) if url.starts_with("http") => out.push_str(text),
                _ => out.push_str(inner),
            }
            rest = &after[j + 1..];
        } else {
            out.push_str(after);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut depth = false;
    for c in s.chars() {
        match c {
            '<' => depth = true,
            '>' if depth => depth = false,
            _ if !depth => out.push(c),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;
    use crate::fixes::http::EngineState;
    use crate::net::mock::MockFetcher;
    use crate::system::LaptopIdentity;

    const PAGE: &str = "Acme Book 14 (AB14-2026)";

    const WIKITEXT: &str = r#"[[Category:Acme]]
{{Laptop style|Missing sections.}}
{| class="wikitable archwiki-table-laptop"
|-
! Hardware !! PCI/USB ID !! Working?
|-
| Wi-Fi || {{ic|8086:a0f0}} || {{Y|Needs firmware}}
|-
| Audio || {{ic|8086:a0c8}} || {{Yes}}
|}

== Wi-Fi ==

The card needs [[Firmware|firmware]] from {{Pkg|linux-firmware}}.<ref>https://example.invalid</ref>

* Disable power saving with {{ic|1=iwlwifi.power_save=0}}
 # modprobe -r iwlwifi

== Video ==

Works out of the box.
"#;

    fn query() -> LaptopQuery {
        LaptopQuery {
            identity: Some(LaptopIdentity {
                vendor: Some("Acme".into()),
                model: Some(PAGE.into()),
                board_name: Some("AB14-MB".into()),
                product_version: None,
                display_name: Some(PAGE.into()),
            }),
        }
    }

    fn search_json(titles: &[&str]) -> String {
        let hits: Vec<_> = titles
            .iter()
            .map(|t| serde_json::json!({ "ns": 0, "title": t }))
            .collect();
        serde_json::json!({ "query": { "search": hits } }).to_string()
    }

    #[test]
    fn title_matching_requires_all_numbers() {
        let titles = vec![
            "Lenovo ThinkPad X1 Carbon (Gen 10)".to_owned(),
            "Lenovo ThinkPad X1 Carbon (Gen 9)".to_owned(),
            "Laptop/Lenovo".to_owned(),
        ];
        assert_eq!(
            best_title("ThinkPad X1 Carbon Gen 9", &titles).as_deref(),
            Some("Lenovo ThinkPad X1 Carbon (Gen 9)")
        );
        assert_eq!(best_title("ThinkPad X1 Carbon Gen 11", &titles), None);
        assert_eq!(best_title("B450M Steel Legend", &titles), None);
    }

    #[test]
    fn sections_support_and_excerpt_are_plain_text() {
        let secs = sections(WIKITEXT);
        assert_eq!(
            secs.iter().map(|s| s.heading.as_str()).collect::<Vec<_>>(),
            ["Wi-Fi", "Video"]
        );
        let ex = excerpt(&secs[0].body).unwrap();
        assert!(ex.starts_with("The card needs firmware from linux-firmware."), "{ex}");
        assert!(ex.contains("• Disable power saving with iwlwifi.power_save=0"), "{ex}");
        assert!(ex.contains("  # modprobe -r iwlwifi"), "{ex}");
        assert!(!ex.contains("example.invalid") && !ex.contains("[[") && !ex.contains("{{"));
        assert_eq!(table_support(WIKITEXT, HardwareSymptom::Wifi), Some(Support::Partial));
        assert_eq!(table_support(WIKITEXT, HardwareSymptom::Audio), Some(Support::Works));
        assert_eq!(table_support(WIKITEXT, HardwareSymptom::Touchpad), None);
    }

    #[test]
    fn article_urls_are_encoded() {
        assert_eq!(
            article_url("Lenovo ThinkPad X1 Carbon (Gen 9)", Some("Wi-Fi")),
            "https://wiki.archlinux.org/title/Lenovo_ThinkPad_X1_Carbon_(Gen_9)#Wi-Fi"
        );
        assert_eq!(
            article_url("Network configuration/Wireless", None),
            "https://wiki.archlinux.org/title/Network_configuration/Wireless"
        );
        assert!(article_url("Página ç", None).ends_with("P%C3%A1gina_%C3%A7"));
    }

    #[tokio::test]
    async fn model_page_sections_and_general_page_end_to_end() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let st = EngineState::default();
        let f = MockFetcher::default();
        f.on(
            &search_url(PAGE, Some("Acme")),
            200,
            &search_json(&["Acme Book 13", PAGE, "Laptop/Acme"]),
        );
        f.on(
            &page_url(PAGE),
            200,
            &serde_json::json!({ "parse": { "title": PAGE, "wikitext": WIKITEXT } }).to_string(),
        );
        let ctx = Ctx {
            fetcher: &f,
            cache: &cache,
            state: &st,
            ttl_secs: 3600,
            force: false,
        };
        let r = search(&ctx, &query(), HardwareSymptom::Wifi).await;
        assert_eq!(r.status.state, ProviderState::Ok);
        assert_eq!(r.model_page.as_deref(), Some(PAGE));
        assert_eq!(r.support, Some(Support::Partial));
        assert_eq!(r.suggestions.len(), 2);
        assert_eq!(r.suggestions[0].fit, Fit::Model);
        assert_eq!(r.suggestions[0].section.as_deref(), Some("Wi-Fi"));
        assert_eq!(r.suggestions[1].fit, Fit::General);
        assert_eq!(r.suggestions[1].page, "Network configuration/Wireless");

        // Sem seção do sintoma: a página do modelo entra inteira, sem citação.
        let r = search(&ctx, &query(), HardwareSymptom::Suspend).await;
        assert_eq!(r.suggestions[0].section, None);
        assert_eq!(r.suggestions[0].fit, Fit::Model);
    }

    #[tokio::test]
    async fn unknown_model_or_offline_still_offers_the_general_page() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let st = EngineState::default();
        let f = MockFetcher::default();
        f.on(&search_url(PAGE, Some("Acme")), 200, &search_json(&["Unrelated page"]));
        let ctx = Ctx {
            fetcher: &f,
            cache: &cache,
            state: &st,
            ttl_secs: 3600,
            force: false,
        };
        let r = search(&ctx, &query(), HardwareSymptom::Audio).await;
        assert_eq!(r.model_page, None);
        assert_eq!(r.status.state, ProviderState::NoData);
        assert_eq!(r.suggestions.len(), 1);
        assert_eq!(r.suggestions[0].fit, Fit::General);
        assert_eq!(f.calls.lock().unwrap().len(), 1, "sem título casado, a página não é buscada");

        let no_id = LaptopQuery::default();
        let r = search(&ctx, &no_id, HardwareSymptom::Firmware).await;
        assert_eq!(r.suggestions.len(), 1);
        assert!(r.status.message.unwrap().contains("não identificado"));
    }

    /// Consulta real à Arch Wiki. `cargo test -- --ignored arch_wiki_live`
    #[tokio::test]
    #[ignore = "rede"]
    async fn arch_wiki_live() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let st = EngineState::default();
        let f = crate::net::ReqwestFetcher::new().unwrap();
        let ctx = Ctx { fetcher: &f, cache: &cache, state: &st, ttl_secs: 3600, force: true };
        let q = LaptopQuery {
            identity: Some(LaptopIdentity {
                vendor: Some("LENOVO".into()),
                model: Some("20XWCTO1WW".into()),
                board_name: Some("20XWCTO1WW".into()),
                product_version: Some("ThinkPad X1 Carbon Gen 9".into()),
                display_name: Some("ThinkPad X1 Carbon Gen 9".into()),
            }),
        };
        let r = search(&ctx, &q, HardwareSymptom::Audio).await;
        println!("{r:#?}");
        assert_eq!(r.model_page.as_deref(), Some("Lenovo ThinkPad X1 Carbon (Gen 9)"));
        assert!(r.suggestions.iter().any(|s| s.section.as_deref() == Some("Audio")));
    }

    /// Padrões reais da wiki, com texto próprio (sem copiar páginas).
    #[test]
    fn recognizes_only_unambiguous_instructions() {
        let body = r#"
Some cards reset under load. Disabling the feature is a workaround for now.

{{hc|/etc/modprobe.d/iwlwifi.conf|2=
options iwlwifi disable_11ax=Y
}}

You can make this permanent by adding the following to {{ic|/etc/modprobe.d/dell.conf}}:
 options dell-smm-hwmon ignore_dmi=1

If resume fails, try putting {{ic|1=nvme.noacpi=1}} in your [[kernel parameters]].

Add the following [[kernel parameter]]:

{{bc|1=
mem_sleep_default=deep
}}

Broken ones are ignored: {{ic|1=acpi_osi="!Windows 2020"}} as a [[kernel parameter]], and {{ic|1=init=/bin/sh}} as a kernel parameter.
 options snd-hda-intel model=$(id)
{{bc|options foo bar}}
 GRUB_CMDLINE_LINUX_DEFAULT="loglevel=3 quiet iommu=pt"
Set {{ic|1=foo=bar}} in some config file.
"#;
        let negatives = r#"
The {{ic|1=enable_psr=1}} kernel parameter appears not to work properly.
To use the kernel parameter {{ic|acpi_rev_override}}, run {{ic|lspci}} and enable {{ic|bumblebeed.service}}.
"#;
        assert!(actions(negatives).is_empty(), "{:#?}", actions(negatives));
        let a = actions(body);
        let got: Vec<_> = a.iter().map(|x| x.snippet.as_str()).collect();
        assert_eq!(
            got,
            [
                "options iwlwifi disable_11ax=Y",
                "options dell-smm-hwmon ignore_dmi=1",
                "nvme.noacpi=1",
                "mem_sleep_default=deep",
            ],
            "{a:#?}"
        );
        assert!(a[0].context.starts_with("Some cards reset under load."));
        assert!(a[0].conditional, "“workaround” pede leitura");
        assert!(a[2].conditional, "“If resume fails” é condição");
        assert_eq!(
            a[0].action,
            SystemAction::AddModprobeOption { module: "iwlwifi".into(), options: vec!["disable_11ax=Y".into()] }
        );
        assert_eq!(a[3].action, SystemAction::AddKernelParameters { params: vec!["mem_sleep_default=deep".into()] });
        // O limite de 4 por seção cortou a linha do GRUB; sozinha, ela é reconhecida sem os genéricos.
        let g = actions(" GRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3 quiet iommu=pt\"\n");
        assert_eq!(g[0].action, SystemAction::AddKernelParameters { params: vec!["iommu=pt".into()] });
        assert!(actions(" GRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3 quiet\"\n").is_empty());
        // O ID depende só da ação: estável entre buscas.
        assert_eq!(a[0].id, actions(body)[0].id);
    }

    /// `cargo test -- --ignored arch_wiki_live_actions`
    #[tokio::test]
    #[ignore = "rede"]
    async fn arch_wiki_live_actions() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let st = EngineState::default();
        let f = crate::net::ReqwestFetcher::new().unwrap();
        let ctx = Ctx { fetcher: &f, cache: &cache, state: &st, ttl_secs: 3600, force: true };
        let q = LaptopQuery {
            identity: Some(LaptopIdentity {
                vendor: Some("Framework".into()),
                model: Some("Laptop (13th Gen Intel Core)".into()),
                board_name: Some("FRANMACP04".into()),
                product_version: None,
                display_name: Some("Framework Laptop 13".into()),
            }),
        };
        let r = search(&ctx, &q, HardwareSymptom::Wifi).await;
        println!("{r:#?}");
        assert_eq!(r.model_page.as_deref(), Some("Framework Laptop 13"));
        assert!(r.suggestions.iter().flat_map(|s| &s.actions).any(|a| a.snippet == "options iwlwifi disable_11ax=Y"));
    }
}

