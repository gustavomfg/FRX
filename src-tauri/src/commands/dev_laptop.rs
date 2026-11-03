//! Fixture de desenvolvimento: simula a identidade DMI de um laptop para ver
//! o fluxo completo da interface num desktop. Só existe em build de debug.
//!
//! Só a identidade é simulada. A busca vai à Arch Wiki de verdade, e a
//! prévia, o backup e a aplicação são os do Fix Engine, em modo manual (o
//! FRX prepara o arquivo e mostra o comando; nada é gravado em `/etc`).
#![cfg(debug_assertions)]

use std::sync::Mutex;

use crate::system::LaptopIdentity;

static DEV_LAPTOP: Mutex<Option<LaptopIdentity>> = Mutex::new(None);

fn identity(vendor: &str, model: &str, board: &str, version: Option<&str>, display: &str) -> LaptopIdentity {
    LaptopIdentity {
        vendor: Some(vendor.into()),
        model: Some(model.into()),
        board_name: Some(board.into()),
        product_version: version.map(Into::into),
        display_name: Some(display.into()),
    }
}

/// Laptops simuláveis. O X1 Carbon tem página na wiki, mas nenhuma instrução
/// aplicável nas seções de sintomas; o Framework 13 tem (Wi-Fi e Suspensão).
fn preset(id: &str) -> Option<LaptopIdentity> {
    match id {
        "thinkpad-x1-carbon-gen9" => Some(identity(
            "LENOVO",
            "20XWCTO1WW",
            "20XWCTO1WW",
            Some("ThinkPad X1 Carbon Gen 9"),
            "ThinkPad X1 Carbon Gen 9",
        )),
        "framework-13" => Some(identity(
            "Framework",
            "Laptop (13th Gen Intel Core)",
            "FRANMACP04",
            Some("A6"),
            "Framework Laptop 13",
        )),
        _ => None,
    }
}

pub fn current() -> Option<LaptopIdentity> {
    DEV_LAPTOP.lock().ok().and_then(|g| g.clone())
}

/// `None` volta para o DMI real.
#[tauri::command]
pub fn set_dev_laptop(preset_id: Option<String>) -> Result<(), String> {
    let next = match preset_id.as_deref() {
        None => None,
        Some(id) => Some(preset(id).ok_or("Laptop de teste desconhecido.")?),
    };
    *DEV_LAPTOP.lock().map_err(|_| "estado indisponível")? = next;
    Ok(())
}
