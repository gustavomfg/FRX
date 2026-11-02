//! Reddit (`r/linux_gaming`, JSON público) — **desativado em `fixes::search`**.
//!
//! Em 02/11/2026, o acesso sem OAuth respondeu 403. O parser fica pronto e testado
//! com respostas simuladas, mas nunca foi validado contra uma resposta real.
//! Antes de ativar, inclua o host em `capabilities/default.json` e confira os
//! termos da Data API.
//!
//! Só leitura: nomes de autor não são guardados, e nada aqui vira receita sem passar
//! pelos validadores do Fix Engine. O nome do jogo na busca não é identidade.

use reqwest::Url;
use serde::Deserialize;
use serde_json::Value;

use super::super::aggregation::{group_mentions, Mention};
use super::super::http::{Ctx, Outcome};
use super::super::models::{ProviderId, ProviderInfo, ProviderStatus};
use super::github::{collect_from, inherited_scope, name_variants};
use super::{
    disabled_status, mark_malformed, mentions_game, reassign_ids, report_finding, safe_lines,
    safe_title, source_scope, summarize, CommunityReport, GameQuery, ProviderResult,
};
use crate::net::{Fetcher, HttpRequest};
use crate::time::iso8601;

const SUBREDDIT: &str = "linux_gaming";
const ACCEPT: &str = "application/json";
const NAME: &str = "Reddit";
const SEARCH_BYTES: usize = 2 * 1024 * 1024;
const THREAD_BYTES: usize = 4 * 1024 * 1024;
const MAX_POSTS: usize = 3;
const MAX_THREADS: usize = 2;
const MAX_COMMENTS: usize = 50;
const MAX_DEPTH: usize = 3;

pub fn info() -> ProviderInfo {
    ProviderInfo {
        id: ProviderId::Reddit,
        name: NAME,
        enabled: false,
        description: "Discussões em r/linux_gaming.",
        reason: Some("O Reddit recusa o acesso público sem OAuth (HTTP 403); o FRX não contorna esse bloqueio."),
    }
}

pub fn status() -> ProviderStatus {
    disabled_status(info())
}

#[derive(Deserialize)]
struct Post {
    id: String,
    title: String,
    #[serde(default)]
    selftext: String,
    subreddit: String,
    #[serde(default)]
    num_comments: u64,
    created_utc: Option<f64>,
}

#[derive(Debug, PartialEq)]
struct Comment {
    id: String,
    body: String,
    created_at: Option<String>,
}

fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 16 && id.chars().all(|c| c.is_ascii_alphanumeric())
}

fn date(seconds: Option<f64>) -> Option<String> {
    seconds.filter(|s| *s > 0.0).map(|s| iso8601(s as u64))
}

fn search_url(q: &GameQuery) -> String {
    let query = match name_variants(&q.name).last() {
        Some(name) => format!("\"{name}\" OR {}", q.app_id),
        None => q.app_id.clone(),
    };
    Url::parse_with_params(
        &format!("https://www.reddit.com/r/{SUBREDDIT}/search.json"),
        &[
            ("q", query.as_str()),
            ("restrict_sr", "1"),
            ("sort", "relevance"),
            ("limit", "10"),
            ("raw_json", "1"),
        ],
    )
    .map(|u| u.to_string())
    .unwrap_or_default()
}

/// `None` quando o corpo não tem o formato de uma listagem.
fn parse_posts(body: &str) -> Option<Vec<Post>> {
    let value: Value = serde_json::from_str(body).ok()?;
    let children = value.pointer("/data/children")?.as_array()?;
    Some(
        children
            .iter()
            .filter(|c| c["kind"] == "t3")
            .filter_map(|c| Post::deserialize(&c["data"]).ok())
            .filter(|p| p.subreddit.eq_ignore_ascii_case(SUBREDDIT) && safe_id(&p.id))
            .collect(),
    )
}

