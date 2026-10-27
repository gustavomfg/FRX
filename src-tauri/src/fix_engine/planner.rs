//! Verificação de compatibilidade e plano (dry-run) — nada é gravado aqui.
//!
//! O plano descreve exatamente o que mudaria, com o estado atual lido do
//! disco. A impressão digital (`fingerprint`) cobre esse estado: se algo mudar
//! entre a prévia e a confirmação, a aplicação é recusada.

use std::path::Path;

use serde::Serialize;

use super::catalog::Catalog;
use super::files::sha256_hex;
use super::models::*;
use super::security;
use crate::fixes::compat::vendor_name;
use crate::steam::launch_options::{add_args, assignment, set_env, LaunchLine};
use crate::steam::{localconfig, processes};
use crate::time::{now_iso, now_unix};

pub const PLAN_TTL_SECS: u64 = 600;

pub struct Computed {
    pub plan: FixPlan,
    pub steps: Vec<Step>,
}

fn rank(s: PlanStatus) -> u8 {
    match s {
        PlanStatus::Ready => 0,
        PlanStatus::RequiresManualStep => 1,
        PlanStatus::Conflict => 2,
        PlanStatus::AlreadyApplied => 3,
        PlanStatus::AlreadyIncluded => 4,
        PlanStatus::Incompatible => 5,
        PlanStatus::NotExecutable => 6,
    }
}

struct Builder {
    status: PlanStatus,
    reasons: Vec<String>,
    checks: Vec<Check>,
}

impl Builder {
    fn ok(&mut self, label: &str, detail: impl Into<String>) {
        self.checks.push(Check {
            label: label.into(),
            state: CheckState::Ok,
            detail: detail.into(),
        });
    }
    fn warn(&mut self, label: &str, detail: impl Into<String>) {
        self.checks.push(Check {
            label: label.into(),
            state: CheckState::Warn,
            detail: detail.into(),
        });
    }
    fn block(&mut self, s: PlanStatus, label: &str, reason: impl Into<String>) {
        let reason = reason.into();
        if rank(s) > rank(self.status) {
            self.status = s;
        }
        let state = if s == PlanStatus::RequiresManualStep {
            CheckState::Warn
        } else {
            CheckState::Fail
        };
        self.checks.push(Check {
            label: label.into(),
            state,
            detail: reason.clone(),
        });
        self.reasons.push(reason);
    }
}

pub fn launch_target_label(game: &str) -> String {
    format!("Steam › {game} › Opções de inicialização (localconfig.vdf)")
}

/// Variável definida em qualquer posição antes de `%command%` (inclusive depois de wrappers).
fn env_anywhere(line: &LaunchLine, key: &str) -> Option<(bool, String)> {
    let leading: Vec<usize> = line.leading_env().into_iter().map(|(i, _, _)| i).collect();
    let end = line.command.unwrap_or(line.words.len());
    line.words[..end].iter().enumerate().find_map(|(i, w)| {
        assignment(&w.value)
            .filter(|(k, _)| *k == key)
            .map(|(_, v)| (leading.contains(&i), v.to_owned()))
    })
}

