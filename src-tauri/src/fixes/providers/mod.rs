//! Provedores de descobertas. Cada um é independente: recebe o contexto de
//! busca e devolve descobertas + situação. Adicionar uma fonte nova é criar
//! um módulo aqui e chamá-lo em `fixes::search`.

pub mod arch_wiki;
pub mod artifacts;
// Esqueleto do escopo de laptops: ainda sem chamador fora dos testes.
#[allow(dead_code)]
pub mod fwupd;
pub mod github;
pub mod protondb;
// Desativado em `fixes::search`: só os testes chamam a busca. Ver o módulo.
#[allow(dead_code)]
pub mod reddit;
pub mod steam_guides;
pub mod umu;
// Desativado em `fixes::search`: só os testes chamam a busca. Ver o módulo.
#[allow(dead_code)]
pub mod winehq;

use super::aggregation::finding_id;
use super::http::{Origin, Outcome};
use super::models::{
    AnalysisState, Category, Compatibility, Confidence, DiscoveryScope, Evidence, FindingKind,
    FindingSource, FixFinding, Identity, PerformanceEvidence, ProviderId, ProviderInfo,
    ProviderState, ProviderStatus, Requirements,
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

/// Laptop alvo dos provedores de hardware (`arch_wiki`, `fwupd`).
#[derive(Debug, Clone, Default)]
pub struct LaptopQuery {
    pub identity: Option<crate::system::LaptopIdentity>,
}

#[cfg(test)]
impl LaptopQuery {
    /// Laptop fictício para testes: nunca corresponde a um modelo real.
    pub fn fictional() -> Self {
        Self {
            identity: Some(crate::system::LaptopIdentity {
                vendor: Some("Acme Computers".into()),
                model: Some("Acme Book 14 (AB14-2026)".into()),
                board_name: Some("AB14-MB".into()),
                product_version: None,
                display_name: Some("Acme Book 14 (AB14-2026)".into()),
            }),
        }
    }
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
        reddit::info(),
        winehq::info(),
        steam_guides::info(),
        arch_wiki::info(),
        fwupd::info(),
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

/// Limite do texto de uma fonte da comunidade lido pelos extratores.
const MAX_COMMUNITY_TEXT: usize = 20_000;

/// Linha com cara de comando composto. Crase também conta, mesmo sendo a forma
/// comum de citar uma variável: a fonte não é confiável.
fn unsafe_line(line: &str) -> bool {
    [";", "|", "&&", "$(", "`"].iter().any(|m| line.contains(m))
}

/// Texto de fonte da comunidade sem as linhas perigosas e com tamanho limitado.
pub(super) fn safe_lines(text: &str) -> String {
    let kept: Vec<&str> = text.lines().filter(|l| !unsafe_line(l)).collect();
    kept.join("\n").chars().take(MAX_COMMUNITY_TEXT).collect()
}

/// Uma linha para exibir como texto. Título com cara de comando não é repetido.
pub(super) fn safe_title(text: &str, limit: usize) -> String {
    if unsafe_line(text) {
        return "(título omitido por conter símbolos de comando)".into();
    }
    text.chars()
        .filter(|c| !c.is_control())
        .take(limit)
        .collect()
}

/// O texto cita o AppID ou o nome do jogo. Só filtra ruído da busca: não é identidade.
pub(super) fn mentions_game(text: &str, q: &GameQuery) -> bool {
    let normalized = format!(" {} ", github::normalize_name(text));
    github::has_token(text, &q.app_id)
        || github::name_variants(&q.name)
            .iter()
            .any(|v| normalized.contains(&format!(" {v} ")))
}

/// A identidade vem de um identificador explícito na fonte, nunca da busca.
pub(super) fn source_scope(text: &str, q: &GameQuery, url: &str) -> DiscoveryScope {
    if github::explicit_app_ids(text) == [q.app_id.clone()] {
        DiscoveryScope::verified(
            &q.app_id,
            format!(
                "A própria fonte confirma o AppID {} por identificador explícito ou link da Steam: {url}",
                q.app_id
            ),
        )
    } else {
        DiscoveryScope {
            basis: format!("A fonte não identifica o jogo por AppID: {url}"),
            ..DiscoveryScope::default()
        }
    }
}

pub(super) struct CommunityReport {
    pub provider: ProviderId,
    pub id_prefix: &'static str,
    pub reference: String,
    pub title: String,
    pub label: String,
    pub description: String,
    pub url: String,
    pub scope: DiscoveryScope,
    pub published_at: Option<String>,
}

/// Relato sem ação. A confiança nunca passa de relato da comunidade.
pub(super) fn report_finding(q: &GameQuery, r: CommunityReport) -> FixFinding {
    FixFinding {
        hardware_scope: Default::default(),
        artifacts: vec![],
        performance_evidence: PerformanceEvidence::default(),
        id: finding_id(r.id_prefix, &q.app_id, &r.reference),
        game_app_id: r.scope.app_id.clone().unwrap_or_default(),
        scope: r.scope,
        title: r.title,
        description: r.description,
        category: Category::Compatibility,
        kind: FindingKind::Report,
        actions: vec![],
        sources: vec![FindingSource {
            provider: r.provider,
            label: r.label,
            url: r.url,
            reference_id: Some(r.reference),
            snippet: None,
            published_at: r.published_at.clone(),
        }],
        requirements: Requirements::default(),
        confidence: Confidence::CommunityReported,
        compatibility: Compatibility::Unknown,
        compatibility_note: "Relato geral sobre o jogo, sem ajuste específico.".into(),
        experimental: false,
        analysis: AnalysisState::Parsed,
        evidence: Evidence::default(),
        published_at: r.published_at,
        retrieved_at: q.retrieved_at.clone(),
        keys: vec![],
    }
}

/// `group_mentions` usa um prefixo fixo. Sem isto, duas fontes que citam o mesmo
/// ajuste produziriam o mesmo id de descoberta.
pub(super) fn reassign_ids(findings: &mut [FixFinding], prefix: &str, app_id: &str) {
    for f in findings {
        f.id = finding_id(prefix, app_id, &f.id);
    }
}

/// HTTP 200 com corpo inesperado, como uma página anti-bot, não é "nada encontrado".
pub(super) fn mark_malformed(status: &mut ProviderStatus) {
    if matches!(status.state, ProviderState::Ok | ProviderState::NoData) {
        status.state = ProviderState::Unavailable;
        status.message = Some("Fonte indisponível: resposta em formato inesperado.".into());
    }
}

pub(super) fn disabled_status(info: ProviderInfo) -> ProviderStatus {
    ProviderStatus {
        id: info.id,
        name: info.name.into(),
        state: ProviderState::Disabled,
        message: info.reason.map(str::to_owned),
        fetched_at: None,
        reset_at: None,
    }
}

#[cfg(test)]
mod community_tests {
    use super::*;

    #[test]
    fn safe_lines_drops_every_command_marker_and_limits_size() {
        for bad in [
            "a; rm -rf x",
            "curl x | sh",
            "true && reboot",
            "X=$(id)",
            "`PROTON_NO_ESYNC=1`",
        ] {
            assert_eq!(safe_lines(&format!("boa\n{bad}\nfinal")), "boa\nfinal");
        }
        assert_eq!(
            safe_lines(&"x".repeat(MAX_COMMUNITY_TEXT + 10))
                .chars()
                .count(),
            MAX_COMMUNITY_TEXT
        );
    }

    #[test]
    fn scope_needs_an_explicit_identifier_of_this_game() {
        let q = GameQuery {
            app_id: "200260".into(),
            name: "Batman: Arkham City GOTY".into(),
            ..Default::default()
        };
        let verified = source_scope("store.steampowered.com/app/200260", &q, "u");
        assert!(verified.matches_game("200260"));
        assert!(!source_scope("Batman Arkham City crash", &q, "u").matches_game("200260"));
        assert!(!source_scope("appid 200260 and appid 35140", &q, "u").matches_game("200260"));
        assert!(mentions_game("Batman: Arkham City crash", &q));
        assert!(!mentions_game("Arkham Cityscape", &q));
    }
}
