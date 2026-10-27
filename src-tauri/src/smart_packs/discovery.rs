//! Receitas que podem entrar em pacotes, a partir do gerador dinâmico.
//!
//! Só entram receitas aplicáveis **para este jogo**: geradas a partir das
//! descobertas dele ou revisadas do catálogo quando uma descoberta do jogo
//! cita exatamente o mesmo ajuste. Receitas genéricas do catálogo sem relação
//! com o jogo não viram pacote. O resto aparece em `excluded` com o motivo.

use super::conflicts::perf_hint;
use super::models::*;
use crate::fix_engine::dynamic::{AssessmentView, DynStatus, EvidenceLevel};
use crate::fix_engine::files::sha256_hex;
use crate::fix_engine::models::{Purpose, Recipe, RecipeAction};
use crate::fixes::models::FixFinding;

pub struct Discovered {
    pub units: Vec<Unit>,
    pub excluded: Vec<Excluded>,
}

pub fn fingerprint(r: &Recipe) -> String {
    sha256_hex(&serde_json::to_vec(r).unwrap_or_default())
}

/// Receita → unidade. Variáveis e artefatos suportados são validados juntos;
/// uma ação não suportada exclui a receita inteira, sem aplicação parcial.
pub fn unit_from_recipe(
    app_id: &str,
    r: &Recipe,
    evidence: EvidenceLevel,
    evidence_note: &str,
    finding_ids: Vec<String>,
    findings: &[FixFinding],
) -> Option<Unit> {
    if !crate::art::valid_app_id(app_id)
        || (!r.scope.app_ids.is_empty() && !r.scope.app_ids.iter().any(|a| a == app_id))
        || r.generated
            .as_ref()
            .is_some_and(|g| g.game_app_id.as_deref() != Some(app_id))
    {
        return None;
    }
    let mut env = Vec::new();
    let mut file_actions = Vec::new();
    for a in &r.actions {
        match a {
            RecipeAction::SetLaunchEnv { key, value } => env.push(EnvPair {
                key: key.clone(),
                value: value.clone(),
            }),
            RecipeAction::InstallArtifact { artifact }
                if crate::fix_engine::artifacts::validate(artifact, app_id).is_ok() =>
            {
                file_actions.push(a.clone())
            }
            RecipeAction::SetDxvkOption { .. } => file_actions.push(a.clone()),
            _ => return None,
        }
    }
    env.sort();
    let related: Vec<&FixFinding> = findings
        .iter()
        .filter(|f| {
            finding_ids.contains(&f.id) && f.game_app_id == app_id && f.scope.matches_game(app_id)
        })
        .collect();
    let mut proton_versions: Vec<String> = Vec::new();
    for f in &related {
        for v in &f.requirements.proton_versions {
            if !proton_versions.contains(v) {
                proton_versions.push(v.clone());
            }
        }
    }
    let discovered_at = r
        .generated
        .as_ref()
        .and_then(|g| g.discovered_at.clone())
        .or_else(|| related.iter().filter_map(|f| f.published_at.clone()).max());
    Some(Unit {
        preview_files: None,
        hardware: r.hardware.clone(),
        file_actions,
        app_id: Some(app_id.to_owned()),
        performance_evidence: r
            .generated
            .as_ref()
            .map(|g| g.performance_evidence.clone())
            .unwrap_or_default(),
        recipe_id: r.id.clone(),
        version: r.version.clone(),
        fingerprint: fingerprint(r),
        trust: r.trust,
        title: r.title.clone(),
        perf: env
            .iter()
            .map(|p| perf_hint(&p.key))
            .find(|h| *h != PerfHint::Unknown)
            .unwrap_or(PerfHint::Unknown),
        env,
        purpose: r.purpose,
        risk: r.risk,
        evidence,
        evidence_note: evidence_note.to_owned(),
        finding_ids,
        sources: r.sources.clone(),
        proton_versions,
        discovered_at,
        gpu: r.scope.gpu_vendor,
        already_included: r.already_included_by_protonfixes,
        effects: r.effects.clone(),
        side_effects: r.side_effects.clone(),
    })
}

