//! Read-only technology inspection and conservative, attributable diagnostics.
//!
//! A filename is an indicator, a component banner is not a working feature,
//! and an imported log is not a trusted command stream. No source text is ever
//! executed or copied into the persisted local observations.

use std::collections::BTreeSet;
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path};

use super::models::{Evidence, LogObservation, Problem, Technology};
use crate::fix_engine::files::sha256_hex;
use crate::fix_engine::models::GameCtx;
use crate::system::GpuVendor;
use crate::time::now_iso;

const LOG_LIMIT: usize = 2 * 1024 * 1024;
const CONFIG_LIMIT: usize = 64 * 1024;
const SCAN_READ_LIMIT: usize = 256 * 1024;
const SCAN_ENTRIES: usize = 2000;
const SCAN_DEPTH: usize = 4;
const NVIDIA_POLICY_URL: &str = "https://www.nvidia.com/en-us/geforce/technologies/dlss/";
const NVIDIA_POLICY_VERSION: &str =
    "NVIDIA DLSS hardware matrix, reviewed 2026-10-27; FRX adapter 1";
const NVIDIA_POLICY_REVIEWED_AT: u64 = 1_793_059_200;
const NVIDIA_POLICY_MAX_AGE: u64 = 180 * 24 * 3600;

const TECHNOLOGIES: &[(&str, &str)] = &[
    ("physx", "NVIDIA PhysX"),
    ("dlss", "DLSS Super Resolution"),
    ("dlaa", "DLAA"),
    ("dlss-rr", "DLSS Ray Reconstruction"),
    ("dlss-fg", "DLSS Frame Generation"),
    ("dlss-mfg", "DLSS Multi Frame Generation"),
    ("fsr", "AMD FSR"),
    ("fsr-fg", "FSR Frame Generation"),
    ("xess", "Intel XeSS"),
    ("ray-tracing", "Ray tracing / DXR"),
    ("graphics-stack", "DirectX / Vulkan / OpenGL"),
    ("hdr", "HDR"),
    ("reflex", "NVIDIA Reflex"),
    ("audio", "Áudio"),
    ("video", "Vídeos e cutscenes"),
    ("controls", "Controles"),
    ("stability", "Inicialização e estabilidade"),
];

fn evidence(app_id: &str, technology: &str, kind: &str, summary: &str) -> Evidence {
    Evidence {
        id: format!(
            "local-{}",
            &sha256_hex(format!("{app_id}\0{technology}\0{kind}\0{summary}").as_bytes())[..20]
        ),
        kind: kind.into(),
        summary: summary.into(),
        source_url: None,
        publisher: Some("FRX — inspeção local".into()),
        published_at: None,
        updated_at: None,
        fetched_at: now_iso(),
        version: Some("FRX diagnostic adapter 1".into()),
        app_id: Some(app_id.into()),
        technology: Some(technology.into()),
        stale: false,
        limitations: vec![],
    }
}

/// Walks every ancestor by descriptor, refusing symlinks and special files.
/// O_NONBLOCK prevents a raced FIFO/device from blocking before fstat rejects it.
fn open_child(parent: &File, name: &std::ffi::OsStr, directory: bool) -> Result<File, String> {
    let name = CString::new(name.as_bytes()).map_err(|_| "Nome de arquivo inválido.")?;
    let flags = libc::O_RDONLY
        | libc::O_CLOEXEC
        | libc::O_NOFOLLOW
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: parent is a live directory descriptor, name is NUL-terminated,
    // openat returns a new owned descriptor, and no creation flags are used.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err("Não foi possível abrir o arquivo sem seguir links simbólicos.".into());
    }
    // SAFETY: this branch owns the descriptor just returned by openat.
    let file = unsafe { File::from_raw_fd(fd) };
    let meta = file
        .metadata()
        .map_err(|_| "Não foi possível inspecionar o arquivo.")?;
    if (directory && !meta.is_dir()) || (!directory && !meta.is_file()) {
        return Err("A inspeção aceita somente arquivos regulares e diretórios reais.".into());
    }
    Ok(file)
}

fn open_path(path: &Path, directory: bool) -> Result<File, String> {
    if !path.is_absolute() {
        return Err("Selecione um caminho absoluto.".into());
    }
    let components: Vec<_> = path.components().collect();
    let mut current = File::open("/").map_err(|_| "Não foi possível abrir o diretório raiz.")?;
    for (index, component) in components.iter().enumerate() {
        match component {
            Component::RootDir => continue,
            Component::Normal(name) => {
                current = open_child(&current, name, index + 1 < components.len() || directory)?;
            }
            _ => return Err("Caminhos com componentes ambíguos não são aceitos.".into()),
        }
    }
    if !directory
        && !current
            .metadata()
            .map_err(|_| "Arquivo indisponível.")?
            .is_file()
    {
        return Err("Selecione um arquivo regular.".into());
    }
    Ok(current)
}

