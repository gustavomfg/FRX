//! Proveniência: de onde veio cada ação. Só fontes rastreáveis entram — URL
//! HTTPS do GitHub, de um provedor integrado. O trecho original é guardado
//! como texto (sem controle) e nunca é interpretado como instrução.

use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::fixes::models::{FindingSource, FixFinding, ProviderId};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Provenance {
    pub provider: ProviderId,
    pub label: String,
    pub url: String,
    pub snippet: Option<String>,
    pub published_at: Option<String>,
}

const TRUSTED_HOSTS: &[&str] = &["github.com"];

pub fn trusted_url(u: &str) -> bool {
    Url::parse(u).is_ok_and(|x| {
        x.scheme() == "https"
            && x.username().is_empty()
            && x.password().is_none()
            && x.port().is_none()
            && x.host_str().is_some_and(|h| TRUSTED_HOSTS.contains(&h))
    })
}

fn trusted(s: &FindingSource) -> bool {
    matches!(s.provider, ProviderId::Github | ProviderId::UmuProtonfixes) && trusted_url(&s.url)
}

fn clean(s: &str, max: usize) -> String {
    let t: String = s.chars().filter(|c| !c.is_control()).take(max).collect();
    t.trim().to_owned()
}

/// Fontes rastreáveis da descoberta (as outras são descartadas).
pub fn collect(f: &FixFinding) -> Vec<Provenance> {
    f.sources
        .iter()
        .filter(|s| trusted(s))
        .take(6)
        .map(|s| Provenance {
            provider: s.provider,
            label: clean(&s.label, 120),
            url: s.url.clone(),
            snippet: s.snippet.as_deref().map(|x| clean(x, 240)),
            published_at: s
                .published_at
                .as_deref()
                .filter(|d| d.len() <= 40)
                .map(str::to_owned),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_https_github_is_trusted() {
        assert!(trusted_url(
            "https://github.com/ValveSoftware/Proton/issues/309#issuecomment-1"
        ));
        for bad in [
            "http://github.com/x",
            "https://github.com.evil.example/x",
            "https://evil.example/github.com",
            "https://user@github.com/x",
            "https://github.com:8443/x",
            "file:///etc/passwd",
            "javascript:alert(1)",
        ] {
            assert!(!trusted_url(bad), "{bad}");
        }
    }
}
