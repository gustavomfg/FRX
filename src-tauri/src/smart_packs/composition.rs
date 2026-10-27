//! Pack Builder: compõe candidatos de forma determinística e limitada.
//!
//! Estratégia (nunca todas as combinações possíveis):
//!
//! 1. **Baseline** — nenhuma correção do FRX.
//! 2. **Receitas individuais** — até `max_units`, na ordem: evidência mais
//!    forte, menor risco, menos variáveis, ID.
//! 3. **Pacotes por perfil** — união gulosa de receitas *elegíveis* e
//!    compatíveis entre si, até `max_pack_size`:
//!    - compatibilidade: contornos com evidência documentada ou confirmada;
//!    - desempenho: receitas que, medidas sozinhas, melhoraram o FPS médio
//!      sem piorar o 1% low;
//!    - equilibrado: receitas que não pioraram FPS médio nem 1% low e
//!      melhoraram um deles.
//! 4. **Pares** de receitas elegíveis sem conflito conhecido.
//!
//! Elegível para combinação = evidência documentada/confirmada **ou**
//! medição local conclusiva sem regressão. Receitas de um relato só, ainda
//! não medidas, só aparecem sozinhas: comentários diferentes não são somados
//! às cegas. Limitadores de FPS nunca entram em combinações (mudam as
//! condições do benchmark). Total limitado por `max_packs`.

use std::collections::HashMap;

use super::benchmark::{baseline_id, pack_id_with_files, unit_file_fixes, units_file_fixes};
use super::conflicts::{graph, validate_combination};
use super::models::*;
use crate::fix_engine::dynamic::compatibility::performance_warnings;
use crate::fix_engine::dynamic::EvidenceLevel;
use crate::fix_engine::files::sha256_hex;
use crate::fix_engine::models::{Purpose, RecipeAction, RecipeSource, Risk};
use crate::fixes::compat::vendor_name;

/// Resultado conclusivo de uma receita medida sozinha: (FPS médio, 1% low).
pub type SingleResults = HashMap<String, (Outcome, Outcome)>;

pub struct Composed {
    pub packs: Vec<SmartPack>,
    pub graph: Vec<Edge>,
    pub excluded: Vec<Excluded>,
    pub notes: Vec<String>,
}

fn order(units: &mut [Unit]) {
    units.sort_by(|a, b| {
        a.evidence
            .cmp(&b.evidence)
            .then(a.risk.cmp(&b.risk))
            .then(a.env.len().cmp(&b.env.len()))
            .then(a.recipe_id.cmp(&b.recipe_id))
    });
}

fn strong(e: EvidenceLevel) -> bool {
    matches!(e, EvidenceLevel::Documented | EvidenceLevel::Corroborated)
}

fn measured_ok(r: Option<&(Outcome, Outcome)>) -> bool {
    r.is_some_and(|(avg, low)| {
        matches!(avg, Outcome::Improvement | Outcome::NoDifference)
            && matches!(low, Outcome::Improvement | Outcome::NoDifference)
    })
}

/// Medição local (mesmo preliminar) indicou regressão: não combina até repetir.
fn regressed(r: Option<&(Outcome, Outcome)>) -> bool {
    r.is_some_and(|(avg, low)| *avg == Outcome::Regression || *low == Outcome::Regression)
}

pub fn eligible(u: &Unit, single: Option<&(Outcome, Outcome)>) -> bool {
    u.perf != PerfHint::FrameCap
        && !u.already_included
        && !regressed(single)
        && (performance_warnings(u.env.iter().map(|p| (p.key.as_str(), p.value.as_str())))
            .is_empty()
            || measured_ok(single))
        && (strong(u.evidence) || measured_ok(single))
}

