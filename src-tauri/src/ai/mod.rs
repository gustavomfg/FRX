//! Pesquisa assistida do FRX. Esta etapa tem reescrita de consulta, provedores
//! mockados e o ranker, sem rede. Nenhuma saída daqui aplica correção.

pub mod driver;
pub mod embedder;
pub mod models;
pub mod query;
#[cfg(any(test, feature = "qwen-query"))]
pub mod qwen;
pub mod ranker;

use embedder::{EmbedError, LexicalEmbedder};
use models::*;
use query::{Deterministic, ProviderKind, QueryError, QueryInput, QueryRewriter};

/// Candidatos sintéticos, marcados como `[MOCK]`, até os provedores serem ligados.
/// Não descrevem nenhuma correção real e não devem ser exibidos como resultado.
pub fn mock_candidates(appid: u32) -> Vec<Candidate> {
    let issue = |n: u32| format!("https://github.com/ValveSoftware/Proton/issues/{n}");
    let candidate = |source, url, title: &str, text: &str, date: &str| Candidate {
        source,
        url,
        title: format!("[MOCK] {title}"),
        snippet: text.to_owned(),
        raw_text: text.to_owned(),
        published_at: Some(date.to_owned()),
    };
    vec![
        candidate(
            SourceKind::GithubComment,
            issue(appid),
            "Relato sintético em GPU NVIDIA",
            "Crash on launch stopped on my RTX 4070 with driver 580.65 and GE-Proton10-4.",
            "2026-06-01T00:00:00Z",
        ),
        candidate(
            SourceKind::GithubComment,
            issue(appid + 1),
            "Relato sintético em GPU AMD",
            "It works on my Radeon RX 7800 XT.",
            "2026-06-01T00:00:00Z",
        ),
        candidate(
            SourceKind::ReleaseNotes,
            "https://github.com/ValveSoftware/Proton/releases".to_owned(),
            "Nota de versão sintética",
            "Crash on launch fixed in Proton 10.0-3.",
            "2026-03-01T00:00:00Z",
        ),
    ]
}

/// O provedor mockado ignora o texto da consulta: só separa os candidatos por fonte.
fn mock_provider(provider: ProviderKind, appid: u32) -> Vec<Candidate> {
    let matches = |source: SourceKind| match provider {
        ProviderKind::Umu => source == SourceKind::Umu,
        ProviderKind::Github => source == SourceKind::GithubComment,
        ProviderKind::ReleaseNotes => source == SourceKind::ReleaseNotes,
        ProviderKind::Reddit => source == SourceKind::Reddit,
        ProviderKind::Blog => source == SourceKind::Blog,
    };
    mock_candidates(appid)
        .into_iter()
        .filter(|c| matches(c.source))
        .collect()
}

#[derive(Debug)]
pub enum ResearchError {
    Query(QueryError),
    Rank(EmbedError),
}

impl std::fmt::Display for ResearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResearchError::Query(e) => write!(f, "{e}"),
            ResearchError::Rank(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ResearchError {}

/// Consulta por provedor, candidatos mockados e ranker.
pub fn research_with(
    query: &ResearchQuery,
    rewriter: &dyn QueryRewriter,
    now_unix: u64,
) -> Result<Vec<ScoredCandidate>, ResearchError> {
    let rewritten = rewriter
        .rewrite(&QueryInput::from(query))
        .map_err(ResearchError::Query)?;
    let candidates: Vec<Candidate> = rewritten
        .iter()
        .flat_map(|r| mock_provider(r.provider, query.appid))
        .collect();
    ranker::rank(query, &candidates, &LexicalEmbedder, now_unix).map_err(ResearchError::Rank)
}

/// A pesquisa com a reescrita determinística e os provedores mockados.
pub fn research_mocked(
    query: &ResearchQuery,
    now_unix: u64,
) -> Result<Vec<ScoredCandidate>, ResearchError> {
    research_with(query, &Deterministic, now_unix)
}
