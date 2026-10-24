//! ProtonDB — **desativado**.
//!
//! O ProtonDB não oferece API pública documentada. Os endpoints usados pelo
//! site são internos e podem mudar sem aviso, e os relatos têm licença e
//! termos próprios. Até existir uma integração autorizada, o FRX não consulta
//! o serviço: só oferece um link para a página do jogo, aberto no navegador.

use super::super::models::{ProviderId, ProviderInfo, ProviderState, ProviderStatus};

pub fn info() -> ProviderInfo {
    ProviderInfo {
        id: ProviderId::Protondb,
        name: "ProtonDB",
        enabled: false,
        description: "Relatos da comunidade sobre compatibilidade.",
        reason: Some("Sem API pública oficial; o FRX não usa endpoints internos nem raspa o site. A página do jogo pode ser aberta no navegador."),
    }
}

pub fn status() -> ProviderStatus {
    ProviderStatus {
        id: ProviderId::Protondb,
        name: "ProtonDB".into(),
        state: ProviderState::Disabled,
        message: info().reason.map(str::to_owned),
        fetched_at: None,
        reset_at: None,
    }
}