fn walk_comments(listing: &Value, depth: usize, out: &mut Vec<Comment>) {
    let Some(children) = listing.pointer("/data/children").and_then(Value::as_array) else {
        return;
    };
    for child in children {
        if out.len() >= MAX_COMMENTS {
            return;
        }
        if child["kind"] != "t1" {
            continue;
        }
        let data = &child["data"];
        let id = data["id"].as_str().unwrap_or_default();
        let body = data["body"].as_str().unwrap_or_default();
        if safe_id(id) && !body.is_empty() && !matches!(body, "[deleted]" | "[removed]") {
            out.push(Comment {
                id: id.to_owned(),
                body: body.to_owned(),
                created_at: date(data["created_utc"].as_f64()),
            });
        }
        if depth < MAX_DEPTH {
            walk_comments(&data["replies"], depth + 1, out);
        }
    }
}

/// A resposta de uma discussão é `[post, comentários]`.
fn parse_comments(body: &str) -> Option<Vec<Comment>> {
    let value: Value = serde_json::from_str(body).ok()?;
    let listing = value.as_array()?.get(1)?;
    listing.pointer("/data/children")?.as_array()?;
    let mut out = Vec::new();
    walk_comments(listing, 1, &mut out);
    Some(out)
}

pub async fn search<F: Fetcher>(ctx: &Ctx<'_, F>, q: &GameQuery) -> ProviderResult {
    let mut outcomes: Vec<Outcome> = Vec::new();
    let mut findings = Vec::new();
    let mut mentions: Vec<Mention> = Vec::new();
    let mut malformed = false;

    let found = ctx
        .get(
            HttpRequest::new(search_url(q), ACCEPT, SEARCH_BYTES),
            Some("reddit"),
            ctx.ttl_secs,
        )
        .await;
    let posts = match found.body.as_deref().map(parse_posts) {
        Some(Some(posts)) => posts,
        Some(None) => {
            malformed = true;
            vec![]
        }
        None => vec![],
    };
    outcomes.push(found);
    let mut posts: Vec<Post> = posts
        .into_iter()
        .filter(|p| mentions_game(&format!("{}\n{}", p.title, p.selftext), q))
        .take(MAX_POSTS)
        .collect();

    for post in &posts {
        let url = format!("https://www.reddit.com/r/{SUBREDDIT}/comments/{}/", post.id);
        let published = date(post.created_utc);
        let text = safe_lines(&format!("{}\n{}", post.title, post.selftext));
        let scope = source_scope(&text, q, &url);
        let title = safe_title(&post.title, 160);
        let reference = format!("r/{SUBREDDIT}/{}", post.id);
        findings.push(report_finding(
            q,
            CommunityReport {
                provider: ProviderId::Reddit,
                id_prefix: "reddit-post",
                title: format!("Relato no Reddit · {reference}"),
                label: format!("{reference} · {}", safe_title(&post.title, 80)),
                description: format!(
                    "“{title}” — {} comentário(s). Relatos da comunidade não foram testados pelo FRX; leia antes de seguir qualquer passo.",
                    post.num_comments
                ),
                reference: reference.clone(),
                url: url.clone(),
                scope: scope.clone(),
                published_at: published.clone(),
            },
        ));
        collect_from(
            ProviderId::Reddit,
            &mut mentions,
            &safe_lines(&post.selftext),
            &url,
            &format!("{reference} · texto"),
            published,
            &scope,
            (false, None),
        );
    }

    posts.sort_by_key(|p| std::cmp::Reverse(p.num_comments));
    for post in posts
        .iter()
        .take(MAX_THREADS)
        .filter(|p| p.num_comments > 0)
    {
        let base = format!("https://www.reddit.com/r/{SUBREDDIT}/comments/{}", post.id);
        let o = ctx
            .get(
                HttpRequest::new(
                    format!("{base}.json?limit={MAX_COMMENTS}&depth={MAX_DEPTH}&raw_json=1"),
                    ACCEPT,
                    THREAD_BYTES,
                ),
                Some("reddit"),
                ctx.ttl_secs,
            )
            .await;
        let comments = match o.body.as_deref().map(parse_comments) {
            Some(Some(comments)) => comments,
            Some(None) => {
                malformed = true;
                vec![]
            }
            None => vec![],
        };
        outcomes.push(o);
        let parent = source_scope(
            &safe_lines(&format!("{}\n{}", post.title, post.selftext)),
            q,
            &format!("{base}/"),
        );
        for c in &comments {
            let text = safe_lines(&c.body);
            collect_from(
                ProviderId::Reddit,
                &mut mentions,
                &text,
                &format!("{base}/comment/{}/", c.id),
                &format!("Comentário em r/{SUBREDDIT}/{}", post.id),
                c.created_at.clone(),
                &inherited_scope(&parent, &text),
                (false, None),
            );
        }
    }

    let mut tweaks = group_mentions(&q.app_id, mentions, &q.retrieved_at);
    reassign_ids(&mut tweaks, "reddit", &q.app_id);
    findings.extend(tweaks);
    let refs: Vec<&Outcome> = outcomes.iter().collect();
    let mut status = summarize(ProviderId::Reddit, NAME, &refs, findings.len());
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
    use crate::net::HttpResponse;
    use serde_json::json;

    const APP: &str = "200260";

    fn query() -> GameQuery {
        GameQuery {
            app_id: APP.into(),
            name: "Batman: Arkham City GOTY".into(),
            retrieved_at: "fixture".into(),
            tool: None,
        }
    }

    fn post(id: &str, title: &str, text: &str, comments: u64) -> Value {
        json!({"kind": "t3", "data": {
            "id": id, "title": title, "selftext": text, "subreddit": "linux_gaming",
            "num_comments": comments, "created_utc": 1_700_000_000.0, "author": "alguem"
        }})
    }

    fn listing(children: Vec<Value>) -> String {
        json!({"kind": "Listing", "data": {"children": children}}).to_string()
    }

    fn comment(id: &str, body: &str, replies: Value) -> Value {
        json!({"kind": "t1", "data": {
            "id": id, "body": body, "created_utc": 1_700_000_100.0, "author": "alguem",
            "replies": replies
        }})
    }

    fn thread_url(id: &str) -> String {
        format!(
            "https://www.reddit.com/r/linux_gaming/comments/{id}.json?limit=50&depth=3&raw_json=1"
        )
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
    fn search_url_is_allowed_and_encodes_the_name() {
        let url = search_url(&query());
        assert!(crate::net::is_allowed(&url));
        assert!(url.contains("restrict_sr=1"));
        assert!(url.contains("batman+arkham+city"), "{url}");
        assert!(url.contains(APP));
    }

    #[tokio::test]
    async fn explicit_steam_link_verifies_the_post_and_comments_become_mentions() {
        let f = MockFetcher::default();
        f.on(
            &search_url(&query()),
            200,
            &listing(vec![post(
                "abc123",
                "Batman Arkham City crash",
                "store.steampowered.com/app/200260",
                2,
            )]),
        );
        let thread = json!([
            listing(vec![]).parse::<Value>().unwrap(),
            {"kind": "Listing", "data": {"children": [
                comment("c1", "PROTON_NO_ESYNC=1 fixed it for me", json!({
                    "kind": "Listing", "data": {"children": [
                        comment("c2", "PROTON_NO_ESYNC=1 fixed the crash here too", json!(""))
                    ]}
                })),
                comment("c3", "[deleted]", json!(""))
            ]}}
        ]);
        f.on(&thread_url("abc123"), 200, &thread.to_string());
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
            "https://www.reddit.com/r/linux_gaming/comments/abc123/"
        );
        let tweak = r
            .findings
            .iter()
            .find(|x| x.kind == FindingKind::Tweak)
            .unwrap();
        assert_eq!(tweak.actions[0].kind, ActionKind::Env);
        assert_eq!(tweak.actions[0].value, "PROTON_NO_ESYNC=1");
        assert_eq!(tweak.evidence.positive, 2);
        assert!(tweak.id.starts_with("reddit-"));
        assert!(tweak
            .sources
            .iter()
            .all(|s| s.provider == ProviderId::Reddit));
        assert!(tweak.sources[0].url.contains("/comment/c"));
        for f in &r.findings {
            assert_ne!(f.confidence, Confidence::Verified);
        }
        assert!(!serde_json::to_string(&r.findings)
            .unwrap()
            .contains("alguem"));
    }

    #[tokio::test]
    async fn the_name_alone_never_verifies_the_game() {
        let f = MockFetcher::default();
        f.on(
            &search_url(&query()),
            200,
            &listing(vec![post(
                "abc123",
                "Batman Arkham City crash",
                "PROTON_NO_ESYNC=1 helped",
                0,
            )]),
        );
        let r = run(&f).await;
        assert!(!r.findings.is_empty());
        assert!(r.findings.iter().all(|x| !x.scope.matches_game(APP)));
    }

    #[tokio::test]
    async fn unrelated_posts_other_subreddits_and_bad_ids_are_dropped() {
        let f = MockFetcher::default();
        let mut other = post("abc123", "Batman Arkham City", "", 0);
        other["data"]["subreddit"] = json!("gaming");
        f.on(
            &search_url(&query()),
            200,
            &listing(vec![
                other,
                post("abc124", "Portal 2 crash", "", 0),
                post("../x", "Batman Arkham City", "", 0),
            ]),
        );
        let r = run(&f).await;
        assert!(r.findings.is_empty());
        assert_eq!(r.status.state, ProviderState::NoData);
    }

    #[tokio::test]
    async fn command_lines_never_become_actions() {
        let f = MockFetcher::default();
        f.on(
            &search_url(&query()),
            200,
            &listing(vec![post(
                "abc123",
                "Batman Arkham City crash",
                "PROTON_A=1 && rm -rf ~\nPROTON_B=1; reboot\ncurl x | sh\nPROTON_C=$(id) %command%\n`PROTON_D=1`\nwinetricks physx; true",
                0,
            )]),
        );
        let r = run(&f).await;
        assert!(
            r.findings.iter().all(|x| x.actions.is_empty()),
            "{:?}",
            r.findings
        );
        assert_eq!(r.findings.len(), 1);
    }

    #[tokio::test]
    async fn blocked_rate_limited_and_html_responses_are_reported_as_such() {
        let f = MockFetcher::default();
        f.on_response(
            &search_url(&query()),
            HttpResponse {
                status: 403,
                retry_after: Some(0),
                ..Default::default()
            },
        );
        let r = run(&f).await;
        assert_eq!(r.status.state, ProviderState::Unavailable);
        assert!(r.status.message.unwrap().contains("403"));

        f.on(&search_url(&query()), 429, "");
        assert_eq!(run(&f).await.status.state, ProviderState::RateLimited);

        f.on(
            &search_url(&query()),
            200,
            "<html>Making sure you're not a bot</html>",
        );
        let r = run(&f).await;
        assert_eq!(r.status.state, ProviderState::Unavailable);
        assert!(r.status.message.unwrap().contains("formato inesperado"));
    }

    #[tokio::test]
    async fn oversized_response_is_rejected() {
        let f = MockFetcher::default();
        f.on(&search_url(&query()), 200, &"x".repeat(SEARCH_BYTES + 1));
        assert_eq!(run(&f).await.status.state, ProviderState::Unavailable);
    }

    #[test]
    fn comments_are_bounded_in_depth_and_count() {
        let mut replies = json!("");
        for i in (0..6).rev() {
            replies = json!({"kind": "Listing", "data": {"children": [
                comment(&format!("d{i}"), "texto", replies)
            ]}});
        }
        let mut out = Vec::new();
        walk_comments(&replies, 1, &mut out);
        assert_eq!(out.len(), MAX_DEPTH);

        let many: Vec<Value> = (0..80)
            .map(|i| comment(&format!("m{i}"), "texto", json!("")))
            .collect();
        let thread = json!([{}, {"data": {"children": many}}]).to_string();
        assert_eq!(parse_comments(&thread).unwrap().len(), MAX_COMMENTS);
        assert_eq!(parse_comments("{}"), None);
    }

    #[test]
    fn stays_disabled_until_access_works() {
        assert_eq!(status().state, ProviderState::Disabled);
        assert!(!info().enabled);
    }
}