pub fn compute(
    catalog: &Catalog,
    recipe: &Recipe,
    game: &GameCtx,
    others: &[Operation],
    proc_root: &Path,
) -> Computed {
    let mut b = Builder {
        status: PlanStatus::Ready,
        reasons: vec![],
        checks: vec![],
    };
    let mut steps = Vec::new();
    let mut changes = Vec::new();
    let mut manual_possible = true;
    let mut environment = game
        .performance_conditions
        .as_ref()
        .and_then(|c| c.environment.clone())
        .unwrap_or_default();
    if environment.proton_version.is_none() && game.uses_proton == Some(true) {
        environment.proton_version = game.tool_name.clone();
    }
    let mut hardware = recipe.hardware.clone();
    if hardware.gpu_vendor.is_none() {
        hardware.gpu_vendor = recipe.scope.gpu_vendor;
    }
    let hardware_match = hardware.evaluate(Some(&environment));
    match hardware_match.state {
        crate::system::context::MatchState::Incompatible => b.block(
            PlanStatus::Incompatible,
            "Hardware e Proton",
            hardware_match.reasons.join(" "),
        ),
        crate::system::context::MatchState::Unknown
            if recipe.actions.iter().any(|a| {
                matches!(
                    a,
                    RecipeAction::InstallArtifact { .. } | RecipeAction::SetDxvkOption { .. }
                )
            }) =>
        {
            b.block(
                PlanStatus::Incompatible,
                "Hardware não confirmado",
                hardware_match.limitations.join(" "),
            )
        }
        crate::system::context::MatchState::Unknown => b.warn(
            "Hardware não confirmado",
            hardware_match.limitations.join(" "),
        ),
        _ => (),
    }

    match recipe.trust {
        Trust::Reviewed => b.ok(
            "Receita revisada",
            format!(
                "Catálogo local {} · versão {}",
                catalog.version, recipe.version
            ),
        ),
        Trust::Candidate | Trust::Discovered => b.block(
            PlanStatus::NotExecutable,
            "Receita revisada",
            "Receita ainda não revisada pelo FRX: só pode ser consultada, não aplicada.",
        ),
        Trust::Unsupported => b.block(
            PlanStatus::NotExecutable,
            "Receita revisada",
            "O FRX não sabe aplicar esta correção com segurança.",
        ),
        Trust::Revoked => b.block(
            PlanStatus::NotExecutable,
            "Receita revisada",
            "Receita revogada do catálogo.",
        ),
        Trust::Pack => match (security::validate_pack(recipe), &recipe.pack) {
            (Ok(()), Some(p)) => b.warn(
                "Smart Pack",
                format!(
                    "Composição de {} receitas validadas uma a uma e em conjunto; aplicada como uma única operação com um backup e um rollback. Composição não comprova ganho de desempenho.",
                    p.members.len()
                ),
            ),
            (r, _) => b.block(
                PlanStatus::NotExecutable,
                "Smart Pack",
                format!(
                    "Recusado pela validação do núcleo: {}",
                    r.err().unwrap_or_else(|| "sem composição registrada".into())
                ),
            ),
        },
        Trust::Generated => match (security::validate_generated(recipe), &recipe.generated) {
            (Ok(()), Some(g)) => b.warn(
                "Receita gerada",
                format!(
                    "Gerada pelo FRX (gerador {}, regras {}) a partir de {} descoberta{}. Validada pelo núcleo, não revisada por pessoas. {}",
                    g.generator_version,
                    g.rules_version,
                    g.finding_ids.len(),
                    if g.finding_ids.len() == 1 { "" } else { "s" },
                    g.evidence_note
                ),
            ),
            (r, _) => b.block(
                PlanStatus::NotExecutable,
                "Receita gerada",
                format!(
                    "Recusada pela validação do núcleo: {}",
                    r.err().unwrap_or_else(|| "sem registro de origem".into())
                ),
            ),
        },
    }

    match recipe.risk {
        Risk::Low => b.ok("Risco", "Baixo"),
        Risk::Moderate => b.warn(
            "Risco",
            recipe
                .side_effects
                .first()
                .cloned()
                .unwrap_or_else(|| "Moderado".into()),
        ),
        Risk::High => b.warn(
            "Risco",
            recipe
                .side_effects
                .first()
                .cloned()
                .unwrap_or_else(|| "Alto".into()),
        ),
    }

    if !recipe.scope.app_ids.is_empty() && !recipe.scope.app_ids.contains(&game.app_id) {
        b.block(
            PlanStatus::Incompatible,
            "Jogo",
            "A receita é específica de outro jogo.",
        );
    }
    if recipe.scope.requires_proton {
        match game.uses_proton {
            Some(true) => b.ok(
                "Proton",
                game.tool_name
                    .clone()
                    .unwrap_or_else(|| "Proton em uso".into()),
            ),
            Some(false) => b.block(
                PlanStatus::Incompatible,
                "Proton",
                "O jogo roda sem Proton; esta correção não se aplica.",
            ),
            None => b.block(
                PlanStatus::Incompatible,
                "Proton",
                "Não foi possível confirmar que o jogo usa Proton.",
            ),
        }
    }
    if let Some(req) = recipe.scope.gpu_vendor {
        if game.gpu != Some(req) {
            b.block(
                PlanStatus::Incompatible,
                "GPU",
                format!("Exige GPU {}.", vendor_name(req)),
            );
        }
    }
    if recipe.already_included_by_protonfixes && game.tool_includes_protonfixes {
        b.block(
            PlanStatus::AlreadyIncluded,
            "Protonfixes",
            "A ferramenta do jogo já aplica esta correção automaticamente.",
        );
    }

    // A pending transaction must be recovered even when its files/variables do
    // not overlap this recipe. This gate belongs to preview and execution too,
    // not only to the availability DTO displayed by the frontend.
    if let Some(pending) = others.iter().find(|op| {
        op.app_id == game.app_id
            && (op.state.in_flight()
                || matches!(
                    op.state,
                    OpState::RequiresManualStep | OpState::RecoveryRequired
                ))
    }) {
        b.block(PlanStatus::Conflict, "Operação pendente", format!(
            "Resolva a operação pendente “{}” no histórico antes de continuar. A conferência e a restauração continuam disponíveis.", pending.recipe_title
        ));
    }

    // Relação com outras operações do mesmo jogo.
    let mine = recipe.env_keys();
    let file_targets: Vec<&str> = recipe
        .actions
        .iter()
        .flat_map(|action| match action {
            RecipeAction::InstallArtifact { artifact } => artifact
                .files
                .iter()
                .map(|f| f.target_path.as_str())
                .collect(),
            RecipeAction::WriteGameFile { path, .. }
            | RecipeAction::PatchKnownConfig { path, .. }
            | RecipeAction::SetDxvkOption { path, .. } => vec![path.as_str()],
            _ => vec![],
        })
        .collect();
    for op in others
        .iter()
        .filter(|o| o.app_id == game.app_id && o.holds_changes())
    {
        if op.state.in_flight()
            || matches!(
                op.state,
                OpState::RequiresManualStep | OpState::RecoveryRequired
            )
        {
            continue;
        }
        if op.recipe_id == recipe.id {
            b.block(
                PlanStatus::AlreadyApplied,
                "Histórico",
                "Esta receita já está aplicada neste jogo.",
            );
            continue;
        }
        let other_conflicts = catalog
            .get(&op.recipe_id)
            .is_some_and(|r| r.conflicts_with.contains(&recipe.id));
        if recipe.conflicts_with.contains(&op.recipe_id) || other_conflicts {
            b.block(
                PlanStatus::Conflict,
                "Conflitos",
                format!(
                    "Conflita com “{}”, aplicada antes. Desfaça-a primeiro.",
                    op.recipe_title
                ),
            );
        } else if op.steps.iter().any(|s| match &s.step {
            Step::GameFile { rel, .. } => file_targets
                .iter()
                .any(|target| target.eq_ignore_ascii_case(rel)),
            _ => false,
        }) {
            b.block(PlanStatus::Conflict,"Conflitos",format!("“{}” já controla um dos mesmos arquivos. Desfaça a operação antes de substituir sua versão.",op.recipe_title));
        } else if op.env_writes().any(|w| mine.contains(&w.key.as_str())) {
            b.block(
                PlanStatus::Conflict,
                "Conflitos",
                format!(
                    "“{}” já controla uma das mesmas variáveis.",
                    op.recipe_title
                ),
            );
        }
    }

    let game_running = processes::game_running(proc_root, &game.app_id);
    let steam_running = processes::steam_running(proc_root);
    if game_running {
        b.block(
            PlanStatus::Conflict,
            "Jogo aberto",
            "Feche o jogo antes de alterar as configurações dele.",
        );
    }

    // Opções de inicialização.
    let launch_actions: Vec<&RecipeAction> = recipe
        .actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                RecipeAction::SetLaunchEnv { .. } | RecipeAction::AddLaunchArgs { .. }
            )
        })
        .collect();
    if !launch_actions.is_empty() {
        launch_step(
            &mut b,
            game,
            &launch_actions,
            recipe.id == format!("frx.capture.{}", game.app_id),
            &mut steps,
            &mut changes,
            &mut manual_possible,
        );
    }

    // One merged file step per documented DXVK destination; no duplicate writes.
    let mut dxvk = std::collections::BTreeMap::<String, Vec<(String, String)>>::new();
    for action in &recipe.actions {
        if let RecipeAction::SetDxvkOption { path, key, value } = action {
            dxvk.entry(path.clone())
                .or_default()
                .push((key.clone(), value.clone()));
        }
    }
    for (path, options) in dxvk {
        manual_possible = false;
        let result: Result<(Step, PlannedChange), String> = (|| {
            if recipe.actions.iter().any(|a| match a {
                RecipeAction::InstallArtifact { artifact } => {
                    artifact.files.iter().any(|f| f.target_path == path)
                }
                RecipeAction::WriteGameFile { path: p, .. }
                | RecipeAction::PatchKnownConfig { path: p, .. } => p == &path,
                _ => false,
            }) {
                return Err("A solução mistura instalação/substituição e edição do mesmo arquivo; requer plano explícito sem etapas ambíguas.".into());
            }
            let root = game
                .install_path
                .as_ref()
                .ok_or("Pasta do jogo desconhecida")?;
            let target = super::confined::Target::open(root, &path, None)?;
            let original_bytes = target.read()?;
            let bytes = original_bytes.as_deref().unwrap_or_default();
            let original =
                std::str::from_utf8(bytes).map_err(|_| "dxvk.conf não é UTF-8 válido")?;
            let content = super::security::merge_dxvk_options(&path, original, &options)?;
            let (step, change) = game_file_step_impl(game, &path, &content, true)?;
            if !matches!(&step, Step::GameFile { before_sha, identity: Some(identity), .. }
                if *before_sha == original_bytes.as_deref().map(sha256_hex) && *identity == target.identity)
            {
                return Err(
                    "O arquivo ou diretório DXVK mudou durante a prévia; atualize a análise."
                        .into(),
                );
            }
            Ok((step, change))
        })();
        match result {
            Ok((step, change)) => {
                steps.push(step);
                changes.push(change);
            }
            Err(e) => b.block(PlanStatus::Conflict, "Configuração DXVK", e),
        }
    }
    // Arquivos do jogo.
    for a in &recipe.actions {
        if let RecipeAction::WriteGameFile { path, content } = a {
            manual_possible = false;
            match game_file_step(game, path, content) {
                Ok((step, change)) => {
                    b.ok("Arquivo do jogo", change.target.clone());
                    steps.push(step);
                    changes.push(change);
                }
                Err(e) => b.block(PlanStatus::Conflict, "Arquivo do jogo", e),
            }
        }
        if let RecipeAction::InstallArtifact { artifact } = a {
            manual_possible = false;
            match artifact_steps(game, artifact) {
                Ok(list) => {
                    for (step, change) in list {
                        b.warn("Download necessário", change.summary.clone());
                        steps.push(step);
                        changes.push(change);
                    }
                }
                Err(e) => b.block(PlanStatus::NotExecutable, "Artefato requer revisão", e),
            }
        }
        if let RecipeAction::PatchKnownConfig {
            path,
            expected_sha256,
            replacements,
        } = a
        {
            manual_possible = false;
            let result = (|| {
                let root = game
                    .install_path
                    .as_ref()
                    .ok_or("Pasta do jogo desconhecida")?;
                let target = super::confined::Target::open(root, path, None)?;
                let bytes = target.read()?.ok_or("A configuração original não existe")?;
                if sha256_hex(&bytes) != *expected_sha256 {
                    return Err(
                        "A versão da configuração original não corresponde ao patch.".into(),
                    );
                }
                let mut content = super::artifacts::validate_text(path, &bytes)?;
                for r in replacements {
                    if r.find.is_empty() || content.matches(&r.find).count() != 1 {
                        return Err("O trecho original precisa ocorrer exatamente uma vez.".into());
                    }
                    content = content.replacen(&r.find, &r.replace, 1);
                }
                super::artifacts::validate_text(path, content.as_bytes())?;
                game_file_step(game, path, &content)
            })();
            match result {
                Ok((step, change)) => {
                    steps.push(step);
                    changes.push(change);
                }
                Err(e) => b.block(PlanStatus::Conflict, "Configuração conhecida", e),
            }
        }
    }

    // Keep already matching members in a partially installed artifact so its
    // complete identity is recorded in benchmark snapshots and rollback. A
    // fully matching plan remains a no-op.
    if !steps.is_empty()
        && steps.iter().all(|s| match s {
            Step::GameFile {
                artifact: Some(a),
                rel,
                before_sha,
                ..
            } => a
                .files
                .iter()
                .any(|f| f.target_path == *rel && f.sha256 == *before_sha),
            Step::GameFile {
                artifact: None,
                before_sha,
                content,
                ..
            } => before_sha.as_deref() == Some(sha256_hex(content.as_bytes()).as_str()),
            _ => false,
        })
    {
        steps.clear();
        changes.clear();
    }
    let mut destinations = std::collections::BTreeSet::new();
    if steps.iter().any(|s| match s {
        Step::GameFile { rel, .. } => !destinations.insert(rel.to_ascii_lowercase()),
        _ => false,
    }) {
        b.block(
            PlanStatus::Conflict,
            "Arquivos conflitantes",
            "Duas ações tentam controlar o mesmo arquivo; nenhuma será aplicada.",
        );
    }
    if steam_running && !steps.is_empty() {
        if manual_possible {
            b.block(
                PlanStatus::RequiresManualStep,
                "Steam aberta",
                "A Steam está aberta e regrava as opções de inicialização ao fechar. Feche a Steam para o FRX aplicar sozinho, ou cole a linha manualmente.",
            );
        } else {
            b.block(
                PlanStatus::Conflict,
                "Steam aberta",
                "Feche a Steam para aplicar esta correção.",
            );
        }
    } else if !steps.is_empty() {
        b.ok(
            "Steam fechada",
            "O FRX pode gravar a configuração com segurança.",
        );
    }
    if steps.is_empty() && b.status == PlanStatus::Ready {
        b.block(
            PlanStatus::AlreadyApplied,
            "Estado atual",
            "Nada a mudar: a configuração já está assim.",
        );
    }

    let mode = match b.status {
        PlanStatus::Ready => Some(ExecMode::Automatic),
        PlanStatus::RequiresManualStep => Some(ExecMode::Manual),
        _ => None,
    };
    let manual_line = (mode == Some(ExecMode::Manual))
        .then(|| {
            steps.iter().find_map(|s| match s {
                Step::LaunchOptions { after, .. } => Some(after.clone()),
                _ => None,
            })
        })
        .flatten();
    #[derive(Serialize)]
    struct Fp<'a> {
        environment: Option<&'a crate::system::context::EnvironmentContext>,
        game_build: Option<&'a str>,
        recipe: &'a str,
        version: &'a str,
        catalog: &'a str,
        recipe_sha: String,
        app: &'a str,
        status: PlanStatus,
        steps: &'a [Step],
        steam: bool,
        game: bool,
        tool: &'a Option<String>,
        proton: Option<bool>,
    }
    let fp = serde_json::to_vec(&Fp {
        environment: Some(&environment),
        game_build: game
            .performance_conditions
            .as_ref()
            .and_then(|c| c.game_build_id.as_deref()),
        recipe: &recipe.id,
        version: &recipe.version,
        catalog: &catalog.sha256,
        recipe_sha: sha256_hex(&serde_json::to_vec(recipe).unwrap_or_default()),
        app: &game.app_id,
        status: b.status,
        steps: &steps,
        steam: steam_running,
        game: game_running,
        tool: &game.tool_name,
        proton: game.uses_proton,
    })
    .unwrap_or_default();
    let plan = FixPlan {
        artifacts: recipe.actions.iter().filter_map(|a|match a {RecipeAction::InstallArtifact{artifact}=>Some((**artifact).clone()),_=>None}).collect(),
        environment: Some(environment),
        hardware_match: Some(hardware_match),
        performance_warnings: recipe.performance_warnings(),
        plan_id: format!("plan-{}", &crate::performance::persistence::new_id()[2..]),
        fingerprint: sha256_hex(&fp)[..32].to_owned(),
        recipe_id: recipe.id.clone(),
        recipe_version: recipe.version.clone(),
        recipe_title: recipe.title.clone(),
        app_id: game.app_id.clone(),
        game_name: game.name.clone(),
        status: b.status,
        mode,
        reasons: b.reasons,
        checks: b.checks,
        changes,
        manual_line,
        backup_note: (mode == Some(ExecMode::Automatic)).then(|| "Antes de gravar, o FRX guarda uma cópia verificada (SHA-256) de cada arquivo em fix-engine/backups/<operação>/.".into()),
        steam_running,
        game_running,
        risk: recipe.risk,
        purpose: recipe.purpose,
        trust: recipe.trust,
        effects: recipe.effects.clone(),
        side_effects: recipe.side_effects.clone(),
        sources: recipe.sources.clone(),
        generated: recipe.generated.clone(),
        pack: recipe.pack.clone(),
        created_at: now_iso(),
        expires_at: now_unix() + PLAN_TTL_SECS,
    };
    Computed { plan, steps }
}

