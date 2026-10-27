//! Documents a resolution path without granting execution authority. Component
//! upgrade variables may download non-pinned binaries; removing the variable
//! cannot restore those files, so existing env support is insufficient consent.
use super::models::*;
use crate::fix_engine::models::GameCtx;

pub fn add_documented_reviews(
    game: &GameCtx,
    symptoms: &[String],
    knowledge: &[Evidence],
    problems: &mut Vec<Problem>,
    plans: &mut Vec<ResolutionPlan>,
) {
    for (symptom, technology, option, title) in [
        (
            "dlss",
            "dlss",
            "PROTON_DLSS_UPGRADE",
            "Revisar atualização de DLSS pelo GE-Proton",
        ),
        (
            "fsr",
            "fsr",
            "PROTON_FSR4_UPGRADE",
            "Revisar atualização de FSR pelo GE-Proton",
        ),
    ] {
        if !symptoms.iter().any(|s| s == symptom) {
            continue;
        }
        let Some(source) = knowledge.iter().find(|s| {
            s.kind == "official-documentation"
                && s.app_id.is_none()
                && s.publisher.as_deref() == Some("GloriousEggroll")
                && s.source_url.as_deref().is_some_and(|u| {
                    u.starts_with("https://github.com/GloriousEggroll/proton-ge-custom/blob/")
                })
                && s.summary.contains(option)
        }) else {
            continue;
        };
        let version_matches = source
            .version
            .as_deref()
            .is_some_and(|v| Some(v) == game.tool_name.as_deref());
        let version_limit = if version_matches {
            "Documento da versão selecionada encontrado; isso não confirma suporte do jogo, driver ou hardware."
        } else {
            "A documentação não corresponde à versão selecionada. Não aplicar opções exclusivas de GE-Proton em Proton Valve ou outra variante."
        };
        let id = format!("resolution.{}.review-{}", game.app_id, symptom);
        let problem_id = format!("review-component-{symptom}");
        let limitations=vec![version_limit.into(),"O mecanismo baixa componentes e pode fazer fallback de versão. Retirar a variável não restaura os arquivos atualizados; falta política de artefato fixado, backup e rollback desse mecanismo.".into()];
        plans.push(ResolutionPlan{
            id:id.clone(),app_id:game.app_id.clone(),title:title.into(),
            objective:format!("Avaliar o mecanismo {option} documentado pelo mantenedor antes de substituir bibliotecas manualmente."),
            problem_id:problem_id.clone(),recipe_id:None,state:"review".into(),
            actions:vec![format!("Revisar requisitos e efeitos documentados de {option}; nenhuma alteração está autorizada por este plano.")],
            dependencies:vec!["Implementação compatível no jogo, versão exata de GE-Proton, GPU e driver compatíveis.".into(),if symptom=="fsr"{"Confirmar implementação FSR 3.1 compatível e requisitos da versão de FSR/manifesto escolhida; FSR não é exclusivo da AMD."}else{"Confirmar suporte do jogo a DLSS e compatibilidade de NGX/NVAPI no ambiente."}.into()],
            risks:vec!["Atualização de bibliotecas executáveis; origem e checksum não garantem benignidade nem desempenho.".into()],
            sources:vec![source.clone()],reversible:false,
            verification_method:"Confirmar componente/versão e observar o recurso em execução por método específico; FPS isolado não comprova funcionamento.".into(),
            success_criterion:"Recurso observado funcionando no jogo e ambiente correspondente; download concluído não satisfaz o critério.".into(),
            operation_id:None,verification:"Revisão necessária; nenhuma atualização executada.".into(),limitations:limitations.clone(),
        });
        problems.push(Problem{id:problem_id,technology:technology.into(),symptom:format!("Investigar mecanismo de atualização de {technology}"),certainty:"hypothesis".into(),evidence:vec![source.clone()],hypotheses:vec!["Uma atualização documentada pode ser relevante, mas a causa local e os requisitos precisam ser confirmados.".into()],ruled_out:vec![],limitations,next_action:"Consultar o documento da versão e os impedimentos do plano. Nenhuma atualização automática é oferecida sem reversibilidade.".into(),plan_ids:vec![id]});
    }
}
