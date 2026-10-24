//! Requisições com cache, revalidação condicional (ETag) e controle de limites.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::cache::{Cache, Entry};
use crate::net::{Fetcher, HttpRequest};
use crate::time::now_unix;

/// Bloqueios temporários por recurso da API (ex.: `search`, `core`) após rate limit.
#[derive(Default)]
pub struct EngineState {
    blocked_until: Mutex<HashMap<&'static str, u64>>,
}

impl EngineState {
    pub fn blocked(&self, resource: &'static str) -> Option<u64> {
        let map = self.blocked_until.lock().ok()?;
        map.get(resource).copied().filter(|&t| t > now_unix())
    }

    pub fn block(&self, resource: &'static str, until: u64) {
        if let Ok(mut map) = self.blocked_until.lock() {
            map.insert(resource, until);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Veio da rede agora (inclui 304 revalidado).
    Network,
    /// Cache dentro da validade, sem consultar a rede.
    Cache,
    /// Rede falhou; usamos um cache vencido.
    StaleCache,
    /// Nada disponível.
    Missing,
}

#[derive(Debug, Clone)]
pub struct Outcome {
    /// `None` com origem válida = recurso inexistente (404).
    pub body: Option<String>,
    pub origin: Origin,
    pub fetched_at: Option<u64>,
    pub error: Option<String>,
    pub rate_limited_until: Option<u64>,
}

impl Outcome {
    fn missing(error: String, rate: Option<u64>) -> Self {
        Self {
            body: None,
            origin: Origin::Missing,
            fetched_at: None,
            error: Some(error),
            rate_limited_until: rate,
        }
    }
}

pub fn url_key(url: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in url.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("u{h:016x}")
}

pub struct Ctx<'a, F: Fetcher> {
    pub fetcher: &'a F,
    pub cache: &'a Cache,
    pub state: &'a EngineState,
    pub ttl_secs: u64,
    pub force: bool,
}

impl<F: Fetcher> Ctx<'_, F> {
    /// GET com cache. `resource` identifica o balde de rate limit do GitHub (ou `None`).
    pub async fn get(
        &self,
        req: HttpRequest,
        resource: Option<&'static str>,
        ttl_secs: u64,
    ) -> Outcome {
        let key = url_key(&req.url);
        let cached: Option<Entry<Option<String>>> = self.cache.get("providers", &key);
        if let Some(e) = &cached {
            if !self.force && e.is_fresh(ttl_secs) {
                return Outcome {
                    body: e.data.clone(),
                    origin: Origin::Cache,
                    fetched_at: Some(e.fetched_at),
                    error: None,
                    rate_limited_until: None,
                };
            }
        }
        let fallback = |error: String, rate: Option<u64>| match &cached {
            Some(e) => Outcome {
                body: e.data.clone(),
                origin: Origin::StaleCache,
                fetched_at: Some(e.fetched_at),
                error: Some(error),
                rate_limited_until: rate,
            },
            None => Outcome::missing(error, rate),
        };
        if let Some(res) = resource {
            if let Some(until) = self.state.blocked(res) {
                return fallback("limite de consultas do GitHub atingido".into(), Some(until));
            }
        }
        let etag = cached
            .as_ref()
            .filter(|e| e.data.is_some())
            .and_then(|e| e.etag.clone());
        match self.fetcher.get(req.with_etag(etag)).await {
            Ok(r) if r.status == 200 => {
                let body = String::from_utf8_lossy(&r.body).into_owned();
                let entry = Entry::new(Some(body.clone()), r.etag);
                let _ = self.cache.put("providers", &key, &entry);
                Outcome {
                    body: Some(body),
                    origin: Origin::Network,
                    fetched_at: Some(entry.fetched_at),
                    error: None,
                    rate_limited_until: None,
                }
            }
            Ok(r) if r.status == 304 => match cached {
                Some(mut e) => {
                    e.fetched_at = now_unix();
                    let _ = self.cache.put("providers", &key, &e);
                    Outcome {
                        body: e.data,
                        origin: Origin::Network,
                        fetched_at: Some(e.fetched_at),
                        error: None,
                        rate_limited_until: None,
                    }
                }
                None => Outcome::missing("resposta 304 sem cache local".into(), None),
            },
            Ok(r) if r.status == 404 => {
                let entry: Entry<Option<String>> = Entry::new(None, None);
                let _ = self.cache.put("providers", &key, &entry);
                Outcome {
                    body: None,
                    origin: Origin::Network,
                    fetched_at: Some(entry.fetched_at),
                    error: None,
                    rate_limited_until: None,
                }
            }
            Ok(r)
                if (r.status == 403 || r.status == 429)
                    && (r.rate_remaining == Some(0)
                        || r.retry_after.is_some()
                        || r.status == 429) =>
            {
                let until = r
                    .rate_reset
                    .or(r.retry_after.map(|s| now_unix() + s))
                    .unwrap_or(now_unix() + 60);
                if let Some(res) = resource {
                    self.state.block(res, until);
                }
                fallback("limite de consultas do GitHub atingido".into(), Some(until))
            }
            Ok(r) => fallback(format!("a fonte respondeu HTTP {}", r.status), None),
            Err(e) => fallback(e.to_string(), None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::mock::MockFetcher;
    use crate::net::{HttpResponse, NetError};

    const URL: &str = "https://raw.githubusercontent.com/o/r/master/a.py";

    fn ctx<'a>(
        f: &'a MockFetcher,
        c: &'a Cache,
        s: &'a EngineState,
        force: bool,
    ) -> Ctx<'a, MockFetcher> {
        Ctx {
            fetcher: f,
            cache: c,
            state: s,
            ttl_secs: 3600,
            force,
        }
    }

    #[tokio::test]
    async fn caches_and_serves_offline() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let st = EngineState::default();
        let f = MockFetcher::default();
        f.on(URL, 200, "conteúdo");
        let r = ctx(&f, &cache, &st, false)
            .get(HttpRequest::new(URL, "*/*", 1000), None, 3600)
            .await;
        assert_eq!(
            (r.origin, r.body.as_deref()),
            (Origin::Network, Some("conteúdo"))
        );
        // Dentro da validade: nenhuma chamada nova.
        let r = ctx(&f, &cache, &st, false)
            .get(HttpRequest::new(URL, "*/*", 1000), None, 3600)
            .await;
        assert_eq!(r.origin, Origin::Cache);
        assert_eq!(f.count(URL), 1);
        // Forçado e sem rede: usa o cache vencido e informa o erro.
        f.fail(URL, NetError::Timeout);
        let r = ctx(&f, &cache, &st, true)
            .get(HttpRequest::new(URL, "*/*", 1000), None, 3600)
            .await;
        assert_eq!(r.origin, Origin::StaleCache);
        assert_eq!(r.body.as_deref(), Some("conteúdo"));
        assert!(r.error.unwrap().contains("tempo limite"));
    }

