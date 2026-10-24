//! Provedores de descobertas. Cada um é independente: recebe o contexto de
//! busca e devolve descobertas + situação. Adicionar uma fonte nova é criar
//! um módulo aqui e chamá-lo em `fixes::search`.

pub mod github;
pub mod protondb;
pub mod umu;

use super::http::{Origin, Outcome};
use super::models::{
    FixFinding, Identity, ProviderId, ProviderInfo, ProviderState, ProviderStatus,
};
use crate::steam::models::CompatTool;
use crate::time::iso8601;

pub struct ProviderResult {
    pub status: ProviderStatus,
    pub findings: Vec<FixFinding>,
    pub identity: Option<Identity>,
}

/// Dados do jogo e do sistema disponíveis para os provedores.
#[derive(Debug, Clone, Default)]
pub struct GameQuery {
    pub app_id: String,
    pub name: String,
    pub tool: Option<CompatTool>,
    pub retrieved_at: String,
}

pub fn catalog() -> Vec<ProviderInfo> {
    vec![
        ProviderInfo {
            id: ProviderId::UmuProtonfixes,
            name: "UMU-Protonfixes",
            enabled: true,
            description: "Correções por jogo mantidas pela Open Wine Components (lidas como texto, nunca executadas).",
            reason: None,
        },
        ProviderInfo {
            id: ProviderId::UmuDatabase,
            name: "UMU Database",
            enabled: true,
            description: "Identidade UMU e nomes alternativos do jogo em outras lojas.",
            reason: None,
        },
        ProviderInfo {
            id: ProviderId::Github,
            name: "GitHub",
            enabled: true,
            description: "Issues com o AppID no título (Proton, GE-Proton, UMU-Protonfixes) e notas de versão oficiais.",
            reason: Some("API pública sem token: até 10 buscas/min e 60 consultas/h. Defina FRX_GITHUB_TOKEN no ambiente para limites maiores."),
        },
        protondb::info(),
    ]
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
