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

    // Relação com outras operações do mesmo jogo.
    let mine = recipe.env_keys();
    for op in others
        .iter()
        .filter(|o| o.app_id == game.app_id && o.holds_changes())
    {
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
        .filter(|a| !matches!(a, RecipeAction::WriteGameFile { .. }))
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
            RecipeAction::WriteGameFile { .. } => {}
        }
    }
    if env.is_empty() && added_args.is_empty() {
        return b.block(
            PlanStatus::AlreadyApplied,
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
    let root = game
        .install_path
        .as_ref()
        .ok_or("Pasta de instalação do jogo desconhecida.")?;
    security::check_game_root(root)?;
    let target = security::resolve_in_root(root, rel)?;
    let existing = match std::fs::read(&target) {
        Ok(b) if b.len() > 256 * 1024 => {
            return Err("O arquivo atual é grande demais para ser substituído.".into())
        }
        Ok(b) => Some(b),
        Err(_) => None,
    };
    if existing.as_deref() == Some(content.as_bytes()) {
        return Err("O arquivo já tem exatamente este conteúdo.".into());
    }
    let change = PlannedChange {
        kind: ChangeKind::GameFile,
        target: format!("{}/{rel}", game.name),
        before: existing
            .as_ref()
            .map(|b| String::from_utf8_lossy(b).chars().take(4000).collect()),
        after: Some(content.chars().take(4000).collect()),
        summary: if existing.is_some() {
            format!("substitui {rel}")
        } else {
            format!("cria {rel}")
        },
    };
    Ok((
        Step::GameFile {
            root: root.clone(),
            rel: rel.to_owned(),
            before_sha: existing.as_deref().map(sha256_hex),
            target,
            content: content.to_owned(),
        },
        change,
    ))
}
