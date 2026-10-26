//! Ligação entre benchmarks do Performance Lab e pacotes.
//!
//! Um pacote é identificado pelas variáveis que ele define (ordenadas), não
//! pelo nome: uma sessão capturada com `PROTON_NO_D3D11=1` ativo pertence ao
//! pacote com essas variáveis, tenha ele sido aplicado como receita avulsa ou
//! como Smart Pack. Sem nenhuma variável do FRX, a sessão é do baseline.

use super::models::*;
use crate::fix_engine::files::sha256_hex;
use crate::fix_engine::models::{Effective, OpState};
use crate::fix_engine::FixEngine;
use crate::performance::models::SessionMeta;
use crate::time::now_iso;

pub fn pack_id(app_id: &str, env: &[EnvPair]) -> String {
    let mut norm: Vec<String> = env
        .iter()
        .map(|p| format!("{}={}", p.key, p.value))
        .collect();
    norm.sort();
    norm.dedup();
    let h = sha256_hex(format!("pack\n{app_id}\n{}", norm.join("\n")).as_bytes());
    format!("pack.{app_id}.{}", &h[..16])
}

pub fn baseline_id(app_id: &str) -> String {
    pack_id(app_id, &[])
}

/// Variáveis que o FRX mantém aplicadas agora no jogo, por operação.
pub fn current_env(engine: &FixEngine, app_id: &str) -> (Vec<EnvPair>, Vec<String>, bool) {
    let mut env: Vec<EnvPair> = Vec::new();
    let mut ops = Vec::new();
    let mut verified = true;
    let applied = engine.applied(Some(app_id));
    for op in engine
        .operations(Some(app_id))
        .into_iter()
        .filter(|o| o.state == OpState::Applied)
    {
        if applied
            .iter()
            .find(|a| a.operation_id == op.id)
            .is_none_or(|a| a.effective != Effective::Active)
        {
            verified = false;
        }
        for w in op.env_writes() {
            env.push(EnvPair {
                key: w.key.clone(),
                value: w.written.clone(),
            });
        }
        ops.push(op.id.clone());
    }
    env.sort();
    env.dedup();
    (env, ops, verified)
}

/// Snapshot gravado no fim da captura assistida.
pub fn snapshot(engine: &FixEngine, app_id: &str) -> PackSnapshot {
    let (env, operation_ids, verified) = current_env(engine, app_id);
    let recipes = engine
        .operations(Some(app_id))
        .into_iter()
        .filter(|o| operation_ids.contains(&o.id))
        .map(|o| SnapshotRecipe {
            recipe_id: o.recipe_id,
            version: o.recipe_version,
        })
        .collect();
    PackSnapshot {
        pack_id: pack_id(app_id, &env),
        env,
        recipes,
        operation_ids,
        verified,
        source: SnapshotSource::Assisted,
        recorded_at: now_iso(),
    }
}

/// Fim da captura assistida: registra as correções ativas e o Smart Pack
/// equivalente, conferidos no disco agora. É o que associa a sessão ao pacote
/// sem nenhuma ação do usuário.
pub fn tag_assisted(engine: &FixEngine, app_id: &str, meta: &mut SessionMeta) {
    meta.active_fixes = Some(engine.active_for(app_id));
    meta.smart_pack = Some(snapshot(engine, app_id));
}

/// Configuração de uma sessão: snapshot do Smart Pack, ou as correções ativas
/// registradas desde a v0.4 (`active_fixes`). `None` = desconhecida.
pub fn session_config(engine: &FixEngine, meta: &SessionMeta) -> Option<PackSnapshot> {
    if let Some(s) = &meta.smart_pack {
        return Some(s.clone());
    }
    let app = meta.app_id.as_deref()?;
    let fixes = meta.active_fixes.as_ref()?;
    let mut env = Vec::new();
    for f in fixes {
        let op = engine.operation(&f.operation_id).ok()?;
        for w in op.env_writes() {
            env.push(EnvPair {
                key: w.key.clone(),
                value: w.written.clone(),
            });
        }
    }
    env.sort();
    env.dedup();
    Some(PackSnapshot {
        pack_id: pack_id(app, &env),
        env,
        recipes: fixes
            .iter()
            .map(|f| SnapshotRecipe {
                recipe_id: f.recipe_id.clone(),
                version: f.recipe_version.clone(),
            })
            .collect(),
        operation_ids: fixes.iter().map(|f| f.operation_id.clone()).collect(),
        verified: true,
        source: SnapshotSource::Assisted,
        recorded_at: meta.created_at.clone(),
    })
}

/// Configuração declarada pelo usuário para uma sessão sem registro.
pub fn declared(app_id: &str, env: Vec<EnvPair>) -> PackSnapshot {
    PackSnapshot {
        pack_id: pack_id(app_id, &env),
        env,
        recipes: vec![],
        operation_ids: vec![],
        verified: false,
        source: SnapshotSource::Declared,
        recorded_at: now_iso(),
    }
}
