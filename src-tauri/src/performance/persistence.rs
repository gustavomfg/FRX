//! Histórico local de sessões.
//!
//! ```text
//! <app_data>/benchmarks/sessions/<id>/
//!   raw.csv      ← log original, intocado (dados brutos)
//!   meta.json    ← metadados + estatísticas processadas
//!   series.json  ← séries reduzidas para o gráfico
//! ```
//!
//! Arquivos JSON com escrita atômica, no mesmo espírito do cache da v0.2.
//! `schema_version` permite migrar o formato; `analysis_version` faz sessões
//! antigas serem reprocessadas a partir do `raw.csv` quando a metodologia muda.
//! Nada é apagado durante atualizações.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::models::*;

#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
    access: Arc<Mutex<()>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPatch {
    pub label: Option<String>,
    pub notes: Option<String>,
    pub app_id: Option<String>,
    pub game_name: Option<String>,
    pub conditions: Option<Conditions>,
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 40
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

pub fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let c = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "s-{:x}-{:x}",
        nanos,
        (std::process::id() as u64) << 16 | (c & 0xffff)
    )
}

/// FNV-1a: identifica o mesmo log importado duas vezes (não é criptográfico).
pub fn content_hash(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}-{}", bytes.len())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, path)
}

#[derive(Serialize, Deserialize)]
struct SeriesFile {
    series: Vec<Series>,
}

impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            access: Arc::new(Mutex::new(())),
        }
    }

    fn sessions(&self) -> PathBuf {
        self.root.join("sessions")
    }

    fn dir(&self, id: &str) -> Option<PathBuf> {
        valid_id(id).then(|| self.sessions().join(id))
    }

    pub fn raw_path(&self, id: &str) -> Option<PathBuf> {
        self.dir(id).map(|d| d.join("raw.csv"))
    }

    pub fn save(&self, session: &Session, raw: &[u8]) -> Result<(), String> {
        let _guard = self.access.lock().map_err(|_| "histórico indisponível")?;
        self.save_unlocked(session, raw)
    }

    fn save_unlocked(&self, session: &Session, raw: &[u8]) -> Result<(), String> {
        let dir = self
            .dir(&session.meta.id)
            .ok_or("identificador de sessão inválido")?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        write_atomic(&dir.join("raw.csv"), raw).map_err(|e| e.to_string())?;
        let series = serde_json::to_vec(&SeriesFile {
            series: session.series.clone(),
        })
        .map_err(|e| e.to_string())?;
        write_atomic(&dir.join("series.json"), &series).map_err(|e| e.to_string())?;
        // meta.json é publicado por último: uma sessão visível sempre possui séries e log.
        self.write_meta(&session.meta)
    }

    fn write_meta(&self, meta: &SessionMeta) -> Result<(), String> {
        let dir = self
            .dir(&meta.id)
            .ok_or("identificador de sessão inválido")?;
        let bytes = serde_json::to_vec_pretty(meta).map_err(|e| e.to_string())?;
        write_atomic(&dir.join("meta.json"), &bytes).map_err(|e| e.to_string())
    }

    /// Lê os metadados, migrando/reprocessando quando necessário.
    fn load_meta(&self, id: &str) -> Option<SessionMeta> {
        let dir = self.dir(id)?;
        let bytes = std::fs::read(dir.join("meta.json")).ok()?;
        let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
        let schema = value
            .get("schemaVersion")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        if schema > SCHEMA_VERSION {
            return None; // criado por uma versão mais nova do FRX: preservado, não tocado
        }
        let meta: SessionMeta = serde_json::from_value(migrate(value, schema)).ok()?;
        if meta.id != id {
            // Um arquivo copiado ou inconsistente não pode se passar por
            // outra sessão e sobrescrever seus metadados na reanálise.
            // O diretório e o conteúdo original ficam intactos para auditoria.
            return None;
        }
        if meta.analysis_version < ANALYSIS_VERSION {
            if let Some(updated) = self.reprocess(&meta) {
                return Some(updated);
            }
        }
        Some(meta)
    }

    /// Recalcula estatísticas e séries a partir do log bruto, preservando o resto.
    fn reprocess(&self, meta: &SessionMeta) -> Option<SessionMeta> {
        let raw = std::fs::read(self.raw_path(&meta.id)?).ok()?;
        let parsed = super::parser::parse(&raw).ok()?;
        let analysis = super::metrics::analyze(&parsed);
        let mut m = meta.clone();
        m.stats = analysis.stats;
        m.quality = analysis.quality;
        // Proveniência e interrupções são fatos da captura; recalcular as
        // métricas não pode transformar dados excluídos em evidência válida.
        for issue in meta.quality.issues.iter().filter(|i| {
            matches!(
                i.code.as_str(),
                "interrupted"
                    | "pack-changed"
                    | "pack-recipe-changed"
                    | "context-edited"
                    | "synthetic"
                    | "fixture"
            )
        }) {
            if !m.quality.issues.iter().any(|i| i.code == issue.code) {
                m.quality.issues.push(issue.clone());
            }
            if m.quality.level == QualityLevel::Good {
                m.quality.level = QualityLevel::Usable;
            }
        }
        m.analysis_version = ANALYSIS_VERSION;
        m.schema_version = SCHEMA_VERSION;
        let session = Session {
            meta: m.clone(),
            series: analysis.series,
        };
        self.save_unlocked(&session, &raw).ok()?;
        Some(m)
    }

    pub fn list(&self) -> Vec<SessionMeta> {
        let Ok(_guard) = self.access.lock() else {
            return vec![];
        };
        let Ok(rd) = std::fs::read_dir(self.sessions()) else {
            return vec![];
        };
        let mut out: Vec<SessionMeta> = rd
            .flatten()
            .filter_map(|e| self.load_meta(&e.file_name().to_string_lossy()))
            .collect();
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        out
    }

    pub fn get(&self, id: &str) -> Option<Session> {
        let _guard = self.access.lock().ok()?;
        let meta = self.load_meta(id)?;
        let series = std::fs::read(self.dir(id)?.join("series.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<SeriesFile>(&b).ok())
            .map(|f| f.series)
            .unwrap_or_default();
        Some(Session { meta, series })
    }

    pub fn find_by_hash(&self, hash: &str) -> Option<SessionMeta> {
        self.list().into_iter().find(|m| m.raw_hash == hash)
    }

    pub fn update(&self, id: &str, patch: SessionPatch) -> Result<SessionMeta, String> {
        let _guard = self.access.lock().map_err(|_| "histórico indisponível")?;
        let mut meta = self.load_meta(id).ok_or("sessão não encontrada")?;
        let clip = |s: String, n: usize| {
            s.chars()
                .filter(|c| !c.is_control() || *c == '\n')
                .take(n)
                .collect::<String>()
        };
        if let Some(l) = patch.label {
            let l = clip(l, 120).trim().to_owned();
            if !l.is_empty() {
                meta.label = l;
            }
        }
        if let Some(n) = patch.notes {
            meta.notes = clip(n, 4000);
        }
        if let Some(a) = patch.app_id {
            if !a.is_empty() && !crate::art::valid_app_id(&a) {
                return Err("AppID inválido.".into());
            }
            let new_app = Some(a).filter(|a| !a.is_empty());
            if meta.app_id != new_app {
                let recorded = meta.source == SessionSource::Assisted
                    || meta.active_fixes.is_some()
                    || meta.smart_pack.as_ref().is_some_and(|s| {
                        s.source == crate::smart_packs::models::SnapshotSource::Assisted
                    });
                if recorded {
                    return Err("O AppID de uma captura registrada pelo FRX não pode ser alterado. A sessão e suas evidências pertencem ao jogo capturado.".into());
                }
                // A atribuição de uma importação é explícita. Uma declaração
                // anterior nunca acompanha a sessão para outro jogo.
                meta.smart_pack = None;
                meta.app_id = new_app;
            }
        }
        if let Some(g) = patch.game_name {
            meta.game_name = Some(clip(g, 200)).filter(|g| !g.trim().is_empty());
        }
        if let Some(c) = patch.conditions {
            if meta.conditions != c {
                if let Some(s) = meta.smart_pack.as_mut() {
                    if s.source == crate::smart_packs::models::SnapshotSource::Assisted {
                        s.verified = false;
                    } else {
                        // Se o contexto declarado mudou, o usuário precisa
                        // confirmar novamente a configuração correspondente.
                        meta.smart_pack = None;
                    }
                }
                if (meta.source == SessionSource::Assisted || meta.active_fixes.is_some())
                    && !meta
                        .quality
                        .issues
                        .iter()
                        .any(|i| i.code == "context-edited")
                {
                    meta.quality.issues.push(Issue::new(
                        Severity::Warning,
                        "context-edited",
                        "As condições foram editadas depois da captura. O registro original não confirma esse novo contexto; esta sessão permanece no histórico, fora do ranking automático.",
                    ));
                }
                meta.conditions = c;
            }
        }
        self.write_meta(&meta)?;
        Ok(meta)
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let _guard = self.access.lock().map_err(|_| "histórico indisponível")?;
        let dir = self.dir(id).ok_or("identificador de sessão inválido")?;
        if !dir.join("meta.json").is_file() {
            return Err("sessão não encontrada".into());
        }
        std::fs::remove_dir_all(dir).map_err(|e| e.to_string())
    }
}

/// Migrações de formato. Versão 0 = arquivos sem `schemaVersion` (pré-lançamento).
fn migrate(mut v: serde_json::Value, from: u32) -> serde_json::Value {
    if from < 1 {
        if let Some(o) = v.as_object_mut() {
            o.entry("fixIds").or_insert(serde_json::json!([]));
            o.entry("notes").or_insert(serde_json::json!(""));
            o.insert("schemaVersion".into(), serde_json::json!(1));
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::super::parser::fixtures::per_frame;
    use super::super::{process, ProcessArgs};
    use super::*;

    fn sample(store: &Store, fts: &[f64]) -> Session {
        let raw = per_frame(fts, 90.0);
        let s = process(
            raw.as_bytes(),
            ProcessArgs {
                source: Some(SessionSource::Import),
                ..Default::default()
            },
        )
        .unwrap();
        store.save(&s, raw.as_bytes()).unwrap();
        s
    }

    #[test]
    fn save_list_get_update_delete() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(tmp.path());
        let s = sample(&store, &vec![10.0; 3000]);
        let list = store.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, s.meta.id);
        let full = store.get(&s.meta.id).unwrap();
        assert!(!full.series.is_empty());
        assert!(
            store.raw_path(&s.meta.id).unwrap().is_file(),
            "o log bruto fica guardado"
        );

        let m = store
            .update(
                &s.meta.id,
                SessionPatch {
                    label: Some("  Antes do fix  ".into()),
                    notes: Some("Mesmo trecho".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(m.label, "Antes do fix");
        assert_eq!(store.get(&s.meta.id).unwrap().meta.notes, "Mesmo trecho");
        assert!(store.find_by_hash(&s.meta.raw_hash).is_some());

        store.delete(&s.meta.id).unwrap();
        assert!(store.list().is_empty());
        assert!(store.delete(&s.meta.id).is_err());
    }

    #[test]
    fn imported_game_reassignment_requires_explicit_input_and_drops_old_pack() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(tmp.path());
        let s = sample(&store, &vec![10.0; 3000]);
        let assigned = store
            .update(
                &s.meta.id,
                SessionPatch {
                    app_id: Some("200260".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(assigned.smart_pack.is_none());
        let mut tagged = store.get(&s.meta.id).unwrap();
        tagged.meta.smart_pack = Some(crate::smart_packs::benchmark::declared("200260", vec![]));
        store
            .save(&tagged, per_frame(&vec![10.0; 3000], 90.0).as_bytes())
            .unwrap();
        let moved = store
            .update(
                &s.meta.id,
                SessionPatch {
                    app_id: Some("480".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(moved.smart_pack.is_none());
        let restarted = Store::new(tmp.path());
        let recovered = restarted.get(&s.meta.id).unwrap();
        assert_eq!(recovered.meta.app_id.as_deref(), Some("480"));
        assert!(recovered.meta.smart_pack.is_none());
        assert_eq!(
            std::fs::read(restarted.raw_path(&s.meta.id).unwrap()).unwrap(),
            per_frame(&vec![10.0; 3000], 90.0).as_bytes()
        );
        assert!(!recovered.series.is_empty());
    }

    #[test]
    fn assisted_identity_is_immutable_and_context_edits_invalidate_evidence() {
        use crate::smart_packs::{benchmark::declared, models::SnapshotSource};
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(tmp.path());
        let mut s = sample(&store, &vec![10.0; 3000]);
        s.meta.source = SessionSource::Assisted;
        s.meta.app_id = Some("200260".into());
        let mut snapshot = declared("200260", vec![]);
        snapshot.source = SnapshotSource::Assisted;
        snapshot.verified = true;
        s.meta.smart_pack = Some(snapshot);
        store
            .save(&s, per_frame(&vec![10.0; 3000], 90.0).as_bytes())
            .unwrap();
        assert!(store
            .update(
                &s.meta.id,
                SessionPatch {
                    app_id: Some("480".into()),
                    ..Default::default()
                }
            )
            .is_err());
        assert_eq!(
            store.get(&s.meta.id).unwrap().meta.app_id.as_deref(),
            Some("200260")
        );
        let mut conditions = s.meta.conditions.clone();
        conditions.proton = Some("different Proton".into());
        let mut edited = store
            .update(
                &s.meta.id,
                SessionPatch {
                    conditions: Some(conditions),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(!edited.smart_pack.as_ref().unwrap().verified);
        assert!(edited
            .quality
            .issues
            .iter()
            .any(|i| i.code == "context-edited"));
        // A metodologia pode mudar, mas fatos de proveniência permanecem.
        edited.analysis_version = 0;
        s.meta = edited;
        store
            .save(&s, per_frame(&vec![10.0; 3000], 90.0).as_bytes())
            .unwrap();
        let recovered = Store::new(tmp.path()).get(&s.meta.id).unwrap();
        assert!(recovered
            .meta
            .quality
            .issues
            .iter()
            .any(|i| i.code == "context-edited"));
        assert!(!recovered.meta.smart_pack.unwrap().verified);
    }

    #[test]
    fn legacy_sessions_keep_known_or_unknown_appid_without_automatic_assignment() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(tmp.path());
        for app in [Some("200260"), None] {
            let mut s = sample(&store, &vec![10.0; 3000]);
            s.meta.app_id = app.map(str::to_owned);
            store
                .save(&s, per_frame(&vec![10.0; 3000], 90.0).as_bytes())
                .unwrap();
            let path = tmp
                .path()
                .join("sessions")
                .join(&s.meta.id)
                .join("meta.json");
            let mut json = serde_json::to_value(&s.meta).unwrap();
            for field in ["schemaVersion", "smartPack", "activeFixes"] {
                json.as_object_mut().unwrap().remove(field);
            }
            std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
            let loaded = Store::new(tmp.path()).get(&s.meta.id).unwrap();
            assert_eq!(loaded.meta.app_id.as_deref(), app);
            assert!(loaded.meta.smart_pack.is_none());
            assert!(loaded.meta.active_fixes.is_none());
            assert!(path.is_file());
        }
    }

    #[test]
    fn rejects_path_traversal_ids() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(tmp.path());
        assert!(store.get("../../etc").is_none());
        assert!(store.delete("../x").is_err());
        assert!(!valid_id("A/B"));
        assert!(valid_id(&new_id()));
    }

    #[test]
    fn mismatched_embedded_session_identity_is_preserved_but_never_loaded() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(tmp.path());
        let sample = sample(&store, &vec![10.0; 3000]);
        let path = tmp
            .path()
            .join("sessions")
            .join(&sample.meta.id)
            .join("meta.json");
        let mut json = serde_json::to_value(&sample.meta).unwrap();
        json["id"] = "s-synthetic-other-game".into();
        json["appId"] = "480".into();
        json["analysisVersion"] = 0.into();
        let inconsistent = serde_json::to_vec(&json).unwrap();
        std::fs::write(&path, &inconsistent).unwrap();
        assert!(store.get(&sample.meta.id).is_none());
        assert!(store.list().is_empty());
        assert!(store
            .update(&sample.meta.id, SessionPatch::default())
            .is_err());
        assert_eq!(std::fs::read(&path).unwrap(), inconsistent);
        assert!(store.raw_path(&sample.meta.id).unwrap().is_file());
        assert!(!tmp.path().join("sessions/s-synthetic-other-game").exists());
    }

    #[test]
    fn concurrent_reads_reprocess_once_and_preserve_capture_warnings() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(tmp.path());
        let mut s = sample(&store, &vec![10.0; 3000]);
        s.meta.analysis_version = 0;
        for code in [
            "interrupted",
            "pack-changed",
            "pack-recipe-changed",
            "synthetic",
            "fixture",
        ] {
            s.meta.quality.issues.push(Issue::new(
                Severity::Warning,
                code,
                "Fixture sintética: fato que não pode ser reconstruído pelo CSV.",
            ));
        }
        store
            .save(&s, &per_frame(&vec![10.0; 3000], 90.0).into_bytes())
            .unwrap();
        let workers: Vec<_> = (0..4)
            .map(|_| {
                let store = store.clone();
                let id = s.meta.id.clone();
                std::thread::spawn(move || store.get(&id).expect("sessão preservada"))
            })
            .collect();
        for worker in workers {
            let session = worker.join().unwrap();
            assert_eq!(session.meta.analysis_version, ANALYSIS_VERSION);
            assert_eq!(session.meta.quality.level, QualityLevel::Usable);
            for code in [
                "interrupted",
                "pack-changed",
                "pack-recipe-changed",
                "synthetic",
                "fixture",
            ] {
                assert!(session.meta.quality.issues.iter().any(|i| i.code == code));
            }
            assert!(!session.series.is_empty());
        }
        let cleared = store
            .update(
                &s.meta.id,
                SessionPatch {
                    app_id: Some(String::new()),
                    game_name: Some(String::new()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(cleared.app_id.is_none() && cleared.game_name.is_none());
    }

    #[test]
    fn old_analysis_versions_are_reprocessed_and_newer_schemas_preserved() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(tmp.path());
        let s = sample(&store, &vec![10.0; 3000]);
        // Simula uma sessão gravada por uma versão anterior da análise, sem schemaVersion.
        let path = tmp
            .path()
            .join("sessions")
            .join(&s.meta.id)
            .join("meta.json");
        let mut v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        v["analysisVersion"] = 0.into();
        v["stats"]["fps"]["avg"] = 1.0.into();
        v.as_object_mut().unwrap().remove("schemaVersion");
        v.as_object_mut().unwrap().remove("fixIds");
        std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
        let m = store.get(&s.meta.id).unwrap().meta;
        assert!(
            (m.stats.fps.avg.unwrap() - 100.0).abs() < 0.01,
            "recalculado a partir do raw.csv"
        );
        assert_eq!(m.analysis_version, ANALYSIS_VERSION);

        // Formato do futuro: ignorado na listagem, mas nunca apagado.
        v["schemaVersion"] = 99.into();
        std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
        assert!(store.list().is_empty());
        assert!(path.is_file());
    }
}
