//! Complete Experience Pack: um pacote recomendado por jogo e ambiente.
//!
//! Reúne, a partir das descobertas do jogo e das melhorias do catálogo que a
//! instalação comprova, as ações que têm evidência suficiente, valem para o
//! hardware e o Proton atuais e não conflitam entre si. Cada ação é uma
//! receita já validada pelo Fix Engine; descobertas que descrevem a mesma
//! operação viram uma ação só, com todas as fontes. O que ficou de fora
//! aparece agrupado, com o motivo.
//!
//! Aplicar usa a receita composta `pack.<appid>.<hash>` dos Smart Packs: uma
//! prévia, um backup, uma gravação e um rollback para o pacote inteiro.

use serde::Serialize;

use super::benchmark::{current_env, current_file_fixes};
use super::composition::{baseline, make};
use super::conflicts::{graph, validate_combination};
use super::models::*;
use super::{discovery, lifecycle, store_for};
use crate::fix_engine::availability::{Availability, AvailabilityState};
use crate::fix_engine::dynamic::compatibility::performance_warnings;
use crate::fix_engine::dynamic::{DynStatus, EvidenceLevel};
use crate::fix_engine::models::{GameCtx, Risk};
use crate::fix_engine::FixEngine;
use crate::fixes::models::{ActionKind, Category, FixFinding};
use crate::time::now_iso;

/// Receitas por pacote: o mesmo limite que a validação de Smart Packs aceita.
pub const MAX_MEMBERS: usize = 6;

