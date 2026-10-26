//! Diário persistente das operações: `<app_data>/fix-engine/operations/<id>.json`.
//!
//! Cada transição de estado é gravada (de forma atômica) antes do passo
//! seguinte. Ao abrir o FRX, operações que ficaram no meio do caminho são
//! reconhecidas e nunca aparecem como concluídas.

use std::path::PathBuf;

use super::files::{atomic_write, private_dir};
use super::models::{OpEvent, OpState, Operation};
use crate::time::now_iso;

#[derive(Debug, Clone)]
pub struct Journal {
    dir: PathBuf,
}

pub fn valid_op_id(id: &str) -> bool {
    id.len() <= 48
        && id.starts_with("op-")
        && id[3..].bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
}

pub fn new_op_id() -> String {
    format!("op-{}", &crate::performance::persistence::new_id()[2..])
}

impl Journal {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn path(&self, id: &str) -> Result<PathBuf, String> {
        if !valid_op_id(id) {
            return Err("Identificador de operação inválido.".into());
        }
        Ok(self.dir.join(format!("{id}.json")))
    }

    pub fn save(&self, op: &Operation) -> Result<(), String> {
        private_dir(&self.dir)?;
        let json = serde_json::to_vec_pretty(op).map_err(|e| e.to_string())?;
        atomic_write(&self.path(&op.id)?, &json, 0o600)
    }

    /// Muda o estado, registra o evento e grava antes de seguir.
    pub fn transition(
        &self,
        op: &mut Operation,
        state: OpState,
        message: impl Into<String>,
    ) -> Result<(), String> {
        let message = message.into();
        op.state = state;
        op.updated_at = now_iso();
        op.events.push(OpEvent {
            at: op.updated_at.clone(),
            state,
            message: message.clone(),
        });
        op.message = Some(message);
        self.save(op)
    }

    pub fn load(&self, id: &str) -> Result<Operation, String> {
        let bytes =
            std::fs::read(self.path(id)?).map_err(|_| "Operação não encontrada.".to_owned())?;
        serde_json::from_slice(&bytes).map_err(|e| format!("Diário da operação ilegível: {e}"))
    }

    /// Todas as operações, mais recentes primeiro.
    pub fn list(&self) -> Vec<Operation> {
        let mut out: Vec<Operation> = std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let id = name.strip_suffix(".json")?;
                valid_op_id(id).then(|| self.load(id).ok()).flatten()
            })
            .collect();
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
        out
    }
}
