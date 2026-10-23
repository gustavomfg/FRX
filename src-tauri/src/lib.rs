//! Camada nativa do FRX.
//!
//! Nesta etapa ela apenas hospeda a interface e expõe comandos somente
//! leitura para validar a ponte TypeScript ↔ Rust. Nenhuma correção é
//! aplicada, nenhum arquivo de jogo é tocado e nenhum comando externo roda.

mod commands;

pub fn run() {
    configure_webkit();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::app_info, commands::ping])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o FRX");
}

/// Ajustes de compatibilidade do WebKitGTK no Linux.
///
/// Em sessões Wayland com GPU NVIDIA, o renderizador DMA-BUF do WebKitGTK
/// pode deixar a janela em branco. Desativá-lo é o contorno recomendado e não
/// afeta outras GPUs de forma perceptível. O usuário pode sobrescrever
/// definindo a variável antes de abrir o app.
fn configure_webkit() {
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }
}