pub fn baseline(app_id: &str) -> SmartPack {
    SmartPack {
        availability: Default::default(),
        artifacts: vec![],
        file_fixes: vec![],
        performance_evidence: Default::default(),
        performance_warnings: vec![],
        id: baseline_id(app_id),
        app_id: app_id.to_owned(),
        version: COMPOSER_VERSION.into(),
        name: "Baseline — sem correções do FRX".into(),
        kind: PackKind::Baseline,
        goal: Goal::Reference,
        goal_text: "Configuração de referência: opções e arquivos do jogo sem nenhuma correção ativa do FRX. É contra ela que os outros pacotes são medidos.".into(),
        members: vec![],
        env: vec![],
        apply_recipe_id: None,
        fingerprint: sha256_hex(format!("baseline-v{PACK_SCHEMA}:{app_id}").as_bytes()),
        requirements: vec![],
        compatibility: vec![],
        conflicts_avoided: vec![],
        sources: vec![],
        evidence: EvidenceLevel::None,
        evidence_note: "Referência: não é uma correção.".into(),
        risk: Risk::Low,
        effects: vec![],
        side_effects: vec![],
        reversible: true,
        state: PackState::Candidate,
        state_detail: String::new(),
        plan_status: None,
        operation_id: None,
    }
}

fn make(
    app_id: &str,
    members: &[&Unit],
    kind: PackKind,
    goal: Goal,
    name: String,
    goal_text: String,
    graph: &[Edge],
) -> SmartPack {
    let mut env: Vec<EnvPair> = members.iter().flat_map(|u| u.env.clone()).collect();
    env.sort();
    env.dedup();
    let file_actions: Vec<_> = members
        .iter()
        .flat_map(|u| u.file_actions.clone())
        .collect();
    let file_fixes = units_file_fixes(
        app_id,
        &members.iter().map(|u| (*u).clone()).collect::<Vec<_>>(),
    )
    .expect("composition only accepts validated complete file actions");
    let artifacts = file_actions
        .iter()
        .filter_map(|a| match a {
            RecipeAction::InstallArtifact { artifact } => Some((**artifact).clone()),
            _ => None,
        })
        .collect();
    let id = pack_id_with_files(app_id, &env, &file_fixes);
    let mut sources: Vec<RecipeSource> = Vec::new();
    for s in members.iter().flat_map(|u| u.sources.iter()) {
        if !sources.iter().any(|x| x.url == s.url) {
            sources.push(s.clone());
        }
    }
    let uniq = |v: Vec<String>| {
        let mut out: Vec<String> = Vec::new();
        for x in v {
            if !out.contains(&x) {
                out.push(x);
            }
        }
        out
    };
    let ids: Vec<&str> = members.iter().map(|u| u.recipe_id.as_str()).collect();
    let mut requirements = vec!["Jogo rodando com Proton.".to_owned()];
    if let Some(g) = members.iter().find_map(|u| u.gpu) {
        requirements.push(format!("GPU {}.", vendor_name(g)));
    }
    let mut compatibility = Vec::new();
    for u in members {
        if !u.proton_versions.is_empty() {
            compatibility.push(format!(
                "“{}” foi relatada com {}{}.",
                u.title,
                u.proton_versions.join(", "),
                u.discovered_at
                    .as_deref()
                    .map(|d| format!(" em {}", &d[..d.len().min(10)]))
                    .unwrap_or_default()
            ));
        }
    }
    let evidence = members
        .iter()
        .map(|u| u.evidence)
        .max()
        .unwrap_or(EvidenceLevel::None);
    SmartPack {
        availability: Default::default(),
        artifacts,
        performance_evidence: if members.len() == 1 {
            members[0].performance_evidence.clone()
        } else {
            Default::default()
        },
        performance_warnings: performance_warnings(
            env.iter().map(|p| (p.key.as_str(), p.value.as_str())),
        ),
        apply_recipe_id: Some(if members.len() == 1 {
            members[0].recipe_id.clone()
        } else {
            id.clone()
        }),
        fingerprint: {
            let mut identities: Vec<String> = members
                .iter()
                .map(|u| format!("{}@{}#{}", u.recipe_id, u.version, u.fingerprint))
                .collect();
            identities.sort();
            sha256_hex(
                &serde_json::to_vec(&(PACK_SCHEMA, app_id, &env, &file_fixes, identities))
                    .unwrap_or_default(),
            )
        },
        id,
        app_id: app_id.to_owned(),
        version: COMPOSER_VERSION.into(),
        name,
        kind,
        goal,
        goal_text,
        members: members
            .iter()
            .map(|u| PackMemberView {
                file_actions: u.file_actions.clone(),
                recipe_id: u.recipe_id.clone(),
                version: u.version.clone(),
                fingerprint: u.fingerprint.clone(),
                trust: u.trust,
                title: u.title.clone(),
                env: u.env.clone(),
                evidence: u.evidence,
                discovered_at: u.discovered_at.clone(),
                proton_versions: u.proton_versions.clone(),
            })
            .collect(),
        env,
        file_fixes,
        requirements,
        compatibility,
        conflicts_avoided: graph
            .iter()
            .filter(|e| {
                e.relation == Relation::Excludes
                    && (ids.contains(&e.a.as_str()) || ids.contains(&e.b.as_str()))
            })
            .cloned()
            .collect(),
        sources,
        evidence,
        evidence_note: members
            .iter()
            .map(|u| format!("{}: {}", u.title, u.evidence_note))
            .collect::<Vec<_>>()
            .join(" "),
        risk: members.iter().map(|u| u.risk).max().unwrap_or(Risk::Low),
        effects: uniq(members.iter().flat_map(|u| u.effects.clone()).collect()),
        side_effects: uniq(
            members
                .iter()
                .flat_map(|u| u.side_effects.clone())
                .collect(),
        ),
        reversible: true,
        state: PackState::Candidate,
        state_detail: String::new(),
        plan_status: None,
        operation_id: None,
    }
}