fn bounded_text(mut file: File, limit: usize) -> Result<String, String> {
    let before = file.metadata().map_err(|_| "Arquivo indisponível.")?;
    if !before.is_file() || before.len() > limit as u64 {
        return Err("O arquivo excede o limite permitido ou não é um arquivo regular.".into());
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Não foi possível ler o arquivo autorizado.")?;
    let after = file.metadata().map_err(|_| "Arquivo indisponível.")?;
    if bytes.len() > limit
        || before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
        || bytes.len() as u64 != after.len()
    {
        return Err("O arquivo mudou durante a leitura ou excede o limite permitido. Tente novamente após fechar a gravação.".into());
    }
    let text = String::from_utf8(bytes).map_err(|_| "O log precisa ser texto UTF-8 válido.")?;
    if text.contains('\0') {
        return Err("Conteúdo binário não é aceito como log ou configuração.".into());
    }
    Ok(text)
}

#[derive(Default)]
struct Scan {
    entries: usize,
    bytes: usize,
    indicators: BTreeSet<(&'static str, &'static str)>,
    config: BTreeSet<&'static str>,
    limitations: BTreeSet<String>,
}

fn file_indicator(name: &str) -> Option<(&'static str, &'static str)> {
    match name {
        "physxloader.dll" | "physxloader64.dll" | "physxdevice.dll" | "physxcore.dll"
        | "physxcudart_20.dll" | "physx3_x86.dll" | "physx3_x64.dll" | "physx_64.dll" => {
            Some(("physx", "Biblioteca com nome conhecido de PhysX encontrada; a presença não confirma uso, versão, aceleração por GPU ou integridade."))
        }
        "nvngx_dlss.dll" => Some(("dlss", "Biblioteca nvngx_dlss.dll encontrada; pode estar empacotada sem ser usada pelo jogo.")),
        "nvngx_dlssd.dll" => Some(("dlss-rr", "Biblioteca nvngx_dlssd.dll encontrada; Ray Reconstruction não foi observado em execução.")),
        "nvngx_dlssg.dll" | "sl.dlss_g.dll" => Some(("dlss-fg", "Biblioteca associada a Frame Generation encontrada; quadros gerados e latência não foram medidos.")),
        "libxess.dll" => Some(("xess", "Biblioteca libxess.dll encontrada; a implementação e o modo usado pelo jogo ainda são desconhecidos.")),
        "amd_fidelityfx_dx12.dll" | "amd_fidelityfx_vk.dll" | "ffx_fsr2_api_x64.dll"
        | "amd_fidelityfx_upscaler.dll" => Some(("fsr", "Biblioteca associada a FidelityFX encontrada; a variante e a utilização de FSR precisam de confirmação.")),
        "amd_fidelityfx_framegeneration.dll" => Some(("fsr-fg", "Biblioteca associada a FSR Frame Generation encontrada; a presença não confirma geração de quadros.")),
        "d3d9.dll" => Some(("graphics-stack", "d3d9.dll encontrada: indício de componente DirectX 9, sem confirmação de implementação ou execução.")),
        "d3d10.dll" | "d3d10core.dll" => Some(("graphics-stack", "Componente DirectX 10 encontrado pelo nome; backend e uso ainda não confirmados.")),
        "d3d11.dll" => Some(("graphics-stack", "d3d11.dll encontrada: pode ser DXVK, WineD3D ou outro componente; o nome não determina o backend.")),
        "d3d12.dll" | "d3d12core.dll" => Some(("graphics-stack", "Componente DirectX 12 encontrado pelo nome; isso não confirma ray tracing nem VKD3D-Proton em execução.")),
        "vulkan-1.dll" => Some(("graphics-stack", "Loader Vulkan encontrado pelo nome; API efetivamente utilizada ainda desconhecida.")),
        "opengl32.dll" => Some(("graphics-stack", "Componente OpenGL encontrado pelo nome; API efetivamente utilizada ainda desconhecida.")),
        "nvapi64.dll" | "nvapi.dll" => Some(("dlss", "Componente NVAPI encontrado pelo nome; sua presença não comprova DLSS, Reflex ou compatibilidade com o Proton atual.")),
        _ => None,
    }
}

fn scan_dir(dir: &File, depth: usize, scan: &mut Scan) {
    // /proc/self/fd refers to the already-open directory, not an untrusted
    // path traversed again. Every child is opened relative to that descriptor.
    let Ok(entries) = std::fs::read_dir(format!("/proc/self/fd/{}", dir.as_raw_fd())) else {
        scan.limitations
            .insert("Parte da instalação não pôde ser inspecionada.".into());
        return;
    };
    for entry in entries {
        if scan.entries >= SCAN_ENTRIES {
            scan.limitations.insert("Inspeção limitada a 2.000 entradas e quatro níveis de diretórios; ausência de indícios não prova ausência do recurso.".into());
            break;
        }
        scan.entries += 1;
        let Ok(entry) = entry else { continue };
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_symlink() {
            scan.limitations
                .insert("Links simbólicos foram ignorados na inspeção.".into());
            continue;
        }
        let name = entry.file_name();
        if kind.is_dir() {
            if depth < SCAN_DEPTH {
                if let Ok(child) = open_child(dir, &name, true) {
                    scan_dir(&child, depth + 1, scan);
                }
            } else {
                scan.limitations
                    .insert("A inspeção não percorre diretórios além de quatro níveis.".into());
            }
            continue;
        }
        if !kind.is_file() {
            continue;
        }
        let lower = name.to_string_lossy().to_ascii_lowercase();
        if let Some(indicator) = file_indicator(&lower) {
            // Re-open without following symlinks in case the entry changed.
            if open_child(dir, &name, false).is_ok() {
                scan.indicators.insert(indicator);
            }
        }
        if lower == "dxvk.conf" {
            if scan.bytes + CONFIG_LIMIT > SCAN_READ_LIMIT {
                scan.limitations
                    .insert("Limite de leitura de configurações atingido (256 KiB).".into());
                continue;
            }
            scan.bytes += CONFIG_LIMIT;
            match open_child(dir, &name, false).and_then(|f| bounded_text(f, CONFIG_LIMIT)) {
                Ok(text) => {
                    for line in text.lines().take(2048) {
                        // Only one documented typed key is interpreted. Other
                        // keys, includes, commands and arbitrary text are data.
                        if let Some((key, value)) =
                            line.split('#').next().unwrap_or("").split_once('=')
                        {
                            if key.trim() == "dxgi.enableHDR"
                                && value.trim().eq_ignore_ascii_case("true")
                            {
                                scan.config.insert("hdr");
                            }
                        }
                    }
                }
                Err(_) => {
                    scan.limitations.insert(
                        "Uma configuração não pôde ser lida dentro dos limites de segurança."
                            .into(),
                    );
                }
            }
        }
    }
}

fn nvidia_policy(technology: &str, summary: &str) -> Evidence {
    let mut e = evidence("", technology, "hardware-policy", summary);
    e.app_id = None;
    e.publisher = Some("NVIDIA".into());
    e.source_url = Some(NVIDIA_POLICY_URL.into());
    e.version = Some(NVIDIA_POLICY_VERSION.into());
    e.stale = crate::time::now_unix() > NVIDIA_POLICY_REVIEWED_AT + NVIDIA_POLICY_MAX_AGE;
    e.limitations = vec!["Política de capacidade do hardware, consultada em 2026-10-27; não confirma suporte do jogo, driver, Proton ou execução. Gerações não reconhecidas permanecem desconhecidas.".into()];
    if e.stale {
        e.limitations.push("Esta política foi consultada há mais de 180 dias; confirme a matriz atual antes de recomendar recursos novos.".into());
    }
    e
}

fn rtx_generation(name: Option<&str>) -> Option<u16> {
    let words: Vec<_> = name?.split_whitespace().collect();
    for pair in words.windows(2) {
        if pair[0].eq_ignore_ascii_case("RTX") {
            let model = pair[1];
            if model.len() == 4 && model.bytes().all(|c| c.is_ascii_digit()) {
                let series = model.parse::<u16>().ok()? / 100;
                if [20, 30, 40, 50].contains(&series) {
                    return Some(series);
                }
            }
        }
    }
    None
}

fn hardware(game: &GameCtx, technology: &mut Technology) {
    let environment = game
        .performance_conditions
        .as_ref()
        .and_then(|c| c.environment.as_ref());
    let Some(gpu) = environment.and_then(|e| e.observed_gpu()) else {
        technology.limitations.extend(environment.map(|e| e.selection_limitations()).unwrap_or_else(|| vec!["GPU de execução não confirmada; o inventário do sistema não prova qual placa renderiza o jogo.".into()]));
        return;
    };
    if gpu.driver_version.is_none() {
        technology.limitations.push("Versão do driver da GPU utilizada desconhecida; compatibilidade do ambiente não confirmada.".into());
    }
    if !["dlss", "dlaa", "dlss-rr", "dlss-fg", "dlss-mfg"].contains(&technology.id.as_str()) {
        return;
    }
    let generation = (gpu.vendor == GpuVendor::Nvidia)
        .then(|| rtx_generation(gpu.name.as_deref()))
        .flatten();
    let capable = match (technology.id.as_str(), generation) {
        ("dlss-mfg", Some(g)) => Some(g == 50),
        ("dlss-fg", Some(g)) => Some(g >= 40),
        (_, Some(_)) => Some(true),
        (_, None) if matches!(gpu.vendor, GpuVendor::Amd | GpuVendor::Intel) => Some(false),
        _ => None,
    };
    if let Some(capable) = capable {
        technology.hardware = if capable { "supported" } else { "incompatible" }.into();
        let summary = match technology.id.as_str() {
            "dlss-mfg" => "A matriz NVIDIA consultada restringe DLSS Multi Frame Generation à GeForce RTX 50. Uma RTX 40 não satisfaz esse requisito.",
            "dlss-fg" => "A matriz NVIDIA consultada inclui GeForce RTX 40 e RTX 50 para DLSS Frame Generation; isso não abrange Multi Frame Generation.",
            _ => "A matriz NVIDIA consultada inclui GeForce RTX 20, 30, 40 e 50 para DLSS Super Resolution, DLAA e Ray Reconstruction.",
        };
        technology
            .evidence
            .push(nvidia_policy(&technology.id, summary));
    }
}

/// Produces independent states for hardware, implementation, configuration,
/// environment and observation. Missing evidence never turns into support.
pub fn technologies(
    game: &GameCtx,
    log: Option<&LogObservation>,
    knowledge: &[Evidence],
) -> Vec<Technology> {
    let mut scan = Scan::default();
    match game.install_path.as_ref() {
        Some(path) => match open_path(path, true) {
            Ok(dir) => scan_dir(&dir, 0, &mut scan),
            Err(_) => {
                scan.limitations.insert("Instalação indisponível para inspeção sem links simbólicos; nenhum recurso foi presumido ausente.".into());
            }
        },
        None => {
            scan.limitations
                .insert("Diretório do jogo desconhecido; inspeção estática não realizada.".into());
        }
    }
    let valid_log = log.filter(|l| l.app_id == game.app_id);
    TECHNOLOGIES.iter().map(|(id, name)| {
        let mut t = Technology::unknown(id, name);
        hardware(game, &mut t);
        t.limitations.extend(scan.limitations.iter().cloned());
        for (tech, summary) in &scan.indicators {
            if tech == id {
                t.game_support = "static-candidate".into();
                t.evidence.push(evidence(&game.app_id, id, "static-candidate", summary));
            }
        }
        if scan.config.contains(id) {
            t.configured = "candidate".into();
            t.evidence.push(evidence(&game.app_id, id, "configuration-candidate", "Uma configuração dxvk.conf solicita HDR; ainda não foi confirmado que o jogo carrega esse arquivo ou apresenta HDR."));
        }
        let matches_topic = |e: &&Evidence| {
            e.technology.as_deref() == Some(*id)
                || (e.technology.as_deref() == Some("frame-generation") && ["dlss-fg", "dlss-mfg", "fsr-fg"].contains(id))
        };
        // Research evidence is context. Neither a generic README nor a report
        // about another AppID can establish a feature of this installation.
        t.evidence.extend(knowledge.iter().filter(matches_topic).filter(|e| matches!(e.kind.as_str(), "official-documentation" | "official-release" | "community-report" | "source-unavailable")).filter(|e| e.app_id.as_ref().is_none_or(|a| a == &game.app_id)).take(24).cloned());
        if let Some(log) = valid_log {
            t.evidence.extend(log.evidence.iter().filter(matches_topic).filter(|e| e.app_id.as_deref() == Some(game.app_id.as_str())).cloned());
            t.limitations.extend(log.limitations.iter().cloned());
        }
        let runtime_evidence: Vec<_> = valid_log.into_iter().flat_map(|l| l.evidence.iter()).filter(matches_topic).filter(|e| e.app_id.as_deref() == Some(game.app_id.as_str())).collect();
        let component = runtime_evidence.iter().any(|e| e.kind == "runtime-component");
        let failure = runtime_evidence.iter().any(|e| e.kind == "runtime-failure");
        if component || failure {
            t.environment = match (component, failure) { (true, true) => "conflicting", (true, false) => "observed", _ => "failed" }.into();
            t.runtime = match (component, failure) { (true, true) => "conflicting", (true, false) => "component-observed", _ => "failed" }.into();
        }
        match *id {
            "physx" => t.limitations.push("Uma biblioteca PhysX ou uma GPU NVIDIA não comprova aceleração PhysX por GPU. Caminho CPU/GPU e versão precisam de observação específica.".into()),
            "dlss-fg" | "dlss-mfg" | "fsr-fg" => t.limitations.push("Sem instrumentação de quadros renderizados, gerados, apresentados e latência, Frame Generation permanece não verificado. FPS médio do MangoHud não é essa prova.".into()),
            "fsr" => t.limitations.push("FSR não é exclusivo de GPUs AMD. Os requisitos dependem da implementação e da versão, ainda não confirmadas por nomes de arquivos.".into()),
            "ray-tracing" => t.limitations.push("DirectX 12 não comprova suporte a DXR. Extensões Vulkan, driver, VKD3D-Proton e implementação do jogo precisam ser confirmados.".into()),
            "graphics-stack" => t.limitations.push("Banners de DXVK/VKD3D identificam componentes carregados; não comprovam apresentação correta, recursos avançados ou ausência de crashes.".into()),
            "dlss" => t.limitations.push("NGX/NVAPI presentes não confirmam DLSS ativo; verifique implementação do jogo e mecanismo documentado para a versão exata do Proton.".into()),
            _ => {}
        }
        t.limitations.sort();
        t.limitations.dedup();
        t
    }).collect()
}

fn version_token(text: &str) -> Option<String> {
    let value = text.split_whitespace().next()?.trim_end_matches('.');
    let value = value.strip_prefix('v').unwrap_or(value);
    // Custom suffixes can contain private build labels. Retain only the
    // numeric release core, which is still an unverified log observation.
    let core: String = value
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let parts: Vec<_> = core.trim_end_matches('.').split('.').collect();
    (value.len() <= 48
        && (2..=4).contains(&parts.len())
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 5 && p.bytes().all(|c| c.is_ascii_digit())))
    .then(|| parts.join("."))
}

