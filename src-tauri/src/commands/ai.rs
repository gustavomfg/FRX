//! Comando da pesquisa assistida: reescreve a consulta e ranqueia candidatos
//! mockados, sem rede. Só lê e calcula: sem a Qwen, a única leitura do disco é a
//! versão do Mesa. Não executa nada, não escreve e não toca o Fix Engine.

use std::path::Path;

use crate::ai::{self, driver, models::*, query};

#[tauri::command]
pub fn research_fix(mut query: ResearchQuery) -> Result<Vec<ScoredCandidate>, String> {
    // A versão do Mesa vem do host, nunca do que a interface enviou.
    query.system.mesa_version = driver::read_host_mesa(Path::new("/")).version;
    let choice = std::env::var(query::REWRITER_ENV).ok();
    let rewriter = query::select_rewriter(choice.as_deref()).map_err(|e| e.to_string())?;
    ai::research_with(&query, rewriter.as_ref(), crate::time::now_unix()).map_err(|e| e.to_string())
}
