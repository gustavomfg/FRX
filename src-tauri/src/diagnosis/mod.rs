//! Diagnóstico por regras, acionado só pelo botão Diagnosticar.
//!
//! `diagnose` é uma função pura: recebe contexto, descobertas e a avaliação do
//! Fix Engine e devolve sugestões. Não acessa rede ou disco e não altera nada.
//! Nenhum componente aprendido participa. Ações vêm dos campos tipados das
//! descobertas e dos validadores existentes; o texto das fontes só é citado.
//! Aplicar continua sendo prévia, confirmação e execução no Fix Engine.

pub mod assemble;
pub mod claims;
pub mod dedup;
pub mod gates;
pub mod mapping;
pub mod models;
pub mod normalize;

#[cfg(test)]
mod tests;

use crate::fixes::models::ProviderState;
use models::*;

fn sources_failed(input: &DiagnosisInput) -> bool {
    input.providers.iter().any(|p| {
        matches!(
            p.state,
            ProviderState::Unavailable | ProviderState::RateLimited
        )
    })
}

pub fn diagnose(input: DiagnosisInput) -> DiagnosisResult {
    let mut candidates = Vec::new();
    let mut no_action = 0usize;
    let mut unattributed = false;
    for view in &input.assessments {
        let Some(f) = input
            .findings
            .iter()
            .find(|f| f.id == view.assessment.finding_id)
        else {
            continue;
        };
        let c = mapping::candidate(view, f, &input.app_id);
        if c.mapping == Mapping::NoAction {
            no_action += 1;
            continue;
        }
        unattributed |= c.unattributed_reports;
        candidates.push(c);
    }
    candidates.extend(input.catalog.iter().map(mapping::from_catalog));
    let grouped = dedup::group(candidates, &input);

    let mut limitations = vec![
        "As sugestões resumem o que as fontes relatam. Nada foi verificado neste computador."
            .to_owned(),
        "A leitura das afirmações usa regras e foi medida só em inglês, sem revisão humana independente."
            .to_owned(),
    ];
    if input.context.gpu_vendor.is_none() {
        limitations.push("A GPU não foi identificada.".into());
    }
    if !input.context.install_known {
        limitations.push(
            "A instalação do jogo não foi encontrada; os arquivos não foram conferidos.".into(),
        );
    }
    if no_action > 0 {
        limitations.push(format!(
            "{no_action} relato(s) não trazem ação técnica e ficaram de fora."
        ));
    }
    if unattributed {
        limitations.push(
            "Alguns relatos contados pelas fontes não puderam ser ligados a um trecho e não contam como evidência."
                .into(),
        );
    }
    for p in &input.providers {
        if matches!(
            p.state,
            ProviderState::Unavailable | ProviderState::RateLimited
        ) {
            limitations.push(format!("{} não pôde ser consultado agora.", p.name));
        } else if p.state == ProviderState::Stale {
            limitations.push(format!("{} veio de um cache desatualizado.", p.name));
        }
    }

    let (verdict, message) = if !grouped.suggestions.is_empty() {
        (
            Verdict::Suggestions,
            "Estas configurações têm sustentação nas fontes. Revise a prévia antes de aplicar.",
        )
    } else if !grouped.in_review.is_empty() {
        (
            Verdict::OnlyInReview,
            "Há relatos, mas não evidência suficiente para sugerir uma correção aplicável.",
        )
    } else if sources_failed(&input) {
        (
            Verdict::SourcesUnavailable,
            "Não foi possível consultar todas as fontes. Isso não significa que não exista solução.",
        )
    } else {
        (
            Verdict::NoEvidence,
            "Não há evidência suficiente para sugerir uma correção. O FRX não inventa correções.",
        )
    };
    DiagnosisResult {
        app_id: input.app_id,
        symptom: input.symptom,
        verdict,
        message: message.into(),
        context: input.context,
        suggestions: grouped.suggestions,
        in_review: grouped.in_review,
        excluded: grouped.excluded,
        providers: input.providers,
        limitations,
        generated_at: input.generated_at,
        origin: Origin {
            method: "Regras e extração determinística".into(),
            learned_components: vec![],
        },
        automatic_changes: false,
    }
}
