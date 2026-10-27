//! Evidence and resolution orchestration. Execution belongs exclusively to FixEngine.
pub mod detection;
pub mod models;
pub mod persistence;
mod plans;
pub mod research;
use crate::fix_engine::files::sha256_hex;
use crate::fix_engine::{
    models::{Effective, GameCtx, OpState},
    FixEngine,
};
use crate::time::now_iso;
use models::*;

pub fn valid_symptoms(values: &[String]) -> Result<(), String> {
    const ALLOWED: &[&str] = &[
        "startup",
        "crash",
        "low-performance",
        "stuttering",
        "physx",
        "dlss",
        "frame-generation",
        "ray-tracing",
        "fsr",
        "graphics",
        "audio",
        "video",
        "controls",
    ];
    if values.len() > 13 || values.iter().any(|s| !ALLOWED.contains(&s.as_str())) {
        return Err("Sintoma não suportado.".into());
    }
    Ok(())
}
fn category_topic(category: crate::fixes::models::Category) -> &'static str {
    use crate::fixes::models::Category::*;
    match category {
        Physics => "physx",
        Performance => "performance",
        Graphics => "graphics-stack",
        Audio => "audio",
        Video => "video",
        Input => "controls",
        _ => "stability",
    }
}
fn finding_sources(f: &crate::fixes::models::FixFinding) -> Vec<Evidence> {
    f.sources
        .iter()
        .take(8)
        .map(|s| Evidence {
            id: sha256_hex(s.url.as_bytes())[..16].into(),
            kind: "community-report".into(),
            summary: f.description.chars().take(1200).collect(),
            source_url: Some(s.url.clone()),
            publisher: Some(s.label.clone()),
            published_at: s.published_at.clone(),
            updated_at: None,
            fetched_at: f.retrieved_at.clone(),
            version: None,
            app_id: Some(f.game_app_id.clone()),
            technology: Some(category_topic(f.category).into()),
            stale: f.retrieved_at.as_str()
                < crate::time::iso8601(crate::time::now_unix().saturating_sub(86400)).as_str(),
            limitations: vec![
                "Relato histórico de origem; não confirma que o problema foi reproduzido neste computador."
                    .into(),
            ],
        })
        .collect()
}

