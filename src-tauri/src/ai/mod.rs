//! Pesquisa assistida do FRX. Esta etapa tem só o ranker determinístico, sem
//! LLM e sem rede. Nenhuma saída daqui toca o disco ou aplica correção.

pub mod driver;
pub mod embedder;
pub mod models;
pub mod ranker;

use embedder::{EmbedError, LexicalEmbedder};
use models::*;

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

/// O que `research_fix` faz hoje: candidatos mockados e similaridade léxica.
pub fn research_mocked(
    query: &ResearchQuery,
    now_unix: u64,
) -> Result<Vec<ScoredCandidate>, EmbedError> {
    ranker::rank(
        query,
        &mock_candidates(query.appid),
        &LexicalEmbedder,
        now_unix,
    )
}