fn launch_step(
    b: &mut Builder,
    game: &GameCtx,
    actions: &[&RecipeAction],
    capture_setup: bool,
    steps: &mut Vec<Step>,
    changes: &mut Vec<PlannedChange>,
    manual_possible: &mut bool,
) {
    let Some(root) = &game.steam_root else {
        b.block(
            PlanStatus::Incompatible,
            "Steam",
            "Instalação da Steam não encontrada.",
        );
        return;
    };
    let Some((_, config)) = localconfig::path_for(root) else {
        b.block(PlanStatus::Incompatible, "Configuração da Steam", "localconfig.vdf da conta mais recente não foi encontrado. Abra a Steam e o jogo uma vez.");
        return;
    };
    let text = match localconfig::read_file(&config) {
        Ok(t) => t,
        Err(e) => {
            *manual_possible = false;
            return b.block(PlanStatus::Conflict, "Configuração da Steam", e);
        }
    };
    let before = match localconfig::launch_options(&text, &game.app_id) {
        Ok(v) => v,
        Err(e) => {
            *manual_possible = false;
            return b.block(
                PlanStatus::Conflict,
                "Configuração da Steam",
                format!("localconfig.vdf ilegível: {e}"),
            );
        }
    };
    b.ok(
        "Opções atuais",
        before.clone().unwrap_or_else(|| "(vazias)".into()),
    );

    let mut line = before.clone();
    let mut env = Vec::new();
    let mut added_args = Vec::new();
    for a in actions {
        match a {
            RecipeAction::SetLaunchEnv { key, value } => {
                // Exceção exclusiva da receita interna resolvida pelo núcleo:
                // nenhum catálogo ou descoberta pode fornecer esses valores.
                let internal = capture_setup
                    && ((key == "MANGOHUD" && value == "1")
                        || (key == "MANGOHUD_CONFIGFILE"
                            && Path::new(value).is_absolute()
                            && value.ends_with(&format!("/mangohud/{}.conf", game.app_id))
                            && !Path::new(value)
                                .components()
                                .any(|c| matches!(c, std::path::Component::ParentDir))));
                if let Err(e) = if internal {
                    Ok(())
                } else {
                    security::check_env(key, value)
                } {
                    return b.block(PlanStatus::NotExecutable, "Segurança", e);
                }
                if let Some(cur) = line.as_deref().filter(|l| !l.trim().is_empty()) {
                    match LaunchLine::parse(cur) {
                        Ok(parsed) => match env_anywhere(&parsed, key) {
                            Some((true, v)) if v == *value => continue,
                            Some((true, v)) => {
                                return b.block(PlanStatus::Conflict, "Opções atuais", format!("A linha já define {key}={v}. Ajuste ou remova essa variável antes."));
                            }
                            Some((false, _)) => {
                                return b.block(PlanStatus::Conflict, "Opções atuais", format!("{key} aparece depois de um wrapper; o FRX não mexe nessa posição."));
                            }
                            None => {}
                        },
                        Err(e) => {
                            *manual_possible = false;
                            return b.block(
                                PlanStatus::Conflict,
                                "Opções atuais",
                                format!(
                                    "O FRX não consegue editar a linha atual com segurança: {e}"
                                ),
                            );
                        }
                    }
                }
                match set_env(line.as_deref(), key, value) {
                    Ok(c) => {
                        env.push(EnvWrite {
                            key: key.clone(),
                            written: value.clone(),
                            previous: c.previous,
                        });
                        line = Some(c.line);
                    }
                    Err(e) => {
                        *manual_possible = false;
                        return b.block(PlanStatus::Conflict, "Opções atuais", e);
                    }
                }
            }
            RecipeAction::AddLaunchArgs { args } => {
                if let Some(e) = args.iter().find_map(|a| security::check_arg(a).err()) {
                    return b.block(PlanStatus::NotExecutable, "Segurança", e);
                }
                match add_args(line.as_deref(), args) {
                    Ok((l, added)) => {
                        if !added.is_empty() {
                            line = Some(l);
                        }
                        added_args.extend(added);
                    }
                    Err(e) => {
                        *manual_possible = false;
                        return b.block(PlanStatus::Conflict, "Opções atuais", e);
                    }
                }
            }
            RecipeAction::SetDxvkOption { .. }
            | RecipeAction::WriteGameFile { .. }
            | RecipeAction::InstallArtifact { .. }
            | RecipeAction::PatchKnownConfig { .. } => {}
        }
    }
    if env.is_empty() && added_args.is_empty() {
        return b.ok(
            "Opções atuais",
            "As opções de inicialização já contêm esta correção.",
        );
    }
    let after = line.unwrap_or_default();
    // Edição posicional possível? Se não, só o modo manual resta.
    if let Err(e) = localconfig::set_launch_options(&text, &game.app_id, &after) {
        b.block(PlanStatus::RequiresManualStep, "Edição segura", format!("O FRX não consegue editar o localconfig.vdf com segurança ({e}). Cole a linha manualmente."));
    }
    let summary = env
        .iter()
        .map(|w| match &w.previous {
            Some(p) => format!("{}: {p} → {}", w.key, w.written),
            None => format!("adiciona {}={}", w.key, w.written),
        })
        .chain(
            added_args
                .iter()
                .map(|a| format!("adiciona o argumento {a}")),
        )
        .collect::<Vec<_>>()
        .join(" · ");
    changes.push(PlannedChange {
        kind: ChangeKind::LaunchOptions,
        target: launch_target_label(&game.name),
        before: before.clone(),
        after: Some(after.clone()),
        summary,
    });
    steps.push(Step::LaunchOptions {
        file_sha: sha256_hex(text.as_bytes()),
        config,
        app_id: game.app_id.clone(),
        before,
        after,
        env,
        args: added_args,
    });
}

