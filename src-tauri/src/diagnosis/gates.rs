//! Regras duras. Valem para qualquer origem: catálogo, repositório ou comentário.

use super::models::{ConfigKey, DiagnosisContext, Symptom};
use crate::fix_engine::dynamic::compatibility::{knowledge, vendor_bound};
use crate::fix_engine::models::Purpose;
use crate::fixes::models::{FixAction, GpuReq};

pub const DLSS_KEY: &str = "PROTON_DLSS_UPGRADE";
pub const DLSS_FILE: &str = "nvngx_dlss.dll";

fn env_items(key: &ConfigKey) -> impl Iterator<Item = (&str, &str)> {
    key.items
        .iter()
        .filter(|i| i.kind == "env")
        .map(|i| (i.name.as_str(), i.value.as_deref().unwrap_or("")))
}

pub fn touches_dlss_upgrade(key: &ConfigKey) -> bool {
    env_items(key).any(|(k, _)| k == DLSS_KEY)
}

/// `Some(motivo)` quando o upgrade de DLSS não vale para este ambiente.
pub fn dlss_block(key: &ConfigKey, ctx: &DiagnosisContext) -> Option<String> {
    if !touches_dlss_upgrade(key) {
        return None;
    }
    if ctx.gpu_vendor != Some(GpuReq::Nvidia) {
        return Some(match ctx.gpu_vendor {
            None => "O upgrade de DLSS exige GPU NVIDIA confirmada, e a GPU não foi identificada."
                .into(),
            Some(_) => "O upgrade de DLSS só vale para GPU NVIDIA.".into(),
        });
    }
    if !ctx.game_files.iter().any(|f| f == DLSS_FILE) {
        return Some(format!(
            "O upgrade de DLSS exige {DLSS_FILE} no jogo, e o arquivo não foi encontrado."
        ));
    }
    None
}

/// Finalidade conhecida da configuração, pela receita ou pela tabela de variáveis.
pub fn purpose_of(key: &ConfigKey, recipe: Option<Purpose>) -> Option<Purpose> {
    recipe.or_else(|| {
        env_items(key)
            .filter_map(|(k, v)| knowledge(k, v))
            .map(|k| k.purpose)
            .next()
    })
}

pub fn is_improvement(key: &ConfigKey, purpose: Option<Purpose>) -> bool {
    touches_dlss_upgrade(key) || purpose == Some(Purpose::Quality)
}

/// Melhoria de imagem não é correção de falha ao iniciar.
pub fn wrong_for_symptom(symptom: Symptom, improvement: bool) -> Option<String> {
    (symptom == Symptom::CrashOrWontStart && improvement).then(|| {
        "Melhoria de imagem, não correção: não é sugerida para crash ou falha ao iniciar.".into()
    })
}

/// Todas as variáveis da configuração já estão nas opções do jogo.
pub fn already_in_options(key: &ConfigKey, ctx: &DiagnosisContext) -> bool {
    let mut any = false;
    for (k, v) in env_items(key) {
        any = true;
        if !ctx.current_env.iter().any(|(ck, cv)| ck == k && cv == v) {
            return false;
        }
    }
    any && key.items.iter().all(|i| i.kind == "env")
}

pub fn vendor_conflict(key: &ConfigKey, ctx: &DiagnosisContext) -> Option<String> {
    let current = ctx.gpu_vendor?;
    env_items(key)
        .filter_map(|(k, v)| {
            let bound = knowledge(k, v)
                .and_then(|known| known.gpu)
                .or(vendor_bound(k));
            bound.map(|vendor| (k, vendor))
        })
        .find(|(_, vendor)| *vendor != current)
        .map(|(k, vendor)| {
            format!(
                "{k} só vale para GPU {}.",
                crate::fixes::compat::vendor_name(vendor)
            )
        })
}

const COMMAND_WORDS: &[&str] = &[
    "curl", "wget", "sudo", "bash", "sh", "zsh", "fish", "rm", "chmod", "chown", "dd", "eval",
    "exec", "python", "python3", "perl", "nc", "mkfs", "pkexec", "su",
];

/// Valor de ação com cara de comando. Fica em revisão, mesmo que algo o aceite.
pub fn command_like(action: &FixAction) -> bool {
    let v = action.value.as_str();
    if ["|", ";", "&&", "`", "$(", ">", "<"]
        .iter()
        .any(|s| v.contains(s))
    {
        return true;
    }
    if v.split_whitespace().any(|w| w.starts_with("LD_")) {
        return true;
    }
    v.split_whitespace().any(|w| {
        let base = w.rsplit('/').next().unwrap_or(w);
        COMMAND_WORDS.contains(&base)
    })
}
