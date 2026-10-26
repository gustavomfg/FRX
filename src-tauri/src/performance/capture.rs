//! Captura assistida com MangoHud.
//!
//! Estratégia (validada com o MangoHud 0.8.4):
//! 1. O FRX escreve um arquivo de configuração **próprio, por jogo** em
//!    `<app_data>/mangohud/<appid>.conf` (nunca toca no `MangoHud.conf` global):
//!    `output_folder` numa pasta do FRX, `log_interval=0` (um registro por quadro),
//!    `log_duration`, `autostart_log` opcional e `control=frx-<appid>`.
//! 2. O usuário cola nas opções de inicialização da Steam uma linha como
//!    `MANGOHUD_CONFIGFILE=… mangohud %command%`. O FRX **não altera** a Steam;
//!    ele só lê as opções atuais (somente leitura) para sugerir a linha mesclada.
//! 3. O FRX observa a pasta de saída e o socket abstrato de controle do MangoHud
//!    (`:logging=1;` / `:logging=0;`, mensagem `LoggingFinished`).
//!
//! Estados só avançam com evidência: `recording` exige um log com dados
//! crescendo; `processing`, o resumo do MangoHud, a mensagem de fim ou o jogo
//! fechado. Nada é declarado concluído só porque um botão foi clicado.

