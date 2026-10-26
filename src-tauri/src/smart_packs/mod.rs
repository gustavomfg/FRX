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
pub mod evidence;
pub mod lifecycle;
pub mod models;
pub mod persistence;
pub mod ranking;

#[cfg(test)]
pub(crate) mod tests;

use std::collections::HashMap;

use crate::fix_engine::dynamic::EvidenceLevel;
use crate::fix_engine::models::*;
use crate::fix_engine::{security, FixEngine};
use crate::performance::models::SessionMeta;
use crate::time::now_iso;
use benchmark::{baseline_id, current_env, pack_id};
use composition::SingleResults;
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

pub struct BuildInput<'a> {
    pub game: &'a GameCtx,
    pub sessions: &'a [SessionMeta],
    pub profile: Profile,
    pub limits: Limits,
    pub launch_line: Option<String>,
}

pub fn build(engine: &FixEngine, i: BuildInput<'_>) -> Result<SmartPackReport, String> {
    let app = i.game.app_id.as_str();
    let store = store_for(engine);
    let views = engine.analyze(i.game)?;
    let found = discovery::discover(&views, &i.game.findings, |id| {
        engine.recipe(id).ok().cloned()
    });
    let base_id = baseline_id(app);
    let grouped = evidence::group(
        app,
        i.sessions,
        |m| benchmark::session_config(engine, m),
        &base_id,
    );
    let metas: HashMap<&str, &SessionMeta> =
        i.sessions.iter().map(|m| (m.id.as_str(), m)).collect();
    let base_m = grouped.measurements.get(&base_id);

    let vs_for = |env: &[EnvPair]| -> Option<VsBaseline> {
        let pm = grouped.measurements.get(&pack_id(app, env))?;
        let b = base_m?;
        (b.used() > 0 && pm.used() > 0).then(|| evidence::compare(b, pm, &metas))
    };
    let mut singles: SingleResults = HashMap::new();
    for u in &found.units {
        if let Some(v) = vs_for(&u.env).filter(|v| !v.limited) {
            let get = |k: &str| v.get(k).map(|d| d.outcome).unwrap_or(Outcome::NotMeasured);
            singles.insert(u.recipe_id.clone(), (get("fpsAvg"), get("fpsLow1")));
        }
    }
    let composed = composition::compose(app, &found.units, &singles, i.limits);
    let mut packs = composed.packs;

    // Registra antes da prévia: a receita composta é resolvida pelo registro.
    store.remember(app, &packs)?;
    let (cur_env, _, _) = current_env(engine, app);
    let ops = engine.operations(Some(app));
    let applied = engine.applied(Some(app));
    let line = i.launch_line.as_deref();
    for p in &mut packs {
        p.compatibility
            .extend(conflicts::launch_notes(&p.env, line));
        let plan = p
            .apply_recipe_id
            .as_deref()
            .map(|r| engine.check(r, i.game));
        lifecycle::derive(p, plan, &ops, &applied, &cur_env);
    }

    let mut vs: HashMap<String, VsBaseline> = HashMap::new();
    for p in packs.iter().filter(|p| p.kind != PackKind::Baseline) {
        if let Some(v) = vs_for(&p.env) {
            vs.insert(p.id.clone(), v);
        }
    }
    let mut excluded = found.excluded;
    excluded.extend(composed.excluded);
    let ranking = ranking::rank(ranking::Input {
        profile: i.profile,
        packs: &packs,
        baseline_id: &base_id,
        measured: &grouped.measurements,
        vs: &vs,
        excluded: &excluded,
    });
    let used: usize = grouped.measurements.values().map(|m| m.used()).sum();
    let file = store.record(app, &packs, &ranking, used)?;

    let cur_id = pack_id(app, &cur_env);
    let current_note = if cur_env.is_empty() {
        "Baseline ativo: nenhuma correção do FRX aplicada neste jogo.".to_owned()
    } else if let Some(p) = packs.iter().find(|p| p.id == cur_id) {
        format!("Ativo agora: {}.", p.name)
    } else {
        "As correções do FRX ativas agora não correspondem a nenhum pacote composto (ex.: uma receita de diagnóstico).".to_owned()
    };
    let mut notes = composed.notes;
    notes.extend(grouped.notes);
    Ok(SmartPackReport {
        app_id: app.to_owned(),
        game_name: i.game.name.clone(),
        profile: i.profile,
        generated_at: now_iso(),
        composer_version: COMPOSER_VERSION.into(),
        rules_version: conflicts::RULES.into(),
        limits: i.limits,
        reference_session_id: grouped.reference.clone(),
        baseline_id: base_id,
        units: found.units,
        graph: composed.graph,
        excluded,
        measurements: grouped.measurements.into_values().collect(),
        ranking,
        history: file.history,
        unassigned_sessions: grouped.unassigned,
        current_pack_id: Some(cur_id),
        current_note,
        windows: store.windows(app),
        packs,
        notes,
    })
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
    let mut recipes = Vec::new();
    let mut units = Vec::new();
    for m in &ids {
        let r = engine.resolve(m, game)?;
        let u = discovery::unit_from_recipe(
            &r,
            EvidenceLevel::None,
            "",
            r.generated
                .as_ref()
                .map(|g| g.finding_ids.clone())
                .unwrap_or_default(),
            &game.findings,
        )
        .ok_or("Uma receita do pacote não é só de variáveis de ambiente.")?;
        units.push(u);
        recipes.push(r);
    }
    if let Err(bad) = conflicts::validate_combination(&units, Limits::default().max_pack_size) {
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
    if pack_id(&game.app_id, &env) != id {
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
        schema: RECIPE_SCHEMA,
        id: id.to_owned(),
        version: COMPOSER_VERSION.into(),
        trust: Trust::Pack,
        title: stored.name.chars().take(120).collect(),
        summary: format!("Define {line} nas opções de inicialização deste jogo."),
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
            requires_proton: true,
            gpu_vendor: recipes.iter().find_map(|r| r.scope.gpu_vendor),
        },
        actions: env
            .iter()
            .map(|p| RecipeAction::SetLaunchEnv {
                key: p.key.clone(),
                value: p.value.clone(),
            })
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

/// Transição da configuração atual para outro pacote, passo a passo.
pub fn switch_plan(engine: &FixEngine, app_id: &str, to: &SmartPack) -> SwitchPlan {
    let ops: Vec<Operation> = engine
        .operations(Some(app_id))
        .into_iter()
        .filter(|o| o.holds_changes() || o.state == OpState::RequiresManualStep)
        .collect();
    let (cur, _, _) = current_env(engine, app_id);
    let keep: Vec<EnvPair> = cur.iter().filter(|p| to.env.contains(p)).cloned().collect();
    let remove: Vec<EnvPair> = cur
        .iter()
        .filter(|p| !to.env.contains(p))
        .cloned()
        .collect();
    let add: Vec<EnvPair> = to
        .env
        .iter()
        .filter(|p| !cur.contains(p))
        .cloned()
        .collect();
    let mut notes = vec!["Cada passo abre a própria prévia e pede confirmação. Desfazer remove só o que o FRX adicionou; mudanças feitas por outros programas são preservadas — se houver, o passo é recusado e nada é sobrescrito.".to_owned()];
    let blocked = ops.iter().find(|o| {
        matches!(
            o.state,
            OpState::RecoveryRequired | OpState::RequiresManualStep
        )
    });
    if let Some(b) = blocked {
        notes.insert(0, format!("Resolva antes a operação pendente “{}” no Histórico (passo manual ou recuperação).", b.recipe_title));
    }
    let same = cur == to.env;
    if same {
        notes.insert(0, "Este pacote já é a configuração ativa.".into());
    }
    let mut steps: Vec<SwitchStep> = ops
        .iter()
        .filter(|o| o.state != OpState::RequiresManualStep)
        .map(|o| SwitchStep {
            kind: "rollback".into(),
            operation_id: Some(o.id.clone()),
            recipe_id: None,
            title: format!("Desfazer “{}”", o.recipe_title),
            detail: "Remove só as variáveis que esta operação adicionou.".into(),
        })
        .collect();
    if let Some(r) = &to.apply_recipe_id {
        steps.push(SwitchStep {
            kind: "apply".into(),
            operation_id: None,
            recipe_id: Some(r.clone()),
            title: format!("Aplicar “{}”", to.name),
            detail: "Prévia, confirmação, backup verificado e gravação.".into(),
        });
    }
    SwitchPlan {
        to: to.id.clone(),
        from: (!cur.is_empty()).then(|| pack_id(app_id, &cur)),
        keep,
        remove,
        add,
        possible: blocked.is_none() && !same && !steps.is_empty(),
        steps: if same { vec![] } else { steps },
        notes,
    }
}

/// Pacote A × pacote B com a mesma metodologia do ranking.
pub fn compare_packs(
    engine: &FixEngine,
    app_id: &str,
    sessions: &[SessionMeta],
    a: &str,
    b: &str,
) -> Result<PackComparison, String> {
    let grouped = evidence::group(
        app_id,
        sessions,
        |m| benchmark::session_config(engine, m),
        &baseline_id(app_id),
    );
    let metas: HashMap<&str, &SessionMeta> = sessions.iter().map(|m| (m.id.as_str(), m)).collect();
    let ma = grouped.measurements.get(a).filter(|m| m.used() > 0);
    let mb = grouped.measurements.get(b).filter(|m| m.used() > 0);
    let (Some(ma), Some(mb)) = (ma, mb) else {
        return Ok(PackComparison {
            a: a.into(),
            b: b.into(),
            deltas: vec![],
            limited: true,
            summary: "Os dois pacotes precisam de pelo menos uma sessão comparável cada para serem comparados.".into(),
            notes: vec![],
        });
    };
    let v = evidence::compare(ma, mb, &metas);
    let avg = v.get("fpsAvg").map(|d| d.outcome);
    let summary = match (avg, v.limited) {
        (Some(Outcome::Improvement), false) => {
            "B teve FPS médio maior que A além da variação medida."
        }
        (Some(Outcome::Regression), false) => {
            "B teve FPS médio menor que A além da variação medida."
        }
        (Some(_), false) => "Sem diferença de FPS médio além da variação medida.",
        (_, true) => "Resultado indicativo: poucas sessões para concluir. Repita as capturas.",
        (None, false) => "Faltam dados de FPS médio em um dos pacotes.",
    }
    .to_owned();
    Ok(PackComparison {
        a: a.into(),
        b: b.into(),
        deltas: v.deltas,
        limited: v.limited,
        summary,
        notes: v.notes,
    })
}
