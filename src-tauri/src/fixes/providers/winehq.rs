//! WineHQ Bugzilla (API REST) — **desativado em `fixes::search`**.
//!
//! Em 02/11/2026, `bugs.winehq.org` não respondeu a um cliente automatizado: todas
//! as tentativas deram timeout, e um timeout atrasaria cada busca. Os formatos
//! seguem a documentação do Bugzilla e foram testados só com respostas simuladas.
//! Antes de ativar, confira uma resposta real e inclua o host em
//! `capabilities/default.json`.
//!
//! Mesmo com bug marcado como corrigido, a confiança fica em relato da comunidade.

use std::collections::BTreeMap;

use reqwest::Url;
use serde::Deserialize;

use super::super::aggregation::{group_mentions, Mention};
use super::super::http::{Ctx, Outcome};
use super::super::models::{ProviderId, ProviderInfo, ProviderStatus};
use super::github::{collect_from, inherited_scope, name_variants};
use super::{
    disabled_status, mark_malformed, mentions_game, reassign_ids, report_finding, safe_lines,
    safe_title, source_scope, summarize, CommunityReport, GameQuery, ProviderResult,
};
use crate::net::{Fetcher, HttpRequest};

const BASE: &str = "https://bugs.winehq.org";
const ACCEPT: &str = "application/json";
const NAME: &str = "WineHQ Bugzilla";
const FIELDS: &str = "id,summary,status,resolution,url,creation_time";
const SEARCH_BYTES: usize = 2 * 1024 * 1024;
const COMMENT_BYTES: usize = 4 * 1024 * 1024;
const MAX_BUGS: usize = 3;
const MAX_THREADS: usize = 2;
const MAX_COMMENTS: usize = 50;

pub fn info() -> ProviderInfo {
    ProviderInfo {
        id: ProviderId::Winehq,
        name: NAME,
        enabled: false,
        description: "Bugs públicos do Wine que citam o jogo.",
        reason: Some("A API do WineHQ não respondeu a clientes automatizados nos testes; fica desativada para não atrasar a busca."),
    }
}

pub fn status() -> ProviderStatus {
    disabled_status(info())
}

#[derive(Deserialize)]
struct BugList {
    bugs: Vec<Bug>,
}

#[derive(Deserialize)]
struct Bug {
    id: u64,
    summary: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    resolution: String,
    #[serde(default)]
    url: String,
    creation_time: Option<String>,
}

#[derive(Deserialize)]
struct CommentResponse {
    bugs: BTreeMap<String, BugComments>,
}

#[derive(Deserialize)]
struct BugComments {
    comments: Vec<Comment>,
}

#[derive(Deserialize)]
struct Comment {
    count: u64,
    text: String,
    creation_time: Option<String>,
}

fn with_params(params: &[(&str, &str)]) -> String {
    let mut all = params.to_vec();
    all.extend([("limit", "5"), ("include_fields", FIELDS)]);
    Url::parse_with_params(&format!("{BASE}/rest/bug"), &all)
        .map(|u| u.to_string())
        .unwrap_or_default()
}

/// Bugs com link da Steam no campo URL.
fn app_id_url(q: &GameQuery) -> String {
    with_params(&[("url", q.app_id.as_str()), ("url_type", "substring")])
}

fn name_url(q: &GameQuery) -> Option<String> {
    let name = name_variants(&q.name).pop()?;
    Some(with_params(&[("quicksearch", &format!("\"{name}\""))]))
}

fn bug_page(id: u64) -> String {
    format!("{BASE}/show_bug.cgi?id={id}")
}

fn state_text(bug: &Bug) -> String {
    let state = safe_title(&bug.status, 20);
    match safe_title(&bug.resolution, 20) {
        resolution if resolution.is_empty() => state,
        resolution => format!("{state} {resolution}"),
    }
}

