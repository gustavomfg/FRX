//! Ligação entre benchmarks do Performance Lab e pacotes.
//!
//! A identidade inclui as variáveis e os arquivos efetivamente instalados,
//! com versão e hash do artefato. IDs legados de pacotes só de variáveis são
//! preservados. O baseline não contém variáveis nem arquivos ativos do FRX.

use super::models::*;
use crate::fix_engine::files::sha256_hex;
use crate::fix_engine::models::{Effective, OpState, Operation, RecipeAction, Step};
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

pub fn pack_id_with_files(app_id: &str, env: &[EnvPair], files: &[FileFixRef]) -> String {
    if files.is_empty() {
        return pack_id(app_id, env);
    }
    let mut file_ids = files.to_vec();
    file_ids.sort();
    file_ids.dedup();
    let h = sha256_hex(
        &serde_json::to_vec(&(
            "file-fixes-v1",
            PACK_SCHEMA,
            app_id,
            pack_id(app_id, env),
            file_ids,
        ))
        .unwrap_or_default(),
    );
    format!("pack.{app_id}.{}", &h[..16])
}

/// Every file action is validated as a whole before it can join a composition.
pub fn action_file_fixes(
    app_id: &str,
    actions: &[RecipeAction],
) -> Result<Vec<FileFixRef>, String> {
    let mut refs = Vec::new();
    for action in actions {
        let RecipeAction::InstallArtifact { artifact } = action else {
            return Err("Este tipo de edição de arquivo ainda exige revisão fora das combinações automáticas.".into());
        };
        crate::fix_engine::artifacts::validate(artifact, app_id)?;
        for f in &artifact.files {
            let hash = f
                .sha256
                .as_ref()
                .or_else(|| {
                    (artifact.format == crate::fixes::models::ArtifactFormat::File)
                        .then_some(artifact.sha256.as_ref())
                        .flatten()
                })
                .ok_or("Hash do arquivo instalado ausente")?;
            refs.push(FileFixRef {
                target_path: f.target_path.clone(),
                sha256: hash.to_ascii_lowercase(),
                artifact_id: Some(artifact.id.clone()),
                artifact_version: Some(artifact.release_tag.clone()),
                artifact_sha256: artifact.sha256.as_ref().map(|h| h.to_ascii_lowercase()),
            });
        }
    }
    let mut targets = std::collections::BTreeSet::new();
    if refs
        .iter()
        .any(|f| !targets.insert(f.target_path.to_ascii_lowercase()))
    {
        return Err("As ações disputam o mesmo arquivo de destino.".into());
    }
    refs.sort();
    Ok(refs)
}

/// Local edits preserve unrelated bytes, so their exact resulting hash comes
/// from the planner, never from a hash of option names or claimed discovery text.
pub fn unit_file_fixes(
    app_id: &str,
    unit: &super::models::Unit,
) -> Result<Vec<FileFixRef>, String> {
    if unit
        .file_actions
        .iter()
        .any(|a| matches!(a, RecipeAction::SetDxvkOption { .. }))
    {
        let files = unit
            .preview_files
            .clone()
            .ok_or("A configuração local ainda não teve o resultado calculado pelo planejador.")?;
        if files.iter().any(|f| {
            crate::fix_engine::security::check_rel_path(&f.target_path).is_err()
                || !crate::fix_engine::artifacts::sha_valid(&f.sha256)
        }) {
            return Err("Prévia de arquivo inválida".into());
        }
        return Ok(files);
    }
    action_file_fixes(app_id, &unit.file_actions)
}
pub fn units_file_fixes(
    app_id: &str,
    units: &[super::models::Unit],
) -> Result<Vec<FileFixRef>, String> {
    let mut files = Vec::new();
    for unit in units {
        files.extend(unit_file_fixes(app_id, unit)?);
    }
    let mut paths = std::collections::BTreeSet::new();
    if files
        .iter()
        .any(|f| !paths.insert(f.target_path.to_ascii_lowercase()))
    {
        return Err("As receitas disputam o mesmo arquivo de destino".into());
    }
    files.sort();
    Ok(files)
}

pub fn operation_file_fixes(op: &Operation) -> Vec<FileFixRef> {
    let mut files: Vec<_> = op
        .steps
        .iter()
        .filter_map(|record| {
            let Step::GameFile { rel, artifact, .. } = &record.step else {
                return None;
            };
            Some(FileFixRef {
                target_path: rel.clone(),
                sha256: record
                    .after_sha
                    .clone()
                    .unwrap_or_else(|| "unverified".into()),
                artifact_id: artifact.as_ref().map(|a| a.id.clone()),
                artifact_version: artifact.as_ref().map(|a| a.release_tag.clone()),
                artifact_sha256: artifact
                    .as_ref()
                    .and_then(|a| a.sha256.clone())
                    .map(|h| h.to_ascii_lowercase()),
            })
        })
        .collect();
    files.sort();
    files.dedup();
    files
}