fn game_file_step(
    game: &GameCtx,
    rel: &str,
    content: &str,
) -> Result<(Step, PlannedChange), String> {
    game_file_step_impl(game, rel, content, false)
}

fn game_file_step_impl(
    game: &GameCtx,
    rel: &str,
    content: &str,
    retain_matching_member: bool,
) -> Result<(Step, PlannedChange), String> {
    let root = game
        .install_path
        .as_ref()
        .ok_or("Pasta de instalação do jogo desconhecida.")?;
    security::check_game_root(root)?;
    let target = security::resolve_in_root(root, rel)?;
    let opened = super::confined::Target::open(root, rel, None)?;
    let existing = opened.read()?;
    let matching = existing.as_deref() == Some(content.as_bytes());
    if matching && !retain_matching_member {
        return Err("O arquivo já tem exatamente este conteúdo.".into());
    }
    let change = PlannedChange {
        kind: ChangeKind::GameFile,
        target: format!("{}/{rel}", game.name),
        before: existing
            .as_ref()
            .map(|b| String::from_utf8_lossy(b).chars().take(4000).collect()),
        after: Some(content.chars().take(4000).collect()),
        summary: if matching {
            format!("preserva {rel}, já correspondente à solução composta")
        } else if existing.is_some() {
            format!("substitui {rel}")
        } else {
            format!("cria {rel}")
        },
    };
    Ok((
        Step::GameFile {
            artifact: None,
            identity: Some(opened.identity),
            root: root.clone(),
            rel: rel.to_owned(),
            before_sha: existing.as_deref().map(sha256_hex),
            target,
            content: content.to_owned(),
        },
        change,
    ))
}