/// União gulosa de receitas que passam no filtro e são compatíveis entre si.
fn greedy<'a>(pool: &[&'a Unit], max: usize) -> Vec<&'a Unit> {
    let mut chosen: Vec<&Unit> = Vec::new();
    for u in pool {
        if chosen.len() >= max {
            break;
        }
        let mut trial: Vec<Unit> = chosen.iter().map(|x| (*x).clone()).collect();
        trial.push((*u).clone());
        if trial.len() < 2 || validate_combination(&trial, max).is_ok() {
            chosen.push(u);
        }
    }
    chosen
}

pub fn compose(app_id: &str, units: &[Unit], singles: &SingleResults, limits: Limits) -> Composed {
    let mut excluded = Vec::new();
    let mut units: Vec<Unit> = units.iter().filter_map(|u| {
        let reason = if u.app_id.as_deref() != Some(app_id) || !crate::art::valid_app_id(app_id) {
            Some("Receita de outro AppID ou sem atribuição confirmada: não pode integrar os pacotes deste jogo.".to_owned())
        } else {
            unit_file_fixes(app_id, u).err()
        };
        if let Some(reason) = reason {
            excluded.push(Excluded { performance_warnings: vec![], id: u.recipe_id.clone(), title: u.title.clone(), status: None, reason });
            None
        } else { Some(u.clone()) }
    }).collect();
    order(&mut units);
    let mut notes = Vec::new();
    if units.len() > limits.max_units {
        for u in units.drain(limits.max_units..) {
            excluded.push(Excluded {
                performance_warnings: performance_warnings(
                    u.env.iter().map(|p| (p.key.as_str(), p.value.as_str())),
                ),
                id: u.recipe_id.clone(),
                title: u.title.clone(),
                status: None,
                reason: format!(
                    "Fora do limite de {} receitas por análise (ordem: evidência, risco, tamanho).",
                    limits.max_units
                ),
            });
        }
    }
    let graph = graph(&units);
    let mut packs = vec![baseline(app_id)];
    let push = |packs: &mut Vec<SmartPack>, p: SmartPack, notes: &mut Vec<String>| {
        if packs.iter().any(|x| x.id == p.id) {
            return;
        }
        if packs.len() >= limits.max_packs {
            if !notes.iter().any(|n: &String| n.starts_with("Limite")) {
                notes.push(format!(
                    "Limite de {} pacotes atingido; combinações adicionais não foram montadas.",
                    limits.max_packs
                ));
            }
            return;
        }
        packs.push(p);
    };

    for u in &units {
        let goal = if u.purpose == Purpose::Performance {
            Goal::Performance
        } else {
            Goal::Compatibility
        };
        let text = format!(
            "{}: “{}”. {}",
            if u.performance_evidence.contextual_candidate(app_id) {
                "Possível melhoria específica deste jogo, ainda sem validação local"
            } else {
                "Alternativa de compatibilidade"
            },
            u.title,
            u.effects.first().cloned().unwrap_or_default()
        );
        push(
            &mut packs,
            make(
                app_id,
                &[u],
                PackKind::Single,
                goal,
                u.title.clone(),
                text,
                &graph,
            ),
            &mut notes,
        );
    }

    let single = |u: &Unit| singles.get(&u.recipe_id);
    let elig: Vec<&Unit> = units.iter().filter(|u| eligible(u, single(u))).collect();

    let compat: Vec<&Unit> = elig
        .iter()
        .copied()
        .filter(|u| u.purpose == Purpose::Compatibility && strong(u.evidence))
        .collect();
    let perf: Vec<&Unit> = elig
        .iter()
        .copied()
        .filter(|u| {
            single(u).is_some_and(|(a, l)| *a == Outcome::Improvement && *l != Outcome::Regression)
        })
        .collect();
    let balanced: Vec<&Unit> = elig
        .iter()
        .copied()
        .filter(|u| {
            single(u).is_some_and(|(a, l)| {
                *a != Outcome::Regression
                    && *l != Outcome::Regression
                    && (*a == Outcome::Improvement || *l == Outcome::Improvement)
            })
        })
        .collect();
    for (pool, goal, name, text) in [
        (&perf, Goal::Performance, "Pack de desempenho", "Reúne receitas que, medidas sozinhas no seu computador, melhoraram o FPS médio sem piorar o 1% low. A combinação ainda precisa ser medida."),
        (&balanced, Goal::Balanced, "Pack equilibrado", "Reúne receitas que, medidas sozinhas, não pioraram FPS médio nem 1% low e melhoraram pelo menos um deles. A combinação ainda precisa ser medida."),
        (&compat, Goal::Compatibility, "Pack de compatibilidade", "Reúne contornos de compatibilidade com evidência documentada ou confirmada por vários relatos, sem conflito conhecido entre eles."),
    ] {
        let chosen = greedy(pool, limits.max_pack_size);
        if chosen.len() >= 2 {
            push(&mut packs, make(app_id, &chosen, PackKind::Combination, goal, name.into(), text.into(), &graph), &mut notes);
        }
    }

    for i in 0..elig.len() {
        for j in i + 1..elig.len() {
            let pair = [elig[i].clone(), elig[j].clone()];
            if validate_combination(&pair, limits.max_pack_size).is_ok() {
                let goal = if pair.iter().all(|u| u.purpose == Purpose::Performance) {
                    Goal::Performance
                } else {
                    Goal::Compatibility
                };
                push(
                    &mut packs,
                    make(
                        app_id,
                        &[elig[i], elig[j]],
                        PackKind::Combination,
                        goal,
                        format!("{} + {}", elig[i].title, elig[j].title),
                        "Combina duas receitas com evidência suficiente e sem conflito conhecido. Nenhum conflito conhecido não prova que funcionem bem juntas: meça antes de manter.".into(),
                        &graph,
                    ),
                    &mut notes,
                );
            }
        }
    }
    let single_only = units.iter().filter(|u| !eligible(u, single(u))).count();
    if single_only > 0 {
        notes.push(format!(
            "{single_only} receita{} só aparece{} sozinha{}: evidência de um relato só (ou nenhuma) e ainda sem medição local conclusiva. Medida sem regressão, passa a poder ser combinada.",
            if single_only == 1 { "" } else { "s" },
            if single_only == 1 { "" } else { "m" },
            if single_only == 1 { "" } else { "s" }
        ));
    }
    Composed {
        packs,
        graph,
        excluded,
        notes,
    }
}
