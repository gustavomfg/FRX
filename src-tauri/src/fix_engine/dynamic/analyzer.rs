//! Analisa cada descoberta e decide a categoria (A, B ou C) com o motivo
//! específico. Cada descoberta é analisada sozinha: ações de comentários
//! diferentes nunca são somadas numa correção só. Descobertas diferentes que
//! levam exatamente às mesmas ações viram **uma** receita, com todas as
//! origens preservadas.

use super::compatibility::{describe, vendor_bound};
use super::extraction::{Deterministic, Extracted, Interpreter};
use super::generator::{self, Input};
use super::validation::{self, EnvVerdict};
use super::{provenance, ActionView, Assessment, DynStatus, EvidenceLevel};
use crate::fix_engine::catalog::Catalog;
use crate::fix_engine::environment;
use crate::fix_engine::models::{PlanStatus, Recipe, RecipeAction, Trust};
use crate::fix_engine::security::validate_generated;
use crate::fixes::compat::vendor_name;
use crate::fixes::models::{AnalysisState, Compatibility, FindingKind, FixFinding, GpuReq};

/// O que a análise precisa saber do sistema (sem varrer a Steam).
pub struct Ctx<'a> {
    pub environment: Option<&'a crate::system::context::EnvironmentContext>,
    pub game_build_id: Option<&'a str>,
    pub app_id: &'a str,
    pub gpu: Option<GpuReq>,
    pub tool_name: Option<&'a str>,
    pub catalog: &'a Catalog,
    /// Jogo, ferramenta e pastas atuais: confere o que o ambiente comprova.
    pub game: Option<&'a crate::fix_engine::models::GameCtx>,
}

fn finish(mut a: Assessment, status: DynStatus, reasons: Vec<String>) -> Assessment {
    a.status = status;
    a.class = status.class();
    a.reasons.extend(reasons);
    a
}

/// Ações do catálogo como conjunto `NOME=valor` (só receitas apenas de variáveis).
fn env_set(r: &Recipe) -> Option<Vec<(String, String)>> {
    let mut v = Vec::new();
    for a in &r.actions {
        match a {
            RecipeAction::SetLaunchEnv { key, value } => v.push((key.clone(), value.clone())),
            _ => return None,
        }
    }
    v.sort();
    Some(v)
}

pub fn assess(f: &FixFinding, ctx: &Ctx<'_>) -> Assessment {
    assess_with(f, ctx, &Deterministic)
}