fn artifact_steps(
    game: &GameCtx,
    a: &crate::fixes::models::ArtifactCandidate,
) -> Result<Vec<(Step, PlannedChange)>, String> {
    super::artifacts::validate(a, &game.app_id)?;
    // The artifact is an independent policy boundary. Do not rely on an
    // enclosing discovery/recipe having copied its requirements correctly.
    let mut hardware = a.requirements.hardware.clone();
    if let Some(legacy_vendor) = a.requirements.gpu_vendor {
        if hardware
            .gpu_vendor
            .is_some_and(|vendor| vendor != legacy_vendor)
        {
            return Err(
                "O artefato declara fabricantes de GPU contraditórios; requer revisão.".into(),
            );
        }
        hardware.gpu_vendor = Some(legacy_vendor);
    }
    if !a.requirements.proton_versions.is_empty() {
        if hardware.proton_versions.is_empty() {
            hardware.proton_versions = a.requirements.proton_versions.clone();
        } else {
            hardware.proton_versions.retain(|version| {
                a.requirements
                    .proton_versions
                    .iter()
                    .any(|legacy| legacy.eq_ignore_ascii_case(version))
            });
            if hardware.proton_versions.is_empty() {
                return Err(
                    "O artefato declara versões do Proton contraditórias; requer revisão.".into(),
                );
            }
        }
    }
    if !hardware.proton_versions.is_empty() && game.uses_proton != Some(true) {
        return Err(
            "O artefato exige uma versão do Proton, mas seu uso pelo jogo não foi confirmado."
                .into(),
        );
    }
    let mut environment = game
        .performance_conditions
        .as_ref()
        .and_then(|c| c.environment.clone())
        .unwrap_or_default();
    if environment.proton_version.is_none() && game.uses_proton == Some(true) {
        environment.proton_version = game.tool_name.clone();
    }
    let compatibility = hardware.evaluate(Some(&environment));
    if compatibility.state != crate::system::context::MatchState::Compatible {
        return Err(format!(
            "Os requisitos do artefato não foram confirmados para este ambiente: {} {}",
            compatibility.reasons.join(" "),
            compatibility.limitations.join(" ")
        ));
    }
    if let Some(required_build) = a.requirements.game_version.as_deref() {
        match game
            .performance_conditions
            .as_ref()
            .and_then(|c| c.game_build_id.as_deref())
        {
            Some(actual) if actual == required_build => (),
            Some(_) => {
                return Err(
                    "A versão/build do jogo difere da exigida pelo artefato; requer revisão."
                        .into(),
                )
            }
            None => return Err(
                "A versão/build do jogo exigida pelo artefato não foi confirmada; requer revisão."
                    .into(),
            ),
        }
    }
    match (a.platform.as_deref(),game.uses_proton) {
        (Some("windows"),Some(false)) | (Some("linux"),Some(true)) => return Err("A plataforma do artefato não corresponde ao modo de execução do jogo; requer revisão.".into()),
        (Some("windows" | "linux"),None) => return Err("O modo de execução do jogo não foi confirmado para a plataforma do artefato; requer revisão.".into()),
        _ => (),
    }
    if matches!(a.architecture.as_deref(), Some("x86" | "x86_64")) {
        return Err("A arquitetura do jogo/artefato ainda não foi confirmada; a arquitetura da CPU não comprova a do jogo. Requer revisão.".into());
    }
    let root = game
        .install_path
        .as_ref()
        .ok_or("Pasta de instalação desconhecida")?;
    let mut output = Vec::new();
    for file in &a.files {
        let opened = super::confined::Target::open(root, &file.target_path, None)?;
        let existing = opened.read()?;
        let before = existing.as_deref().map(sha256_hex);
        let already_matching = before == file.sha256;
        if !already_matching
            && before.is_some()
            && (!file.replace || file.expected_original_sha256 != before)
        {
            return Err(format!("{} já existe e não corresponde ao original documentado; a substituição exige SHA-256 do original.",file.target_path));
        }
        if !already_matching && before.is_none() && file.expected_original_sha256.is_some() {
            return Err("O original exigido pela substituição não está instalado.".into());
        }
        output.push((
            Step::GameFile {
                artifact: Some(Box::new(a.clone())),
                identity: Some(opened.identity),
                root: root.clone(),
                rel: file.target_path.clone(),
                target: root
                    .canonicalize()
                    .map_err(|e| e.to_string())?
                    .join(&file.target_path),
                before_sha: before,
                content: String::new(),
            },
            PlannedChange {
                kind: ChangeKind::GameFile,
                target: format!("{}/{}", game.name, file.target_path),
                before: existing
                    .as_ref()
                    .map(|b| String::from_utf8_lossy(b).chars().take(4000).collect()),
                after: Some(format!(
                    "Download verificado: {} · SHA-256 {}",
                    a.asset_name,
                    file.sha256.as_deref().unwrap_or("desconhecido")
                )),
                summary: format!(
                    "{} {} · {} bytes para staging",
                    if already_matching {
                        "preserva conteúdo correspondente com backup"
                    } else if existing.is_some() {
                        "substitui com backup"
                    } else {
                        "instala"
                    },
                    file.target_path,
                    a.size_bytes
                ),
            },
        ));
    }
    Ok(output)
}

