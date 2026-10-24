//! GitHub (API REST pública).
//!
//! - Busca issues com o **AppID no título** em repositórios de compatibilidade
//!   conhecidos — resultados genéricos por nome não são usados como evidência;
//! - lê os comentários mais recentes e extrai apenas ajustes reconhecidos;
//! - comentários de mantenedores dizendo que algo foi corrigido e notas de
//!   versão oficiais que citam o jogo viram descobertas "verificadas".

use reqwest::Url;
use serde::Deserialize;

use super::super::aggregation::{finding_id, group_mentions, Mention};
use super::super::http::{Ctx, Outcome};
use super::super::models::*;
use super::super::tweaks::scan_text;
use super::{summarize, GameQuery, ProviderResult};
use crate::net::{Fetcher, HttpRequest};

const REPOS: &[&str] = &[
    "ValveSoftware/Proton",
    "Open-Wine-Components/umu-protonfixes",
    "GloriousEggroll/proton-ge-custom",
];
const RELEASE_REPOS: &[&str] = &["ValveSoftware/Proton", "GloriousEggroll/proton-ge-custom"];
const ACCEPT: &str = "application/vnd.github+json";
const MAX_ISSUES: usize = 3;
const MAX_COMMENT_PAGES: usize = 2;

#[derive(Deserialize)]
struct SearchResp {
    #[serde(default)]
    items: Vec<Issue>,
}

#[derive(Deserialize, Clone)]
struct Label {
    name: String,
}