pub fn assess_with(f: &FixFinding, ctx: &Ctx<'_>, interp: &dyn Interpreter) -> Assessment {
    let (evidence, evidence_note) = validation::evidence(f);
    let a = Assessment {
        performance_evidence: Default::default(),
        performance_warnings: vec![],
        finding_id: f.id.clone(),
        app_id: ctx.app_id.to_owned(),
        class: super::DynClass::Informational,
        status: DynStatus::NoActions,
        reasons: vec![],
        evidence,
        evidence_note,
        experimental: false,
        actions: vec![],
        recipe_id: None,
        recipe: None,
        outlook: None,
        provenance: provenance::collect(f),
        interpreter: interp.name().into(),
    };

    if f.game_app_id != ctx.app_id || !crate::art::valid_app_id(&f.game_app_id) {
        return finish(
            a,
            DynStatus::Invalid,
            vec![format!(
                "A descoberta pertence a outro jogo (AppID {}); nada é gerado para este.",
                f.game_app_id.chars().take(12).collect::<String>()
            )],
        );
    }
    if !f.scope.matches_game(ctx.app_id) {
        return finish(a, DynStatus::InsufficientData, vec![
            "A fonte não confirma o vínculo com este AppID. Documentação geral e registros antigos sem atribuição ficam para consulta; pesquise novamente para verificar a origem.".into()
        ]);
    }
    if a.provenance.is_empty() {
        return finish(
            a,
            DynStatus::Invalid,
            vec!["Sem fonte rastreável (HTTPS no GitHub); nada é gerado.".into()],
        );
    }
    let mut source_hardware = f.requirements.hardware.clone();
    if source_hardware.gpu_vendor.is_none() {
        source_hardware.gpu_vendor = f.requirements.gpu_vendor;
    }
    let hw = source_hardware.evaluate(ctx.environment);
    if hw.state == crate::system::context::MatchState::Incompatible {
        return finish(a, DynStatus::Incompatible, hw.reasons);
    }
    if !f.artifacts.is_empty()
        || f.actions
            .iter()
            .any(|a| a.kind == crate::fixes::models::ActionKind::ConfigFile)
    {
        return assess_structured(f, ctx, a, interp);
    }
    if f.compatibility == Compatibility::AlreadyIncluded {
        return finish(
            a,
            DynStatus::AlreadyIncluded,
            vec![f.compatibility_note.clone()],
        );
    }

    let items = match interp.interpret(f) {
        Ok(i) => i,
        Err(e) => return finish(a, DynStatus::Invalid, vec![e]),
    };
    let mut a = a;
    a.actions = items
        .iter()
        .map(|x| match x {
            Extracted::Env { kind, raw, .. } => ActionView {
                kind: *kind,
                raw: raw.clone(),
                supported: false,
                note: String::new(),
            },
            Extracted::Unsupported {
                kind, raw, reason, ..
            } => ActionView {
                kind: *kind,
                raw: raw.clone(),
                supported: false,
                note: reason.clone(),
            },
        })
        .collect();
    if items.is_empty() {
        let why = match f.kind {
            FindingKind::Report => "Relato geral sobre o jogo, sem ação técnica estruturada. Consulte a fonte original.",
            FindingKind::ReleaseNote => "Indica que o problema foi corrigido numa versão da ferramenta; não há ação para aplicar.",
            _ => "Nenhuma ação técnica reconhecida nesta descoberta.",
        };
        return finish(a, DynStatus::NoActions, vec![why.into()]);
    }

    let unsupported: Vec<(super::DynClass, String)> = items
        .iter()
        .filter_map(|x| match x {
            Extracted::Unsupported { class, reason, .. } => Some((*class, reason.clone())),
            _ => None,
        })
        .collect();
    let mut envs: Vec<(String, String)> = Vec::new();
    let mut conditional = false;
    for x in &items {
        if let Extracted::Env {
            key,
            value,
            conditional: c,
            ..
        } = x
        {
            conditional |= *c;
            match envs.iter().find(|(k, _)| k == key) {
                Some((_, v)) if v != value => {
                    return finish(a, DynStatus::Invalid, vec![format!("A descoberta define {key} com valores diferentes; não dá para saber qual vale.")])
                }
                Some(_) => {}
                None => envs.push((key.clone(), value.clone())),
            }
        }
    }

    // Opções que a ferramenta instalada não lê não têm efeito aqui: saem do
    // plano, registradas, sem invalidar o restante da mesma fonte.
    let tool = ctx.tool_name.unwrap_or("a ferramenta selecionada");
    let mut omitted: Vec<String> = Vec::new();
    if let Some(root) = ctx.game.and_then(|g| g.tool_path.as_deref()) {
        envs.retain(|(k, _)| {
            let ignored = environment::tool_recognizes(root, k) == Some(false);
            if ignored {
                omitted.push(k.clone());
            }
            !ignored
        });
    }
    for (view, item) in a.actions.iter_mut().zip(&items) {
        if let Extracted::Env { key, .. } = item {
            if omitted.contains(key) {
                view.note = format!(
                    "Ignorada: {tool} não reconhece esta opção, então ela não teria efeito."
                );
            }
        }
    }
    if envs.is_empty() && !omitted.is_empty() && unsupported.is_empty() {
        return finish(
            a,
            DynStatus::Incompatible,
            vec![format!(
                "{tool} não reconhece {}: aplicar não teria efeito nesta versão.",
                omitted.join(", ")
            )],
        );
    }
    if let Some((key, req)) = envs.iter().find_map(|(k, _)| {
        vendor_bound(k)
            .filter(|r| ctx.gpu.is_some_and(|g| g != *r))
            .map(|r| (k, r))
    }) {
        return finish(
            a,
            DynStatus::Incompatible,
            vec![format!(
                "{key} só afeta o driver Mesa RADV de GPUs {}; seu sistema usa {}.",
                vendor_name(req),
                ctx.gpu.map(vendor_name).unwrap_or("outra GPU")
            )],
        );
    }

    a.performance_warnings = super::compatibility::performance_warnings(
        envs.iter().map(|(k, v)| (k.as_str(), v.as_str())),
    );
    a.performance_evidence = super::game_evidence::assess(f, ctx, &envs);
    for view in a.actions.iter_mut().filter(|x| x.note.is_empty()) {
        view.note = "Variável reconhecida; avaliada junto com o restante da correção.".into();
    }
    if !unsupported.is_empty() {
        let needs = unsupported
            .iter()
            .any(|(c, _)| *c == super::DynClass::NeedsValidation);
        let mut reasons: Vec<String> = unsupported.into_iter().map(|(_, r)| r).collect();
        if !envs.is_empty() {
            reasons.push("A correção tem outras partes além das variáveis; aplicar só uma parte mudaria o procedimento, então nada é gerado.".into());
        }
        return finish(
            a,
            if needs {
                DynStatus::NeedsValidation
            } else {
                DynStatus::NoActions
            },
            reasons,
        );
    }
    if f.analysis != AnalysisState::Parsed {
        return finish(a, DynStatus::NeedsValidation, vec!["Parte do arquivo original não foi interpretada com segurança; a correção completa pode ter passos que o FRX não vê.".into()]);
    }
    if conditional {
        return finish(a, DynStatus::NeedsValidation, vec!["No script original a variável só é definida sob certas condições; aplicá-la sempre mudaria o comportamento.".into()]);
    }

    // Segurança da operação, variável por variável.
    let mut known = Vec::new();
    let mut blocked = Vec::new();
    let mut unknown = Vec::new();
    let mut notes: Vec<(String, bool, String)> = Vec::new();
    for (k, v) in &envs {
        match validation::env(k, v) {
            EnvVerdict::Ok(kn) => {
                let mut note = describe(kn, v);
                if let Some(seen) = ctx.game.and_then(|g| environment::component_note(g, k, v)) {
                    note = format!("{note} {seen}");
                }
                notes.push((k.clone(), true, note));
                known.push((k.clone(), v.clone(), kn));
            }
            EnvVerdict::Blocked(r) => {
                notes.push((k.clone(), false, r.clone()));
                blocked.push(r);
            }
            EnvVerdict::Unknown(r) => {
                notes.push((k.clone(), false, r.clone()));
                unknown.push(r);
            }
        }
    }
    for (view, item) in a.actions.iter_mut().zip(&items) {
        if let Extracted::Env { key, .. } = item {
            if let Some((_, ok, n)) = notes.iter().find(|(k, ..)| k == key) {
                view.supported = *ok;
                view.note = n.clone();
            }
        }
    }
    if !blocked.is_empty() {
        return finish(a, DynStatus::Blocked, blocked);
    }
    if !unknown.is_empty() {
        return finish(a, DynStatus::NeedsValidation, unknown);
    }
    // O ambiente atual comprova que cada opção teria efeito? (mesma
    // verificação do planejador, feita de novo na prévia e na aplicação)
    if let Some(game) = ctx.game {
        for (k, v, _) in &known {
            if let Err((status, why)) = environment::check_launch_env(game, k, v) {
                let status = if status == PlanStatus::AlreadyIncluded {
                    DynStatus::AlreadyIncluded
                } else {
                    DynStatus::Incompatible
                };
                return finish(a, status, vec![why]);
            }
        }
    }

    // Uma receita do catálogo faz exatamente isto?
    // Restrições da descoberta não desaparecem quando a ação coincide com
    // um modelo técnico genérico do catálogo.
    if f.compatibility == Compatibility::Incompatible
        || f.requirements
            .gpu_vendor
            .is_some_and(|gpu| ctx.gpu != Some(gpu))
    {
        return finish(
            a,
            DynStatus::Incompatible,
            vec![f.compatibility_note.clone()],
        );
    }
    let mut set = envs.clone();
    set.sort();
    if let Some(r) = ctx.catalog.recipes.iter().find(|r| {
        (r.scope.app_ids.is_empty() || r.scope.app_ids.iter().any(|app| app == ctx.app_id))
            && env_set(r).as_ref() == Some(&set)
    }) {
        match r.trust {
            Trust::Reviewed => {
                let mut a = finish(
                    a,
                    DynStatus::Reviewed,
                    vec!["Uma receita revisada do FRX faz exatamente este ajuste.".into()],
                );
                a.recipe_id = Some(r.id.clone());
                if evidence == EvidenceLevel::Contested {
                    a.reasons.push(a.evidence_note.clone());
                }
                return a;
            }
            Trust::Revoked => {
                return finish(
                    a,
                    DynStatus::Blocked,
                    vec![
                        "O FRX revogou a receita para este ajuste por um problema conhecido."
                            .into(),
                    ],
                )
            }
            _ => {}
        }
    }

    // Compatibilidade com o sistema.
    if f.compatibility == Compatibility::Incompatible {
        return finish(
            a,
            DynStatus::Incompatible,
            vec![f.compatibility_note.clone()],
        );
    }
    if let Some(req) = known.iter().find_map(|(_, _, k)| k.gpu) {
        if ctx.gpu != Some(req) {
            return finish(
                a,
                DynStatus::Incompatible,
                vec![match ctx.gpu {
                    Some(g) => format!(
                        "Só faz sentido em GPUs {}; seu sistema usa {}.",
                        vendor_name(req),
                        vendor_name(g)
                    ),
                    None => format!(
                        "Só faz sentido em GPUs {}, e não foi possível identificar a sua.",
                        vendor_name(req)
                    ),
                }],
            );
        }
    }

    // Confiabilidade.
    if evidence == EvidenceLevel::Contested {
        let note = a.evidence_note.clone();
        return finish(
            a,
            DynStatus::Contested,
            vec![format!(
                "{note} O FRX não oferece aplicar uma correção que os relatos contestam."
            )],
        );
    }

    let recipe = generator::build(Input {
        finding: f,
        app_id: ctx.app_id,
        actions: known,
        evidence,
        evidence_note: a.evidence_note.clone(),
        performance_evidence: a.performance_evidence.clone(),
        provenance: a.provenance.clone(),
        interpreter: interp.name(),
        omitted: omitted.clone(),
    });
    if let Err(e) = validate_generated(&recipe) {
        return finish(
            a,
            DynStatus::NeedsValidation,
            vec![format!(
                "A receita gerada não passou na validação do núcleo: {e}"
            )],
        );
    }
    let mut reasons = vec!["Receita gerada automaticamente a partir da descoberta. Não foi revisada por pessoas: confira a prévia antes de aplicar.".to_owned()];
    let experimental = !matches!(
        evidence,
        EvidenceLevel::Documented | EvidenceLevel::Corroborated
    );
    if experimental {
        reasons.push(format!("Experimental: {}", a.evidence_note));
    }
    if !omitted.is_empty() {
        reasons.push(format!(
            "A fonte também cita {}; {tool} não reconhece essas opções, então elas ficaram de fora sem mudar o efeito.",
            omitted.join(", ")
        ));
    }
    if !f.requirements.proton_versions.is_empty() {
        reasons.push(format!(
            "Relatada com {}{}.",
            f.requirements.proton_versions.join(", "),
            ctx.tool_name
                .map(|t| format!("; o jogo usa {t}"))
                .unwrap_or_default()
        ));
    }
    let mut a = finish(a, DynStatus::Generated, reasons);
    a.experimental = experimental;
    a.recipe_id = Some(recipe.id.clone());
    a.outlook = super::compatibility::recipe_outlook(&recipe.actions);
    a.recipe = Some(recipe);
    a
}