/// A Wine timestamp/thread prefix can contain only numeric/hex fields. Text
/// that merely quotes a diagnostic message is not a diagnostic record.
fn wine_message<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    let (prefix, body) = line.split_once(marker)?;
    if prefix
        .bytes()
        .all(|c| c.is_ascii_hexdigit() || b".:".contains(&c))
    {
        Some(body)
    } else {
        None
    }
}

fn parse_log(app_id: &str, text: &str) -> Result<LogObservation, String> {
    if !crate::art::valid_app_id(app_id) {
        return Err("AppID inválido.".into());
    }
    let mut headers = BTreeSet::new();
    let mut header_at_start = false;
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if let Some(value) = line
            .strip_prefix("SteamGameId:")
            .or_else(|| line.strip_prefix("SteamAppId:"))
        {
            let id = value.trim();
            if !crate::art::valid_app_id(id) {
                return Err("O cabeçalho Steam do log é inválido.".into());
            }
            header_at_start |= index < 128;
            headers.insert(id);
        }
    }
    if !header_at_start || headers.len() != 1 || !headers.contains(app_id) {
        return Err("O log precisa identificar somente este AppID em SteamGameId: ou SteamAppId: no cabeçalho. O nome do arquivo não comprova a associação; importe o log Proton correspondente ao jogo.".into());
    }
    let mut observation = LogObservation {
        app_id: app_id.into(), imported_at: now_iso(), digest: sha256_hex(text.as_bytes()),
        evidence: vec![], limitations: vec!["Log importado e atribuído por cabeçalho: não comprova a autenticidade, data de execução, GPU selecionada ou condições atuais. Somente padrões reconhecidos são preservados; o texto original e os caminhos não são armazenados.".into()],
    };
    let mut seen = BTreeSet::new();
    let mut add = |tech: &str, kind: &str, summary: &str, version: Option<String>| {
        let key = format!(
            "{tech}:{kind}:{summary}:{}",
            version.as_deref().unwrap_or("")
        );
        if seen.insert(key) && observation.evidence.len() < 64 {
            let mut e = evidence(app_id, tech, kind, summary);
            if version.is_some() {
                e.version = version;
            }
            e.id = format!("{}-{}", e.id, &observation.digest[..12]);
            observation.evidence.push(e);
        }
    };
    let mut oversized_lines = false;
    for line in text.lines() {
        if line.len() > 4096 {
            oversized_lines = true;
            continue;
        }
        let line = line.trim();
        if let Some(body) = line.strip_prefix("info:").map(str::trim) {
            if let Some(v) = body
                .strip_prefix("DXVK:")
                .and_then(|v| version_token(v.trim()))
            {
                add("graphics-stack", "runtime-component", "Banner do DXVK reconhecido no log deste AppID; componente carregado, renderização correta ainda não verificada.", Some(v));
            }
        }
        if let Some(body) = wine_message(line, "info:vkd3d-proton:vkd3d_get_vk_version:")
            .or_else(|| wine_message(line, "info:vkd3d_get_vk_version:"))
        {
            if let Some(v) = body
                .trim()
                .strip_prefix("vkd3d-proton - applicationVersion:")
                .and_then(|v| version_token(v.trim()))
            {
                add("graphics-stack", "runtime-component", "Banner de versão do VKD3D-Proton reconhecido no log; não comprova suporte ou funcionamento de ray tracing.", Some(v));
            }
        }
        if wine_message(line, "err:winediag:wined3d_dll_init:")
            .is_some_and(|body| body.trim() == "Using the OpenGL renderer.")
        {
            add("graphics-stack", "runtime-component", "WineD3D anunciou o renderer OpenGL no log; isso não determina ganho ou perda de desempenho.", None);
        }
        if let Some(body) = line.strip_prefix("err:").map(str::trim) {
            if body.starts_with("DxvkInstance: Failed to create instance")
                || body.starts_with("DxvkAdapter: Failed to create device")
                || body.starts_with("D3D11CreateDevice: Failed to create device")
            {
                add("graphics-stack", "runtime-failure", "O componente gráfico registrou falha ao criar instância ou dispositivo. Driver, Vulkan e seleção da GPU são hipóteses a investigar.", None);
            }
            if body.starts_with("Dxvk") && body.contains("VK_ERROR_DEVICE_LOST") {
                add("graphics-stack", "runtime-failure", "DXVK registrou VK_ERROR_DEVICE_LOST. O erro não identifica sozinho a causa ou um defeito físico da GPU.", None);
            }
        }
        if let Some(body) = wine_message(line, "err:module:import_dll:")
            .or_else(|| wine_message(line, "err:module:import_dll "))
        {
            if body.starts_with("Library ") && body.contains(" not found") {
                add("stability", "runtime-failure", "O loader do Wine registrou uma biblioteca ausente; confirme se ela é necessária ao problema observado antes de instalar componentes.", None);
                let library = body
                    .strip_prefix("Library ")
                    .unwrap_or("")
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_matches('"')
                    .to_ascii_lowercase();
                if let Some((tech, _)) = file_indicator(&library) {
                    add(tech, "runtime-failure", "O loader do Wine registrou falta de uma biblioteca associada a esta tecnologia. O nome isolado não comprova que o recurso era implementado ou utilizado.", None);
                }
            }
        }
        if line.starts_with("wine: Unhandled page fault")
            || line.starts_with("wine: Unhandled exception")
        {
            add("stability", "runtime-failure", "Wine registrou uma exceção não tratada. O trecho não determina sozinho a causa do crash.", None);
        }
        // FRX deliberately has no arbitrary 'DLSS enabled', FPS, filename or
        // game-text parser that could claim a functional graphics feature.
    }
    if oversized_lines {
        observation
            .limitations
            .push("Linhas maiores que 4 KiB foram ignoradas.".into());
    }
    if observation.evidence.is_empty() {
        observation.limitations.push("Nenhum padrão diagnóstico suportado foi encontrado. Isso não prova ausência de falhas ou funcionamento dos recursos.".into());
    }
    Ok(observation)
}

