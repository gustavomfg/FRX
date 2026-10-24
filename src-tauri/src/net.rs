//! Cliente HTTP restrito.
//!
//! - só HTTPS e só para uma lista fechada de hosts (inclusive em redirecionamentos);
//! - tempo limite e tamanho máximo de resposta;
//! - User-Agent identificado;
//! - token do GitHub opcional, lido de `FRX_GITHUB_TOKEN` apenas no Rust.
//!
//! O trait [`Fetcher`] permite substituir a rede por respostas falsas nos testes.

use std::future::Future;
use std::time::Duration;

use reqwest::Url;

pub const USER_AGENT: &str = concat!(
    "FRX/",
    env!("CARGO_PKG_VERSION"),
    " (Linux game compatibility helper; read-only)"
);

pub const ALLOWED_HOSTS: &[&str] = &[
    "api.github.com",
    "raw.githubusercontent.com",
    "github.com",
    "shared.akamai.steamstatic.com",
    "cdn.cloudflare.steamstatic.com",
    "cdn.akamai.steamstatic.com",
    "steamcdn-a.akamaihd.net",
];

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub url: String,
    pub accept: &'static str,
    pub etag: Option<String>,
    pub max_bytes: usize,
}

impl HttpRequest {
    pub fn new(url: impl Into<String>, accept: &'static str, max_bytes: usize) -> Self {
        Self {
            url: url.into(),
            accept,
            etag: None,
            max_bytes,
        }
    }

    pub fn with_etag(mut self, etag: Option<String>) -> Self {
        self.etag = etag;
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
    pub etag: Option<String>,
    pub rate_remaining: Option<u64>,
    pub rate_reset: Option<u64>,
    pub retry_after: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NetError {
    Timeout,
    Offline(String),
    TooLarge,
    NotAllowed(String),
    Other(String),
}

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetError::Timeout => write!(f, "tempo limite esgotado"),
            NetError::Offline(e) => write!(f, "sem conexão ({e})"),
            NetError::TooLarge => write!(f, "resposta maior que o limite permitido"),
            NetError::NotAllowed(u) => write!(f, "endereço não permitido: {u}"),
            NetError::Other(e) => write!(f, "{e}"),
        }
    }
}

pub trait Fetcher: Send + Sync {
    fn get(&self, req: HttpRequest) -> impl Future<Output = Result<HttpResponse, NetError>> + Send;
}

/// Aceita só HTTPS para hosts da lista.
pub fn is_allowed(url: &str) -> bool {
    match Url::parse(url) {
        Ok(u) => {
            u.scheme() == "https"
                && u.host_str().is_some_and(|h| ALLOWED_HOSTS.contains(&h))
                && u.username().is_empty()
        }
        Err(_) => false,
    }
}

pub struct ReqwestFetcher {
    client: reqwest::Client,
    github_token: Option<String>,
}

impl ReqwestFetcher {
    pub fn new() -> Result<Self, String> {
        let policy = reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 || !is_allowed(attempt.url().as_str()) {
                attempt.stop()
            } else {
                attempt.follow()
            }
        });
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(6))
            .redirect(policy)
            .https_only(true)
            .build()
            .map_err(|e| e.to_string())?;
        let github_token = std::env::var("FRX_GITHUB_TOKEN")
            .ok()
            .map(|t| t.trim().to_owned())
            .filter(|t| !t.is_empty());
        Ok(Self {
            client,
            github_token,
        })
    }
}

fn header_u64(h: &reqwest::header::HeaderMap, k: &str) -> Option<u64> {
    h.get(k)?.to_str().ok()?.trim().parse().ok()
}