fn assess_structured(
    f: &FixFinding,
    ctx: &Ctx<'_>,
    mut a: Assessment,
    interp: &dyn Interpreter,
) -> Assessment {
    let hardware = match generator::structured_hardware(f) {
        Ok(hardware) => hardware,
        Err(reason) => return finish(a, DynStatus::NeedsValidation, vec![reason]),
    };
    let mut environment = ctx.environment.cloned().unwrap_or_default();
    if environment.proton_version.is_none() {
        environment.proton_version = ctx.tool_name.map(str::to_owned);
    }
    let matched = hardware.evaluate(Some(&environment));
    match matched.state {
        crate::system::context::MatchState::Incompatible => {
            return finish(a, DynStatus::Incompatible, matched.reasons);
        }
        crate::system::context::MatchState::Unknown => {
            return finish(a, DynStatus::NeedsValidation, matched.limitations);
        }
        crate::system::context::MatchState::Compatible => (),
    }
    // Every generated-plan lookup repeats this check against the actual build;
    // a persisted recipe is never executed without current source regeneration.
    if let Some(required) = f.requirements.game_version.as_deref() {
        match ctx.game_build_id {
            Some(actual) if actual == required => (),
            Some(_) => return finish(a, DynStatus::Incompatible, vec![
                "A versão/build do jogo difere da exigida pela solução; pesquise uma solução compatível.".into()
            ]),
            None => return finish(a, DynStatus::NeedsValidation, vec![
                "A versão/build do jogo exigida pela solução não foi confirmada; requer revisão.".into()
            ]),
        }
    }
    if f.analysis != AnalysisState::Parsed || f.actions.iter().any(|a| a.conditional) {
        return finish(
            a,
            DynStatus::NeedsValidation,
            vec![
                "Procedimento incompleto ou condicional; nenhuma etapa será aplicada isoladamente."
                    .into(),
            ],
        );
    }
    if f.compatibility == Compatibility::Incompatible {
        return finish(
            a,
            DynStatus::Incompatible,
            vec![f.compatibility_note.clone()],
        );
    }
    if f.compatibility == Compatibility::AlreadyIncluded {
        return finish(
            a,
            DynStatus::AlreadyIncluded,
            vec![f.compatibility_note.clone()],
        );
    }
    if a.evidence == EvidenceLevel::Contested {
        let note = a.evidence_note.clone();
        return finish(a, DynStatus::Contested, vec![note]);
    }
    let mut extras = vec![];
    for action in f
        .actions
        .iter()
        .filter(|a| a.kind == crate::fixes::models::ActionKind::ConfigFile)
    {
        match super::extraction::dxvk_option(&action.value) {
            Ok((path, key, value)) => extras.push(RecipeAction::SetDxvkOption { path, key, value }),
            Err(e) => return finish(a, DynStatus::NeedsValidation, vec![e]),
        }
    }
    let mut remaining = f.clone();
    remaining.artifacts.clear();
    remaining
        .actions
        .retain(|a| a.kind != crate::fixes::models::ActionKind::ConfigFile);
    let mut env_recipe = None;
    if !remaining.actions.is_empty() {
        let result = assess_with(&remaining, ctx, interp);
        if !matches!(result.status, DynStatus::Generated | DynStatus::Reviewed) {
            return finish(a, result.status, result.reasons);
        }
        env_recipe = result.recipe.or_else(|| {
            result
                .recipe_id
                .as_ref()
                .and_then(|id| ctx.catalog.get(id).cloned())
        });
        let Some(recipe) = &env_recipe else {
            return finish(
                a,
                DynStatus::Invalid,
                vec!["Receita das etapas adicionais não pôde ser resolvida.".into()],
            );
        };
        extras.extend(recipe.actions.clone());
        a.performance_warnings = result.performance_warnings;
        a.performance_evidence = result.performance_evidence;
    }
    for artifact in &f.artifacts {
        if let Err(e) = crate::fix_engine::artifacts::validate(artifact, ctx.app_id) {
            return finish(a, DynStatus::NeedsValidation, vec![e]);
        }
    }
    let mut recipe = generator::artifact_recipe(f, ctx.app_id, a.provenance.clone(), hardware);
    recipe.scope.requires_proton =
        !extras.is_empty() || !recipe.hardware.proton_versions.is_empty();
    if let Some(env) = env_recipe {
        recipe.scope.gpu_vendor = env.scope.gpu_vendor.or(recipe.scope.gpu_vendor);
        recipe.side_effects.extend(env.side_effects);
    }
    recipe.actions.extend(extras);
    if !f.actions.is_empty() {
        recipe.id = format!(
            "gen.{}.{}",
            ctx.app_id,
            &crate::fix_engine::files::sha256_hex(
                &serde_json::to_vec(&(ctx.app_id, &recipe.actions, &f.requirements))
                    .unwrap_or_default()
            )[..16]
        );
        recipe.summary =
            "Aplica integralmente as opções e configurações documentadas para este jogo.".into();
        recipe.description = "Gerada de instruções explícitas da fonte para este AppID. Todas as etapas precisam ser suportadas e compatíveis. A confirmação autoriza somente as alterações da prévia; configuração aplicada não comprova funcionamento do recurso.".into();
        if f.artifacts.is_empty() {
            recipe.effects.clear();
        }
        if recipe
            .actions
            .iter()
            .any(|action| matches!(action, RecipeAction::SetDxvkOption { .. }))
        {
            recipe.effects.push("Mescla somente opções DXVK reconhecidas e preserva as demais linhas; aplica as outras etapas suportadas da mesma solução.".into());
            recipe.side_effects.push("As opções podem alterar apresentação, qualidade ou latência. Presença/configuração não comprova que o jogo carregou DXVK ou que o recurso funciona.".into());
            if recipe.actions.iter().any(|action| matches!(action, RecipeAction::SetDxvkOption { key, .. } if key.ends_with(".maxFrameRate"))) {
                recipe.side_effects.push("maxFrameRate limita o FPS; não é uma otimização universal. A documentação DXVK alerta que seu limitador interno pode produzir resultados inferiores aos de limitadores externos, dependendo do jogo.".into());
            }
            recipe.sources.truncate(7);
            recipe
                .sources
                .push(crate::fix_engine::models::RecipeSource {
                    label: "DXVK — referência de configuração (consultada 2026-10-27)".into(),
                    url: "https://github.com/doitsujin/dxvk/blob/master/dxvk.conf".into(),
                });
        }
    }
    if let Some(g) = recipe.generated.as_mut() {
        g.interpreter = format!("{}+structured-operations", interp.name());
        g.evidence = a.evidence;
        g.evidence_note = a.evidence_note.clone();
        g.performance_evidence = a.performance_evidence.clone();
    }
    if let Err(e) = validate_generated(&recipe) {
        return finish(a, DynStatus::NeedsValidation, vec![e]);
    }
    a.actions = f
        .actions
        .iter()
        .map(|x| ActionView {
            kind: x.kind,
            raw: x.value.clone(),
            supported: true,
            note: "Etapa estruturada e validada junto com toda a solução.".into(),
        })
        .collect();
    a.actions.extend(f.artifacts.iter().map(|x| ActionView {
        kind: crate::fixes::models::ActionKind::ConfigFile,
        raw: x.asset_name.clone(),
        supported: true,
        note: "Download verificado, staging, backup e rollback.".into(),
    }));
    a.experimental = !matches!(
        a.evidence,
        EvidenceLevel::Documented | EvidenceLevel::Corroborated
    );
    a.recipe_id = Some(recipe.id.clone());
    a.outlook = super::compatibility::recipe_outlook(&recipe.actions);
    a.recipe = Some(recipe);
    finish(a,DynStatus::Generated,vec!["Solução estruturada completa; o planejador valida destinos, requisitos e estado atual antes de oferecer aplicação.".into()])
}