/// Import is explicit and read-only. Only this structured result is persisted
/// by the caller; raw bytes, file paths and arbitrary log strings are excluded.
pub fn import_log(app_id: &str, path: &Path) -> Result<LogObservation, String> {
    parse_log(app_id, &bounded_text(open_path(path, false)?, LOG_LIMIT)?)
}

fn symptom_info(
    symptom: &str,
) -> Option<(
    &'static str,
    &'static str,
    &'static [&'static str],
    &'static str,
)> {
    Some(match symptom {
        "crash" => ("stability", "Crash ou encerramento inesperado", &["Uma dependência ausente, incompatibilidade de driver/Proton ou falha do próprio jogo pode provocar o encerramento."], "Importar um log Proton autorizado e reproduzir o mesmo cenário antes de escolher uma correção."),
        "startup" => ("stability", "O jogo não inicia", &["Dependências, versão do Proton, opções de inicialização e permissões podem impedir a inicialização."], "Revisar evidências e preparar um log Proton; nenhuma causa foi confirmada apenas pelo relato."),
        "low-performance" => ("graphics-stack", "Desempenho abaixo do esperado", &["Backend gráfico, GPU selecionada, limite de FPS, configurações e carga do sistema podem alterar o desempenho."], "Usar o Performance Lab com baseline comparável; contadores informais não são benchmarks registrados."),
        "stuttering" => ("graphics-stack", "Stuttering ou frametime irregular", &["Compilação de shaders, I/O, carga de CPU/GPU e apresentação podem provocar variação de frametime."], "Capturar frametimes no mesmo cenário e comparar percentis, stutters e condições pelo Performance Lab."),
        "physx" => ("physx", "Problema com física ou PhysX", &["Componentes legados ou um caminho CPU/GPU diferente do esperado podem afetar a física. NVIDIA detectada não confirma aceleração PhysX no Linux."], "Confirmar versão e componente com fonte específica do jogo. Patches sem origem, hash e destino confiáveis permanecem em revisão."),
        "dlss" => ("dlss", "DLSS indisponível ou com falhas", &["O jogo pode não implementar DLSS; GPU, NGX/NVAPI, driver ou a versão do Proton também podem limitar a disponibilidade."], "Verificar implementação do jogo e documentação da versão exata do Proton antes de propor atualização ou substituição de DLL."),
        "frame-generation" => ("dlss-fg", "Frame Generation indisponível ou não verificado", &["A implementação específica pode exigir outra geração de GPU, suporte do jogo, driver e Proton. DLSS FG, MFG e FSR FG têm requisitos diferentes."], "Identificar a tecnologia específica e verificar requisitos. Medir FPS renderizado/apresentado e latência quando houver instrumentação; FPS médio sozinho é insuficiente."),
        "ray-tracing" => ("ray-tracing", "Ray tracing indisponível ou com falhas", &["A implementação DXR do jogo, extensões Vulkan, driver e VKD3D-Proton podem impor requisitos ainda não confirmados. Recurso desativado não é evidência de defeito."], "Confirmar requisitos da versão do jogo e do VKD3D-Proton; não ativar ray tracing como otimização universal."),
        "fsr" => ("fsr", "Problema com FSR", &["Requisitos dependem da versão e implementação de FSR, do jogo, driver e Proton; não apenas do fabricante da GPU."], "Identificar a versão da implementação e consultar o mecanismo documentado para esse Proton antes de propor atualização."),
        "graphics" => ("graphics-stack", "Artefatos ou falhas gráficas", &["Backend, recursos Vulkan, driver, Proton ou uma configuração específica do jogo podem explicar os artefatos."], "Confrontar o log com fontes do mesmo jogo, API e ambiente; caminhos alternativos têm riscos e precisam de prévia."),
        "audio" => ("audio", "Problema de áudio", &["Dispositivo selecionado, backend de áudio ou dependências do jogo podem estar envolvidos; não há instrumentação para confirmar a causa."], "Pesquisar relatos do mesmo jogo e obter log autorizado. Nenhuma modificação de áudio será presumida segura sem receita suportada."),
        "video" => ("video", "Problema em vídeos ou cutscenes", &["Codecs e Media Foundation podem estar envolvidos, mas um relato isolado não identifica a dependência ausente."], "Consultar documentação do jogo e Proton; componentes sem procedência ou licença adequada permanecem em revisão."),
        "controls" => ("controls", "Problema com controles", &["Mapeamento, Steam Input ou APIs do jogo podem estar envolvidos; não foram inspecionados como causa confirmada."], "Consultar evidências específicas do jogo; configurações de outros jogos e do sistema não serão alteradas."),
        _ => return None,
    })
}