pub async fn search<F: Fetcher>(ctx: &Ctx<'_, F>, q: &GameQuery) -> ProviderResult {
    let mut outcomes: Vec<Outcome> = Vec::new();
    let mut findings = Vec::new();
    let mut mentions: Vec<Mention> = Vec::new();
    let mut malformed = false;
    let mut bugs: Vec<Bug> = Vec::new();

    let urls: Vec<String> = std::iter::once(app_id_url(q)).chain(name_url(q)).collect();
    for url in urls {
        let o = ctx
            .get(
                HttpRequest::new(url, ACCEPT, SEARCH_BYTES),
                Some("winehq"),
                ctx.ttl_secs,
            )
            .await;
        if let Some(body) = &o.body {
            match serde_json::from_str::<BugList>(body) {
                Ok(list) => {
                    for bug in list.bugs {
                        let identity = format!("{}\n{}", bug.summary, bug.url);
                        if bugs.len() < MAX_BUGS
                            && mentions_game(&identity, q)
                            && !bugs.iter().any(|b| b.id == bug.id)
                        {
                            bugs.push(bug);
                        }
                    }
                }
                Err(_) => malformed = true,
            }
        }
        outcomes.push(o);
    }

    for bug in &bugs {
        let page = bug_page(bug.id);
        let scope = source_scope(
            &safe_lines(&format!("{}\n{}", bug.summary, bug.url)),
            q,
            &page,
        );
        let reference = format!("WineHQ #{}", bug.id);
        findings.push(report_finding(
            q,
            CommunityReport {
                provider: ProviderId::Winehq,
                id_prefix: "winehq-bug",
                title: format!("Bug do Wine · {reference}"),
                label: format!("{reference} · {}", safe_title(&bug.summary, 80)),
                description: format!(
                    "“{}” — {}. Bugs descrevem problemas e soluções relatadas; o FRX não os testou.",
                    safe_title(&bug.summary, 160),
                    state_text(bug)
                ),
                reference,
                url: page,
                scope,
                published_at: bug.creation_time.clone(),
            },
        ));
    }

    for bug in bugs.iter().take(MAX_THREADS) {
        let o = ctx
            .get(
                HttpRequest::new(
                    format!("{BASE}/rest/bug/{}/comment", bug.id),
                    ACCEPT,
                    COMMENT_BYTES,
                ),
                Some("winehq"),
                ctx.ttl_secs,
            )
            .await;
        let comments = match o
            .body
            .as_deref()
            .map(serde_json::from_str::<CommentResponse>)
        {
            Some(Ok(mut r)) => r
                .bugs
                .remove(&bug.id.to_string())
                .map(|b| b.comments)
                .unwrap_or_default(),
            Some(Err(_)) => {
                malformed = true;
                vec![]
            }
            None => vec![],
        };
        outcomes.push(o);
        let page = bug_page(bug.id);
        let parent = source_scope(
            &safe_lines(&format!("{}\n{}", bug.summary, bug.url)),
            q,
            &page,
        );
        let skip = comments.len().saturating_sub(MAX_COMMENTS);
        for c in comments.iter().skip(skip) {
            let text = safe_lines(&c.text);
            collect_from(
                ProviderId::Winehq,
                &mut mentions,
                &text,
                &format!("{page}#c{}", c.count),
                &format!("Comentário em WineHQ #{}", bug.id),
                c.creation_time.clone(),
                &inherited_scope(&parent, &text),
                (false, None),
            );
        }
    }

    let mut tweaks = group_mentions(&q.app_id, mentions, &q.retrieved_at);
    reassign_ids(&mut tweaks, "winehq", &q.app_id);
    findings.extend(tweaks);
    let refs: Vec<&Outcome> = outcomes.iter().collect();
    let mut status = summarize(ProviderId::Winehq, NAME, &refs, findings.len());
    if malformed {
        mark_malformed(&mut status);
    }
    ProviderResult {
        status,
        findings,
        identity: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;
    use crate::fixes::http::EngineState;
    use crate::fixes::models::{ActionKind, Confidence, FindingKind, ProviderState};
    use crate::net::mock::MockFetcher;
    use crate::net::NetError;
    use serde_json::{json, Value};

    const APP: &str = "200260";

    fn query() -> GameQuery {
        GameQuery {
            app_id: APP.into(),
            name: "Batman: Arkham City GOTY".into(),
            retrieved_at: "fixture".into(),
            tool: None,
        }
    }

    fn bug(id: u64, summary: &str, url: &str) -> Value {
        json!({
            "id": id, "summary": summary, "status": "RESOLVED", "resolution": "FIXED",
            "url": url, "creation_time": "2024-01-02T03:04:05Z"
        })
    }

    fn comment_url(id: u64) -> String {
        format!("{BASE}/rest/bug/{id}/comment")
    }

    fn comments(id: u64, texts: &[&str]) -> String {
        let list: Vec<Value> = texts
            .iter()
            .enumerate()
            .map(|(i, t)| {
                json!({"id": i + 100, "count": i, "text": t, "creator": "alguem@example.com",
                       "creation_time": "2024-01-03T00:00:00Z"})
            })
            .collect();
        json!({"bugs": {id.to_string(): {"comments": list}}, "comments": {}}).to_string()
    }

    async fn run(f: &MockFetcher) -> ProviderResult {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let state = EngineState::default();
        let ctx = Ctx {
            fetcher: f,
            cache: &cache,
            state: &state,
            ttl_secs: 3600,
            force: false,
        };
        search(&ctx, &query()).await
    }

    #[test]
    fn urls_are_allowed() {
        assert!(crate::net::is_allowed(&app_id_url(&query())));
        let name = name_url(&query()).unwrap();
        assert!(crate::net::is_allowed(&name));
        assert!(name.contains("batman+arkham+city"), "{name}");
    }

    #[tokio::test]
    async fn steam_link_in_the_bug_verifies_it_and_comments_become_mentions() {
        let f = MockFetcher::default();
        f.on(
            &app_id_url(&query()),
            200,
            &json!({"bugs": [bug(5, "Arkham City crashes", "https://store.steampowered.com/app/200260/")]})
                .to_string(),
        );
        f.on(
            &comment_url(5),
            200,
            &comments(
                5,
                &[
                    "WINEDLLOVERRIDES=\"foo=n,b\"",
                    "PROTON_NO_ESYNC=1 fixed it for me",
                ],
            ),
        );
        let r = run(&f).await;
        assert_eq!(r.status.state, ProviderState::Ok);
        let report = r
            .findings
            .iter()
            .find(|x| x.kind == FindingKind::Report)
            .unwrap();
        assert!(report.scope.matches_game(APP));
        assert_eq!(
            report.sources[0].url,
            "https://bugs.winehq.org/show_bug.cgi?id=5"
        );
        assert!(report.description.contains("RESOLVED FIXED"));
        let tweak = r
            .findings
            .iter()
            .find(|x| {
                x.kind == FindingKind::Tweak
                    && x.actions.iter().any(|a| a.value == "PROTON_NO_ESYNC=1")
            })
            .unwrap();
        assert_eq!(tweak.actions[0].kind, ActionKind::Env);
        assert_eq!(
            tweak.sources[0].url,
            "https://bugs.winehq.org/show_bug.cgi?id=5#c1"
        );
        assert_eq!(tweak.sources[0].provider, ProviderId::Winehq);
        assert!(tweak.id.starts_with("winehq-"));
        for f in &r.findings {
            assert_ne!(
                f.confidence,
                Confidence::Verified,
                "FIXED não vira verificado"
            );
        }
        assert!(!serde_json::to_string(&r.findings)
            .unwrap()
            .contains("alguem"));
    }

    #[tokio::test]
    async fn name_alone_does_not_verify_and_duplicates_are_merged() {
        let f = MockFetcher::default();
        let list = json!({"bugs": [bug(5, "Batman: Arkham City crashes", "")]}).to_string();
        f.on(&app_id_url(&query()), 200, &list);
        f.on(&name_url(&query()).unwrap(), 200, &list);
        let r = run(&f).await;
        assert_eq!(r.findings.len(), 1);
        assert!(!r.findings[0].scope.matches_game(APP));
    }

    #[tokio::test]
    async fn unrelated_bugs_are_dropped() {
        let f = MockFetcher::default();
        f.on(
            &name_url(&query()).unwrap(),
            200,
            &json!({"bugs": [bug(6, "Notepad crashes", "")]}).to_string(),
        );
        let r = run(&f).await;
        assert!(r.findings.is_empty());
        assert_eq!(r.status.state, ProviderState::NoData);
    }

    #[tokio::test]
    async fn command_lines_never_become_actions() {
        let f = MockFetcher::default();
        f.on(
            &app_id_url(&query()),
            200,
            &json!({"bugs": [bug(5, "Arkham City crashes", "")]}).to_string(),
        );
        f.on(
            &comment_url(5),
            200,
            &comments(5, &["PROTON_A=1 && rm -rf ~\nPROTON_B=1; reboot\ncurl x | sh\nPROTON_C=$(id)\n`PROTON_D=1`"]),
        );
        let r = run(&f).await;
        assert!(
            r.findings.iter().all(|x| x.actions.is_empty()),
            "{:?}",
            r.findings
        );
    }

    #[tokio::test]
    async fn timeout_and_anti_bot_page_are_reported_as_such() {
        let f = MockFetcher::default();
        f.fail(&app_id_url(&query()), NetError::Timeout);
        f.fail(&name_url(&query()).unwrap(), NetError::Timeout);
        let r = run(&f).await;
        assert_eq!(r.status.state, ProviderState::Unavailable);
        assert!(r.status.message.unwrap().contains("tempo limite"));

        let f = MockFetcher::default();
        f.on(
            &app_id_url(&query()),
            200,
            "<html>Making sure you're not a bot</html>",
        );
        let r = run(&f).await;
        assert_eq!(r.status.state, ProviderState::Unavailable);
    }

    #[tokio::test]
    async fn oversized_response_is_rejected() {
        let f = MockFetcher::default();
        f.on(&app_id_url(&query()), 200, &"x".repeat(SEARCH_BYTES + 1));
        f.on(
            &name_url(&query()).unwrap(),
            200,
            &"x".repeat(SEARCH_BYTES + 1),
        );
        assert_eq!(run(&f).await.status.state, ProviderState::Unavailable);
    }

    #[test]
    fn stays_disabled_until_access_works() {
        assert_eq!(status().state, ProviderState::Disabled);
        assert!(!info().enabled);
    }
}
