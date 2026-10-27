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

/// Fingerprint separado do ID estável das ações: inclui o jogo, formato e
/// identidades/versionamento das receitas registradas na captura.
pub fn snapshot_fingerprint(app_id: &str, env: &[EnvPair], recipes: &[SnapshotRecipe]) -> String {
    let mut actions = env.to_vec();
    actions.sort();
    actions.dedup();
    let mut ids: Vec<String> = recipes
        .iter()
        .map(|r| {
            format!(
                "{}@{}#{}",
                r.recipe_id,
                r.version,
                r.fingerprint.as_deref().unwrap_or("legacy-unrecorded")
            )
        })
        .collect();
    ids.sort();
    ids.dedup();
    sha256_hex(&serde_json::to_vec(&(PACK_SCHEMA, app_id, actions, ids)).unwrap_or_default())
}

/// Nenhum snapshot pode transportar as ações ou a identidade de outro jogo.
/// Legados sem campos novos só são aceitos quando o ID antigo confirma o
/// AppID e exatamente as mesmas ações. Nada é inferido pelo jogo aberto.
pub fn snapshot_matches_app(app_id: &str, snapshot: &PackSnapshot) -> bool {
    crate::art::valid_app_id(app_id)
        && snapshot.app_id.as_deref().is_none_or(|a| a == app_id)
        && snapshot.pack_id == pack_id(app_id, &snapshot.env)
        && snapshot.recipes.iter().all(|r| {
            (!r.recipe_id.starts_with("gen.") || r.recipe_id.starts_with(&format!("gen.{app_id}.")))
                && (!r.recipe_id.starts_with("pack.")
                    || r.recipe_id.starts_with(&format!("pack.{app_id}.")))
        })
        && snapshot
            .fingerprint
            .as_ref()
            .is_none_or(|fp| *fp == snapshot_fingerprint(app_id, &snapshot.env, &snapshot.recipes))
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
    if engine.operations(Some(app_id)).iter().any(|o| {
        o.state.in_flight()
            || matches!(
                o.state,
                OpState::RequiresManualStep | OpState::RecoveryRequired
            )
    }) {
        verified = false;
    }
    for op in engine.operations(Some(app_id)).into_iter().filter(|o| {
        o.state == OpState::Applied && !crate::fix_engine::is_capture_setup(&o.recipe_id)
    }) {
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
    let recipes: Vec<SnapshotRecipe> = engine
        .operations(Some(app_id))
        .into_iter()
        .filter(|o| operation_ids.contains(&o.id))
        .map(|o| SnapshotRecipe {
            fingerprint: o.recipe_fingerprint,
            recipe_id: o.recipe_id,
            version: o.recipe_version,
        })
        .collect();
    PackSnapshot {
        app_id: Some(app_id.to_owned()),
        fingerprint: Some(snapshot_fingerprint(app_id, &env, &recipes)),
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
    if meta.app_id.as_deref() != Some(app_id) {
        meta.active_fixes = None;
        meta.smart_pack = None;
        meta.quality
            .issues
            .push(crate::performance::models::Issue::new(
            crate::performance::models::Severity::Warning,
            "pack-scope-mismatch",
            "A captura e o pacote pertencem a jogos diferentes; nenhuma associação foi registrada.",
        ));
        return;
    }
    meta.active_fixes = Some(engine.active_for(app_id));
    meta.smart_pack = Some(snapshot(engine, app_id));
}

/// Configuração de uma sessão: snapshot do Smart Pack, ou as correções ativas
/// registradas desde a v0.4 (`active_fixes`). `None` = desconhecida.
pub fn session_config(engine: &FixEngine, meta: &SessionMeta) -> Option<PackSnapshot> {
    let app = meta.app_id.as_deref()?;
    if !crate::art::valid_app_id(app)
        || meta
            .quality
            .issues
            .iter()
            .any(|i| i.code == "context-edited")
    {
        return None;
    }
    if let Some(s) = &meta.smart_pack {
        if !snapshot_matches_app(app, s) {
            return None;
        }
        if s.source == SnapshotSource::Assisted {
            let mut recorded_env = Vec::new();
            for id in &s.operation_ids {
                let op = engine.operation(id).ok()?;
                if op.app_id != app
                    || !s.recipes.iter().any(|r| {
                        r.recipe_id == op.recipe_id
                            && r.version == op.recipe_version
                            && r.fingerprint == op.recipe_fingerprint
                    })
                {
                    return None;
                }
                recorded_env.extend(op.env_writes().map(|w| EnvPair {
                    key: w.key.clone(),
                    value: w.written.clone(),
                }));
            }
            recorded_env.sort();
            recorded_env.dedup();
            let mut claimed = s.env.clone();
            claimed.sort();
            claimed.dedup();
            if recorded_env != claimed {
                return None;
            }
        }
        return Some(s.clone());
    }
    let fixes = meta.active_fixes.as_ref()?;
    let mut env = Vec::new();
    let mut recipes = Vec::new();
    let mut operation_ids = Vec::new();
    for f in fixes {
        let op = engine.operation(&f.operation_id).ok()?;
        if op.app_id != app || op.recipe_id != f.recipe_id || op.recipe_version != f.recipe_version
        {
            return None;
        }
        if crate::fix_engine::is_capture_setup(&op.recipe_id) {
            continue;
        }
        operation_ids.push(op.id.clone());
        recipes.push(SnapshotRecipe {
            fingerprint: op.recipe_fingerprint.clone(),
            recipe_id: op.recipe_id.clone(),
            version: op.recipe_version.clone(),
        });
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
        app_id: Some(app.to_owned()),
        fingerprint: Some(snapshot_fingerprint(app, &env, &recipes)),
        pack_id: pack_id(app, &env),
        env,
        recipes,
        operation_ids,
        verified: true,
        source: SnapshotSource::Assisted,
        recorded_at: meta.created_at.clone(),
    })
}

/// Configuração declarada pelo usuário para uma sessão sem registro.
pub fn declared(app_id: &str, env: Vec<EnvPair>) -> PackSnapshot {
    PackSnapshot {
        app_id: Some(app_id.to_owned()),
        fingerprint: Some(snapshot_fingerprint(app_id, &env, &[])),
        pack_id: pack_id(app_id, &env),
        env,
        recipes: vec![],
        operation_ids: vec![],
        verified: false,
        source: SnapshotSource::Declared,
        recorded_at: now_iso(),
    }
}

/// Associa apenas uma configuração estável entre início e fim da captura.
pub fn tag_capture(
    engine: &FixEngine,
    app_id: &str,
    started: &PackSnapshot,
    meta: &mut SessionMeta,
) {
    tag_assisted(engine, app_id, meta);
    let Some(finished) = meta.smart_pack.as_mut() else {
        return;
    };
    if meta.app_id.as_deref() != Some(app_id)
        || !snapshot_matches_app(app_id, started)
        || !started.verified
        || started.env != finished.env
        || started.operation_ids != finished.operation_ids
        || started.recipes != finished.recipes
    {
        finished.verified = false;
        meta.quality.issues.push(crate::performance::models::Issue::new(
            crate::performance::models::Severity::Warning,
            "pack-changed",
            "A configuração do FRX mudou durante a captura. Repita com o pacote esperado ativo e o jogo reaberto; esta sessão não entra no ranking.",
        ));
    }
}

/// New captures record full recipe hashes. Legacy hashes are never guessed:
/// historical operation identity and exact writes remain the only evidence.
pub fn recipe_context_matches(
    engine: &FixEngine,
    game: &crate::fix_engine::models::GameCtx,
    snapshot: &PackSnapshot,
) -> bool {
    snapshot_matches_app(&game.app_id, snapshot)
        && snapshot.recipes.iter().all(|r| {
            r.fingerprint.as_ref().is_none_or(|hash| {
                engine.resolve(&r.recipe_id, game).is_ok_and(|current| {
                    current.version == r.version && super::discovery::fingerprint(&current) == *hash
                })
            })
        })
}