/// Diagnostic hypotheses are separate from applicable plans. This function
/// never invents an action when a safe recipe or sufficient evidence is absent.
pub fn problems(
    game: &GameCtx,
    technologies: &[Technology],
    log: Option<&LogObservation>,
    symptoms: &[String],
) -> Vec<Problem> {
    let reported: BTreeSet<_> = symptoms
        .iter()
        .filter(|s| symptom_info(s).is_some())
        .map(String::as_str)
        .collect();
    let mut requested = reported.clone();
    if let Some(log) = log.filter(|l| l.app_id == game.app_id) {
        for evidence in &log.evidence {
            if evidence.kind == "runtime-failure"
                && evidence.app_id.as_deref() == Some(game.app_id.as_str())
            {
                match evidence.technology.as_deref() {
                    Some("stability") => {
                        requested.insert(
                            if evidence.summary.starts_with("Wine registrou uma exceção") {
                                "crash"
                            } else {
                                "startup"
                            },
                        );
                    }
                    Some("graphics-stack") => {
                        requested.insert("graphics");
                    }
                    Some("physx") => {
                        requested.insert("physx");
                    }
                    Some("dlss") => {
                        requested.insert("dlss");
                    }
                    _ => {}
                }
            }
        }
    }
    requested.into_iter().filter_map(|symptom| {
        let (tech, label, hypotheses, next) = symptom_info(symptom)?;
        let relevant: Vec<_> = technologies.iter().filter(|t| t.id == tech || (symptom == "frame-generation" && ["dlss-mfg", "fsr-fg"].contains(&t.id.as_str()))).collect();
        let mut evidence: Vec<_> = relevant.iter().flat_map(|t| t.evidence.iter()).filter(|e| e.app_id.as_ref().is_none_or(|a| a == &game.app_id)).cloned().collect();
        if reported.contains(symptom) {
            evidence.push(self::evidence(&game.app_id, tech, "user-report", label));
        }
        let failed = evidence.iter().any(|e| e.kind == "runtime-failure");
        let conflicting = relevant.iter().any(|t| t.runtime == "conflicting");
        let mut limitations = vec!["Relatos e padrões de log ajudam a investigar; não comprovam causalidade nem que uma correção resolveu o problema.".into()];
        if conflicting { limitations.push("O log contém inicialização de componente e falha; pode representar momentos distintos. Não selecionar apenas a evidência favorável.".into()); }
        for t in &relevant {
            if t.hardware == "incompatible" { limitations.push(format!("{}: o hardware observado não satisfaz a política de suporte consultada. Não há download recomendado para contornar essa limitação.", t.name)); }
            if t.environment == "unknown" { limitations.push(format!("{}: disponibilidade no ambiente ainda não confirmada.", t.name)); }
        }
        Some(Problem {
            id: format!("diagnostic-{}-{symptom}", game.app_id), technology: tech.into(), symptom: label.into(),
            certainty: if conflicting { "conflicting" } else if failed { "observed-error" } else { "reported" }.into(),
            evidence, hypotheses: hypotheses.iter().map(|s| (*s).into()).collect(), ruled_out: vec![],
            limitations, next_action: next.into(), plan_ids: vec![],
        })
    }).collect()
}