/// Melhorias revisadas do catálogo que o próprio planejador só libera quando
/// a instalação comprova a necessidade (GPU, ferramenta, arquivos do jogo).
const IMPROVEMENTS: &[&str] = &["frx.proton.dlss-upgrade", crate::resolution::physx::RECIPE];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperiencePack {
    pub app_id: String,
    pub game_name: String,
    pub generated_at: String,
    pub environment: Vec<String>,
    /// Ações recomendadas; `selected = false` quando o usuário desligou.
    pub actions: Vec<PackAction>,
    pub left_out: Vec<LeftOut>,
    pub areas: Vec<String>,
    pub risks: Vec<String>,
    pub compatibility: Vec<String>,
    /// Arquivos do jogo que o pacote grava, com o hash do conteúdo final.
    pub file_fixes: Vec<FileFixRef>,
    pub pack_id: Option<String>,
    pub apply_recipe_id: Option<String>,
    pub availability: Availability,
    pub state: PackState,
    pub state_detail: String,
    pub operation_id: Option<String>,
    /// Há melhorias encontradas que o FRX ainda não sabe aplicar.
    pub partial: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackAction {
    pub recipe_id: String,
    pub title: String,
    pub area: String,
    pub changes: Vec<String>,
    pub effects: Vec<String>,
    pub side_effects: Vec<String>,
    pub risk: Risk,
    pub evidence: EvidenceLevel,
    pub evidence_note: String,
    /// Relatos que dizem que resolveu / que não resolveu ou piorou, somados
    /// entre todas as descobertas com a mesma operação.
    pub positive: u32,
    pub negative: u32,
    pub sources: Vec<SourceRef>,
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceRef {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum LeftOutKind {
    /// Relatos contrários iguais ou maiores que os favoráveis.
    Negative,
    /// Não vale para este hardware, Proton ou instalação.
    Incompatible,
    /// Alternativa a uma ação já incluída.
    Conflict,
    /// Risco conhecido de perda de desempenho ou limitação de FPS.
    Risky,
    /// O Proton já aplica.
    AlreadyIncluded,
    /// Pouca evidência: um relato só ou nenhum resultado relatado.
    WeakEvidence,
    /// O FRX ainda não sabe aplicar com segurança.
    Unsupported,
    /// Relatos sem ação técnica ou sem vínculo com este jogo.
    NoAction,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeftOut {
    pub title: String,
    pub reason: String,
    pub kind: LeftOutKind,
    /// Variáveis da ação, quando o FRX conseguiu lê-las.
    pub changes: Vec<String>,
    /// Descobertas reunidas nesta linha.
    pub count: usize,
}

/// Variáveis que a descoberta define, em ordem: a identidade da operação.
fn finding_env(f: &FixFinding) -> Vec<EnvPair> {
    let mut env: Vec<EnvPair> = Vec::new();
    for a in &f.actions {
        let tokens: Vec<&str> = match a.kind {
            ActionKind::Env => vec![a.value.as_str()],
            ActionKind::LaunchOption => a
                .value
                .split_whitespace()
                .take_while(|t| *t != "%command%")
                .collect(),
            _ => return vec![],
        };
        for t in tokens {
            if let Some((k, v)) = t.split_once('=') {
                env.push(EnvPair {
                    key: k.to_owned(),
                    value: v.to_owned(),
                });
            }
        }
    }
    env.sort();
    env.dedup();
    env
}

/// Relatos e fontes de todas as descobertas que fazem exatamente a mesma
/// operação, em qualquer escopo: um relato negativo não some porque veio de
/// um comentário sem atribuição confirmada ao jogo.
fn same_operation(u: &Unit, findings: &[FixFinding]) -> (u32, u32, Vec<SourceRef>) {
    let (mut pos, mut neg, mut sources) = (0, 0, Vec::<SourceRef>::new());
    let mut push = |label: &str, url: &str| {
        if !sources.iter().any(|s| s.url == url) {
            sources.push(SourceRef {
                label: label.to_owned(),
                url: url.to_owned(),
            });
        }
    };
    for f in findings
        .iter()
        .filter(|f| u.finding_ids.contains(&f.id) || (!u.env.is_empty() && finding_env(f) == u.env))
    {
        pos += f.evidence.positive;
        neg += f.evidence.negative;
        for s in &f.sources {
            push(&s.label, &s.url);
        }
    }
    for s in &u.sources {
        push(&s.label, &s.url);
    }
    (pos, neg, sources)
}

fn area(c: Category) -> &'static str {
    match c {
        Category::Performance => "Desempenho",
        Category::Graphics | Category::Video => "Gráficos",
        Category::Physics => "Física (PhysX)",
        Category::Audio => "Áudio",
        Category::Input => "Controles",
        _ => "Compatibilidade",
    }
}

fn left_kind(status: Option<DynStatus>, reason: &str) -> LeftOutKind {
    match status {
        Some(DynStatus::Contested) => LeftOutKind::Negative,
        Some(DynStatus::Incompatible) => LeftOutKind::Incompatible,
        Some(DynStatus::AlreadyIncluded) => LeftOutKind::AlreadyIncluded,
        Some(DynStatus::Invalid | DynStatus::NoActions | DynStatus::InsufficientData) => {
            LeftOutKind::NoAction
        }
        _ if reason.starts_with("Relato geral") => LeftOutKind::NoAction,
        _ => LeftOutKind::Unsupported,
    }
}

/// Aplicável agora, já aplicado (o pacote continua reconhecendo a ação) ou
/// bloqueado só por um impedimento temporário (Steam aberta, operação do FRX
/// sobre as mesmas variáveis): o estado do pacote diz se dá para aplicar.
fn usable(a: &Availability) -> bool {
    matches!(
        a.state,
        AvailabilityState::Available | AvailabilityState::Applied | AvailabilityState::Temporary
    )
}

/// Cadeia de PhysX por GPU do jogo (jogo com PhysX, GPU, driver, prefixo).
fn physx_plan(
    engine: &FixEngine,
    game: &GameCtx,
) -> Option<crate::resolution::models::ResolutionPlan> {
    crate::resolution::physx::add(
        engine,
        game,
        super::launch_line(game).as_deref(),
        std::path::Path::new("/"),
        &mut [],
    )
    .map(|(_, plan)| plan)
}

/// Melhorias do catálogo que o planejador libera para esta instalação.
pub fn catalog_units(engine: &FixEngine, game: &GameCtx) -> Vec<Unit> {
    let (frx_env, _, _) = current_env(engine, &game.app_id);
    IMPROVEMENTS
        .iter()
        .filter(|id| usable(&engine.availability(id, game)))
        .filter_map(|id| {
            let r = engine.recipe(id).ok()?;
            discovery::unit_from_recipe(
                &game.app_id,
                r,
                EvidenceLevel::Documented,
                "Opção documentada pela ferramenta e conferida nesta instalação.",
                vec![],
                &[],
            )
        })
        // PhysX: só quando a cadeia real do jogo pede a receita, ou quando
        // foi o FRX que a aplicou (o pacote continua podendo ser desfeito).
        .filter(|u| {
            u.recipe_id != crate::resolution::physx::RECIPE
                || u.env.iter().all(|p| frx_env.contains(p))
                || physx_plan(engine, game).is_some_and(|p| p.recipe_id.is_some())
        })
        .collect()
}

pub fn build(
    engine: &FixEngine,
    game: &GameCtx,
    launch_line: Option<&str>,
    without: &[String],
) -> Result<ExperiencePack, String> {
    let app = game.app_id.as_str();
    if !crate::art::valid_app_id(app) {
        return Err("AppID inválido.".into());
    }
    let views = engine.analyze(game)?;
    let found = discovery::discover(app, &views, &game.findings, |id| {
        engine.recipe(id).ok().cloned()
    });
    let mut units = found.units;
    for u in catalog_units(engine, game) {
        if !units.iter().any(|x| x.env == u.env) {
            units.push(u);
        }
    }

    let mut left: Vec<LeftOut> = Vec::new();
    let mut note = |title: &str, changes: Vec<String>, reason: String, kind: LeftOutKind| match left
        .iter_mut()
        .find(|l| {
            l.kind == kind
                && l.reason == reason
                && (kind == LeftOutKind::NoAction || (l.title == title && l.changes == changes))
        }) {
        Some(l) => l.count += 1,
        None => left.push(LeftOut {
            title: title.to_owned(),
            reason,
            kind,
            changes,
            count: 1,
        }),
    };
    let pairs = |env: &[EnvPair]| -> Vec<String> {
        env.iter()
            .map(|p| format!("{}={}", p.key, p.value))
            .collect()
    };
    if let Some(p) = physx_plan(engine, game)
        .filter(|p| p.state == "already-present")
        .filter(|_| {
            !units
                .iter()
                .any(|u| u.recipe_id == crate::resolution::physx::RECIPE)
        })
    {
        note(
            &p.title,
            vec![],
            p.availability
                .reasons
                .first()
                .map(|r| r.message.clone())
                .unwrap_or_default(),
            LeftOutKind::AlreadyIncluded,
        );
    }
    for x in &found.excluded {
        let changes = game
            .findings
            .iter()
            .find(|f| f.id == x.id)
            .map(|f| pairs(&finding_env(f)))
            .unwrap_or_default();
        note(
            &x.title,
            changes,
            x.reason.clone(),
            left_kind(x.status, &x.reason),
        );
    }

    // Candidatas: evidência suficiente, sem relatos negativos dominantes,
    // sem risco conhecido de desempenho e aplicáveis agora.
    let mut candidates: Vec<(Unit, u32, u32, Vec<SourceRef>)> = Vec::new();
    for mut u in units {
        if let Err(reason) = super::populate_preview_files(engine, game, &mut u)
            .and_then(|_| super::composition::admissible(app, &u))
        {
            note(&u.title, pairs(&u.env), reason, LeftOutKind::Unsupported);
            continue;
        }
        let (pos, neg, sources) = same_operation(&u, &game.findings);
        let warnings =
            performance_warnings(u.env.iter().map(|p| (p.key.as_str(), p.value.as_str())));
        let reason = if u.already_included {
            Some((
                "O Proton já aplica este ajuste.".to_owned(),
                LeftOutKind::AlreadyIncluded,
            ))
        } else if neg > 0 && neg >= pos {
            Some((
                format!("{neg} relato(s) dizem que não resolveu ou piorou."),
                LeftOutKind::Negative,
            ))
        } else if u.perf == PerfHint::FrameCap || !warnings.is_empty() {
            Some((
                warnings
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "Limita o FPS; não é uma melhoria.".into()),
                LeftOutKind::Risky,
            ))
        } else if !matches!(
            u.evidence,
            EvidenceLevel::Documented | EvidenceLevel::Corroborated
        ) {
            Some((
                if pos == 0 {
                    "Citado sem resultado relatado; ninguém disse que resolveu.".to_owned()
                } else {
                    format!("Só {pos} relato favorável; o FRX exige documentação ou relatos confirmados.")
                },
                LeftOutKind::WeakEvidence,
            ))
        } else {
            let a = engine.availability(&u.recipe_id, game);
            (!usable(&a)).then(|| {
                (
                    a.reasons
                        .iter()
                        .map(|r| r.message.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                    LeftOutKind::Incompatible,
                )
            })
        };
        match reason {
            Some((r, kind)) => note(&u.title, pairs(&u.env), r, kind),
            None => candidates.push((u, pos, neg, sources)),
        }
    }

    // Ordem: evidência mais forte, mais relatos favoráveis, menor risco,
    // menos variáveis, ID.
    candidates.sort_by(|a, b| {
        a.0.evidence
            .cmp(&b.0.evidence)
            .then(b.1.cmp(&a.1))
            .then(a.0.risk.cmp(&b.0.risk))
            .then(a.0.env.len().cmp(&b.0.env.len()))
            .then(a.0.recipe_id.cmp(&b.0.recipe_id))
    });
    let mut chosen: Vec<(Unit, u32, u32, Vec<SourceRef>)> = Vec::new();
    for c in candidates {
        let mut trial: Vec<Unit> = chosen.iter().map(|x| x.0.clone()).collect();
        trial.push(c.0.clone());
        if chosen.len() >= MAX_MEMBERS {
            note(
                &c.0.title,
                pairs(&c.0.env),
                format!("Limite de {MAX_MEMBERS} ações por pacote."),
                LeftOutKind::Conflict,
            );
        } else if let Err(edges) = validate_combination(&trial, MAX_MEMBERS).or_else(|e| {
            if trial.len() == 1 {
                Ok(())
            } else {
                Err(e)
            }
        }) {
            note(
                &c.0.title,
                pairs(&c.0.env),
                edges.first().map(|e| e.reason.clone()).unwrap_or_default(),
                LeftOutKind::Conflict,
            );
        } else {
            chosen.push(c);
        }
    }
    left.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.title.cmp(&b.title)));

    let actions: Vec<PackAction> = chosen
        .iter()
        .map(|(u, pos, neg, sources)| {
            let recipe = engine.resolve(&u.recipe_id, game).ok();
            PackAction {
                recipe_id: u.recipe_id.clone(),
                title: recipe.as_ref().map_or(u.title.clone(), |r| r.title.clone()),
                area: recipe
                    .as_ref()
                    .map_or("Compatibilidade", |r| area(r.category))
                    .into(),
                changes: pairs(&u.env)
                    .into_iter()
                    .chain(
                        u.preview_files
                            .iter()
                            .flatten()
                            .map(|f| f.target_path.clone()),
                    )
                    .collect(),
                effects: u.effects.clone(),
                side_effects: u.side_effects.clone(),
                risk: u.risk,
                evidence: u.evidence,
                evidence_note: u.evidence_note.clone(),
                positive: *pos,
                negative: *neg,
                sources: sources.clone(),
                selected: !without.contains(&u.recipe_id),
            }
        })
        .collect();
    let members: Vec<&Unit> = chosen
        .iter()
        .map(|c| &c.0)
        .filter(|u| !without.contains(&u.recipe_id))
        .collect();

    let mut areas: Vec<String> = Vec::new();
    for a in actions.iter().filter(|a| a.selected) {
        if !areas.contains(&a.area) {
            areas.push(a.area.clone());
        }
    }
    let mut environment = Vec::new();
    if let Some(g) = game.gpu {
        environment.push(format!("GPU {}", crate::fixes::compat::vendor_name(g)));
    }
    if let Some(d) = &game.driver {
        environment.push(format!("driver {d}"));
    }
    if let Some(t) = &game.tool_name {
        environment.push(t.clone());
    }

    let mut pack = if members.is_empty() {
        baseline(app)
    } else {
        let all: Vec<Unit> = chosen.iter().map(|c| c.0.clone()).collect();
        make(
            app,
            &members,
            if members.len() == 1 {
                PackKind::Single
            } else {
                PackKind::Combination
            },
            Goal::Balanced,
            "Complete Experience Pack".into(),
            "Reúne as melhorias com evidência suficiente para este jogo, este hardware e este Proton, sem conflitos entre si.".into(),
            &graph(&all),
        )
    };
    let partial = left.iter().any(|l| l.kind == LeftOutKind::Unsupported);
    if members.is_empty() {
        return Ok(ExperiencePack {
            app_id: app.to_owned(),
            game_name: game.name.clone(),
            generated_at: now_iso(),
            environment,
            actions,
            left_out: left,
            areas,
            risks: vec![],
            compatibility: vec![],
            file_fixes: vec![],
            pack_id: None,
            apply_recipe_id: None,
            availability: Availability::reason(
                AvailabilityState::Investigation,
                "empty-pack",
                "Nenhuma melhoria tem evidência e suporte suficientes para entrar no pacote.",
            ),
            state: PackState::Candidate,
            state_detail: String::new(),
            operation_id: None,
            partial,
        });
    }
    // Registrado antes da prévia: a receita composta é resolvida pelo registro.
    store_for(engine).remember(app, std::slice::from_ref(&pack))?;
    pack.compatibility
        .extend(super::conflicts::launch_notes(&pack.env, launch_line));
    let apply = pack.apply_recipe_id.clone().unwrap_or_default();
    let (cur_env, _, _) = current_env(engine, app);
    lifecycle::derive(
        &mut pack,
        Some(engine.check(&apply, game)),
        &engine.operations(Some(app)),
        &engine.applied(Some(app)),
        &cur_env,
        &current_file_fixes(engine, app),
    );
    let mut risks = pack.side_effects.clone();
    risks.extend(pack.performance_warnings.clone());
    Ok(ExperiencePack {
        app_id: app.to_owned(),
        game_name: game.name.clone(),
        generated_at: now_iso(),
        environment,
        actions,
        left_out: left,
        areas,
        risks,
        compatibility: pack.compatibility.clone(),
        file_fixes: pack.file_fixes.clone(),
        availability: engine.availability(&apply, game),
        pack_id: Some(pack.id.clone()),
        apply_recipe_id: Some(apply),
        state: pack.state,
        state_detail: pack.state_detail.clone(),
        operation_id: pack.operation_id.clone(),
        partial,
    })
}