pub fn diagnose(
    engine: &FixEngine,
    game: &GameCtx,
    symptoms: &[String],
    log: Option<&LogObservation>,
    knowledge: Vec<Evidence>,
    sessions: &[crate::performance::models::SessionMeta],
) -> Result<Report, String> {
    valid_symptoms(symptoms)?;
    if !crate::art::valid_app_id(&game.app_id) {
        return Err("AppID inválido".into());
    }
    if log.is_some_and(|l| l.app_id != game.app_id) {
        return Err("Log de outro jogo.".into());
    }
    let knowledge: Vec<_> = knowledge
        .into_iter()
        .filter(|e| e.app_id.as_deref().is_none_or(|id| id == game.app_id))
        .take(64)
        .collect();
    let environment = game
        .performance_conditions
        .as_ref()
        .and_then(|c| c.environment.clone());
    let context_fingerprint = sha256_hex(
        &serde_json::to_vec(&(
            game.app_id.as_str(),
            &game.performance_conditions,
            &game.tool_name,
            &game.gpu,
            &game.driver,
            &game.findings,
        ))
        .map_err(|e| e.to_string())?,
    );
    let technologies = detection::technologies(game, log, &knowledge);
    let mut problems = detection::problems(game, &technologies, log, symptoms);
    let mut plans = vec![];
    let views = engine.analyze(game)?;
    for finding in game
        .findings
        .iter()
        .filter(|f| f.game_app_id == game.app_id && f.scope.matches_game(&game.app_id))
        .take(64)
    {
        let Some(view) = views.iter().find(|v| v.assessment.finding_id == finding.id) else {
            continue;
        };
        let sources = finding_sources(finding);
        let problem_id = format!("finding-{}", finding.id);
        let id = format!(
            "resolution.{}.{}",
            game.app_id,
            &sha256_hex(finding.id.as_bytes())[..16]
        );
        let recipe_id = view.assessment.recipe_id.clone();
        let availability = view.availability.clone();
        let executable =
            availability.state == crate::fix_engine::availability::AvailabilityState::Available;
        let mut limitations = view.assessment.reasons.clone();
        limitations.extend(availability.reasons.iter().map(|r| r.message.clone()));
        let recipe = recipe_id
            .as_ref()
            .and_then(|id| engine.resolve(id, game).ok());
        let topic = category_topic(finding.category);
        plans.push(ResolutionPlan {
            availability,
            id: id.clone(),
            app_id: game.app_id.clone(),
            title: finding.title.clone(),
            objective: format!(
                "Investigar/corrigir o problema relatado de {topic}; não implica ganho de FPS."
            ),
            problem_id: problem_id.clone(),
            recipe_id,
            state: if executable {
                "applicable"
            } else {
                "review"
            }
            .into(),
            actions: finding
                .actions
                .iter()
                .map(|a| a.label.clone())
                .chain(finding.artifacts.iter().map(|a| {
                    format!(
                        "Preparar {} após confirmar origem, requisitos e destinos",
                        a.asset_name
                    )
                }))
                .collect(),
            dependencies: vec![
                "Revalidar jogo, hardware, fonte e estado atual na prévia do Fix Engine.".into(),
            ],
            risks: recipe.as_ref()
                .map(|r| r.side_effects.clone())
                .unwrap_or_else(|| {
                    vec!["Procedimento ainda não validado para aplicação automática.".into()]
                }),
            sources,
            reversible: executable,
            verification_method: if topic == "performance" {
                "Performance Lab: comparar sessões do mesmo cenário e ambiente"
            } else {
                "Conferência do estado aplicado, seguida de observação em execução ou teste do sintoma"
            }
            .into(),
            success_criterion:
                "Configuração persistida confirma somente a aplicação. O sintoma precisa de evidência independente para ser considerado corrigido."
                    .into(),
            operation_id: None,
            verification: "Funcionamento ainda não verificado.".into(),
            limitations,
        });
        problems.push(Problem {
            id: problem_id,
            technology: topic.into(),
            symptom: finding.title.clone(),
            certainty: "reported".into(),
            evidence: finding_sources(finding),
            hypotheses: vec![
                "A fonte relaciona esta configuração ao problema; a causa local ainda precisa ser confirmada."
                    .into(),
            ],
            ruled_out: vec![],
            limitations: vec![
                "Problema conhecido relatado, não necessariamente presente nesta instalação.".into(),
            ],
            next_action: if executable {
                "Revisar plano e seus riscos"
            } else {
                "Consultar o impedimento e a fonte"
            }
            .into(),
            plan_ids: vec![id],
        });
    }
    // A documented diagnostic operation is useful even with no community recipe.
    if symptoms.iter().any(|s| s == "startup" || s == "crash") && game.uses_proton == Some(true) {
        let rid = "frx.diagnostics.proton-log";
        let availability = engine.availability(rid, game);
        let executable =
            availability.state == crate::fix_engine::availability::AvailabilityState::Available;
        let id = format!("resolution.{}.proton-log", game.app_id);
        plans.push(ResolutionPlan {
            availability: availability.clone(),
            id: id.clone(),
            app_id: game.app_id.clone(),
            title: "Preparar diagnóstico do Proton".into(),
            objective:
                "Registrar a próxima execução para investigar a falha; esta ação não corrige o crash."
                    .into(),
            problem_id: "diagnostic-log".into(),
            recipe_id: Some(rid.into()),
            state: if executable {
                "applicable"
            } else {
                "review"
            }
            .into(),
            actions: vec![
                "Definir PROTON_LOG=1 nas opções deste jogo, preservando as demais.".into(),
            ],
            dependencies: vec!["Jogo usa Proton; execução posterior depende de você.".into()],
            risks: vec![
                "O log pode conter caminhos pessoais e crescer durante a execução. Importe apenas o arquivo que autorizar."
                    .into(),
            ],
            sources: vec![Evidence {
                id: "proton-runtime-log".into(),
                kind: "official-documentation".into(),
                summary: "PROTON_LOG habilita diagnóstico; não é correção de desempenho.".into(),
                source_url: Some(
                    "https://github.com/ValveSoftware/Proton#runtime-config-options".into(),
                ),
                publisher: Some("ValveSoftware".into()),
                published_at: None,
                updated_at: None,
                fetched_at: now_iso(),
                version: Some("FRX diagnostic adapter 1".into()),
                app_id: None,
                technology: Some("stability".into()),
                stale: false,
                limitations: vec![
                    "Documentação técnica geral, não prova de um bug neste jogo.".into(),
                ],
            }],
            reversible: true,
            verification_method:
                "Conferir LaunchOptions pelo Fix Engine; depois importar log autorizado do mesmo AppID."
                    .into(),
            success_criterion:
                "Variável persistida verifica a preparação. Um log novo fornece evidências; ausência de erro no trecho não prova que o crash foi corrigido."
                    .into(),
            operation_id: None,
            verification: "Diagnóstico ainda não preparado.".into(),
            limitations: availability.reasons.iter().map(|r|r.message.clone()).collect(),
        });
        problems.push(Problem {
            id: "diagnostic-log".into(),
            technology: "stability".into(),
            symptom: "Investigar inicialização ou crash relatado".into(),
            certainty: "user-reported".into(),
            evidence: vec![],
            hypotheses: vec![
                "Faltam dados da execução para distinguir dependência, driver e falha do jogo."
                    .into(),
            ],
            ruled_out: vec![],
            limitations: vec!["O FRX não inicia o jogo nem ativa logs sem confirmação.".into()],
            next_action: "Preparar diagnóstico, reproduzir voluntariamente e importar o log."
                .into(),
            plan_ids: vec![id],
        });
    }
    plans::add_documented_reviews(game, symptoms, &knowledge, &mut problems, &mut plans);
    synchronize(engine, game, &mut plans);
    let mut packs = vec![];
    if !plans.is_empty() {
        if let Ok(smart) = crate::smart_packs::build(
            engine,
            crate::smart_packs::BuildInput {
                game,
                sessions,
                profile: crate::smart_packs::models::Profile::Balanced,
                limits: Default::default(),
                launch_line: crate::smart_packs::launch_line(game),
            },
        ) {
            for pack in smart.packs.iter().filter(|p| !p.members.is_empty()) {
                let linked: Vec<_> = plans
                    .iter()
                    .filter(|p| {
                        p.recipe_id
                            .as_ref()
                            .is_some_and(|r| pack.members.iter().any(|m| &m.recipe_id == r))
                    })
                    .collect();
                // No category filler and no grouping unrelated problems just to make a pack.
                if linked.len() != pack.members.len()
                    || linked.is_empty()
                    || linked.iter().any(|p| p.problem_id != linked[0].problem_id)
                {
                    continue;
                }
                packs.push(ResolutionPack {
                    id: format!("resolution-{}", pack.id),
                    app_id: game.app_id.clone(),
                    purpose: problems
                        .iter()
                        .find(|p| p.id == linked[0].problem_id)
                        .map(|p| p.technology.clone())
                        .unwrap_or_else(|| "compatibility".into()),
                    title: pack.name.clone(),
                    plan_ids: linked.iter().map(|p| p.id.clone()).collect(),
                    smart_pack_id: Some(pack.id.clone()),
                    explanation:
                        "Composição existente do Smart Packs, restrita ao mesmo problema e às ações validadas."
                            .into(),
                });
            }
        }
    }
    Ok(Report {
        symptoms: symptoms.to_vec(),
        schema: 1,
        app_id: game.app_id.clone(),
        game_name: game.name.clone(),
        generated_at: now_iso(),
        context_fingerprint,
        environment,
        technologies,
        problems,
        plans,
        packs,
        knowledge,
        limitations: vec![
            "Diagnóstico estático e fontes técnicas não atestam funcionamento em execução.".into(),
            "MangoHud mede FPS/frametime; isso não comprova frame generation, latência ou qualidade visual."
                .into(),
            "DLLs, troca de Proton e atualizações de componentes sem política específica permanecem em revisão."
                .into(),
        ],
    })
}
fn synchronize(engine: &FixEngine, game: &GameCtx, plans: &mut [ResolutionPlan]) {
    let ops = engine.operations(Some(&game.app_id));
    let applied = engine.applied(Some(&game.app_id));
    for p in plans {
        if let Some(op) = ops.iter().find(|o| {
            p.recipe_id.as_deref() == Some(&o.recipe_id)
                || p.problem_id
                    .strip_prefix("finding-")
                    .is_some_and(|finding| o.finding_ids.iter().any(|id| id == finding))
        }) {
            p.operation_id = Some(op.id.clone());
            if matches!(
                op.state,
                OpState::RecoveryRequired | OpState::Conflict | OpState::Failed
            ) && op.steps.iter().any(|step| step.backup.is_some())
            {
                p.reversible = true;
            }
            if op.state == OpState::Applied {
                p.reversible = true;
                let current = p
                    .recipe_id
                    .as_ref()
                    .and_then(|id| engine.resolve(id, game).ok());
                let fingerprint = current
                    .as_ref()
                    .map(crate::smart_packs::discovery::fingerprint);
                if fingerprint.is_none() || fingerprint != op.recipe_fingerprint {
                    p.state = "error".into();
                    p.verification = "A receita ou sua fonte mudou desde a aplicação. A operação antiga foi preservada; revise o histórico e o rollback antes de testar.".into();
                    continue;
                }
            }
            match op.state {
                OpState::Applied => {
                    p.reversible = true;
                    let active = applied
                        .iter()
                        .any(|a| a.operation_id == op.id && a.effective == Effective::Active);
                    p.state = if active {
                        "applied-unverified"
                    } else {
                        "error"
                    }
                    .into();
                    p.verification = if active {
                        "Correção aplicada — funcionamento ainda não verificado."
                    } else {
                        "A configuração divergiu no disco; confira o histórico antes de testar."
                    }
                    .into();
                }
                OpState::RolledBack => {
                    p.state = "rolled-back".into();
                    p.verification =
                        "Configuração restaurada pelo Fix Engine; o resultado do jogo continua independente."
                            .into();
                }
                OpState::RecoveryRequired | OpState::Conflict | OpState::Failed => {
                    p.state = "error".into();
                    p.verification = op.message.clone().unwrap_or_default()
                }
                _ => (),
            }
        }
    }
}
pub fn verify(engine: &FixEngine, report: &mut Report, plan_id: &str) -> Result<(), String> {
    let p = report
        .plans
        .iter_mut()
        .find(|p| p.id == plan_id && p.app_id == report.app_id)
        .ok_or("Plano não encontrado para este jogo")?;
    if p.state == "error" {
        // Source/context drift detected during fresh diagnosis must not be
        // overwritten by a check of the older operation's bytes alone.
        return Ok(());
    }
    let op_id = p
        .operation_id
        .as_ref()
        .ok_or("Aplique uma correção antes de conferir seu estado")?;
    let op = engine.operation(op_id)?;
    if op.app_id != report.app_id || p.recipe_id.as_deref() != Some(&op.recipe_id) {
        return Err("Operação de outro plano ou jogo".into());
    }
    if op.state == OpState::Applied
        && engine
            .applied(Some(&report.app_id))
            .iter()
            .any(|a| a.operation_id == op.id && a.effective == Effective::Active)
    {
        p.state = "verified-configuration".into();
        p.verification =
            "Configuração aplicada e persistida conferida no disco. Funcionamento do recurso/sintoma ainda não verificado; faça o teste indicado."
                .into();
    } else {
        p.verification =
            "Não foi possível confirmar a configuração aplicada. Consulte o histórico/rollback."
                .into();
        p.state = "error".into();
    }
    Ok(())
}
