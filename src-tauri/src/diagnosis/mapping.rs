//! Da análise do Fix Engine ao candidato. Quem decide se existe ação é o
//! planejador e os validadores existentes; aqui só se lê a decisão deles.

use super::models::{Candidate, Claim, ClaimKind, Mapping};
use super::{claims, gates, normalize};
use crate::fix_engine::availability::AvailabilityState;
use crate::fix_engine::dynamic::{AssessmentView, DynStatus};
use crate::fixes::models::{Compatibility, FixFinding};

fn first_reason(view: &AssessmentView, fallback: &str) -> String {
    view.availability
        .reasons
        .first()
        .map(|r| r.message.clone())
        .or_else(|| view.assessment.reasons.first().cloned())
        .unwrap_or_else(|| fallback.to_owned())
}

fn mapping(view: &AssessmentView, f: &FixFinding, app_id: &str) -> Mapping {
    if f.actions.is_empty() || view.assessment.status == DynStatus::NoActions {
        return Mapping::NoAction;
    }
    if f.actions.iter().any(gates::command_like) {
        return Mapping::Review {
            code: "command-rejected".into(),
            reason: "A fonte descreve um comando. Texto de fonte nunca autoriza execução, e o FRX não gera receita a partir disso.".into(),
        };
    }
    if !f.scope.matches_game(app_id) {
        return Mapping::Review {
            code: "game-unverified".into(),
            reason: "A fonte não confirma que o relato é sobre este jogo.".into(),
        };
    }
    if view.assessment.status == DynStatus::AlreadyIncluded
        || f.compatibility == Compatibility::AlreadyIncluded
    {
        return Mapping::AlreadyIncluded {
            reason: first_reason(view, "A ferramenta atual já aplica esta configuração."),
        };
    }
    if view.assessment.status == DynStatus::Incompatible
        || f.compatibility == Compatibility::Incompatible
        || view.availability.state == AvailabilityState::Incompatible
    {
        // O planejador fala sobre este ambiente; a nota da fonte só vale se ela o reprovou.
        let reason =
            if f.compatibility == Compatibility::Incompatible && !f.compatibility_note.is_empty() {
                f.compatibility_note.clone()
            } else {
                first_reason(view, "Não vale para este ambiente.")
            };
        return Mapping::Incompatible { reason };
    }
    match &view.assessment.recipe_id {
        Some(id) => Mapping::Mapped {
            recipe_id: id.clone(),
            availability: view.availability.clone(),
        },
        None => Mapping::Review {
            code: view
                .availability
                .reasons
                .first()
                .map(|r| r.code.clone())
                .unwrap_or_else(|| "recipe-not-generated".into()),
            reason: first_reason(view, "O FRX ainda não sabe aplicar isto com segurança."),
        },
    }
}

pub fn candidate(view: &AssessmentView, f: &FixFinding, app_id: &str) -> Candidate {
    // Valor rejeitado usa a forma crua: a receita, se existisse, não é a identidade.
    let rejected = f.actions.iter().any(gates::command_like);
    let key = match (&view.assessment.recipe, rejected) {
        (Some(r), false) => normalize::from_recipe(r),
        _ => normalize::from_finding(f),
    };
    let (claims, unattributed_reports) = claims::extract(f, &key, app_id);
    let title = view
        .assessment
        .recipe
        .as_ref()
        .map(|r| r.title.clone())
        .unwrap_or_else(|| f.title.clone());
    let purpose = view.assessment.recipe.as_ref().map(|r| r.purpose);
    Candidate {
        purpose: gates::purpose_of(&key, purpose),
        mapping: mapping(view, f, app_id),
        compatibility_note: f.compatibility_note.clone(),
        hardware_scope: crate::fixes::compat::finding_scope(f),
        third_party_binary: f.artifacts.iter().any(|a| !a.dlls.is_empty()),
        key,
        title,
        claims,
        unattributed_reports,
    }
}

/// Melhoria do catálogo: documentada pela receita revisada, avaliada pelo planejador.
pub fn from_catalog(option: &super::models::CatalogOption) -> Candidate {
    use crate::fixes::models::{FindingSource, ProviderId};
    let recipe = &option.recipe;
    let key = normalize::from_recipe(recipe);
    let claims: Vec<Claim> = recipe
        .sources
        .iter()
        .map(|s| Claim {
            kind: ClaimKind::Documented,
            option: key
                .items
                .first()
                .map(|i| i.name.clone())
                .unwrap_or_default(),
            disabled: false,
            combined_with: vec![],
            quote: None,
            source: FindingSource {
                provider: ProviderId::Github,
                label: s.label.clone(),
                url: s.url.clone(),
                reference_id: None,
                snippet: None,
                published_at: None,
            },
            game_verified: false,
        })
        .collect();
    let mapping = if option.availability.state == AvailabilityState::Incompatible {
        Mapping::Incompatible {
            reason: option
                .availability
                .reasons
                .first()
                .map(|r| r.message.clone())
                .unwrap_or_else(|| "Não vale para este ambiente.".into()),
        }
    } else {
        Mapping::Mapped {
            recipe_id: recipe.id.clone(),
            availability: option.availability.clone(),
        }
    };
    Candidate {
        purpose: Some(recipe.purpose),
        title: recipe.title.clone(),
        compatibility_note: String::new(),
        hardware_scope: recipe.scope.gpu_vendor.map_or_else(
            || {
                crate::fixes::compat::scope_of_env(
                    key.items
                        .iter()
                        .filter(|i| i.kind == "env")
                        .map(|i| (i.name.as_str(), i.value.as_deref().unwrap_or(""))),
                )
            },
            Into::into,
        ),
        third_party_binary: recipe.actions.iter().any(|a| {
            matches!(a, crate::fix_engine::models::RecipeAction::InstallArtifact { artifact } if !artifact.dlls.is_empty())
        }),
        key,
        claims,
        mapping,
        unattributed_reports: false,
    }
}