/// Mesmas ações em descobertas diferentes → uma receita só, com todas as
/// origens e a melhor evidência entre elas.
pub fn merge_duplicates(list: &mut [Assessment]) {
    for i in 0..list.len() {
        let Some(id) = list[i].recipe.as_ref().map(|r| r.id.clone()) else {
            continue;
        };
        for j in 0..i {
            if list[j].recipe.as_ref().is_some_and(|r| r.id == id) {
                let (head, tail) = list.split_at_mut(i);
                let (first, dup) = (&mut head[j], &mut tail[0]);
                let (Some(r1), Some(r2)) = (first.recipe.as_mut(), dup.recipe.as_ref()) else {
                    break;
                };
                let (Some(g1), Some(g2)) = (r1.generated.as_mut(), r2.generated.as_ref()) else {
                    break;
                };
                if g1.game_app_id != g2.game_app_id || first.app_id != dup.app_id {
                    break;
                }
                g1.performance_evidence.merge(&g2.performance_evidence);
                first.performance_evidence.merge(&dup.performance_evidence);
                for fid in &g2.finding_ids {
                    if !g1.finding_ids.contains(fid) {
                        g1.finding_ids.push(fid.clone());
                    }
                }
                for p in &g2.provenance {
                    if g1.provenance.len() < 8 && !g1.provenance.iter().any(|x| x.url == p.url) {
                        g1.provenance.push(p.clone());
                    }
                }
                for s in &r2.sources {
                    if r1.sources.len() < 8 && !r1.sources.iter().any(|x| x.url == s.url) {
                        r1.sources.insert(r1.sources.len() - 1, s.clone());
                    }
                }
                g1.discovered_at = g1.discovered_at.clone().max(g2.discovered_at.clone());
                if g2.evidence < g1.evidence {
                    g1.evidence = g2.evidence;
                    g1.evidence_note = g2.evidence_note.clone();
                }
                dup.recipe = Some(r1.clone());
                break;
            }
        }
    }
    // A cópia mais completa vale para todas as descobertas da mesma receita.
    for i in 0..list.len() {
        if let Some(id) = list[i].recipe.as_ref().map(|r| r.id.clone()) {
            if let Some(first) = list
                .iter()
                .position(|x| x.recipe.as_ref().is_some_and(|r| r.id == id))
            {
                if first != i {
                    list[i].recipe = list[first].recipe.clone();
                }
            }
        }
    }
}

pub fn assess_all(findings: &[FixFinding], ctx: &Ctx<'_>) -> Vec<Assessment> {
    let mut v: Vec<Assessment> = findings.iter().map(|f| assess(f, ctx)).collect();
    merge_duplicates(&mut v);
    v
}

/// Regenera a receita a partir das descobertas atuais e a revalida. Usado
/// antes de cada prévia e aplicação: o registro em disco nunca é executado.
pub fn resolve(findings: &[FixFinding], ctx: &Ctx<'_>, recipe_id: &str) -> Result<Recipe, String> {
    let r = assess_all(findings, ctx)
        .into_iter()
        .find_map(|a| a.recipe.filter(|r| r.id == recipe_id))
        .ok_or("A receita gerada não corresponde mais às descobertas atuais (a pesquisa, o sistema ou as regras mudaram). Analise a correção de novo.")?;
    validate_generated(&r).map_err(|e| format!("Receita gerada recusada: {e}"))?;
    Ok(r)
}
