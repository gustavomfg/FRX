//! Monta a entrada do diagnóstico a partir do Fix Engine. Não escreve em jogo,
//! Steam ou prefixo; só dados próprios do FRX, como qualquer análise do motor.

use super::models::*;
use crate::fix_engine::environment::installed_files;
use crate::fix_engine::models::GameCtx;
use crate::fix_engine::FixEngine;
use crate::fixes::models::ProviderStatus;
use crate::smart_packs::experience::catalog_units;
use crate::steam::launch_options::LaunchLine;
use crate::time::now_iso;

pub fn input(
    engine: &FixEngine,
    game: &GameCtx,
    symptom: Symptom,
    gpu_name: Option<String>,
    launch_line: Option<&str>,
    providers: Vec<ProviderStatus>,
) -> Result<DiagnosisInput, String> {
    let current_env = launch_line
        .and_then(|l| LaunchLine::parse(l).ok())
        .map(|l| {
            l.leading_env()
                .into_iter()
                .map(|(_, k, v)| (k, v))
                .collect()
        })
        .unwrap_or_default();
    let context = DiagnosisContext {
        gpu_vendor: game.gpu,
        gpu_name,
        driver: game.driver.clone(),
        proton_tool: game.tool_name.clone(),
        uses_proton: game.uses_proton,
        tool_includes_protonfixes: game.tool_includes_protonfixes,
        game_files: game
            .install_path
            .as_deref()
            .map(installed_files)
            .unwrap_or_default()
            .into_iter()
            .map(str::to_owned)
            .collect(),
        install_known: game.install_path.is_some(),
        current_env,
    };
    let assessments = engine.analyze(game)?;
    let catalog = catalog_units(engine, game)
        .into_iter()
        .filter_map(|u| {
            let recipe = engine.recipe(&u.recipe_id).ok()?.clone();
            let availability = engine.availability(&u.recipe_id, game);
            Some(CatalogOption {
                recipe,
                availability,
            })
        })
        .collect();
    Ok(DiagnosisInput {
        app_id: game.app_id.clone(),
        symptom,
        context,
        findings: game.findings.clone(),
        assessments,
        catalog,
        providers,
        generated_at: now_iso(),
    })
}