/// Used only to identify an operation while staging or after a failed download.
/// Benchmark snapshots always use operation_file_fixes and its written hashes.
pub fn operation_planned_file_fixes(op: &Operation) -> Vec<FileFixRef> {
    let mut files: Vec<_> = op
        .steps
        .iter()
        .filter_map(|record| {
            let Step::GameFile {
                rel,
                artifact,
                content,
                ..
            } = &record.step
            else {
                return None;
            };
            let hash = record
                .after_sha
                .clone()
                .or_else(|| {
                    artifact.as_ref().and_then(|a| {
                        a.files
                            .iter()
                            .find(|f| f.target_path == *rel)
                            .and_then(|f| f.sha256.clone())
                    })
                })
                .unwrap_or_else(|| sha256_hex(content.as_bytes()));
            Some(FileFixRef {
                target_path: rel.clone(),
                sha256: hash.to_ascii_lowercase(),
                artifact_id: artifact.as_ref().map(|a| a.id.clone()),
                artifact_version: artifact.as_ref().map(|a| a.release_tag.clone()),
                artifact_sha256: artifact
                    .as_ref()
                    .and_then(|a| a.sha256.clone())
                    .map(|h| h.to_ascii_lowercase()),
            })
        })
        .collect();
    files.sort();
    files.dedup();
    files
}

pub fn current_file_fixes(engine: &FixEngine, app_id: &str) -> Vec<FileFixRef> {
    let mut files: Vec<_> = engine
        .operations(Some(app_id))
        .iter()
        .filter(|o| {
            o.state == OpState::Applied && !crate::fix_engine::is_capture_setup(&o.recipe_id)
        })
        .flat_map(operation_file_fixes)
        .collect();
    files.sort();
    files.dedup();
    files
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

pub fn snapshot_fingerprint_with_files(
    app_id: &str,
    env: &[EnvPair],
    recipes: &[SnapshotRecipe],
    files: &[FileFixRef],
) -> String {
    let old = snapshot_fingerprint(app_id, env, recipes);
    if files.is_empty() {
        return old;
    }
    let mut refs = files.to_vec();
    refs.sort();
    refs.dedup();
    sha256_hex(&serde_json::to_vec(&("file-snapshot-v1", old, refs)).unwrap_or_default())
}

/// Nenhum snapshot pode transportar as ações ou a identidade de outro jogo.
/// Legados sem campos novos só são aceitos quando o ID antigo confirma o
/// AppID e exatamente as mesmas ações. Nada é inferido pelo jogo aberto.
pub fn snapshot_matches_app(app_id: &str, snapshot: &PackSnapshot) -> bool {
    crate::art::valid_app_id(app_id)
        && snapshot.app_id.as_deref().is_none_or(|a| a == app_id)
        && snapshot.pack_id == pack_id_with_files(app_id, &snapshot.env, &snapshot.file_fixes)
        && snapshot.file_fixes.iter().all(|f| {
            crate::fix_engine::security::check_rel_path(&f.target_path).is_ok()
                && crate::fix_engine::artifacts::sha_valid(&f.sha256)
        })
        && snapshot.recipes.iter().all(|r| {
            (!r.recipe_id.starts_with("gen.") || r.recipe_id.starts_with(&format!("gen.{app_id}.")))
                && (!r.recipe_id.starts_with("pack.")
                    || r.recipe_id.starts_with(&format!("pack.{app_id}.")))
        })
        && snapshot.fingerprint.as_ref().is_none_or(|fp| {
            *fp == snapshot_fingerprint_with_files(
                app_id,
                &snapshot.env,
                &snapshot.recipes,
                &snapshot.file_fixes,
            )
        })
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
    let file_fixes = current_file_fixes(engine, app_id);
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
        fingerprint: Some(snapshot_fingerprint_with_files(
            app_id,
            &env,
            &recipes,
            &file_fixes,
        )),
        pack_id: pack_id_with_files(app_id, &env, &file_fixes),
        file_fixes,
        app_id: Some(app_id.to_owned()),
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
            let mut recorded_files = Vec::new();
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
                recorded_files.extend(operation_file_fixes(&op));
            }
            recorded_env.sort();
            recorded_env.dedup();
            let mut claimed = s.env.clone();
            claimed.sort();
            claimed.dedup();
            recorded_files.sort();
            recorded_files.dedup();
            let mut claimed_files = s.file_fixes.clone();
            claimed_files.sort();
            claimed_files.dedup();
            if recorded_env != claimed || recorded_files != claimed_files {
                return None;
            }
        }
        return Some(s.clone());
    }
    let fixes = meta.active_fixes.as_ref()?;
    let mut env = Vec::new();
    let mut recipes = Vec::new();
    let mut operation_ids = Vec::new();
    let mut file_fixes = Vec::new();
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
        file_fixes.extend(operation_file_fixes(&op));
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
    file_fixes.sort();
    file_fixes.dedup();
    Some(PackSnapshot {
        fingerprint: Some(snapshot_fingerprint_with_files(
            app,
            &env,
            &recipes,
            &file_fixes,
        )),
        pack_id: pack_id_with_files(app, &env, &file_fixes),
        file_fixes,
        app_id: Some(app.to_owned()),
        env,
        recipes,
        operation_ids,
        verified: true,
        source: SnapshotSource::Assisted,
        recorded_at: meta.created_at.clone(),
    })
}

/// Configuração declarada de uma sessão (sessões sintéticas dos testes).
#[cfg(test)]
pub fn declared(app_id: &str, env: Vec<EnvPair>) -> PackSnapshot {
    declared_with_files(app_id, env, vec![])
}

#[cfg(test)]
pub fn declared_with_files(
    app_id: &str,
    env: Vec<EnvPair>,
    file_fixes: Vec<FileFixRef>,
) -> PackSnapshot {
    PackSnapshot {
        fingerprint: Some(snapshot_fingerprint_with_files(
            app_id,
            &env,
            &[],
            &file_fixes,
        )),
        pack_id: pack_id_with_files(app_id, &env, &file_fixes),
        file_fixes,
        app_id: Some(app_id.to_owned()),
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
        || started.file_fixes != finished.file_fixes
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
