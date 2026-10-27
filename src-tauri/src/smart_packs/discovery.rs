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

/// Receita → unidade (só receitas que apenas definem variáveis).
pub fn unit_from_recipe(
    r: &Recipe,
    evidence: EvidenceLevel,
    evidence_note: &str,
    finding_ids: Vec<String>,
    findings: &[FixFinding],
) -> Option<Unit> {
    let mut env = Vec::new();
    for a in &r.actions {
        match a {
            RecipeAction::SetLaunchEnv { key, value } => env.push(EnvPair {
                key: key.clone(),
                value: value.clone(),
            }),
            _ => return None,
        }
    }
    env.sort();
    let related: Vec<&FixFinding> = findings
        .iter()
        .filter(|f| finding_ids.contains(&f.id))
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
        let Some(u) = unit_from_recipe(&r, a.evidence, &a.evidence_note, finding_ids, findings)
        else {
            continue;
        };
        // Mesmas variáveis (gerada e revisada, ou duas descobertas): uma unidade só.
        match units.iter_mut().find(|x| x.env == u.env) {
            Some(x) => {
                for f in u.finding_ids {
                    if !x.finding_ids.contains(&f) {
                        x.finding_ids.push(f);
                    }
                }
                if u.evidence < x.evidence {
                    x.evidence = u.evidence;
                    x.evidence_note = u.evidence_note;
                }
            }
            None => units.push(u),
        }
    }
    Discovered { units, excluded }
}