/// Read-only effective-access checks for the transaction's storage. No probe
/// files are created. ACLs and the effective process credentials are considered;
/// actual writes still revalidate identity and handle later races or I/O errors.
pub fn validate_storage(computed: &mut Computed, engine_root: &Path) {
    if computed.steps.is_empty() {
        return;
    }
    let mut failures = Vec::new();
    for name in ["backups", "operations"] {
        if let Err(error) = writable_directory(&engine_root.join(name)) {
            failures.push((
                "Backup e recuperação",
                format!("O armazenamento de {name} não permite backup e recuperação: {error}"),
            ));
        }
    }
    let mut checked = std::collections::BTreeSet::new();
    for step in &computed.steps {
        let target = match step {
            Step::LaunchOptions { config, .. } => config,
            Step::GameFile { target, .. } => target,
        };
        let Some(parent) = target.parent() else {
            continue;
        };
        if checked.insert(parent.to_owned()) {
            if let Err(error) = writable_directory(parent) {
                failures.push(("Permissão de escrita", format!("O diretório de destino não permite instalar e restaurar o arquivo: {error}")));
            }
        }
    }
    if failures.is_empty() {
        return;
    }
    for (label, message) in failures {
        computed.plan.checks.push(Check {
            label: label.into(),
            state: CheckState::Fail,
            detail: message.clone(),
        });
        computed.plan.reasons.push(message);
    }
    computed.plan.status = PlanStatus::NotExecutable;
    computed.plan.mode = None;
    computed.plan.backup_note = None;
    computed.plan.fingerprint = sha256_hex(
        &serde_json::to_vec(&(&computed.plan.fingerprint, &computed.plan.reasons))
            .unwrap_or_default(),
    )[..32]
        .to_owned();
}
fn writable_directory(path: &Path) -> Result<(), String> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("Destino não é um diretório regular autorizado.".into());
    }
    let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| "Caminho inválido")?;
    // SAFETY: path is NUL terminated; faccessat performs no writes and reports
    // effective credential access, including ACLs, for this existing directory.
    if unsafe {
        libc::faccessat(
            libc::AT_FDCWD,
            path.as_ptr(),
            libc::W_OK | libc::X_OK,
            libc::AT_EACCESS,
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}