#[derive(Deserialize, Clone)]
struct Issue {
    number: u64,
    title: String,
    html_url: String,
    state: String,
    #[serde(default)]
    comments: u64,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    labels: Vec<Label>,
    created_at: Option<String>,
    repository_url: String,
    #[serde(default)]
    pull_request: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct Comment {
    html_url: String,
    #[serde(default)]
    body: Option<String>,
    created_at: Option<String>,
    #[serde(default)]
    author_association: Option<String>,
}

#[derive(Deserialize)]
struct Release {
    #[serde(default)]
    name: Option<String>,
    tag_name: String,
    html_url: String,
    #[serde(default)]
    body: Option<String>,
    published_at: Option<String>,
}

fn has_token(hay: &str, digits: &str) -> bool {
    hay.match_indices(digits).any(|(i, _)| {
        !hay[..i].chars().last().is_some_and(|c| c.is_ascii_digit())
            && !hay[i + digits.len()..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
    })
}

/// Só URLs do próprio GitHub entram nas fontes (o restante é dado não confiável).
fn safe_github_url(u: &str) -> bool {
    Url::parse(u).is_ok_and(|x| x.scheme() == "https" && x.host_str() == Some("github.com"))
}

fn repo_of(repository_url: &str) -> String {
    repository_url
        .trim_start_matches("https://api.github.com/repos/")
        .to_owned()
}

/// Nome normalizado para comparar com notas de versão.
pub fn normalize_name(s: &str) -> String {
    let lower = s.to_lowercase().replace(['™', '®', '©'], "");
    let cleaned: String = lower
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Variantes do nome sem sufixos de edição ("GOTY", "Game of the Year Edition"…).
pub fn name_variants(name: &str) -> Vec<String> {
    let base = normalize_name(name);
    let mut out = vec![base.clone()];
    for suffix in [
        " goty",
        " game of the year edition",
        " game of the year",
        " definitive edition",
        " complete edition",
        " enhanced edition",
        " remastered",
    ] {
        if let Some(v) = base.strip_suffix(suffix) {
            out.push(v.trim().to_owned());
        }
    }
    out.retain(|v| v.len() >= 6);
    out.dedup();
    out
}

fn line_is_name(line: &str, variant: &str) -> bool {
    let n = normalize_name(line);
    n == variant
}

const FIXED_PHRASES: &[&str] = &[
    "should be fixed",
    "is fixed",
    "has been fixed",
    "fixed in",
    "now fixed",
    "should now work",
    "should work now",
    "fix is in",
    "is now playable",
];

pub async fn search<F: Fetcher>(ctx: &Ctx<'_, F>, q: &GameQuery) -> ProviderResult {
    let mut outcomes: Vec<Outcome> = Vec::new();
    let mut findings: Vec<FixFinding> = Vec::new();
    let mut mentions: Vec<Mention> = Vec::new();

    // 1. Issues com o AppID no título.
    let query = format!(
        "{} in:title {}",
        q.app_id,
        REPOS
            .iter()
            .map(|r| format!("repo:{r}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let url = Url::parse_with_params(
        "https://api.github.com/search/issues",
        &[
            ("q", query.as_str()),
            ("per_page", "10"),
            ("sort", "comments"),
            ("order", "desc"),
        ],
    )
    .map(|u| u.to_string())
    .unwrap_or_default();
    let so = ctx
        .get(
            HttpRequest::new(url, ACCEPT, 2 * 1024 * 1024),
            Some("search"),
            ctx.ttl_secs,
        )
        .await;
    let issues: Vec<Issue> = so
        .body
        .as_deref()
        .and_then(|b| serde_json::from_str::<SearchResp>(b).ok())
        .map(|r| r.items)
        .unwrap_or_default()
        .into_iter()
        .filter(|i| {
            i.pull_request.is_none()
                && has_token(&i.title, &q.app_id)
                && safe_github_url(&i.html_url)
        })
        .take(MAX_ISSUES)
        .collect();
    outcomes.push(so);

    for issue in &issues {
        let repo = repo_of(&issue.repository_url);
        let reference = format!("{repo}#{}", issue.number);
        let labels: Vec<String> = issue
            .labels
            .iter()
            .map(|l| l.name.clone())
            .take(4)
            .collect();
        let state = if issue.state == "closed" {
            "fechado"
        } else {
            "aberto"
        };
        findings.push(FixFinding {
            id: finding_id("gh-issue", &q.app_id, &reference),
            game_app_id: q.app_id.clone(),
            title: format!("Relato de compatibilidade · {reference}"),
            description: format!(
                "“{}” — {state}, {} comentário{}{}. Relatos descrevem problemas e soluções testadas por outros jogadores; leia antes de seguir qualquer passo.",
                issue.title.chars().take(160).collect::<String>(),
                issue.comments,
                if issue.comments == 1 { "" } else { "s" },
                if labels.is_empty() { String::new() } else { format!(", rótulos: {}", labels.join(", ")) }
            ),
            category: Category::Compatibility,
            kind: FindingKind::Report,
            actions: vec![],
            sources: vec![FindingSource {
                provider: ProviderId::Github,
                label: format!("{reference} · {}", issue.title.chars().take(80).collect::<String>()),
                url: issue.html_url.clone(),
                reference_id: Some(reference.clone()),
                snippet: None,
                published_at: issue.created_at.clone(),
            }],
            requirements: Requirements::default(),
            confidence: Confidence::CommunityReported,
            compatibility: Compatibility::Unknown,
            compatibility_note: "Relato geral sobre o jogo, sem ajuste específico.".into(),
            experimental: false,
            analysis: AnalysisState::Parsed,
            evidence: Evidence::default(),
            published_at: issue.created_at.clone(),
            retrieved_at: q.retrieved_at.clone(),
            keys: vec![],
        });
        if let Some(body) = &issue.body {
            collect(
                &mut mentions,
                body,
                &issue.html_url,
                &format!("{reference} · descrição"),
                issue.created_at.clone(),
            );
        }
    }

    // 2. Comentários mais recentes das issues mais discutidas.
    let mut by_comments = issues.clone();
    by_comments.sort_by_key(|i| std::cmp::Reverse(i.comments));
    for issue in by_comments.iter().take(2).filter(|i| i.comments > 0) {
        let repo = repo_of(&issue.repository_url);
        let last_page = issue.comments.div_ceil(100).max(1) as usize;
        for page in (1..=last_page).rev().take(MAX_COMMENT_PAGES) {
            let url = format!(
                "https://api.github.com/repos/{repo}/issues/{}/comments?per_page=100&page={page}",
                issue.number
            );
            let o = ctx
                .get(
                    HttpRequest::new(url, ACCEPT, 4 * 1024 * 1024),
                    Some("core"),
                    ctx.ttl_secs,
                )
                .await;
            let comments: Vec<Comment> = o
                .body
                .as_deref()
                .and_then(|b| serde_json::from_str(b).ok())
                .unwrap_or_default();
            outcomes.push(o);
            for c in comments.iter().filter(|c| safe_github_url(&c.html_url)) {
                let Some(body) = &c.body else { continue };
                let label = format!("Comentário em {repo}#{}", issue.number);
                collect(
                    &mut mentions,
                    body,
                    &c.html_url,
                    &label,
                    c.created_at.clone(),
                );
                if let Some(f) = maintainer_statement(q, &repo, issue.number, c, body) {
                    findings.push(f);
                }
            }
        }
    }

    // 3. Notas de versão oficiais que citam o jogo.
    for repo in RELEASE_REPOS {
        let url = format!("https://api.github.com/repos/{repo}/releases?per_page=30");
        let o = ctx
            .get(
                HttpRequest::new(url, ACCEPT, 4 * 1024 * 1024),
                Some("core"),
                ctx.ttl_secs.max(12 * 3600),
            )
            .await;
        let releases: Vec<Release> = o
            .body
            .as_deref()
            .and_then(|b| serde_json::from_str(b).ok())
            .unwrap_or_default();
        outcomes.push(o);
        findings.extend(release_mentions(q, repo, &releases));
    }

    findings.extend(group_mentions(&q.app_id, mentions, &q.retrieved_at));
    let refs: Vec<&Outcome> = outcomes.iter().collect();
    let found = findings.len();
    ProviderResult {
        status: summarize(ProviderId::Github, "GitHub", &refs, found),
        findings,
        identity: None,
    }
}

fn collect(out: &mut Vec<Mention>, text: &str, url: &str, label: &str, date: Option<String>) {
    let s = scan_text(text);
    for (tweak, line) in s.tweaks {
        out.push(Mention {
            tweak,
            source: FindingSource {
                provider: ProviderId::Github,
                label: label.to_owned(),
                url: url.to_owned(),
                reference_id: None,
                snippet: Some(line),
                published_at: date.clone(),
            },
            positive: s.positive,
            negative: s.negative,
            gpus: s.gpus.clone(),
            proton_versions: s.proton_versions.clone(),
        });
    }
}

fn maintainer_statement(
    q: &GameQuery,
    repo: &str,
    number: u64,
    c: &Comment,
    body: &str,
) -> Option<FixFinding> {
    let assoc = c.author_association.as_deref().unwrap_or("");
    if !matches!(assoc, "MEMBER" | "OWNER" | "COLLABORATOR") {
        return None;
    }
    let line = body.lines().find(|l| {
        let low = l.to_lowercase();
        FIXED_PHRASES.iter().any(|p| low.contains(p))
    })?;
    // Só a versão citada na própria frase da correção conta.
    let versions = scan_text(line).proton_versions;
    let snippet: String = line.chars().filter(|c| !c.is_control()).take(240).collect();
    let title = match versions.first() {
        Some(v) => format!("Mantenedor indica correção em {v}"),
        None => "Mantenedor indica que o problema foi corrigido".into(),
    };
    Some(FixFinding {
        id: finding_id("gh-maint", &q.app_id, &c.html_url),
        game_app_id: q.app_id.clone(),
        title,
        description: format!(
            "Comentário de um mantenedor de {repo} na issue #{number} afirma que o problema foi corrigido. Use a versão citada (ou mais nova) da ferramenta e confira se o problema sumiu no seu sistema."
        ),
        category: Category::Compatibility,
        kind: FindingKind::ReleaseNote,
        actions: versions.iter().take(2).map(|v| FixAction { kind: ActionKind::ProtonVersion, label: "Versão citada pelo mantenedor".into(), value: v.clone(), conditional: false }).collect(),
        sources: vec![FindingSource { provider: ProviderId::Github, label: format!("Mantenedor em {repo}#{number}"), url: c.html_url.clone(), reference_id: None, snippet: Some(snippet), published_at: c.created_at.clone() }],
        requirements: Requirements { gpu_vendor: None, proton_versions: versions, game_version: None },
        confidence: Confidence::Verified,
        compatibility: Compatibility::Unknown,
        compatibility_note: String::new(),
        experimental: false,
        analysis: AnalysisState::Parsed,
        evidence: Evidence { positive: 1, negative: 0 },
        published_at: c.created_at.clone(),
        retrieved_at: q.retrieved_at.clone(),
        keys: vec![],
    })
}

fn release_mentions(q: &GameQuery, repo: &str, releases: &[Release]) -> Vec<FixFinding> {
    let variants = name_variants(&q.name);
    let mut out = Vec::new();
    for r in releases.iter().filter(|r| safe_github_url(&r.html_url)) {
        let Some(body) = &r.body else { continue };
        // Nome: a linha inteira precisa ser o nome (listas "Now playable:"), para
        // não confundir "Arkham City" com "Arkham City Lockdown".
        let hit = body
            .lines()
            .find(|l| has_token(l, &q.app_id) || variants.iter().any(|v| line_is_name(l, v)));
        let Some(line) = hit else { continue };
        let release = r
            .name
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| r.tag_name.clone());
        let snippet: String = line
            .trim()
            .trim_start_matches(['-', '*', ' '])
            .chars()
            .filter(|c| !c.is_control())
            .take(240)
            .collect();
        out.push(FixFinding {
            id: finding_id("gh-release", &q.app_id, &r.html_url),
            game_app_id: q.app_id.clone(),
            title: format!("Citado nas notas de versão de {release}"),
            description: format!(
                "As notas oficiais de {repo} ({}) mencionam este jogo. Normalmente indica correção ou suporte novo nessa versão — confira o trecho e use essa versão ou uma mais nova.",
                r.tag_name
            ),
            category: Category::Compatibility,
            kind: FindingKind::ReleaseNote,
            actions: vec![FixAction { kind: ActionKind::ProtonVersion, label: "Versão da ferramenta citada".into(), value: r.tag_name.clone(), conditional: false }],
            sources: vec![FindingSource { provider: ProviderId::Github, label: format!("{repo} · {release}"), url: r.html_url.clone(), reference_id: Some(r.tag_name.clone()), snippet: Some(snippet), published_at: r.published_at.clone() }],
            requirements: Requirements { gpu_vendor: None, proton_versions: vec![r.tag_name.clone()], game_version: None },
            confidence: Confidence::Verified,
            compatibility: Compatibility::Unknown,
            compatibility_note: String::new(),
            experimental: false,
            analysis: AnalysisState::Parsed,
            evidence: Evidence::default(),
            published_at: r.published_at.clone(),
            retrieved_at: q.retrieved_at.clone(),
            keys: vec![],
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;
    use crate::fixes::http::EngineState;
    use crate::net::mock::MockFetcher;
    use crate::net::{HttpResponse, NetError};

    fn q() -> GameQuery {
        GameQuery {
            app_id: "200260".into(),
            name: "Batman: Arkham City GOTY".into(),
            tool: None,
            retrieved_at: "2026-10-24T00:00:00Z".into(),
        }
    }

    fn search_url() -> String {
        let query = format!(
            "200260 in:title {}",
            REPOS
                .iter()
                .map(|r| format!("repo:{r}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
        Url::parse_with_params(
            "https://api.github.com/search/issues",
            &[
                ("q", query.as_str()),
                ("per_page", "10"),
                ("sort", "comments"),
                ("order", "desc"),
            ],
        )
        .unwrap()
        .to_string()
    }

    const SEARCH: &str = r#"{"total_count":2,"items":[
        {"number":309,"title":"Batman: Arkham City GOTY (200260)","html_url":"https://github.com/ValveSoftware/Proton/issues/309","state":"open","comments":2,
         "body":"Crashes at start. PROTON_USE_WINED3D=1 works for me on GTX 1060","labels":[{"name":".NET"}],"created_at":"2018-08-23T18:41:25Z",
         "repository_url":"https://api.github.com/repos/ValveSoftware/Proton"},
        {"number":1,"title":"Something about 2002606","html_url":"https://github.com/x/y/issues/1","state":"open","comments":0,"repository_url":"https://api.github.com/repos/x/y"},
        {"number":2,"title":"PR 200260","html_url":"https://github.com/x/y/pull/2","state":"open","comments":0,"repository_url":"https://api.github.com/repos/x/y","pull_request":{}}
    ]}"#;

    const COMMENTS: &str = r#"[
        {"html_url":"https://github.com/ValveSoftware/Proton/issues/309#issuecomment-1","body":"Confirmed: PROTON_USE_WINED3D=1 %command% fixed it on my RTX 3060","created_at":"2024-01-01T00:00:00Z","author_association":"NONE"},
        {"html_url":"https://github.com/ValveSoftware/Proton/issues/309#issuecomment-2","body":"This should be fixed in Proton 9.0-3, please retest.","created_at":"2024-05-01T00:00:00Z","author_association":"COLLABORATOR"}
    ]"#;

    const RELEASES: &str = r#"[{"name":"Proton 9.0-3","tag_name":"proton-9.0-3","html_url":"https://github.com/ValveSoftware/Proton/releases/tag/proton-9.0-3",
        "body":"Now playable:\n- Batman: Arkham City\n- Other Game","published_at":"2024-06-01T00:00:00Z"}]"#;

    #[tokio::test]
    async fn end_to_end_with_mocks() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let st = EngineState::default();
        let f = MockFetcher::default();
        f.on(&search_url(), 200, SEARCH);
        f.on("https://api.github.com/repos/ValveSoftware/Proton/issues/309/comments?per_page=100&page=1", 200, COMMENTS);
        f.on(
            "https://api.github.com/repos/ValveSoftware/Proton/releases?per_page=30",
            200,
            RELEASES,
        );
        f.on(
            "https://api.github.com/repos/GloriousEggroll/proton-ge-custom/releases?per_page=30",
            200,
            "[]",
        );
        let ctx = Ctx {
            fetcher: &f,
            cache: &cache,
            state: &st,
            ttl_secs: 3600,
            force: false,
        };
        let r = search(&ctx, &q()).await;

        let reports: Vec<_> = r
            .findings
            .iter()
            .filter(|f| f.kind == FindingKind::Report)
            .collect();
        assert_eq!(
            reports.len(),
            1,
            "AppID precisa casar como número inteiro e PRs ficam fora"
        );
        assert!(
            r.findings
                .iter()
                .any(|f| f.confidence == Confidence::Verified && f.title.contains("Proton 9.0-3")),
            "comentário de mantenedor"
        );
        assert!(
            r.findings
                .iter()
                .any(|f| f.title.contains("notas de versão")),
            "nota de versão por nome sem GOTY"
        );
        let wined3d = r
            .findings
            .iter()
            .find(|f| f.title.contains("PROTON_USE_WINED3D=1"))
            .unwrap();
        assert_eq!(wined3d.kind, FindingKind::Tweak);
        assert_eq!(
            wined3d.requirements.gpu_vendor, None,
            "um relato só não define requisito de GPU"
        );
        assert_eq!(r.status.state, ProviderState::Ok);
    }

    #[tokio::test]
    async fn rate_limited_search_is_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let st = EngineState::default();
        let f = MockFetcher::default();
        f.on_response(
            &search_url(),
            HttpResponse {
                status: 403,
                rate_remaining: Some(0),
                rate_reset: Some(crate::time::now_unix() + 300),
                ..Default::default()
            },
        );
        f.fail(
            "https://api.github.com/repos/ValveSoftware/Proton/releases?per_page=30",
            NetError::Timeout,
        );
        f.fail(
            "https://api.github.com/repos/GloriousEggroll/proton-ge-custom/releases?per_page=30",
            NetError::Timeout,
        );
        let ctx = Ctx {
            fetcher: &f,
            cache: &cache,
            state: &st,
            ttl_secs: 3600,
            force: false,
        };
        let r = search(&ctx, &q()).await;
        assert!(r.findings.is_empty());
        assert_eq!(r.status.state, ProviderState::RateLimited);
        assert!(r.status.reset_at.is_some());
    }

    #[tokio::test]
    async fn malformed_json_does_not_break_anything() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let st = EngineState::default();
        let f = MockFetcher::default();
        f.on(&search_url(), 200, "{ isto não é json");
        f.on(
            "https://api.github.com/repos/ValveSoftware/Proton/releases?per_page=30",
            200,
            "[{\"lixo\":1}]",
        );
        let ctx = Ctx {
            fetcher: &f,
            cache: &cache,
            state: &st,
            ttl_secs: 3600,
            force: false,
        };
        let r = search(&ctx, &q()).await;
        assert!(r.findings.is_empty());
    }

    #[test]
    fn name_matching_is_word_based() {
        assert_eq!(
            name_variants("Batman: Arkham City GOTY"),
            vec!["batman arkham city goty", "batman arkham city"]
        );
        assert!(line_is_name("- Batman™: Arkham City", "batman arkham city"));
        assert!(!line_is_name(
            "- Batman: Arkham City Lockdown",
            "batman arkham city"
        ));
        assert!(!line_is_name(
            "Fixed crash in Batman: Arkham City",
            "batman arkham city"
        ));
        assert!(has_token("Game (200260)", "200260"));
        assert!(!has_token("Game 2002601", "200260"));
    }
}