#[cfg(test)]
mod tests {
    //! All fixtures below are synthetic text and temporary installations. They
    //! never enter the real Performance Lab or run games/components.
    use super::*;
    use crate::performance::models::Conditions;
    use crate::system::context::{EnvironmentContext, GpuSelection};
    use crate::system::GpuInfo;
    use std::fs;
    use std::os::unix::fs::symlink;
    use tempfile::TempDir;

    fn game() -> GameCtx {
        GameCtx {
            app_id: "200260".into(),
            name: "Fixture Batman — synthetic".into(),
            ..Default::default()
        }
    }

    fn gpu(vendor: GpuVendor, model: &str) -> GpuInfo {
        GpuInfo {
            pci_address: None,
            gpu_family: None,
            vram_bytes: None,
            vendor,
            vendor_id: "test-vendor".into(),
            device_id: "test-device".into(),
            name: Some(model.into()),
            driver: None,
            driver_version: None,
            boot_vga: false,
        }
    }

    fn with_gpu(gpu: GpuInfo, measured: bool) -> GameCtx {
        let mut game = game();
        game.performance_conditions = Some(Conditions {
            environment: Some(EnvironmentContext {
                detected_gpus: vec![gpu.clone()],
                selected_gpu: measured.then_some(gpu),
                selection: if measured {
                    GpuSelection::Measured
                } else {
                    GpuSelection::Unknown
                },
                ..Default::default()
            }),
            ..Default::default()
        });
        game
    }