use std::io::{Read, Seek, SeekFrom, Write};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr, UnixStream};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::models::SessionInput;
use crate::steam::launch_options::{shell_quote, shell_words, ShellWord};
use crate::time::now_iso;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureState {
    Idle,
    CheckingDependencies,
    Ready,
    WaitingForGame,
    Recording,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

impl CaptureState {
    pub fn is_active(self) -> bool {
        matches!(
            self,
            CaptureState::CheckingDependencies
                | CaptureState::WaitingForGame
                | CaptureState::Recording
                | CaptureState::Processing
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum StartMode {
    /// `autostart_log`: começa sozinho alguns segundos depois de o jogo abrir.
    Auto,
    /// O usuário inicia pelo FRX (socket) ou pelo atalho Shift esquerdo + F2.
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRequest {
    pub app_id: String,
    pub game_name: String,
    pub duration_s: u32,
    pub start: StartMode,
    pub start_delay_s: u32,
    pub show_hud: bool,
    #[serde(default)]
    pub input: SessionInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapturePlan {
    pub app_id: String,
    pub game_name: String,
    pub duration_s: u32,
    pub start: StartMode,
    pub start_delay_s: u32,
    pub show_hud: bool,
    pub config_path: String,
    pub output_dir: String,
    pub socket_name: String,
    /// Linha a colar na Steam (Steam nativa).
    pub launch_options: String,
    /// Variante para a Steam Flatpak (sem o script `mangohud`, só a camada Vulkan).
    pub launch_options_flatpak: String,
    /// Opções que o jogo já tinha na Steam (lidas, não alteradas).
    pub existing_launch_options: Option<String>,
    pub flatpak_override: Option<String>,
    pub created_at: String,
    pub input: SessionInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureStatus {
    pub state: CaptureState,
    pub plan: Option<CapturePlan>,
    /// O MangoHud do jogo respondeu no socket do FRX.
    pub hud_attached: bool,
    pub hud_version: Option<String>,
    pub device: Option<String>,
    /// Iniciar/parar pela interface só é possível com o socket conectado.
    pub controllable: bool,
    pub log_file: Option<String>,
    pub frames: u64,
    pub recorded_s: Option<f64>,
    pub message: Option<String>,
    pub session_id: Option<String>,
    pub interrupted: bool,
    pub updated_at: String,
}

impl CaptureStatus {
    pub fn idle() -> Self {
        Self {
            state: CaptureState::Idle,
            plan: None,
            hud_attached: false,
            hud_version: None,
            device: None,
            controllable: false,
            log_file: None,
            frames: 0,
            recorded_s: None,
            message: None,
            session_id: None,
            interrupted: false,
            updated_at: now_iso(),
        }
    }
}

// ---------------------------------------------------------------------------
// Configuração e opções de inicialização
// ---------------------------------------------------------------------------

pub fn socket_name(app_id: &str) -> String {
    format!("frx-{app_id}")
}

/// O FRX inicia a gravação pelo socket (HUD oculto + início automático).
pub fn frx_starts_logging(plan: &CapturePlan) -> bool {
    plan.start == StartMode::Auto && !plan.show_hud
}

/// Conteúdo do arquivo de configuração por jogo. `armed=false` gera a versão
/// "em repouso" (sem gravação automática), usada depois da captura.
pub fn render_config(plan: &CapturePlan, armed: bool) -> String {
    let mut s = String::new();
    s.push_str(
        "# Gerado pelo FRX. Só é usado quando MANGOHUD_CONFIGFILE aponta para este arquivo.\n",
    );
    s.push_str(
        "# Para desativar, remova a linha do FRX das opções de inicialização do jogo na Steam.\n",
    );
    s.push_str(&format!("output_folder={}\n", plan.output_dir));
    s.push_str("log_interval=0\n");
    s.push_str("log_versioning\n");
    s.push_str(&format!("control={}\n", plan.socket_name));
    s.push_str("toggle_logging=Shift_L+F2\n");
    s.push_str(&format!("log_duration={}\n", plan.duration_s));
    // MangoHud 0.8.4: com `no_display`, o autostart_log não dispara (verificado na prática).
    // Com o HUD oculto, quem inicia a gravação é o FRX, pelo socket de controle.
    if armed && plan.start == StartMode::Auto && plan.show_hud {
        s.push_str(&format!("autostart_log={}\n", plan.start_delay_s.max(1)));
    }
    // Sensores precisam estar habilitados para aparecerem no log.
    for p in [
        "fps",
        "frametime",
        "gpu_stats",
        "cpu_stats",
        "cpu_temp",
        "gpu_temp",
        "cpu_power",
        "gpu_power",
        "gpu_core_clock",
        "gpu_mem_clock",
        "cpu_mhz",
        "vram",
        "ram",
    ] {
        s.push_str(p);
        s.push('\n');
    }
    if !(armed && plan.show_hud) {
        s.push_str("no_display\n");
    }
    s
}

/// Mescla a linha do FRX com as opções existentes **sem remover nada**:
/// a variável vai no começo e `mangohud` imediatamente antes de `%command%`.
pub fn merge_launch_options(
    existing: Option<&str>,
    config_path: &str,
    use_wrapper: bool,
) -> Result<String, String> {
    let env = format!("MANGOHUD_CONFIGFILE={}", shell_quote(config_path));
    let existing = existing.map(str::trim).filter(|s| !s.is_empty());
    let Some(ex) = existing else {
        return Ok(if use_wrapper {
            format!("{env} mangohud %command%")
        } else {
            format!("{env} MANGOHUD=1 %command%")
        });
    };
    let words = shell_words(ex)?;
    let commands: Vec<_> = words.iter().filter(|w| w.value == "%command%").collect();
    if commands.len() > 1
        || words
            .iter()
            .any(|w| w.value.contains("%command%") && &ex[w.start..w.end] != "%command%")
    {
        return Err("A linha precisa ter um único %command%, fora de aspas, para ser mesclada com segurança.".into());
    }
    let command = commands.first().copied();
    let end = command.map_or(ex.len(), |w| w.start);
    let has_wrapper = words.iter().any(|w| {
        w.start < end && (&ex[w.start..w.end] == "mangohud" || w.value.ends_with("/mangohud"))
    });
    let mut edits: Vec<(usize, usize, String)> = words
        .iter()
        .filter(|w| {
            w.start < end
                && (w.value.starts_with("MANGOHUD_CONFIGFILE=")
                    || (!use_wrapper && w.value.starts_with("MANGOHUD=")))
        })
        .map(|w| (w.start, w.end, String::new()))
        .collect();
    if let Some(w) = command {
        if use_wrapper && !has_wrapper {
            edits.push((w.start, w.start, "mangohud ".into()));
        }
    } else {
        // Sem placeholder, aceita apenas variáveis de ambiente e argumentos do jogo.
        let assignment = |w: &&ShellWord| {
            w.value.split_once('=').is_some_and(|(k, _)| {
                !k.is_empty()
                    && k.chars().enumerate().all(|(i, c)| {
                        c == '_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit())
                    })
            })
        };
        let first_arg = words.iter().find(|w| !assignment(w));
        if first_arg.is_some_and(|w| !w.value.starts_with('-')) {
            return Err(
                "Inclua %command% na linha atual antes de preparar a captura com wrappers.".into(),
            );
        }
        let at = first_arg.map_or(ex.len(), |w| w.start);
        let prefix = if at > 0 && !ex[..at].ends_with(char::is_whitespace) {
            " "
        } else {
            ""
        };
        let wrapper = if use_wrapper { "mangohud " } else { "" };
        edits.push((at, at, format!("{prefix}{wrapper}%command% ")));
    }
    edits.sort_by_key(|e| std::cmp::Reverse(e.0));
    let mut merged = ex.to_owned();
    for (a, b, replacement) in edits {
        merged.replace_range(a..b, &replacement);
    }
    let layer = if use_wrapper { "" } else { "MANGOHUD=1 " };
    Ok(format!("{env} {layer}{}", merged.trim()))
}

pub fn build_plan(
    req: &CaptureRequest,
    data_dir: &Path,
    existing: Option<String>,
    flatpak: bool,
) -> Result<CapturePlan, String> {
    if !crate::art::valid_app_id(&req.app_id) {
        return Err("AppID inválido".into());
    }
    if !(10..=1800).contains(&req.duration_s) {
        return Err("A duração precisa ficar entre 10 s e 30 min.".into());
    }
    if data_dir.to_string_lossy().chars().any(char::is_control) {
        return Err("A pasta de dados do FRX contém caracteres de controle.".into());
    }
    let config_path = data_dir
        .join("mangohud")
        .join(format!("{}.conf", req.app_id));
    let output_dir = data_dir.join("captures").join(&req.app_id);
    let cp = config_path.to_string_lossy().into_owned();
    let out = output_dir.to_string_lossy().into_owned();
    Ok(CapturePlan {
        app_id: req.app_id.clone(),
        game_name: req
            .game_name
            .chars()
            .filter(|c| !c.is_control())
            .take(200)
            .collect(),
        duration_s: req.duration_s,
        start: req.start,
        start_delay_s: req.start_delay_s.clamp(1, 600),
        show_hud: req.show_hud,
        launch_options: merge_launch_options(existing.as_deref(), &cp, true)?,
        launch_options_flatpak: merge_launch_options(existing.as_deref(), &cp, false)?,
        existing_launch_options: existing,
        flatpak_override: flatpak.then(|| {
            format!(
                "flatpak override --user --filesystem={} com.valvesoftware.Steam",
                shell_quote(&data_dir.to_string_lossy())
            )
        }),
        socket_name: socket_name(&req.app_id),
        config_path: cp,
        output_dir: out,
        created_at: now_iso(),
        input: req.input.clone(),
    })
}

pub fn write_config(plan: &CapturePlan, armed: bool) -> Result<(), String> {
    let path = Path::new(&plan.config_path);
    let content = render_config(plan, armed);
    // O hot reload do MangoHud recria recursos de controle. Não o dispare quando
    // nada mudou (especialmente entre capturas com HUD oculto no mesmo processo).
    if std::fs::read(path).is_ok_and(|old| old == content.as_bytes()) {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&plan.output_dir).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("conf.tmp");
    std::fs::write(&tmp, content).map_err(|e| e.to_string())?;
    std::fs::rename(tmp, path).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Socket de controle do MangoHud
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ControlMessages {
    pub version: Option<String>,
    pub device: Option<String>,
    pub logging_finished: bool,
}

/// Interpreta mensagens `:Chave=Valor;` (e `:LoggingFinished`, sem `;`).
pub fn parse_control(buf: &str, into: &mut ControlMessages) {
    for part in buf.split(':').filter(|s| !s.is_empty()) {
        let part = part.trim_end_matches(';');
        match part.split_once('=') {
            Some(("MangoHudVersion", v)) => into.version = Some(v.chars().take(60).collect()),
            Some(("DeviceName", v)) => into.device = Some(v.chars().take(120).collect()),
            None if part.starts_with("LoggingFinished") => into.logging_finished = true,
            _ => {}
        }
    }
}

pub struct ControlClient {
    stream: UnixStream,
    pending: Vec<u8>,
    pub messages: ControlMessages,
    pub closed: bool,
}

impl ControlClient {
    /// Conecta ao socket abstrato (namespace de rede compartilhado com o jogo).
    pub fn connect(name: &str) -> Option<Self> {
        let addr = SocketAddr::from_abstract_name(name.as_bytes()).ok()?;
        let stream = UnixStream::connect_addr(&addr).ok()?;
        stream.set_nonblocking(true).ok()?;
        Some(Self {
            stream,
            pending: Vec::new(),
            messages: ControlMessages::default(),
            closed: false,
        })
    }

    pub fn poll(&mut self) {
        let mut buf = [0u8; 4096];
        loop {
            match self.stream.read(&mut buf) {
                Ok(0) => {
                    self.closed = true;
                    break;
                }
                Ok(n) => {
                    self.pending.extend_from_slice(&buf[..n]);
                    loop {
                        let end = if self.pending.starts_with(b":LoggingFinished") {
                            Some(b":LoggingFinished".len())
                        } else {
                            self.pending.iter().position(|b| *b == b';').map(|i| i + 1)
                        };
                        let Some(end) = end else { break };
                        parse_control(
                            &String::from_utf8_lossy(&self.pending[..end]),
                            &mut self.messages,
                        );
                        self.pending.drain(..end);
                    }
                    if self.pending.len() > 16 * 1024 {
                        self.closed = true;
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => {
                    self.closed = true;
                    break;
                }
            }
        }
    }

    pub fn set_logging(&mut self, on: bool) -> bool {
        let cmd: &[u8] = if on { b":logging=1;" } else { b":logging=0;" };
        if self.stream.write_all(cmd).is_err() {
            return false;
        }
        if on {
            self.messages.logging_finished = false;
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Observação da pasta de saída
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct LogFile {
    pub path: PathBuf,
    pub size: u64,
    pub has_summary: bool,
}

/// Logs (não-resumo) criados depois de `since`, do mais novo para o mais antigo.
pub fn scan_logs(dir: &Path, ignore: &[PathBuf]) -> Vec<LogFile> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let mut out: Vec<(std::time::SystemTime, LogFile)> = rd
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if !e.file_type().ok()?.is_file() {
                return None;
            }
            let name = p.file_name()?.to_string_lossy().into_owned();
            if !name.ends_with(".csv") || name.ends_with("_summary.csv") || ignore.contains(&p) {
                return None;
            }
            let meta = e.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            let summary =
                p.with_file_name(format!("{}_summary.csv", name.trim_end_matches(".csv")));
            Some((
                meta.modified().ok()?,
                LogFile {
                    size: meta.len(),
                    has_summary: summary.is_file(),
                    path: p,
                },
            ))
        })
        .collect();
    out.sort_by_key(|x| std::cmp::Reverse(x.0));
    out.into_iter().map(|(_, f)| f).collect()
}

/// Lê só o trecho novo do arquivo para contar linhas e achar o último `elapsed`.
#[derive(Debug, Default)]
pub struct Tail {
    offset: u64,
    partial: String,
    /// Linhas de dados (primeiro campo numérico), sem cabeçalhos.
    pub data_rows: u64,
    first_elapsed_s: Option<f64>,
    last_elapsed_s: Option<f64>,
}

impl Tail {
    pub fn update(&mut self, path: &Path) -> u64 {
        let Ok(mut f) = std::fs::File::open(path) else {
            return self.data_rows;
        };
        if f.metadata().is_ok_and(|m| m.len() < self.offset) {
            *self = Self::default();
        }
        if f.seek(SeekFrom::Start(self.offset)).is_err() {
            return self.data_rows;
        }
        let mut buf = Vec::new();
        let _ = Read::by_ref(&mut f)
            .take(8 * 1024 * 1024)
            .read_to_end(&mut buf);
        self.offset += buf.len() as u64;
        self.partial.push_str(&String::from_utf8_lossy(&buf));
        let end = self.partial.rfind('\n').map_or(0, |i| i + 1);
        let remaining = self.partial.split_off(end);
        let complete = std::mem::replace(&mut self.partial, remaining);
        for line in complete.lines() {
            let fields: Vec<&str> = line.trim().split(',').collect();
            if fields.len() >= 3 && fields[0].trim().parse::<f64>().is_ok() {
                self.data_rows += 1;
                if let Ok(ns) = fields[fields.len() - 1].trim().parse::<f64>() {
                    let s = ns / 1e9;
                    self.first_elapsed_s.get_or_insert(s);
                    self.last_elapsed_s = Some(s);
                }
            }
        }
        if self.partial.len() > 8192 {
            self.partial.clear();
        }
        self.data_rows
    }

    /// Tempo gravado até agora, pela coluna `elapsed`.
    pub fn recorded_s(&self) -> Option<f64> {
        Some(self.last_elapsed_s? - self.first_elapsed_s? + 0.0)
    }
}

// ---------------------------------------------------------------------------
// Máquina de estados (pura, testável)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum SocketObs {
    /// Nenhum jogo com a configuração do FRX está escutando.
    Absent,
    Attached {
        version: Option<String>,
        device: Option<String>,
        logging_finished: bool,
    },
    /// Estava conectado e o jogo fechou.
    Closed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Observation {
    pub socket: SocketObs,
    pub log: Option<LogFile>,
    /// Linhas de dados no log atual (sem cabeçalhos).
    pub data_rows: u64,
    pub recorded_s: Option<f64>,
    /// Segundos sem crescimento do log.
    pub idle_s: f64,
    /// Tempo desde que o FRX começou a aguardar o jogo.
    pub waiting_s: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    None,
    Process { path: PathBuf, interrupted: bool },
    Fail(String),
}

/// Quanto tempo esperar o resumo depois de "LoggingFinished" ou do jogo fechar.
const SETTLE_S: f64 = 3.0;
/// Sem socket (ex.: contêiner sem rede compartilhada): log parado por tanto tempo = fim.
const STALL_S: f64 = 15.0;

pub fn step(status: &mut CaptureStatus, obs: &Observation) -> Action {
    match &obs.socket {
        SocketObs::Attached {
            version, device, ..
        } => {
            status.hud_attached = true;
            status.controllable = true;
            status.hud_version = version.clone().or(status.hud_version.take());
            status.device = device.clone().or(status.device.take());
        }
        SocketObs::Closed => status.controllable = false,
        SocketObs::Absent => {}
    }
    status.frames = obs.data_rows;
    status.recorded_s = obs.recorded_s;
    status.log_file = obs
        .log
        .as_ref()
        .map(|l| l.path.to_string_lossy().into_owned());
    status.updated_at = now_iso();
    if obs
        .log
        .as_ref()
        .is_some_and(|l| l.size > super::parser::MAX_BYTES as u64)
    {
        status.state = CaptureState::Failed;
        status.message =
            Some("O log ultrapassou o limite de 256 MB. Use uma captura mais curta.".into());
        return Action::Fail(status.message.clone().unwrap_or_default());
    }

    match status.state {
        CaptureState::WaitingForGame => {
            if obs.waiting_s >= 900.0 {
                status.state = CaptureState::Failed;
                status.message = Some("Nenhuma gravação começou em 15 minutos. Confira as opções de inicialização e prepare uma nova sessão.".into());
                return Action::Fail(status.message.clone().unwrap_or_default());
            } else if obs.log.is_some() && obs.data_rows >= 2 {
                status.state = CaptureState::Recording;
                status.message = Some("Gravando: o MangoHud está registrando cada quadro.".into());
            } else if obs.socket == SocketObs::Closed {
                status.state = CaptureState::Failed;
                status.message = Some(
                    "O jogo foi fechado antes de a gravação começar. Nenhum dado foi registrado."
                        .into(),
                );
                return Action::Fail(status.message.clone().unwrap_or_default());
            } else if status.hud_attached {
                status.message = Some(match status.plan.as_ref().map(|p| (p.start, p.start_delay_s)) {
                    Some((StartMode::Auto, d)) => format!("MangoHud ativo no jogo. A gravação começa sozinha em ~{d} s."),
                    _ => "MangoHud ativo no jogo. Inicie a gravação pelo FRX ou com Shift esquerdo + F2.".into(),
                });
            }
            Action::None
        }
        CaptureState::Recording => {
            let Some(log) = &obs.log else {
                return Action::None;
            };
            let finished = matches!(
                obs.socket,
                SocketObs::Attached {
                    logging_finished: true,
                    ..
                }
            );
            if log.has_summary && obs.idle_s >= 0.5 {
                status.state = CaptureState::Processing;
                status.message = Some("Captura concluída pelo MangoHud. Analisando…".into());
                return Action::Process {
                    path: log.path.clone(),
                    interrupted: false,
                };
            }
            if (finished || obs.socket == SocketObs::Closed) && obs.idle_s >= SETTLE_S {
                let interrupted = obs.socket == SocketObs::Closed && !finished;
                status.state = CaptureState::Processing;
                status.interrupted = interrupted;
                status.message = Some(if interrupted {
                    "O jogo fechou durante a gravação. Analisando o que foi registrado…".into()
                } else {
                    "Gravação encerrada. Analisando…".into()
                });
                return Action::Process {
                    path: log.path.clone(),
                    interrupted,
                };
            }
            if (!status.hud_attached && obs.idle_s >= STALL_S) || obs.idle_s >= 120.0 {
                status.state = CaptureState::Processing;
                status.interrupted = true;
                status.message =
                    Some("O log parou de crescer. Analisando o que foi registrado…".into());
                return Action::Process {
                    path: log.path.clone(),
                    interrupted: true,
                };
            }
            Action::None
        }
        _ => Action::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn plan(start: StartMode) -> CapturePlan {
        let req = CaptureRequest {
            app_id: "200260".into(),
            game_name: "Batman".into(),
            duration_s: 60,
            start,
            start_delay_s: 10,
            show_hud: false,
            input: SessionInput::default(),
        };
        build_plan(
            &req,
            Path::new("/home/u/.local/share/io.frx.FRX"),
            None,
            false,
        )
        .unwrap()
    }

    fn status(state: CaptureState) -> CaptureStatus {
        let mut s = CaptureStatus::idle();
        s.state = state;
        s.plan = Some(plan(StartMode::Auto));
        s
    }

    fn log(summary: bool) -> Option<LogFile> {
        Some(LogFile {
            path: "/x/BatmanAC_2026-10-24_18-01-29.csv".into(),
            size: 100,
            has_summary: summary,
        })
    }

    fn obs(socket: SocketObs, log: Option<LogFile>, rows: u64, idle: f64) -> Observation {
        Observation {
            socket,
            log,
            data_rows: rows,
            recorded_s: None,
            idle_s: idle,
            waiting_s: 0.0,
        }
    }

    fn attached(done: bool) -> SocketObs {
        SocketObs::Attached {
            version: Some("MangoHud 0.8.4-1+".into()),
            device: Some("RTX 4060".into()),
            logging_finished: done,
        }
    }

    #[test]
    fn config_is_per_game_and_never_global() {
        let p = plan(StartMode::Auto);
        assert!(p.config_path.ends_with("io.frx.FRX/mangohud/200260.conf"));
        let armed = render_config(&p, true);
        assert!(armed.contains("log_interval=0"));
        assert!(
            !armed.contains("autostart_log"),
            "HUD oculto: quem inicia é o FRX"
        );
        assert!(frx_starts_logging(&p));
        let mut visible = p.clone();
        visible.show_hud = true;
        assert!(render_config(&visible, true).contains("autostart_log=10"));
        assert!(!render_config(&visible, true).contains("no_display"));
        assert!(armed.contains("control=frx-200260"));
        assert!(armed.contains("log_duration=60"));
        assert!(armed.contains("no_display"));
        let resting = render_config(&p, false);
        assert!(
            !resting.contains("autostart_log"),
            "em repouso não grava sozinho"
        );
        assert_eq!(p.launch_options, "MANGOHUD_CONFIGFILE=/home/u/.local/share/io.frx.FRX/mangohud/200260.conf mangohud %command%");
        assert!(p.launch_options_flatpak.contains("MANGOHUD=1 %command%"));
    }

    #[test]
    fn merges_existing_launch_options_without_removing_anything() {
        fn merge_launch_options(ex: Option<&str>, path: &str, wrapper: bool) -> String {
            super::merge_launch_options(ex, path, wrapper).unwrap()
        }
        let c = "/d/x.conf";
        assert_eq!(
            merge_launch_options(Some("PROTON_LOG=1 %command% -dx11"), c, true),
            "MANGOHUD_CONFIGFILE=/d/x.conf PROTON_LOG=1 mangohud %command% -dx11"
        );
        assert_eq!(
            merge_launch_options(Some("-dx11 -skipintro"), c, true),
            "MANGOHUD_CONFIGFILE=/d/x.conf mangohud %command% -dx11 -skipintro"
        );
        assert_eq!(
            merge_launch_options(Some("gamescope -f -- %command%"), c, true),
            "MANGOHUD_CONFIGFILE=/d/x.conf gamescope -f -- mangohud %command%"
        );
        assert_eq!(
            merge_launch_options(Some("mangohud %command%"), c, true),
            "MANGOHUD_CONFIGFILE=/d/x.conf mangohud %command%",
            "não duplica o mangohud"
        );
        assert_eq!(
            merge_launch_options(
                Some("MANGOHUD_CONFIGFILE=/old.conf mangohud %command%"),
                c,
                true
            ),
            "MANGOHUD_CONFIGFILE=/d/x.conf mangohud %command%",
            "atualiza a linha antiga do FRX"
        );
        assert_eq!(
            merge_launch_options(Some("%command%"), "/a b/x.conf", false),
            "MANGOHUD_CONFIGFILE='/a b/x.conf' MANGOHUD=1 %command%"
        );
    }

    #[test]
    fn rejects_bad_requests() {
        let mut req = CaptureRequest {
            app_id: "../1".into(),
            game_name: "x".into(),
            duration_s: 60,
            start: StartMode::Manual,
            start_delay_s: 5,
            show_hud: true,
            input: SessionInput::default(),
        };
        assert!(build_plan(&req, Path::new("/d"), None, false).is_err());
        req.app_id = "1".into();
        req.duration_s = 5;
        assert!(build_plan(&req, Path::new("/d"), None, false).is_err());
    }

    #[test]
    fn launch_options_preserve_quoted_arguments_and_replace_the_entire_old_path() {
        let ex = r#"PROTON_LOG=1 MANGOHUD_CONFIGFILE="/old path/game.conf" gamescope --title "Two   spaces" -f -- %command% --name 'A B'"#;
        let merged = merge_launch_options(Some(ex), "/new path/it's $safe.conf", true).unwrap();
        assert!(merged.starts_with("MANGOHUD_CONFIGFILE='/new path/it'\\''s $safe.conf'"));
        assert!(!merged.contains("/old path"));
        assert!(merged
            .contains(r#"gamescope --title "Two   spaces" -f -- mangohud %command% --name 'A B'"#));
        assert_eq!(
            merge_launch_options(Some("PROTON_LOG=1"), "/d/x.conf", true).unwrap(),
            "MANGOHUD_CONFIGFILE=/d/x.conf PROTON_LOG=1 mangohud %command%"
        );
    }

    #[test]
    fn unchanged_config_keeps_its_inode_between_hidden_captures() {
        use std::os::unix::fs::MetadataExt;
        let tmp = tempfile::tempdir().unwrap();
        let mut p = plan(StartMode::Auto);
        p.config_path = tmp.path().join("x.conf").to_string_lossy().into_owned();
        p.output_dir = tmp.path().join("logs").to_string_lossy().into_owned();
        write_config(&p, true).unwrap();
        let ino = std::fs::metadata(&p.config_path).unwrap().ino();
        write_config(&p, false).unwrap();
        write_config(&p, true).unwrap();
        assert_eq!(std::fs::metadata(&p.config_path).unwrap().ino(), ino);
    }

    #[test]
    fn launch_options_reject_ambiguous_shell_without_executing_it() {
        for ex in [
            "gamescope -f",
            "echo x; %command%",
            "env A=$(whoami) %command%",
            "sh -c '%command%'",
            "\"unfinished",
            "%command% | tee /tmp/log",
            "%command% %command%",
        ] {
            assert!(
                merge_launch_options(Some(ex), "/d/x.conf", true).is_err(),
                "{ex}"
            );
        }
    }

    #[test]
    fn capture_waits_for_a_finished_file_and_has_timeouts() {
        let mut s = status(CaptureState::Recording);
        assert_eq!(
            step(&mut s, &obs(attached(true), log(true), 600, 0.0)),
            Action::None
        );
        assert!(matches!(
            step(&mut s, &obs(attached(false), log(false), 600, 121.0)),
            Action::Process {
                interrupted: true,
                ..
            }
        ));
        let mut waiting = status(CaptureState::WaitingForGame);
        let mut o = obs(SocketObs::Absent, None, 0, 0.0);
        o.waiting_s = 901.0;
        assert!(matches!(step(&mut waiting, &o), Action::Fail(_)));
    }

    #[test]
    fn tail_handles_partial_lines_truncation_and_symlink_logs() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("x.csv");
        std::fs::write(&path, "fps,frametime,elapsed\n100,10,").unwrap();
        let mut t = Tail::default();
        assert_eq!(t.update(&path), 0);
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"10000000\n")
            .unwrap();
        assert_eq!(t.update(&path), 1);
        std::fs::write(&path, "fps,frametime,elapsed\n").unwrap();
        assert_eq!(t.update(&path), 0);
        let link = tmp.path().join("linked.csv");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert_eq!(scan_logs(tmp.path(), &[]).len(), 1);
    }

    #[test]
    fn socket_reassembles_fragmented_messages_and_resets_completion() {
        use std::os::unix::net::UnixStream;
        let (client, mut server) = UnixStream::pair().unwrap();
        client.set_nonblocking(true).unwrap();
        let mut c = ControlClient {
            stream: client,
            pending: vec![],
            messages: ControlMessages::default(),
            closed: false,
        };
        server.write_all(b":MangoHudVer").unwrap();
        c.poll();
        assert!(c.messages.version.is_none());
        server.write_all(b"sion=0.8.4;:LoggingFin").unwrap();
        c.poll();
        assert_eq!(c.messages.version.as_deref(), Some("0.8.4"));
        assert!(!c.messages.logging_finished);
        server.write_all(b"ished").unwrap();
        c.poll();
        assert!(c.messages.logging_finished);
        assert!(c.set_logging(true));
        assert!(!c.messages.logging_finished);
    }

    #[test]
    fn control_messages_are_parsed() {
        let mut m = ControlMessages::default();
        parse_control(":MangoHudControlVersion=1;:DeviceName=NVIDIA GeForce RTX 4060;:MangoHudVersion=MangoHud 0.8.4-1+;", &mut m);
        assert_eq!(m.device.as_deref(), Some("NVIDIA GeForce RTX 4060"));
        assert_eq!(m.version.as_deref(), Some("MangoHud 0.8.4-1+"));
        assert!(!m.logging_finished);
        parse_control(":LoggingFinished", &mut m);
        assert!(m.logging_finished);
    }

    #[test]
    fn recording_requires_real_data() {
        let mut s = status(CaptureState::WaitingForGame);
        assert_eq!(
            step(&mut s, &obs(attached(false), None, 0, 0.0)),
            Action::None
        );
        assert_eq!(
            s.state,
            CaptureState::WaitingForGame,
            "HUD conectado ainda não é gravação"
        );
        assert!(s.hud_attached && s.controllable);
        assert_eq!(
            step(&mut s, &obs(attached(false), log(false), 1, 0.0)),
            Action::None
        );
        assert_eq!(
            s.state,
            CaptureState::WaitingForGame,
            "só o cabeçalho não conta"
        );
        step(&mut s, &obs(attached(false), log(false), 50, 0.0));
        assert_eq!(s.state, CaptureState::Recording);
    }

    #[test]
    fn completes_only_with_evidence() {
        let mut s = status(CaptureState::Recording);
        assert_eq!(
            step(&mut s, &obs(attached(false), log(false), 500, 1.0)),
            Action::None
        );
        let a = step(&mut s, &obs(attached(true), log(true), 600, 0.5));
        assert!(matches!(
            a,
            Action::Process {
                interrupted: false,
                ..
            }
        ));
        assert_eq!(s.state, CaptureState::Processing);
    }

    #[test]
    fn game_closed_mid_recording_is_processed_as_interrupted() {
        let mut s = status(CaptureState::Recording);
        s.hud_attached = true;
        assert_eq!(
            step(&mut s, &obs(SocketObs::Closed, log(false), 400, 1.0)),
            Action::None,
            "espera o arquivo assentar"
        );
        let a = step(&mut s, &obs(SocketObs::Closed, log(false), 400, 3.5));
        assert!(matches!(
            a,
            Action::Process {
                interrupted: true,
                ..
            }
        ));
        assert!(s.interrupted);
    }

    #[test]
    fn game_closed_before_recording_fails() {
        let mut s = status(CaptureState::WaitingForGame);
        step(&mut s, &obs(attached(false), None, 0, 0.0));
        let a = step(&mut s, &obs(SocketObs::Closed, None, 0, 0.0));
        assert!(matches!(a, Action::Fail(_)));
        assert_eq!(s.state, CaptureState::Failed);
    }

    #[test]
    fn without_socket_a_stalled_log_ends_the_capture() {
        let mut s = status(CaptureState::Recording);
        assert_eq!(
            step(&mut s, &obs(SocketObs::Absent, log(false), 300, 10.0)),
            Action::None
        );
        assert!(matches!(
            step(&mut s, &obs(SocketObs::Absent, log(false), 300, 16.0)),
            Action::Process {
                interrupted: true,
                ..
            }
        ));
    }

    #[test]
    fn scan_logs_ignores_summaries_and_old_files_and_tail_counts_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let old = tmp.path().join("old_2026-01-01_00-00-00.csv");
        std::fs::write(&old, "x\n").unwrap();
        let cur = tmp.path().join("game_2026-10-24_18-01-29.csv");
        std::fs::write(&cur, "fps,frametime,elapsed\n100,10,10000000\n").unwrap();
        std::fs::write(tmp.path().join("game_2026-10-24_18-01-29_summary.csv"), "x").unwrap();
        let logs = scan_logs(tmp.path(), &[old]);
        assert_eq!(logs.len(), 1);
        assert!(logs[0].has_summary);
        let mut t = Tail::default();
        assert_eq!(t.update(&cur), 1, "o cabeçalho não conta como dado");
        std::fs::OpenOptions::new()
            .append(true)
            .open(&cur)
            .unwrap()
            .write_all(b"100,10,2010000000\n")
            .unwrap();
        assert_eq!(t.update(&cur), 2);
        assert!((t.recorded_s().unwrap() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn control_socket_roundtrip_with_a_fake_mangohud() {
        use std::os::unix::net::UnixListener;
        let name = format!("frx-test-{}", std::process::id());
        let addr = SocketAddr::from_abstract_name(name.as_bytes()).unwrap();
        let listener = UnixListener::bind_addr(&addr).unwrap();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            s.write_all(
                b":MangoHudControlVersion=1;:DeviceName=GPU;:MangoHudVersion=MangoHud 0.8.4;",
            )
            .unwrap();
            let mut buf = [0u8; 32];
            let n = s.read(&mut buf).unwrap();
            assert_eq!(&buf[..n], b":logging=1;");
            s.write_all(b":LoggingFinished").unwrap();
        });
        let mut c = ControlClient::connect(&name).expect("conecta ao socket abstrato");
        assert!(c.set_logging(true));
        server.join().unwrap();
        std::thread::sleep(Duration::from_millis(50));
        c.poll();
        assert_eq!(c.messages.version.as_deref(), Some("MangoHud 0.8.4"));
        assert!(c.messages.logging_finished);
        assert!(ControlClient::connect("frx-nao-existe-123").is_none());
        let _ = Instant::now();
    }
}
