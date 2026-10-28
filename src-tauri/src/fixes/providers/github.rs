//! GitHub (API REST pública).
//!
//! - Busca issues com o **AppID no título** em repositórios de compatibilidade
//!   conhecidos — resultados genéricos por nome não são usados como evidência;
//! - lê os comentários mais recentes e extrai apenas ajustes reconhecidos;
//! - comentários de mantenedores dizendo que algo foi corrigido e notas de
//!   versão oficiais que citam o jogo viram descobertas "verificadas".

use reqwest::Url;
use serde::Deserialize;

use super::super::aggregation::{component_keys, finding_id, group_mentions, Mention};
use super::super::http::{Ctx, Outcome};
use super::super::models::*;
use super::super::tweaks::scan_text;
use super::{summarize, GameQuery, ProviderResult};
use crate::net::{Fetcher, HttpRequest};

const REPOS: &[&str] = &[
    "ValveSoftware/Proton",
    "Open-Wine-Components/umu-protonfixes",
    "GloriousEggroll/proton-ge-custom",
    "CachyOS/proton-cachyos",
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
struct User {
    login: String,
}

#[derive(Deserialize, Clone)]
struct Issue {
    #[serde(default)]
    user: Option<User>,
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
    #[serde(default)]
    user: Option<User>,
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
    Url::parse(u).is_ok_and(|x| {
        x.scheme() == "https"
            && x.host_str() == Some("github.com")
            && x.username().is_empty()
            && x.password().is_none()
            && x.port().is_none()
    })
}

fn repo_of(repository_url: &str) -> String {
    repository_url
        .trim_start_matches("https://api.github.com/repos/")
        .to_owned()
}

/// Explicit identifiers found in source content. Plain years, FPS and version
/// numbers are not identities. No title/name similarity is used here.
fn explicit_app_ids(text: &str) -> Vec<String> {
    let low = text.to_ascii_lowercase();
    let mut ids = Vec::new();
    for prefix in [
        "store.steampowered.com/app/",
        "steamcommunity.com/app/",
        "appid",
        "app id",
        "steam app id",
        "steam appid of the game",
        "steam appid of game",
        "app id of the game",
    ] {
        for (i, _) in low.match_indices(prefix) {
            // Formulários de issue costumam destacar o valor em Markdown: `**3321460**`.
            let tail =
                low[i + prefix.len()..].trim_start_matches([' ', ':', '=', '`', '#', '*', '_']);
            let id: String = tail.chars().take_while(char::is_ascii_digit).collect();
            if crate::art::valid_app_id(&id) && !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    ids
}

/// Bare bracketed numbers are ambiguous identity hints, not proof: they can
/// describe a GPU model, year or FPS count. They may only corroborate an
/// explicit AppID label or Steam app URL from the source.
fn identity_hints(text: &str) -> Vec<String> {
    let low = text.to_ascii_lowercase();
    let mut ids = explicit_app_ids(text);
    for (open, close) in [('(', ')'), ('[', ']')] {
        for part in low.split(open).skip(1) {
            if let Some((id, _)) = part.split_once(close) {
                let id = id.trim();
                if crate::art::valid_app_id(id) && !ids.iter().any(|i| i == id) {
                    ids.push(id.to_owned());
                }
            }
        }
    }
    ids
}

fn issue_scope(issue: &Issue, app_id: &str) -> DiscoveryScope {
    let repo = repo_of(&issue.repository_url);
    let ids = identity_hints(&issue.title);
    let title_ids = explicit_app_ids(&issue.title);
    let body_ids = explicit_app_ids(issue.body.as_deref().unwrap_or_default());
    let confirmed = title_ids.len() == 1 && title_ids[0] == app_id
        || body_ids.len() == 1 && body_ids[0] == app_id;
    let expected = format!("https://github.com/{repo}/issues/{}", issue.number);
    if REPOS.contains(&repo.as_str())
        && issue.html_url == expected
        && (ids.is_empty() && body_ids == [app_id] || ids == [app_id])
        && confirmed
        && body_ids.iter().all(|id| id == app_id)
    {
        DiscoveryScope::verified(
            app_id,
            format!("A própria issue confirma o AppID {app_id} por identificador explícito ou link da Steam: {expected}"),
        )
    } else {
        DiscoveryScope {
            basis: format!(
                "A identidade da issue é ambígua ou não pertence a um tracker integrado: {}",
                issue.html_url
            ),
            ..DiscoveryScope::default()
        }
    }
}

fn inherited_scope(parent: &DiscoveryScope, text: &str) -> DiscoveryScope {
    let ids = identity_hints(text);
    let low = text.to_ascii_lowercase();
    // Com AppID explícito no texto, só identificadores explícitos decidem:
    // números entre parênteses podem ser modelos de GPU, como "(4070)".
    let explicit = explicit_app_ids(text);
    let hints = if explicit.is_empty() { &ids } else { &explicit };
    let other_id = parent
        .app_id
        .as_deref()
        .is_some_and(|app| hints.iter().any(|id| id != app));
    // Um AppID explícito do mesmo jogo (ex.: "AppID **3321460**") prevalece
    // sobre as frases genéricas abaixo, comuns em "in game settings".
    let confirmed = !other_id && parent.app_id.is_some() && !explicit.is_empty();
    if other_id
        || !confirmed
            && [
                "another game",
                "other game",
                "different game",
                "outro jogo",
                "outros jogos",
                "another title",
                "different title",
                "for me in ",
                "also works in ",
                "also works for ",
                "in game ",
                "for game ",
            ]
            .iter()
            .any(|p| low.contains(p))
    {
        DiscoveryScope { basis: "A fonte discute outro jogo ou vários jogos; a issue original não basta para atribuir estas ações.".into(), ..DiscoveryScope::default() }
    } else {
        parent.clone()
    }
}

/// Conservative claim extraction: one explicit action, one line, an observed
/// directional performance claim. Compatibility successes and speculative
/// suggestions never count as FPS evidence.
fn performance_reports(
    tweak: &super::super::tweaks::Tweak,
    line: &str,
    scope: &DiscoveryScope,
    url: &str,
    date: Option<String>,
    maintainer: bool,
    author: Option<&str>,
) -> Vec<PerformanceReport> {
    let Some(app_id) = scope
        .app_id
        .as_deref()
        .filter(|app| scope.matches_game(app))
    else {
        return vec![];
    };
    let keys = component_keys(tweak);
    if keys.len() != 1 || !keys[0].starts_with("env:") {
        return vec![];
    }
    let scan = scan_text(line);
    if scan.tweaks.len() != 1 {
        return vec![];
    }
    let low = line.to_ascii_lowercase();
    if [
        "may ",
        "might ",
        "could ",
        "should ",
        "possibly",
        "potential",
        "try ",
        "does not improve",
        "doesn't improve",
        "did not improve",
        "didn't improve",
        "no performance improvement",
        "not better",
        "not higher",
        "not faster",
        "not worse",
        "not lower",
        "not reduced",
        "does not reduce",
        "doesn't reduce",
        "did not reduce",
        "didn't reduce",
        "without ",
        "removing ",
        "removed ",
        "unset",
        "not using",
        "turned off",
        "after disabling",
    ]
    .iter()
    .any(|p| low.contains(p))
        || low.contains('?')
    {
        return vec![];
    }
    let positive = [
        "improved performance",
        "improves performance",
        "better performance",
        "higher fps",
        "increased fps",
        "fps increased",
        "fps boost",
        "improved fps",
        "improves fps",
        "reduces stutter",
        "reduced stutter",
        "less stutter",
        "lower frametime",
    ]
    .iter()
    .any(|p| low.contains(p));
    let negative = [
        "worse performance",
        "lower fps",
        "fps dropped",
        "decreased fps",
        "reduces fps",
        "reduced fps",
        "performance regression",
        "more stutter",
        "higher frametime",
    ]
    .iter()
    .any(|p| low.contains(p));
    [PerformanceOutcome::Positive, PerformanceOutcome::Negative].into_iter()
        .filter(|outcome| match outcome { PerformanceOutcome::Positive => positive, PerformanceOutcome::Negative => negative })
        .map(|outcome| PerformanceReport {
            hardware: Default::default(),
            app_id: app_id.to_owned(), action_key: keys[0].clone(), source_url: url.to_owned(), author: author.map(str::to_owned), reported_at: date.clone(), statement: line.chars().take(240).collect(), outcome,
            proton_version: scan.proton_versions.first().cloned(),
            gpu_vendor: if scan.gpus.len() == 1 { scan.gpus.first().copied() } else { None },
            confidence: if maintainer { Confidence::Verified } else { Confidence::CommunityReported },
            limitations: vec!["Relato público; não é medição local do FRX. Driver, cenário e configurações completas não foram verificados.".into()],
        }).collect()
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
    let mut artifact_documents: Vec<super::artifacts::Document> = Vec::new();

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
    let mut issues: Vec<Issue> = so
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

    // A source can discuss an actual file solution without putting the AppID
    // in its title. Search the normalized name too, but never use the query
    // as identity evidence: ambiguous hits remain explicitly unattributed.
    if let Some(name) = name_variants(&q.name).last().filter(|n| n.len() >= 6) {
        let query = format!(
            "\"{name}\" in:title,body {}",
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
                ("per_page", "5"),
                ("sort", "updated"),
                ("order", "desc"),
            ],
        )
        .map(|u| u.to_string())
        .unwrap_or_default();
        let o = ctx
            .get(
                HttpRequest::new(url, ACCEPT, 2 * 1024 * 1024),
                Some("search"),
                ctx.ttl_secs,
            )
            .await;
        if let Some(body) = &o.body {
            if let Ok(found) = serde_json::from_str::<SearchResp>(body) {
                for i in found.items.into_iter().filter(|i| {
                    i.pull_request.is_none()
                        && safe_github_url(&i.html_url)
                        && REPOS.contains(&repo_of(&i.repository_url).as_str())
                }) {
                    if issues.len() >= MAX_ISSUES {
                        break;
                    }
                    if !issues.iter().any(|old| old.html_url == i.html_url) {
                        issues.push(i);
                    }
                }
            }
        }
        outcomes.push(o);
    }

    for issue in &issues {
        let repo = repo_of(&issue.repository_url);
        let reference = format!("{repo}#{}", issue.number);
        let scope = issue_scope(issue, &q.app_id);
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
            artifacts: vec![],
            scope: scope.clone(),
            performance_evidence: PerformanceEvidence::default(),
            id: finding_id("gh-issue", &q.app_id, &reference),
            game_app_id: scope.app_id.clone().unwrap_or_default(),
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
            artifact_documents.push(super::artifacts::Document {
                source: FindingSource {
                    provider: ProviderId::Github,
                    label: format!("{reference} · descrição"),
                    url: issue.html_url.clone(),
                    reference_id: Some(reference.clone()),
                    snippet: None,
                    published_at: issue.created_at.clone(),
                },
                scope: scope.clone(),
                text: body.clone(),
            });
            collect(
                &mut mentions,
                body,
                &issue.html_url,
                &format!("{reference} · descrição"),
                issue.created_at.clone(),
                &scope,
                (false, issue.user.as_ref().map(|u| u.login.as_str())),
            );
        }
    }

    // 2. Comentários mais recentes das issues mais discutidas.
    let mut by_comments = issues.clone();
    by_comments.sort_by_key(|i| std::cmp::Reverse(i.comments));
    for issue in by_comments.iter().take(2).filter(|i| i.comments > 0) {
        let repo = repo_of(&issue.repository_url);
        if !REPOS.contains(&repo.as_str()) {
            continue;
        }
        let scope = issue_scope(issue, &q.app_id);
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
            let parent_url = format!(
                "https://github.com/{repo}/issues/{}#issuecomment-",
                issue.number
            );
            for c in comments
                .iter()
                .filter(|c| safe_github_url(&c.html_url) && c.html_url.starts_with(&parent_url))
            {
                let Some(body) = &c.body else { continue };
                let label = format!("Comentário em {repo}#{}", issue.number);
                artifact_documents.push(super::artifacts::Document {
                    source: FindingSource {
                        provider: ProviderId::Github,
                        label: label.clone(),
                        url: c.html_url.clone(),
                        reference_id: None,
                        snippet: None,
                        published_at: c.created_at.clone(),
                    },
                    scope: inherited_scope(&scope, body),
                    text: body.clone(),
                });
                collect(
                    &mut mentions,
                    body,
                    &c.html_url,
                    &label,
                    c.created_at.clone(),
                    &scope,
                    (
                        matches!(
                            c.author_association.as_deref(),
                            Some("MEMBER" | "OWNER" | "COLLABORATOR")
                        ),
                        c.user.as_ref().map(|u| u.login.as_str()),
                    ),
                );
                if let Some(f) = maintainer_statement(q, &repo, issue.number, c, body, &scope) {
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
        for r in &releases {
            if let Some(body) = &r.body {
                if explicit_app_ids(body) == [q.app_id.clone()] {
                    artifact_documents.push(super::artifacts::Document {
                        source: FindingSource {
                            provider: ProviderId::Github,
                            label: format!("{repo} · {}", r.tag_name),
                            url: r.html_url.clone(),
                            reference_id: Some(r.tag_name.clone()),
                            snippet: None,
                            published_at: r.published_at.clone(),
                        },
                        scope: DiscoveryScope::verified(
                            &q.app_id,
                            format!("Release identifica explicitamente o AppID {}", q.app_id),
                        ),
                        text: format!("{}\n{}", r.html_url, body),
                    });
                }
            }
        }
    }

    findings.extend(group_mentions(&q.app_id, mentions, &q.retrieved_at));
    let (artifact_findings, artifact_outcomes) =
        super::artifacts::discover(ctx, q, &artifact_documents).await;
    findings.extend(artifact_findings);
    outcomes.extend(artifact_outcomes);
    let refs: Vec<&Outcome> = outcomes.iter().collect();
    let found = findings.len();
    ProviderResult {
        status: summarize(ProviderId::Github, "GitHub", &refs, found),
        findings,
        identity: None,
    }
}

fn collect(
    out: &mut Vec<Mention>,
    text: &str,
    url: &str,
    label: &str,
    date: Option<String>,
    parent: &DiscoveryScope,
    reporter: (bool, Option<&str>),
) {
    let (maintainer, author) = reporter;
    let scope = inherited_scope(parent, text);
    let s = scan_text(text);
    for (tweak, line) in s.tweaks {
        out.push(Mention {
            scope: scope.clone(),
            performance_reports: performance_reports(
                &tweak,
                &line,
                &scope,
                url,
                date.clone(),
                maintainer,
                author,
            ),
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
    parent: &DiscoveryScope,
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
        artifacts: vec![],
        scope: inherited_scope(parent, body),
        performance_evidence: PerformanceEvidence::default(),
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
        requirements: Requirements { hardware: Default::default(), gpu_vendor: None, proton_versions: versions, game_version: None },
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
        let explicit = explicit_app_ids(line);
        let scope = if explicit.len() == 1 && explicit[0] == q.app_id {
            DiscoveryScope::verified(
                &q.app_id,
                format!(
                    "O trecho das notas de versão identifica explicitamente o Steam AppID {}: {}",
                    q.app_id, r.html_url
                ),
            )
        } else {
            DiscoveryScope {
                basis: "Menção somente por nome nas notas de versão; a atribuição ao Steam AppID não foi confirmada.".into(),
                ..DiscoveryScope::default()
            }
        };
        let verified = scope.matches_game(&q.app_id);
        out.push(FixFinding {
            artifacts: vec![],
            game_app_id: scope.app_id.clone().unwrap_or_default(),
            scope,
            performance_evidence: PerformanceEvidence::default(),
            id: finding_id("gh-release", &q.app_id, &r.html_url),
            title: format!("Citado nas notas de versão de {release}"),
            description: if verified {
                format!("As notas oficiais de {repo} ({}) identificam este Steam AppID. Consulte o trecho para entender a alteração de compatibilidade; não é medição de desempenho local.", r.tag_name)
            } else {
                format!("Possível menção por nome nas notas de {repo} ({}). A fonte não confirma o Steam AppID; esta referência não pode gerar receita nem evidência de desempenho para o jogo selecionado.", r.tag_name)
            },
            category: Category::Compatibility,
            kind: FindingKind::ReleaseNote,
            actions: vec![FixAction { kind: ActionKind::ProtonVersion, label: "Versão da ferramenta citada".into(), value: r.tag_name.clone(), conditional: false }],
            sources: vec![FindingSource { provider: ProviderId::Github, label: format!("{repo} · {release}"), url: r.html_url.clone(), reference_id: Some(r.tag_name.clone()), snippet: Some(snippet), published_at: r.published_at.clone() }],
            requirements: Requirements { hardware: Default::default(), gpu_vendor: None, proton_versions: vec![r.tag_name.clone()], game_version: None },
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
         "body":"Steam AppID of the game: 200260. Crashes at start. PROTON_USE_WINED3D=1 works for me on GTX 1060","labels":[{"name":".NET"}],"created_at":"2018-08-23T18:41:25Z",
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

    #[tokio::test]
    async fn name_query_finds_file_references_but_only_publisher_appid_confirms_recipe_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let state = EngineState::default();
        let fetcher = MockFetcher::default();
        let query = format!(
            "\"batman arkham city\" in:title,body {}",
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
                ("per_page", "5"),
                ("sort", "updated"),
                ("order", "desc"),
            ],
        )
        .unwrap()
        .to_string();
        fetcher.on(&url,200,r#"{"items":[{"number":999999,"title":"GPU PhysX / CUDA compatibility","html_url":"https://github.com/GloriousEggroll/proton-ge-custom/issues/999999","state":"open","comments":0,"body":"Synthetic report about Batman Arkham City: https://github.com/ValveSoftware/Proton/releases/tag/fixture-file","repository_url":"https://api.github.com/repos/GloriousEggroll/proton-ge-custom"}]}"#);
        let metadata = serde_json::json!({"id":4242,"tag_name":"fixture-file","html_url":"https://github.com/ValveSoftware/Proton/releases/tag/fixture-file","author":{"login":"ValveSoftware"},"body":"Steam AppID: 200260\nInstall: Config/fix.ini <- fix.ini","assets":[{"id":7,"name":"fix.ini","size":100,"state":"uploaded","digest":format!("sha256:{}","a".repeat(64)),"browser_download_url":"https://github.com/ValveSoftware/Proton/releases/download/fixture-file/fix.ini"}]});
        fetcher.on(
            "https://api.github.com/repos/ValveSoftware/Proton/releases/tags/fixture-file",
            200,
            &metadata.to_string(),
        );
        let ctx = Ctx {
            fetcher: &fetcher,
            cache: &cache,
            state: &state,
            ttl_secs: 3600,
            force: false,
        };
        let result = search(&ctx, &q()).await;
        let community = result
            .findings
            .iter()
            .find(|f| f.kind == FindingKind::Report)
            .unwrap();
        assert_eq!(community.scope.kind, DiscoveryScopeKind::UnknownScope);
        assert!(community.game_app_id.is_empty());
        let artifact = result
            .findings
            .iter()
            .find(|f| !f.artifacts.is_empty())
            .unwrap();
        assert!(artifact.scope.matches_game("200260"));
        assert_eq!(
            artifact.artifacts[0].trust,
            ArtifactTrust::OriginAndIntegrity
        );
        assert!(artifact.artifacts[0].scope.basis.contains("publicador"));
        assert_eq!(
            fetcher.count("https://github.com/"),
            0,
            "discovery never fetches payloads"
        );
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
    #[test]
    fn attribution_uses_the_source_identity_not_query_or_name_similarity() {
        let issue: Issue = serde_json::from_str(r#"{"number":309,"title":"Game B (570), see also AppID 200260","html_url":"https://github.com/ValveSoftware/Proton/issues/309","state":"open","repository_url":"https://api.github.com/repos/ValveSoftware/Proton","created_at":null}"#).unwrap();
        assert!(!issue_scope(&issue, "200260").matches_game("200260"));
        let issue = serde_json::from_str::<SearchResp>(SEARCH)
            .unwrap()
            .items
            .remove(0);
        assert!(issue_scope(&issue, "200260").matches_game("200260"));
        assert!(!issue_scope(&issue, "570").matches_game("570"));
        let parent = issue_scope(&issue, "200260");
        assert!(!inherited_scope(
            &parent,
            "For another game, PROTON_NO_D3D11=1 improves performance"
        )
        .matches_game("200260"));
        assert!(
            !inherited_scope(&parent, "AppID: 570 PROTON_NO_D3D11=1 improves performance")
                .matches_game("200260")
        );
        assert!(inherited_scope(&parent, "PROTON_NO_D3D11=1 fixed my crash").matches_game("200260"));
        // AppID explícito do mesmo jogo vence a frase genérica "in game ".
        assert!(inherited_scope(&parent, "Game: Arkham City (AppID **200260**). Changed in game settings, PROTON_NO_D3D11=1 fixed it").matches_game("200260"));
        assert!(!inherited_scope(
            &parent,
            "AppID 200260 and AppID 570: in game PROTON_NO_D3D11=1"
        )
        .matches_game("200260"));
        assert!(inherited_scope(
            &parent,
            "AppID 200260 on RTX (4070): PROTON_NO_D3D11=1 fixed it"
        )
        .matches_game("200260"));
    }

    #[test]
    fn gpu_models_and_years_in_titles_do_not_establish_appid() {
        let template = serde_json::from_str::<SearchResp>(SEARCH)
            .unwrap()
            .items
            .remove(0);
        for (title, app_id) in [
            ("GPU regression on RTX (4060)", "4060"),
            ("Crash reports (2024)", "2024"),
        ] {
            let mut issue = template.clone();
            issue.title = title.into();
            issue.body = Some("PROTON_NO_D3D11=1 improved performance".into());
            assert!(!issue_scope(&issue, app_id).matches_game(app_id));
        }
        let mut issue = template.clone();
        issue.body = None;
        assert!(
            !issue_scope(&issue, "200260").matches_game("200260"),
            "bare bracketed number is unconfirmed without a source AppID label or Steam URL"
        );
        issue.body = Some("Steam app: https://store.steampowered.com/app/200260/".into());
        assert!(issue_scope(&issue, "200260").matches_game("200260"));
        issue.body = Some("Steam AppID: 570; source discusses another title".into());
        assert!(!issue_scope(&issue, "200260").matches_game("200260"));
        issue.title = "Compatibility report — AppID 200260".into();
        issue.body = None;
        assert!(
            issue_scope(&issue, "200260").matches_game("200260"),
            "explicit AppID in title is itself a source identity"
        );
        assert!(explicit_app_ids("RTX (4060), year (2024), 90 FPS").is_empty());
        // Formulário do tracker do Proton, com o valor em negrito.
        assert_eq!(
            explicit_app_ids("- Steam AppID of the game: **3321460**"),
            ["3321460"]
        );
    }

    #[test]
    fn source_body_can_confirm_appid_even_when_the_issue_title_has_no_id() {
        let mut issue = serde_json::from_str::<SearchResp>(SEARCH)
            .unwrap()
            .items
            .remove(0);
        issue.title = "NVIDIA PhysX / CUDA compatibility".into();
        issue.body = Some("Steam AppID: 200260; publisher release linked below".into());
        assert!(issue_scope(&issue, "200260").matches_game("200260"));
        issue.body =
            Some("Batman Arkham City; a similarly named game is not identity proof".into());
        assert!(!issue_scope(&issue, "200260").matches_game("200260"));
    }

    #[test]
    fn name_only_release_mentions_remain_unattributed() {
        let releases: Vec<Release> = serde_json::from_str(RELEASES).unwrap();
        let found = release_mentions(&q(), "ValveSoftware/Proton", &releases);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].scope.kind, DiscoveryScopeKind::UnknownScope);
        assert!(found[0].game_app_id.is_empty());
        assert_eq!(
            found[0].performance_evidence,
            PerformanceEvidence::default()
        );
        let exact: Vec<Release> = serde_json::from_str(&RELEASES.replace(
            "- Batman: Arkham City",
            "- Batman: Arkham City (Steam AppID: 200260)",
        ))
        .unwrap();
        let found = release_mentions(&q(), "ValveSoftware/Proton", &exact);
        assert!(found[0].scope.matches_game("200260"));
    }

    #[test]
    fn performance_claims_are_action_specific_not_compatibility_votes() {
        let scope = DiscoveryScope::verified("200260", "fixture: title AppID");
        let mut mentions = Vec::new();
        for (author, text) in [
            (
                "one",
                "PROTON_NO_D3D11=1 improved performance on Proton 9.0-4 with my RTX 4060",
            ),
            ("two", "PROTON_NO_D3D11=1 gives higher FPS"),
            ("three", "PROTON_NO_D3D11=1 causes worse performance"),
        ] {
            collect(
                &mut mentions,
                text,
                &format!(
                    "https://github.com/ValveSoftware/Proton/issues/309#issuecomment-{author}"
                ),
                "fixture",
                None,
                &scope,
                (false, Some(author)),
            );
        }
        let result = group_mentions("200260", mentions, "fixture");
        let e = &result[0].performance_evidence;
        assert_eq!((e.positive, e.negative), (2, 1));
        assert_eq!(e.reports[0].app_id, "200260");
        assert!(e
            .reports
            .iter()
            .all(|r| r.action_key == "env:PROTON_NO_D3D11=1" && r.author.is_some()));
        assert!(e
            .reports
            .iter()
            .any(|r| r.gpu_vendor == Some(GpuReq::Nvidia)
                && r.proton_version.as_deref() == Some("Proton 9.0-4")));
        for text in [
            "PROTON_NO_D3D11=1 works and fixed the crash",
            "PROTON_NO_D3D11=1 might give higher FPS",
            "PROTON_NO_D3D11=1 does not improve performance",
            "PROTON_NO_D3D11=1 not higher FPS",
            "Without PROTON_NO_D3D11=1 I get higher FPS",
            "Removed PROTON_NO_D3D11=1 and saw improved performance",
            "PROTON_NO_D3D11=1 does not reduce FPS",
            "PROTON_NO_D3D11=1 improves performance; PROTON_NO_ESYNC=1 lowers FPS",
        ] {
            let mut mentions = Vec::new();
            collect(
                &mut mentions,
                text,
                "https://github.com/ValveSoftware/Proton/issues/309#issuecomment-1",
                "fixture",
                None,
                &scope,
                (true, Some("maintainer")),
            );
            assert!(
                mentions.iter().all(|m| m.performance_reports.is_empty()),
                "{text}"
            );
        }
        let mut mentions = Vec::new();
        collect(
            &mut mentions,
            "PROTON_NO_D3D11=1 improves performance",
            "https://github.com/ValveSoftware/Proton/issues/309#issuecomment-1",
            "fixture",
            None,
            &DiscoveryScope::default(),
            (true, Some("maintainer")),
        );
        assert!(
            mentions[0].performance_reports.is_empty(),
            "Unknown scope cannot yield performance evidence"
        );
    }
}
