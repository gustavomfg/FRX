//! FRX Smart Packs — composição, avaliação e classificação de pacotes de
//! correção, com base em evidências e benchmarks locais.
//!
//! Camadas (cada uma num módulo, sem fórmulas opacas):
//!
//! - **Pack Builder** (`discovery`, `composition`): receitas aplicáveis do
//!   jogo → pacotes candidatos, de forma determinística e limitada;
//! - **Compatibility Engine** (`conflicts`, `lifecycle`): grafo de conflitos e
//!   estado operacional pela prévia do Fix Engine;
//! - **Evidence Engine** (`evidence`): sessões do Performance Lab agrupadas
//!   por configuração, filtradas por comparabilidade;
//! - **Ranking Engine** (`ranking`): ranking medido × candidatos × não
//!   recomendados, por perfil;
//! - **Benchmark Integration** (`benchmark`): snapshot da configuração em
//!   cada sessão e declaração para sessões antigas;
//! - **Persistência** (`persistence`).
//!
//! Aplicar um pacote usa o Fix Engine existente: um pacote de uma receita
//! abre a própria receita; um de várias abre a receita composta
//! `pack.<appid>.<hash>`, aplicada numa única operação (um backup, uma
//! gravação atômica, um rollback). Não existe outro executor.

pub mod benchmark;
pub mod composition;
pub mod conflicts;
pub mod discovery;
pub mod evaluation;
pub mod evidence;
pub mod experience;
pub mod lifecycle;
pub mod models;
pub mod persistence;

#[cfg(test)]
pub(crate) mod tests;

use std::collections::HashMap;

use crate::fix_engine::dynamic::EvidenceLevel;
use crate::fix_engine::models::*;
use crate::fix_engine::{security, FixEngine};
use crate::performance::models::SessionMeta;
use benchmark::{
    baseline_id, current_env, current_file_fixes, pack_id_with_files, units_file_fixes,
};
use models::*;
use persistence::SmartStore;

pub fn store_for(engine: &FixEngine) -> SmartStore {
    SmartStore::new(engine.root().join("smart-packs"))
}

/// Linha atual de opções de inicialização (só leitura), para as notas de
/// interação com MangoHud, gamescope e variáveis já definidas.
pub fn launch_line(game: &GameCtx) -> Option<String> {
    let root = game.steam_root.as_ref()?;
    let (_, path) = crate::steam::localconfig::path_for(root)?;
    let text = crate::steam::localconfig::read_file(&path).ok()?;
    crate::steam::localconfig::launch_options(&text, &game.app_id)
        .ok()
        .flatten()
}

pub(crate) fn populate_preview_files(
    engine: &FixEngine,
    game: &GameCtx,
    unit: &mut Unit,
) -> Result<(), String> {
    if unit
        .file_actions
        .iter()
        .any(|a| matches!(a, RecipeAction::SetDxvkOption { .. }))
    {
        unit.preview_files = Some(engine.preview_file_fixes(&unit.recipe_id, game)?);
    }
    Ok(())
}

fn name_lookup(packs: &[SmartPack]) -> impl Fn(&str) -> Option<String> + '_ {
    move |id: &str| packs.iter().find(|p| p.id == id).map(|p| p.name.clone())
}

/// Avalia uma sessão (normalmente a que acabou de ser capturada) contra o
/// baseline, com os pacotes compostos atuais para dar nome à configuração.
pub fn evaluate_session(
    engine: &FixEngine,
    sessions: &[SessionMeta],
    session_id: &str,
) -> Result<PackEvaluation, String> {
    let meta = sessions
        .iter()
        .find(|m| m.id == session_id)
        .ok_or("Sessão não encontrada.")?;
    let app = meta
        .app_id
        .clone()
        .ok_or("A sessão não está associada a um jogo.")?;
    let base_id = baseline_id(&app);
    let grouped = evidence::group_in_context(
        &app,
        sessions,
        |m| benchmark::session_config(engine, m),
        &base_id,
        Some(&meta.conditions),
    );
    let metas: HashMap<&str, &SessionMeta> = sessions
        .iter()
        .filter(|m| m.app_id.as_deref() == Some(app.as_str()))
        .map(|m| (m.id.as_str(), m))
        .collect();
    let stored: Vec<SmartPack> = store_for(engine)
        .load(&app)
        .map(|f| {
            f.packs
                .into_iter()
                .map(|s| s.pack)
                .filter(|p| {
                    p.app_id == app && p.id == pack_id_with_files(&app, &p.env, &p.file_fixes)
                })
                .collect()
        })
        .unwrap_or_default();
    let name_of = name_lookup(&stored);
    let ctx = evaluation::Ctx {
        app_id: &app,
        baseline_id: &base_id,
        grouped: &grouped,
        metas: &metas,
        name_of: &name_of,
    };
    let cfg = benchmark::session_config(engine, meta);
    let mut e = evaluation::evaluate(&ctx, meta, cfg.as_ref());
    e.current_pack_id = Some(pack_id_with_files(
        &app,
        &current_env(engine, &app).0,
        &current_file_fixes(engine, &app),
    ));
    Ok(e)
}

