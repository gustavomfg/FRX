//! fwupd / LVFS — **esqueleto, ainda não implementado**.
//!
//! Plano: falar com o daemon local `fwupd` pelo D-Bus do sistema
//! (`org.freedesktop.fwupd`, métodos `GetDevices` e `GetUpgrades`) para listar
//! os dispositivos do laptop e as atualizações de firmware publicadas no LVFS.
//! Somente leitura: instalar firmware fica fora deste provedor e, quando
//! existir, passa pelo Fix Engine com prévia e consentimento explícito.
//! Sem o daemon instalado/ativo, o estado é "indisponível", não erro.

use super::super::models::{ProviderId, ProviderInfo, ProviderState, ProviderStatus};
use super::{LaptopQuery, ProviderResult};

const NAME: &str = "fwupd (LVFS)";

pub fn info() -> ProviderInfo {
    ProviderInfo {
        id: ProviderId::Fwupd,
        name: NAME,
        enabled: false,
        description: "Dispositivos e atualizações de firmware conhecidos pelo fwupd/LVFS (somente leitura).",
        reason: Some("Ainda não implementado."),
    }
}

pub fn status() -> ProviderStatus {
    ProviderStatus {
        id: ProviderId::Fwupd,
        name: NAME.into(),
        state: ProviderState::Disabled,
        message: info().reason.map(str::to_owned),
        fetched_at: None,
        reset_at: None,
    }
}

/// Consulta o fwupd. Por enquanto não abre o D-Bus: só informa o estado.
pub fn search(_query: &LaptopQuery) -> ProviderResult {
    ProviderResult {
        status: status(),
        findings: Vec::new(),
        identity: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_is_disabled_and_returns_nothing() {
        let r = search(&LaptopQuery::fictional());
        assert_eq!(r.status.state, ProviderState::Disabled);
        assert!(r.findings.is_empty());
        assert!(!info().enabled);
    }
}
