//! Registro local: `fix-engine/smart-packs/<appid>.json` (pasta 0700,
//! arquivos 0600, gravação atômica).
//!
//! Guarda cada pacote já composto (mesmo os que deixaram de ser oferecidos,
//! marcados `retired`) e o histórico de mudanças do ranking. Nada é apagado
//! quando o ranking muda. O arquivo é cache e histórico: para aplicar, as
//! receitas são sempre refeitas e revalidadas a partir das descobertas.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::models::*;
use crate::fix_engine::files::{atomic_write, private_dir};
use crate::time::now_iso;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StoredPack {
    pub pack: SmartPack,
    pub first_seen: String,
    pub last_seen: String,
    pub retired: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PackFile {
    /// Ausente (0) = formato anterior ao versionamento; migrado ao ler.
    #[serde(default)]
    pub schema: u32,
    pub app_id: String,
    #[serde(default)]
    pub composer_version: String,
    #[serde(default)]
    pub packs: Vec<StoredPack>,
    #[serde(default)]
    pub history: Vec<RankingSnapshot>,
}

pub struct SmartStore {
    dir: PathBuf,
}

/// Validate contained identity as well as the per-game file name. Legacy
/// records remain on disk for audit, but malformed/cross-game records are
/// never returned as executable pack pointers.
fn belongs(app_id: &str, pack: &SmartPack) -> bool {
    pack.app_id == app_id
        && pack.id == super::benchmark::pack_id_with_files(app_id, &pack.env, &pack.file_fixes)
        && pack.members.iter().all(|m| {
            !m.recipe_id.starts_with("pack.")
                && (!m.recipe_id.starts_with("gen.")
                    || m.recipe_id.starts_with(&format!("gen.{app_id}.")))
        })
}

impl SmartStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn path(&self, app_id: &str) -> Option<PathBuf> {
        crate::art::valid_app_id(app_id).then(|| self.dir.join(format!("{app_id}.json")))
    }

    /// Lê e migra. `Err` = formato mais novo que este FRX (não é sobrescrito).
    pub fn load(&self, app_id: &str) -> Result<PackFile, String> {
        let Some(path) = self.path(app_id) else {
            return Err("AppID inválido".into());
        };
        let Ok(bytes) = std::fs::read(&path) else {
            return Ok(PackFile {
                schema: PACK_SCHEMA,
                app_id: app_id.into(),
                ..Default::default()
            });
        };
        let mut f: PackFile = match serde_json::from_slice(&bytes) {
            Ok(f) => f,
            // Corrompido: recomeça, mas guarda a cópia para inspeção.
            Err(_) => {
                let _ = std::fs::rename(&path, path.with_extension("json.corrupt"));
                return Ok(PackFile {
                    schema: PACK_SCHEMA,
                    app_id: app_id.into(),
                    ..Default::default()
                });
            }
        };
        if f.schema > PACK_SCHEMA {
            return Err(format!(
                "O registro de Smart Packs deste jogo está num formato mais novo ({}) que este FRX entende ({PACK_SCHEMA}); ele não será alterado.",
                f.schema
            ));
        }
        if f.schema == 0 {
            f.schema = PACK_SCHEMA;
        }
        if f.app_id != app_id {
            return Err("Registro de Smart Packs de outro jogo.".into());
        }
        Ok(f)
    }

    fn save(&self, f: &PackFile) -> Result<(), String> {
        let path = self.path(&f.app_id).ok_or("AppID inválido")?;
        private_dir(&self.dir)?;
        let json = serde_json::to_vec_pretty(f).map_err(|e| e.to_string())?;
        atomic_write(&path, &json, 0o600)
    }

    /// Registra os pacotes compostos (sem mexer no histórico do ranking).
    pub fn remember(&self, app_id: &str, packs: &[SmartPack]) -> Result<(), String> {
        if packs.iter().any(|p| !belongs(app_id, p)) {
            return Err(
                "Registro recusado: um pacote pertence a outro jogo ou tem identidade inválida."
                    .into(),
            );
        }
        let mut f = self.load(app_id)?;
        let now = now_iso();
        for p in packs {
            match f.packs.iter_mut().find(|s| s.pack.id == p.id) {
                Some(s) => {
                    s.pack = p.clone();
                    s.last_seen = now.clone();
                    s.retired = false;
                }
                None => f.packs.push(StoredPack {
                    pack: p.clone(),
                    first_seen: now.clone(),
                    last_seen: now.clone(),
                    retired: false,
                }),
            }
        }
        f.composer_version = COMPOSER_VERSION.into();
        self.save(&f)
    }

    /// Receitas de um pacote registrado (ponteiro: tudo é revalidado ao aplicar).
    pub fn members(&self, app_id: &str, pack_id: &str) -> Option<Vec<String>> {
        self.load(app_id)
            .ok()?
            .packs
            .into_iter()
            .find(|s| s.pack.id == pack_id && belongs(app_id, &s.pack))
            .map(|s| s.pack.members.iter().map(|m| m.recipe_id.clone()).collect())
    }

    pub fn pack(&self, app_id: &str, pack_id: &str) -> Option<SmartPack> {
        self.load(app_id)
            .ok()?
            .packs
            .into_iter()
            .find(|s| s.pack.id == pack_id && belongs(app_id, &s.pack))
            .map(|s| s.pack)
    }
}