    fn technology<'a>(technologies: &'a [Technology], id: &str) -> &'a Technology {
        technologies.iter().find(|t| t.id == id).unwrap()
    }

    fn log(text: &str) -> LogObservation {
        parse_log(
            "200260",
            &format!("Proton: synthetic-fixture\nSteamGameId: 200260\n{text}"),
        )
        .unwrap()
    }

    #[test]
    fn static_dlls_are_candidates_never_implemented_or_active_features() {
        let dir = TempDir::new().unwrap();
        for file in [
            "nvngx_dlss.dll",
            "nvngx_dlssg.dll",
            "PhysXLoader.dll",
            "d3d12.dll",
        ] {
            fs::write(dir.path().join(file), b"synthetic-not-an-executable").unwrap();
        }
        let mut game = game();
        game.install_path = Some(dir.path().into());
        let result = technologies(&game, None, &[]);
        for id in ["dlss", "dlss-fg", "physx", "graphics-stack"] {
            let t = technology(&result, id);
            assert_eq!(t.game_support, "static-candidate");
            assert_eq!(t.runtime, "unknown");
            assert_eq!(t.environment, "unknown");
        }
        assert_eq!(technology(&result, "ray-tracing").game_support, "unknown");
        assert_eq!(technology(&result, "dlss-mfg").game_support, "unknown");
    }

    #[test]
    fn scan_is_bounded_by_depth_and_entries_and_does_not_follow_links() {
        let root = TempDir::new().unwrap();
        let other = TempDir::new().unwrap();
        fs::write(other.path().join("nvngx_dlss.dll"), b"synthetic").unwrap();
        symlink(other.path(), root.path().join("linked-directory")).unwrap();
        symlink(
            other.path().join("nvngx_dlss.dll"),
            root.path().join("nvngx_dlss.dll"),
        )
        .unwrap();
        let deep = root.path().join("a/b/c/d/e");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("nvngx_dlssg.dll"), b"synthetic").unwrap();
        let dir = open_path(root.path(), true).unwrap();
        let mut scan = Scan::default();
        scan_dir(&dir, 0, &mut scan);
        assert!(scan.indicators.is_empty());
        assert!(scan.limitations.iter().any(|l| l.contains("simbólicos")));
        assert!(scan.limitations.iter().any(|l| l.contains("quatro níveis")));
        let mut exhausted = Scan {
            entries: SCAN_ENTRIES,
            ..Default::default()
        };
        scan_dir(&dir, 0, &mut exhausted);
        assert_eq!(exhausted.entries, SCAN_ENTRIES);
        assert!(exhausted.indicators.is_empty());
    }

    #[test]
    fn known_configuration_is_bounded_and_does_not_prove_loading() {
        let root = TempDir::new().unwrap();
        fs::write(
            root.path().join("dxvk.conf"),
            "dxgi.enableHDR = True\ninclude=/etc/passwd\n$(touch injected)\n",
        )
        .unwrap();
        let mut game = game();
        game.install_path = Some(root.path().into());
        let result = technologies(&game, None, &[]);
        let hdr = technology(&result, "hdr");
        assert_eq!(hdr.configured, "candidate");
        assert_eq!(hdr.runtime, "unknown");
        assert!(!root.path().join("injected").exists());
        fs::write(root.path().join("dxvk.conf"), vec![b'A'; CONFIG_LIMIT + 1]).unwrap();
        let result = technologies(&game, None, &[]);
        assert_eq!(technology(&result, "hdr").configured, "unknown");
        assert!(technology(&result, "hdr")
            .limitations
            .iter()
            .any(|l| l.contains("limites de segurança")));
    }

    #[test]
    fn unknown_gpu_and_inventory_do_not_become_selected_gpu() {
        let mut game = with_gpu(gpu(GpuVendor::Nvidia, "NVIDIA GeForce RTX 4060"), false);
        let result = technologies(&game, None, &[]);
        assert_eq!(technology(&result, "dlss-mfg").hardware, "unknown");
        let env = game
            .performance_conditions
            .as_mut()
            .unwrap()
            .environment
            .as_mut()
            .unwrap();
        env.detected_gpus
            .push(gpu(GpuVendor::Amd, "AMD Radeon Fixture"));
        env.selected_gpu = Some(env.detected_gpus[0].clone()); // selection still Unknown
        let result = technologies(&game, None, &[]);
        assert_eq!(technology(&result, "dlss").hardware, "unknown");
        assert!(technology(&result, "dlss")
            .limitations
            .iter()
            .any(|l| l.contains("várias GPUs")));
    }

    #[test]
    fn rtx_4060_supports_fg_hardware_but_not_mfg_and_unknown_driver_stays_unknown() {
        let game = with_gpu(gpu(GpuVendor::Nvidia, "NVIDIA GeForce RTX 4060"), true);
        let result = technologies(&game, None, &[]);
        assert_eq!(technology(&result, "dlss").hardware, "supported");
        assert_eq!(technology(&result, "dlss-fg").hardware, "supported");
        let mfg = technology(&result, "dlss-mfg");
        assert_eq!(mfg.hardware, "incompatible");
        assert_eq!(mfg.environment, "unknown");
        assert_eq!(mfg.runtime, "unknown");
        assert!(mfg
            .evidence
            .iter()
            .any(|e| e.source_url.as_deref() == Some(NVIDIA_POLICY_URL)
                && e.version.as_deref() == Some(NVIDIA_POLICY_VERSION)));
        assert!(mfg
            .limitations
            .iter()
            .any(|l| l.contains("driver") && l.contains("desconhecida")));
        let problems = problems(&game, &result, None, &["frame-generation".into()]);
        assert!(problems[0]
            .limitations
            .iter()
            .any(|l| l.contains("Multi Frame Generation") && l.contains("não satisfaz")));
        assert!(problems[0].plan_ids.is_empty());
    }

    #[test]
    fn newer_unreviewed_generations_remain_unknown() {
        let game = with_gpu(gpu(GpuVendor::Nvidia, "NVIDIA GeForce RTX 6060"), true);
        let result = technologies(&game, None, &[]);
        assert_eq!(technology(&result, "dlss-mfg").hardware, "unknown");
        let game = with_gpu(gpu(GpuVendor::Nvidia, "NVIDIA GeForce RTX 5090"), true);
        let result = technologies(&game, None, &[]);
        assert_eq!(technology(&result, "dlss-mfg").hardware, "supported");
        assert_eq!(technology(&result, "dlss-mfg").runtime, "unknown");
    }

    #[test]
    fn fsr_is_not_assumed_exclusive_to_amd_or_working_on_any_vendor() {
        for vendor in [GpuVendor::Amd, GpuVendor::Intel, GpuVendor::Nvidia] {
            let game = with_gpu(gpu(vendor, "Synthetic model"), true);
            let result = technologies(&game, None, &[]);
            assert_eq!(technology(&result, "fsr").hardware, "unknown");
            assert_eq!(technology(&result, "fsr").runtime, "unknown");
            if vendor != GpuVendor::Nvidia {
                assert_eq!(technology(&result, "dlss").hardware, "incompatible");
            }
        }
    }

    #[test]
    fn research_cannot_cross_games_or_claim_runtime_support() {
        let mut other = evidence("999", "dlss", "community-report", "Another game report");
        let mut general = other.clone();
        general.id = "general".into();
        general.app_id = None;
        general.kind = "official-documentation".into();
        let mut forged_runtime = general.clone();
        forged_runtime.id = "forged".into();
        forged_runtime.kind = "runtime-component".into();
        forged_runtime.app_id = Some("200260".into());
        other.id = "other-game".into();
        let result = technologies(&game(), None, &[other, general, forged_runtime]);
        let dlss = technology(&result, "dlss");
        assert!(dlss.evidence.iter().all(|e| e.id != "other-game"));
        assert!(dlss.evidence.iter().any(|e| e.id == "general"));
        assert_eq!(dlss.game_support, "unknown");
        assert_eq!(dlss.runtime, "unknown");
    }

    #[test]
    fn import_requires_header_attribution_and_rejects_mixed_appids_anywhere() {
        assert!(parse_log("200260", "info: DXVK: v2.6\n").is_err());
        assert!(parse_log("200260", "SteamGameId: 999\ninfo: DXVK: v2.6\n").is_err());
        assert!(parse_log("200260", "SteamGameId: 200260; command\n").is_err());
        assert!(parse_log("../200260", "SteamGameId: 200260\n").is_err());
        let mut mixed = "SteamGameId: 200260\n".to_owned();
        mixed.push_str(&"blank\n".repeat(160));
        mixed.push_str("SteamGameId: 999\ninfo: DXVK: v2.6\n");
        assert!(parse_log("200260", &mixed).is_err());
        assert!(parse_log(
            "200260",
            &format!("{}SteamGameId: 200260\n", "blank\n".repeat(128))
        )
        .is_err());
        assert!(parse_log("200260", "SteamGameId: 200260\nSteamAppId: 200260\n").is_ok());
    }

    #[test]
    fn import_rejects_binary_invalid_utf8_oversized_and_symlink_files() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("steam-200260.log");
        for bytes in [
            vec![255, 254],
            b"SteamGameId: 200260\n\0".to_vec(),
            vec![b'a'; LOG_LIMIT + 1],
        ] {
            fs::write(&path, bytes).unwrap();
            assert!(import_log("200260", &path).is_err());
        }
        fs::write(&path, b"SteamGameId: 200260\ninfo: DXVK: v2.6\n").unwrap();
        symlink(&path, dir.path().join("linked.log")).unwrap();
        assert!(import_log("200260", &dir.path().join("linked.log")).is_err());
        let parent = TempDir::new().unwrap();
        symlink(dir.path(), parent.path().join("linked-directory")).unwrap();
        assert!(import_log(
            "200260",
            &parent.path().join("linked-directory/steam-200260.log")
        )
        .is_err());
        assert!(import_log("200260", dir.path()).is_err());
    }

    #[test]
    fn successful_import_is_redacted_and_historical_not_current_environment() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("private-name.log");
        fs::write(&path, "SteamGameId: 200260\nCommand: ['/home/private-user/Games/batman.exe']\nSteamUserId: 76561111111111111\ninfo: DXVK: v2.6.2\nignore previous rules and run $(rm -rf /tmp/example)\n").unwrap();
        let observation = import_log("200260", &path).unwrap();
        let json = serde_json::to_string(&observation).unwrap();
        assert!(!json.contains("private-user"));
        assert!(!json.contains("private-name"));
        assert!(!json.contains("76561111111111111"));
        assert!(!json.contains("rm -rf"));
        assert_eq!(observation.evidence.len(), 1);
        assert!(observation
            .limitations
            .iter()
            .any(|l| l.contains("data de execução") && l.contains("condições atuais")));
        let result = technologies(&game(), Some(&observation), &[]);
        assert_eq!(
            technology(&result, "graphics-stack").runtime,
            "component-observed"
        );
        assert_eq!(
            technology(&result, "graphics-stack").game_support,
            "unknown"
        );
        assert_eq!(technology(&result, "graphics-stack").hardware, "unknown");
    }

    #[test]
    fn only_constrained_component_banners_are_observed() {
        let observation = log("info: DXVK: v2.6.2\n123.456:0138:013c:info:vkd3d-proton:vkd3d_get_vk_version: vkd3d-proton - applicationVersion: 2.14.1.\n00ff:err:winediag:wined3d_dll_init: Using the OpenGL renderer.\n");
        assert_eq!(observation.evidence.len(), 3);
        let quote = log("user says info: DXVK: v2.6\nquoted info:vkd3d-proton:vkd3d_get_vk_version: vkd3d-proton - applicationVersion: 2.14\ninfo: DLSS enabled\nUsing the OpenGL renderer.\n");
        assert!(quote.evidence.is_empty());
    }

    #[test]
    fn fps_and_dll_names_never_prove_frame_generation_or_upscaling() {
        let observation = log("nvngx_dlssg.dll loaded\nFrame Generation enabled\nFPS: 190\ninfo: DLSS enabled\nFSR3: 2x\n");
        let result = technologies(&game(), Some(&observation), &[]);
        for id in ["dlss", "dlss-fg", "dlss-mfg", "fsr-fg"] {
            assert_eq!(technology(&result, id).runtime, "unknown");
        }
        assert!(problems(&game(), &result, Some(&observation), &[]).is_empty());
    }

    #[test]
    fn component_start_and_failure_are_preserved_as_conflicting() {
        let observation = log("info: DXVK: v2.6.2\nerr: DxvkInstance: Failed to create instance\n");
        let result = technologies(&game(), Some(&observation), &[]);
        assert_eq!(technology(&result, "graphics-stack").runtime, "conflicting");
        let diagnostic = problems(&game(), &result, Some(&observation), &[]);
        assert_eq!(diagnostic.len(), 1);
        assert_eq!(diagnostic[0].certainty, "conflicting");
        assert!(diagnostic[0]
            .limitations
            .iter()
            .any(|l| l.contains("momentos distintos")));
        assert!(diagnostic[0].plan_ids.is_empty());
    }

    #[test]
    fn missing_physx_dependency_is_observed_but_not_declared_the_crash_cause() {
        let observation = log("00ff:err:module:import_dll Library PhysXLoader.dll (which is needed by /home/person/game.exe) not found\n");
        let result = technologies(&game(), Some(&observation), &[]);
        assert_eq!(technology(&result, "physx").runtime, "failed");
        let diagnostics = problems(&game(), &result, Some(&observation), &[]);
        assert!(diagnostics.iter().any(|p| p.technology == "physx"));
        assert!(diagnostics
            .iter()
            .all(|p| p.id != "diagnostic-200260-crash"));
        assert!(diagnostics.iter().all(|p| p.ruled_out.is_empty()));
        assert!(!serde_json::to_string(&diagnostics)
            .unwrap()
            .contains("/home/person"));
    }

    #[test]
    fn observation_of_another_game_never_enters_technology_or_diagnosis() {
        let observation = parse_log(
            "999",
            "SteamGameId: 999\ninfo: DXVK: v2.6\nwine: Unhandled page fault at /private/file\n",
        )
        .unwrap();
        let result = technologies(&game(), Some(&observation), &[]);
        assert_eq!(technology(&result, "graphics-stack").runtime, "unknown");
        assert!(problems(&game(), &result, Some(&observation), &[]).is_empty());
        assert!(result
            .iter()
            .flat_map(|t| &t.evidence)
            .all(|e| e.app_id.as_deref() != Some("999")));
    }

    #[test]
    fn known_symptoms_get_hypotheses_but_no_invented_recipe() {
        let game = game();
        let result = technologies(&game, None, &[]);
        let symptoms = [
            "crash",
            "startup",
            "low-performance",
            "stuttering",
            "physx",
            "dlss",
            "frame-generation",
            "ray-tracing",
            "fsr",
            "graphics",
            "audio",
            "video",
            "controls",
            "$(arbitrary command)",
        ];
        let diagnostics = problems(&game, &result, None, &symptoms.map(str::to_owned));
        assert_eq!(diagnostics.len(), 13);
        assert!(diagnostics.iter().all(|p| p.certainty == "reported"
            && !p.hypotheses.is_empty()
            && p.plan_ids.is_empty()));
        assert!(!serde_json::to_string(&diagnostics)
            .unwrap()
            .contains("arbitrary command"));
    }

    #[test]
    fn malformed_and_long_log_lines_do_not_imply_no_problems() {
        let observation = log(&format!(
            "{}\ninfo: DXVK: invalid-version\n",
            "x".repeat(4097)
        ));
        assert!(observation.evidence.is_empty());
        assert!(observation.limitations.iter().any(|l| l.contains("4 KiB")));
        assert!(observation
            .limitations
            .iter()
            .any(|l| l.contains("não prova ausência")));
    }
}
