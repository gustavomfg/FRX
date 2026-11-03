//! Busca de soluções de hardware para o laptop detectado e aplicação das
//! instruções reconhecidas pelo Fix Engine.
//!
//! A interface nunca envia ação, caminho ou conteúdo: só o sintoma e os IDs
//! da sugestão e da ação. O núcleo refaz a busca (pelo cache) e encontra a
//! ação ele mesmo.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use super::fix_engine::{blocking, emitter, engine};
use super::system::effective_identity;
use super::AppCtx;
use crate::fix_engine::models::{FixPlan, Operation};
use crate::fix_engine::system::SystemFixRequest;
use crate::fixes::http::Ctx;
use crate::fixes::providers::arch_wiki::{self, HardwareSymptom, WikiSearch};
use crate::fixes::providers::LaptopQuery;
use crate::system::LaptopIdentity;
use crate::time::now_iso;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareSearchArgs {
    pub symptom: HardwareSymptom,
    /// Ignora o cache válido e consulta a fonte de novo.
    pub force: bool,
    pub max_age_hours: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareReport {
    pub identity: LaptopIdentity,
    pub simulated: bool,
    pub symptom: HardwareSymptom,
    pub retrieved_at: String,
    #[serde(flatten)]
    pub wiki: WikiSearch,
}

async fn search(
    ctx: &AppCtx,
    identity: LaptopIdentity,
    symptom: HardwareSymptom,
    force: bool,
    max_age_hours: u64,
) -> WikiSearch {
    let http = Ctx {
        fetcher: &ctx.fetcher,
        cache: &ctx.cache,
        state: &ctx.fixes,
        ttl_secs: super::fixes::ttl(max_age_hours),
        force,
    };
    let query = LaptopQuery {
        identity: Some(identity),
    };
    arch_wiki::search(&http, &query, symptom).await
}

#[tauri::command]
pub async fn search_hardware_fixes(
    ctx: State<'_, AppCtx>,
    args: HardwareSearchArgs,
) -> Result<HardwareReport, String> {
    let info = tauri::async_runtime::spawn_blocking(effective_identity)
        .await
        .map_err(|e| e.to_string())?;
    let retrieved_at = now_iso();
    let wiki = search(&ctx, info.identity.clone(), args.symptom, args.force, args.max_age_hours).await;
    Ok(HardwareReport {
        identity: info.identity,
        simulated: info.simulated,
        symptom: args.symptom,
        retrieved_at,
        wiki,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemFixArgs {
    pub symptom: HardwareSymptom,
    pub suggestion_id: String,
    pub action_id: String,
    pub max_age_hours: u64,
}

/// Prévia (dry-run) de uma instrução reconhecida na wiki. Nada é escrito.
#[tauri::command]
pub async fn prepare_system_fix(
    ctx: State<'_, AppCtx>,
    args: SystemFixArgs,
) -> Result<FixPlan, String> {
    let info = tauri::async_runtime::spawn_blocking(effective_identity)
        .await
        .map_err(|e| e.to_string())?;
    let laptop = info
        .identity
        .display_name
        .clone()
        .unwrap_or_else(|| "Este computador".into());
    let wiki = search(&ctx, info.identity, args.symptom, false, args.max_age_hours).await;
    const GONE: &str = "Esta sugestão não está mais na busca. Pesquise o sintoma de novo.";
    let suggestion = wiki
        .suggestions
        .into_iter()
        .find(|s| s.id == args.suggestion_id)
        .ok_or(GONE)?;
    let found = suggestion
        .actions
        .iter()
        .find(|a| a.id == args.action_id)
        .cloned()
        .ok_or(GONE)?;
    let where_ = suggestion.section.as_deref().unwrap_or(&suggestion.page);
    let req = SystemFixRequest {
        action: found.action,
        title: format!("{where_}: {}", found.snippet),
        context: found.context,
        source_label: format!("Arch Wiki · {}", suggestion.section.as_ref().map_or(suggestion.page.clone(), |s| format!("{} › {s}", suggestion.page))),
        source_url: suggestion.url,
        laptop,
    };
    let e = engine(&ctx);
    blocking(move || e.prepare_system(req)).await
}

/// Confirma a prévia. Com o sistema real, o resultado é um passo manual:
/// backup feito, arquivo preparado e o comando `sudo` para o usuário rodar.
#[tauri::command]
pub async fn apply_system_fix(
    app: AppHandle,
    ctx: State<'_, AppCtx>,
    plan_id: String,
    fingerprint: String,
) -> Result<Operation, String> {
    let e = engine(&ctx);
    blocking(move || e.apply_system(&plan_id, &fingerprint, &emitter(app))).await
}