/// Receita composta de um Smart Pack. As receitas de origem são refeitas e
/// revalidadas pelo Fix Engine; o registro só diz quais são.
pub fn resolve_pack(engine: &FixEngine, id: &str, game: &GameCtx) -> Result<Recipe, String> {
    if !security::valid_pack_id(id) || !id.starts_with(&format!("pack.{}.", game.app_id)) {
        return Err("Smart Pack inválido.".into());
    }
    let store = store_for(engine);
    let stored = store
        .pack(&game.app_id, id)
        .ok_or("Smart Pack não encontrado. Abra Smart Packs de novo para recompor.")?;
    let ids = store.members(&game.app_id, id).unwrap_or_default();
    if ids.len() < 2 || ids.iter().any(|m| m.starts_with("pack.")) {
        return Err("Composição inválida.".into());
    }
    let views = engine.analyze(game)?;
    let mut relevant = discovery::discover(&game.app_id, &views, &game.findings, |rid| {
        engine.recipe(rid).ok().cloned()
    });
    relevant
        .units
        .extend(experience::catalog_units(engine, game));
    let mut recipes = Vec::new();
    let mut units = Vec::new();
    for m in &ids {
        if !relevant
            .units
            .iter()
            .any(|u| u.recipe_id == *m && u.app_id.as_deref() == Some(&game.app_id))
        {
            if let Some(view) = views
                .iter()
                .find(|v| v.assessment.recipe_id.as_ref() == Some(m))
            {
                return Err(format!("Uma receita deixou de ser aplicável: {} Reabra os Smart Packs para revalidar a composição.",
                    view.availability.reasons.iter().map(|r| r.message.as_str()).collect::<Vec<_>>().join(" ")));
            }
            return Err("Uma receita não possui mais descoberta verificável para este AppID. Reabra os Smart Packs deste jogo.".into());
        }
        let r = engine.resolve(m, game)?;
        if !stored.members.iter().any(|sm| {
            sm.recipe_id == r.id
                && sm.version == r.version
                && sm.fingerprint == discovery::fingerprint(&r)
        }) {
            return Err("A receita mudou desde a composição do pacote. Reabra os Smart Packs para revisar a nova configuração.".into());
        }
        let mut u = discovery::unit_from_recipe(
            &game.app_id,
            &r,
            EvidenceLevel::None,
            "",
            r.generated
                .as_ref()
                .map(|g| g.finding_ids.clone())
                .unwrap_or_default(),
            &game.findings,
        )
        .ok_or("Uma receita do pacote inclui uma ação de arquivo não validada integralmente.")?;
        populate_preview_files(engine, game, &mut u)?;
        units.push(u);
        recipes.push(r);
    }
    if let Err(bad) = conflicts::validate_combination(&units, experience::MAX_MEMBERS) {
        return Err(format!(
            "A composição não passa mais na validação: {}",
            bad.first()
                .map(|e| e.reason.as_str())
                .unwrap_or("tamanho inválido")
        ));
    }
    let mut env: Vec<EnvPair> = units.iter().flat_map(|u| u.env.clone()).collect();
    env.sort();
    env.dedup();
    let file_actions: Vec<_> = units.iter().flat_map(|u| u.file_actions.clone()).collect();
    let file_fixes = units_file_fixes(&game.app_id, &units)?;
    if pack_id_with_files(&game.app_id, &env, &file_fixes) != id {
        return Err(
            "As receitas do pacote mudaram desde a composição. Abra Smart Packs de novo.".into(),
        );
    }
    let mut sources: Vec<RecipeSource> = Vec::new();
    for s in recipes.iter().flat_map(|r| r.sources.iter()) {
        if !sources.iter().any(|x| x.url == s.url) && sources.len() < 8 {
            sources.push(s.clone());
        }
    }
    let line = env
        .iter()
        .map(|p| format!("{}={}", p.key, p.value))
        .collect::<Vec<_>>()
        .join(" ");
    let collect = |f: &dyn Fn(&Recipe) -> Vec<String>| {
        let mut out: Vec<String> = Vec::new();
        for x in recipes.iter().flat_map(f) {
            if !out.contains(&x) {
                out.push(x);
            }
        }
        out
    };
    let r = Recipe {
        hardware: conflicts::combined_hardware(&units)?,
        schema: RECIPE_SCHEMA,
        id: id.to_owned(),
        version: COMPOSER_VERSION.into(),
        trust: Trust::Pack,
        title: stored.name.chars().take(120).collect(),
        summary: format!("Aplica as alterações validadas deste jogo: {} arquivo(s) e opções {line}.", file_fixes.len()),
        description: "Smart Pack composto pelo FRX a partir de receitas validadas individualmente e em conjunto. Não foi revisado por pessoas; a composição não comprova ganho de desempenho.".into(),
        category: recipes[0].category,
        purpose: if recipes.iter().all(|r| r.purpose == Purpose::Performance) {
            Purpose::Performance
        } else {
            Purpose::Compatibility
        },
        risk: recipes.iter().map(|r| r.risk).max().unwrap_or(Risk::Low),
        scope: Scope {
            app_ids: vec![game.app_id.clone()],
            requires_proton: recipes.iter().any(|r| r.scope.requires_proton),
            gpu_vendor: recipes.iter().find_map(|r| r.scope.gpu_vendor),
        },
        actions: env
            .iter()
            .map(|p| RecipeAction::SetLaunchEnv {
                key: p.key.clone(),
                value: p.value.clone(),
            })
            .chain(file_actions)
            .collect(),
        already_included_by_protonfixes: recipes.iter().any(|r| r.already_included_by_protonfixes),
        conflicts_with: vec![],
        related_keys: env.iter().map(|p| format!("env:{}={}", p.key, p.value)).collect(),
        effects: collect(&|r| r.effects.clone()),
        side_effects: collect(&|r| r.side_effects.clone()),
        sources,
        review: None,
        generated: None,
        pack: Some(PackInfo {
            members: recipes
                .iter()
                .map(|r| PackMember {
                    recipe_id: r.id.clone(),
                    version: r.version.clone(),
                    fingerprint: discovery::fingerprint(r),
                    trust: r.trust,
                })
                .collect(),
            goal: stored.goal_text.chars().take(400).collect(),
        }),
    };
    security::validate_pack(&r).map_err(|e| format!("Smart Pack recusado: {e}"))?;
    Ok(r)
}
