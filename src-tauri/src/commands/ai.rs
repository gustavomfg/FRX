//! Comando da pesquisa assistida. Hoje ranqueia candidatos mockados, sem rede
//! e sem modelo. Só lê e calcula: a única leitura do disco é a versão do Mesa,
//! sem executar nada. Não escreve e não toca o Fix Engine.

use std::path::Path;

use crate::ai::{self, driver, models::*};

#[tauri::command]
pub fn research_fix(mut query: ResearchQuery) -> Result<Vec<ScoredCandidate>, String> {
    // A versão do Mesa vem do host, nunca do que a interface enviou.
    query.system.mesa_version = driver::read_host_mesa(Path::new("/")).version;
    ai::research_mocked(&query, crate::time::now_unix()).map_err(|e| e.to_string())
}