impl Fetcher for ReqwestFetcher {
    async fn get(&self, req: HttpRequest) -> Result<HttpResponse, NetError> {
        if !is_allowed(&req.url) {
            return Err(NetError::NotAllowed(req.url));
        }
        let mut rb = self
            .client
            .get(&req.url)
            .header(reqwest::header::ACCEPT, req.accept);
        if let Some(tag) = &req.etag {
            rb = rb.header(reqwest::header::IF_NONE_MATCH, tag);
        }
        if req.url.starts_with("https://api.github.com/") {
            rb = rb.header("X-GitHub-Api-Version", "2022-11-28");
            if let Some(t) = &self.github_token {
                rb = rb.bearer_auth(t);
            }
        }
        let mut resp = rb.send().await.map_err(map_err)?;
        let h = resp.headers().clone();
        if resp
            .content_length()
            .is_some_and(|l| l as usize > req.max_bytes)
        {
            return Err(NetError::TooLarge);
        }
        let mut body = Vec::new();
        while let Some(chunk) = resp.chunk().await.map_err(map_err)? {
            if body.len() + chunk.len() > req.max_bytes {
                return Err(NetError::TooLarge);
            }
            body.extend_from_slice(&chunk);
        }
        Ok(HttpResponse {
            status: resp.status().as_u16(),
            body,
            etag: h
                .get(reqwest::header::ETAG)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned),
            rate_remaining: header_u64(&h, "x-ratelimit-remaining"),
            rate_reset: header_u64(&h, "x-ratelimit-reset"),
            retry_after: header_u64(&h, "retry-after"),
        })
    }
}

fn map_err(e: reqwest::Error) -> NetError {
    if e.is_timeout() {
        NetError::Timeout
    } else if e.is_connect() {
        NetError::Offline(e.to_string())
    } else {
        NetError::Other(e.to_string())
    }
}

#[cfg(test)]
pub mod mock {
    //! Fetcher falso para testes: respostas por URL e registro das chamadas.
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct MockFetcher {
        pub routes: Mutex<HashMap<String, Result<HttpResponse, NetError>>>,
        pub calls: Mutex<Vec<HttpRequest>>,
    }

    impl MockFetcher {
        pub fn on(&self, url: &str, status: u16, body: &str) -> &Self {
            self.routes.lock().unwrap().insert(
                url.into(),
                Ok(HttpResponse {
                    status,
                    body: body.as_bytes().to_vec(),
                    ..Default::default()
                }),
            );
            self
        }
        pub fn on_response(&self, url: &str, r: HttpResponse) -> &Self {
            self.routes.lock().unwrap().insert(url.into(), Ok(r));
            self
        }
        pub fn fail(&self, url: &str, e: NetError) -> &Self {
            self.routes.lock().unwrap().insert(url.into(), Err(e));
            self
        }
        pub fn count(&self, prefix: &str) -> usize {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .filter(|r| r.url.starts_with(prefix))
                .count()
        }
    }

    impl Fetcher for MockFetcher {
        async fn get(&self, req: HttpRequest) -> Result<HttpResponse, NetError> {
            if !is_allowed(&req.url) {
                return Err(NetError::NotAllowed(req.url));
            }
            self.calls.lock().unwrap().push(req.clone());
            let r = self.routes.lock().unwrap().get(&req.url).cloned();
            let r = r.unwrap_or_else(|| {
                Ok(HttpResponse {
                    status: 404,
                    ..Default::default()
                })
            })?;
            if r.body.len() > req.max_bytes {
                return Err(NetError::TooLarge);
            }
            Ok(r)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_https_allowlisted_hosts() {
        assert!(is_allowed("https://api.github.com/search/issues?q=1"));
        assert!(is_allowed("https://raw.githubusercontent.com/a/b/c.py"));
        assert!(!is_allowed("http://api.github.com/"));
        assert!(!is_allowed("https://evil.example.com/"));
        assert!(!is_allowed("https://api.github.com.evil.com/"));
        assert!(!is_allowed("https://user@api.github.com/"));
        assert!(!is_allowed("file:///etc/passwd"));
        assert!(!is_allowed("não é url"));
    }
}
