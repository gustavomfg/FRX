//! Situação das fontes consultadas.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderId {
    ArchWiki,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderState {
    Ok,
    Cached,
    Stale,
    NoData,
    Unavailable,
    RateLimited,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatus {
    pub id: ProviderId,
    pub name: String,
    pub state: ProviderState,
    pub message: Option<String>,
    pub fetched_at: Option<String>,
    pub reset_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: ProviderId,
    pub name: &'static str,
    pub enabled: bool,
    pub description: &'static str,
    pub reason: Option<&'static str>,
}
