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

const MAX_HISTORY: usize = 500;

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

fn signature(r: &Ranking) -> Vec<(String, Option<u32>, Tier, Outcome)> {
    r.entries
        .iter()
        .map(|e| (e.pack_id.clone(), e.position, e.tier, e.outcome))
        .collect()
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

    /// Registra os pacotes atuais e, se a ordem mudou, um ponto no histórico.
    pub fn record(
        &self,
        app_id: &str,
        packs: &[SmartPack],
        ranking: &Ranking,
        sessions: usize,
    ) -> Result<PackFile, String> {
        let mut f = self.load(app_id)?;
        let now = now_iso();
        for s in &mut f.packs {
            s.retired = !packs.iter().any(|p| p.id == s.pack.id);
        }
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
        let sig = signature(ranking);
        let last = f
            .history
            .iter()
            .rev()
            .find(|h| h.profile == ranking.profile);
        if last.is_none_or(|h| h.entries != sig || h.version != ranking.version) {
            let reason = match last {
                None => "Primeira classificação.".to_owned(),
                Some(h) if h.version != ranking.version => {
                    "Nova versão da metodologia de ranking.".to_owned()
                }
                Some(h) if h.sessions != sessions => {
                    format!("Sessões comparáveis: {} → {sessions}.", h.sessions)
                }
                Some(_) => "Pacotes, receitas ou estado mudaram.".to_owned(),
            };
            f.history.push(RankingSnapshot {
                at: now,
                profile: ranking.profile,
                version: ranking.version.clone(),
                reason,
                entries: sig,
                sessions,
            });
            if f.history.len() > MAX_HISTORY {
                let cut = f.history.len() - MAX_HISTORY;
                f.history.drain(..cut);
            }
        }
        f.composer_version = COMPOSER_VERSION.into();
        self.save(&f)?;
        Ok(f)
    }

    /// Receitas de um pacote registrado (ponteiro: tudo é revalidado ao aplicar).
    pub fn members(&self, app_id: &str, pack_id: &str) -> Option<Vec<String>> {
        self.load(app_id)
            .ok()?
            .packs
            .into_iter()
            .find(|s| s.pack.id == pack_id)
            .map(|s| s.pack.members.iter().map(|m| m.recipe_id.clone()).collect())
    }

    pub fn pack(&self, app_id: &str, pack_id: &str) -> Option<SmartPack> {
        self.load(app_id)
            .ok()?
            .packs
            .into_iter()
            .find(|s| s.pack.id == pack_id)
            .map(|s| s.pack)
    }

    /// Referência Windows, se o usuário tiver importado uma (nenhuma fonte é
    /// integrada nesta versão). Formato inválido = indisponível.
    pub fn windows(&self, app_id: &str) -> Option<WindowsReference> {
        let p = self.dir.join("windows").join(format!("{app_id}.json"));
        let w: WindowsReference = serde_json::from_slice(&std::fs::read(p).ok()?).ok()?;
        (!w.origin.trim().is_empty() && w.avg_fps.is_none_or(|v| v.is_finite() && v > 0.0))
            .then_some(w)
    }
}
