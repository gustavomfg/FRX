//! Guias da Steam — **desativado**.
//!
//! `IPublishedFileService/QueryFiles` exige uma chave da Web API, e um app desktop
//! não tem como guardá-la em segredo. Sem chave, a API responde 403. O FRX também
//! não raspa as páginas da comunidade.

use super::super::models::{ProviderId, ProviderInfo, ProviderStatus};

pub fn info() -> ProviderInfo {
    ProviderInfo {
        id: ProviderId::SteamGuides,
        name: "Guias da Steam",
        enabled: false,
        description: "Guias publicados pela comunidade da Steam.",
        reason: Some("A busca de guias exige chave da Web API da Steam; o FRX não tem chave nem raspa as páginas."),
    }
}

pub fn status() -> ProviderStatus {
    super::disabled_status(info())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixes::models::ProviderState;

    #[test]
    fn stays_disabled_with_a_reason() {
        let status = status();
        assert_eq!(status.state, ProviderState::Disabled);
        assert!(status.message.unwrap().contains("chave"));
        assert!(!info().enabled);
    }
}
