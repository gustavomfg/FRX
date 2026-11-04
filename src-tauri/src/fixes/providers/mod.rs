//! Fontes de hardware. Cada uma recebe o contexto da busca e devolve as
//! sugestões com a situação da consulta. Hoje há só a Arch Wiki.

pub mod arch_wiki;

use super::http::Origin;
use super::http::Outcome;
use super::models::{ProviderId, ProviderInfo, ProviderState, ProviderStatus};
use crate::time::iso8601;

/// Laptop alvo da busca, lido do DMI.
#[derive(Debug, Clone, Default)]
pub struct LaptopQuery {
    pub identity: Option<crate::system::LaptopIdentity>,
}

pub fn catalog() -> Vec<ProviderInfo> {
    vec![arch_wiki::info()]
}

/// Consolida o estado de várias requisições de um provedor.
pub fn summarize(
    id: ProviderId,
    name: &str,
    outcomes: &[&Outcome],
    found: usize,
) -> ProviderStatus {
    let any = |o: Origin| outcomes.iter().any(|x| x.origin == o);
    let rate = outcomes.iter().filter_map(|o| o.rate_limited_until).max();
    let error = outcomes.iter().find_map(|o| o.error.clone());
    let fetched = outcomes.iter().filter_map(|o| o.fetched_at).max();
    let state = if outcomes.is_empty() {
        ProviderState::NoData
    } else if rate.is_some() && !any(Origin::Network) && !any(Origin::Cache) {
        ProviderState::RateLimited
    } else if outcomes.iter().all(|o| o.origin == Origin::Missing) {
        ProviderState::Unavailable
    } else if any(Origin::StaleCache) || (any(Origin::Missing) && error.is_some()) {
        ProviderState::Stale
    } else if found == 0 {
        ProviderState::NoData
    } else if any(Origin::Cache) && !any(Origin::Network) {
        ProviderState::Cached
    } else {
        ProviderState::Ok
    };
    let message = match state {
        ProviderState::RateLimited => {
            Some("Limite de consultas atingido; tente mais tarde.".to_owned())
        }
        ProviderState::Unavailable => error.map(|e| format!("Fonte indisponível: {e}")),
        ProviderState::Stale => Some(format!(
            "Usando dados salvos — {}",
            error.unwrap_or_else(|| "consulta incompleta".into())
        )),
        ProviderState::Cached => Some("Resultado do cache local, dentro da validade.".to_owned()),
        _ => None,
    };
    ProviderStatus {
        id,
        name: name.to_owned(),
        state,
        message,
        fetched_at: fetched.map(iso8601),
        reset_at: rate.map(iso8601),
    }
}