    #[tokio::test]
    async fn not_found_is_cached_and_missing_without_cache_is_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let st = EngineState::default();
        let f = MockFetcher::default();
        let r = ctx(&f, &cache, &st, false)
            .get(HttpRequest::new(URL, "*/*", 1000), None, 3600)
            .await;
        assert_eq!((r.origin, r.body), (Origin::Network, None));
        let other = "https://raw.githubusercontent.com/o/r/master/b.py";
        f.fail(other, NetError::Offline("x".into()));
        let r = ctx(&f, &cache, &st, false)
            .get(HttpRequest::new(other, "*/*", 1000), None, 3600)
            .await;
        assert_eq!(r.origin, Origin::Missing);
    }

    #[tokio::test]
    async fn rate_limit_blocks_further_requests() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let st = EngineState::default();
        let f = MockFetcher::default();
        let api = "https://api.github.com/search/issues?q=1";
        f.on_response(
            api,
            HttpResponse {
                status: 403,
                rate_remaining: Some(0),
                rate_reset: Some(now_unix() + 600),
                ..Default::default()
            },
        );
        let r = ctx(&f, &cache, &st, false)
            .get(HttpRequest::new(api, "*/*", 1000), Some("search"), 3600)
            .await;
        assert!(r.rate_limited_until.is_some());
        let r = ctx(&f, &cache, &st, true)
            .get(HttpRequest::new(api, "*/*", 1000), Some("search"), 3600)
            .await;
        assert!(r.rate_limited_until.is_some());
        assert_eq!(f.count(api), 1, "não insiste enquanto estiver bloqueado");
    }

    #[tokio::test]
    async fn etag_revalidation_and_http_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let st = EngineState::default();
        let f = MockFetcher::default();
        f.on_response(
            URL,
            HttpResponse {
                status: 200,
                body: b"v1".to_vec(),
                etag: Some("\"e1\"".into()),
                ..Default::default()
            },
        );
        ctx(&f, &cache, &st, false)
            .get(HttpRequest::new(URL, "*/*", 1000), None, 3600)
            .await;
        f.on_response(
            URL,
            HttpResponse {
                status: 304,
                ..Default::default()
            },
        );
        let r = ctx(&f, &cache, &st, true)
            .get(HttpRequest::new(URL, "*/*", 1000), None, 3600)
            .await;
        assert_eq!((r.origin, r.body.as_deref()), (Origin::Network, Some("v1")));
        assert_eq!(
            f.calls.lock().unwrap().last().unwrap().etag.as_deref(),
            Some("\"e1\"")
        );
        f.on(URL, 502, "bad gateway");
        let r = ctx(&f, &cache, &st, true)
            .get(HttpRequest::new(URL, "*/*", 1000), None, 3600)
            .await;
        assert_eq!(r.origin, Origin::StaleCache);
        assert!(r.error.unwrap().contains("502"));
    }

    #[tokio::test]
    async fn oversized_responses_are_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let st = EngineState::default();
        let f = MockFetcher::default();
        f.on(URL, 200, &"x".repeat(5000));
        let r = ctx(&f, &cache, &st, false)
            .get(HttpRequest::new(URL, "*/*", 100), None, 3600)
            .await;
        assert_eq!(r.origin, Origin::Missing);
    }
}