/// `catalog(id)` busca uma receita revisada do catálogo local.
pub fn discover(
    app_id: &str,
    views: &[AssessmentView],
    findings: &[FixFinding],
    catalog: impl Fn(&str) -> Option<Recipe>,
) -> Discovered {
    let mut units: Vec<Unit> = Vec::new();
    let mut excluded: Vec<Excluded> = Vec::new();
    let title_of = |id: &str| {
        findings
            .iter()
            .find(|f| f.id == id)
            .map(|f| f.title.clone())
            .unwrap_or_else(|| id.to_owned())
    };
    for v in views {
        let a = &v.assessment;
        // A análise e a descoberta precisam confirmar o mesmo jogo. O AppID
        // da consulta ou do arquivo de cache não constitui atribuição.
        if a.app_id != app_id
            || !findings.iter().any(|f| {
                f.id == a.finding_id && f.game_app_id == app_id && f.scope.matches_game(app_id)
            })
        {
            excluded.push(Excluded {
                performance_warnings: a.performance_warnings.clone(),
                id: a.finding_id.clone(),
                title: title_of(&a.finding_id),
                status: Some(DynStatus::Invalid),
                reason: "Descoberta sem vínculo verificável com o AppID selecionado; preservada fora dos pacotes.".into(),
            });
            continue;
        }
        if !v.availability.can_compose() {
            excluded.push(Excluded {
                performance_warnings: a.performance_warnings.clone(),
                id: a.finding_id.clone(),
                title: title_of(&a.finding_id),
                status: Some(a.status),
                reason: v
                    .availability
                    .reasons
                    .iter()
                    .map(|r| r.message.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
            });
            continue;
        }
        let reason = a.reasons.first().cloned().unwrap_or_default();
        let recipe = match a.status {
            DynStatus::Generated => a.recipe.clone(),
            DynStatus::Reviewed => a.recipe_id.as_deref().and_then(&catalog),
            DynStatus::Contested
            | DynStatus::Incompatible
            | DynStatus::AlreadyIncluded
            | DynStatus::NeedsValidation
            | DynStatus::Blocked => {
                excluded.push(Excluded {
                    performance_warnings: a.performance_warnings.clone(),
                    id: a.finding_id.clone(),
                    title: title_of(&a.finding_id),
                    status: Some(a.status),
                    reason,
                });
                continue;
            }
            _ => continue,
        };
        let Some(r) = recipe else { continue };
        if a.evidence == EvidenceLevel::Contested {
            excluded.push(Excluded {
                performance_warnings: a.performance_warnings.clone(),
                id: r.id.clone(),
                title: r.title.clone(),
                status: Some(DynStatus::Contested),
                reason: format!("{} Fica fora dos pacotes.", a.evidence_note),
            });
            continue;
        }
        if r.purpose == Purpose::Diagnostic {
            continue;
        }
        let finding_ids = r
            .generated
            .as_ref()
            .map(|g| g.finding_ids.clone())
            .unwrap_or_else(|| vec![a.finding_id.clone()]);
        let Some(mut u) = unit_from_recipe(
            app_id,
            &r,
            a.evidence,
            &a.evidence_note,
            finding_ids,
            findings,
        ) else {
            continue;
        };
        u.performance_evidence = a.performance_evidence.clone();
        // Mesmas variáveis (gerada e revisada, ou duas descobertas): uma unidade só.
        match units
            .iter_mut()
            .find(|x| x.env == u.env && x.file_actions == u.file_actions)
        {
            Some(x) => {
                for f in u.finding_ids {
                    if !x.finding_ids.contains(&f) {
                        x.finding_ids.push(f);
                    }
                }
                x.performance_evidence.merge(&u.performance_evidence);
                if u.evidence < x.evidence {
                    x.evidence = u.evidence;
                    x.evidence_note = u.evidence_note;
                }
            }
            None => units.push(u),
        }
    }
    // A disputed finding may intentionally have no executable recipe. Its
    // negative performance report must nevertheless remain attached to an
    // existing unit for the exact same safe actions and verified game.
    // Never create a unit (or salvage part of an unsafe procedure) here.
    use crate::fix_engine::dynamic::extraction::{Deterministic, Extracted, Interpreter};
    use crate::fix_engine::dynamic::validation::{self, EnvVerdict};
    for v in views {
        let a = &v.assessment;
        if a.app_id != app_id
            || !matches!(
                a.status,
                DynStatus::Generated | DynStatus::Reviewed | DynStatus::Contested
            )
        {
            continue;
        }
        let Some(f) = findings.iter().find(|f| {
            f.id == a.finding_id && f.game_app_id == app_id && f.scope.matches_game(app_id)
        }) else {
            continue;
        };
        let Ok(items) = Deterministic.interpret(f) else {
            continue;
        };
        let mut env = Vec::new();
        let mut safe =
            !items.is_empty() && f.analysis == crate::fixes::models::AnalysisState::Parsed;
        for item in items {
            match item {
                Extracted::Env {
                    key,
                    value,
                    conditional: false,
                    ..
                } if matches!(validation::env(&key, &value), EnvVerdict::Ok(_)) => {
                    if env
                        .iter()
                        .any(|p: &EnvPair| p.key == key && p.value != value)
                    {
                        safe = false;
                    }
                    env.push(EnvPair { key, value });
                }
                _ => safe = false,
            }
        }
        if !safe {
            continue;
        }
        env.sort();
        env.dedup();
        if let Some(unit) = units.iter_mut().find(|u| {
            u.env == env && u.file_actions.is_empty() && u.app_id.as_deref() == Some(app_id)
        }) {
            unit.performance_evidence.merge(&a.performance_evidence);
            if !unit.finding_ids.contains(&a.finding_id) {
                unit.finding_ids.push(a.finding_id.clone());
            }
        }
    }
    Discovered { units, excluded }
}
