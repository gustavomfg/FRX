//! Comandos do Performance Lab.
//!
//! Importação e processamento rodam fora da thread principal; o progresso da
//! captura assistida chega ao front-end pelo evento `frx://capture`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use super::AppCtx;
use crate::performance::capture::{
    self, Action, CaptureRequest, CaptureState, CaptureStatus, ControlClient, Observation,
    SocketObs, Tail,
};
use crate::performance::comparison::{self, ComparisonResult};
use crate::performance::detection::{self, MangoHudStatus};
use crate::performance::models::*;
use crate::performance::persistence::{SessionPatch, Store};
use crate::performance::{self, ProcessArgs};
use crate::steam::models::{InstallKind, ScanResult};

pub const CAPTURE_EVENT: &str = "frx://capture";
pub const IMPORT_EVENT: &str = "frx://import";

/// Estado da captura assistida (uma por vez).
#[derive(Default)]
pub struct CaptureManager {
    /// Serializa transições e a gravação final; cancelar nunca deixa uma sessão oculta salva.
    transition: Mutex<()>,
    status: Mutex<Option<CaptureStatus>>,
    control: Mutex<Option<ControlClient>>,
    /// Incrementado a cada nova captura/cancelamento: encerra monitores antigos.
    generation: AtomicU64,
    /// Pedido do usuário para encerrar agora e analisar o que já foi gravado.
    finish_now: AtomicU64,
}

impl CaptureManager {
    fn stop_control(&self) {
        if let Ok(mut g) = self.control.lock() {
            if let Some(c) = g.as_mut() {
                let _ = c.set_logging(false);
            }
            *g = None;
        }
    }

    /// Fecha os recursos do FRX sem encerrar o jogo nem alterar a Steam.
    pub fn shutdown(&self) {
        if let Ok(_guard) = self.transition.lock() {
            self.generation.fetch_add(1, Ordering::SeqCst);
            self.stop_control();
            if let Some(plan) = self.status().plan {
                let _ = capture::write_config(&plan, false);
            }
        }
    }

    pub fn status(&self) -> CaptureStatus {
        self.status
            .lock()
            .ok()
            .and_then(|s| s.clone())
            .unwrap_or_else(CaptureStatus::idle)
    }

    fn set(&self, app: &AppHandle, s: CaptureStatus) {
        if let Ok(mut g) = self.status.lock() {
            *g = Some(s.clone());
        }
        let _ = app.emit(CAPTURE_EVENT, s);
    }
}

pub struct PerfCtx {
    pub store: Store,
    pub data_dir: PathBuf,
    pub capture: Arc<CaptureManager>,
}

fn library(ctx: &AppCtx) -> Option<ScanResult> {
    ctx.cache
        .get::<ScanResult>("library", "steam")
        .map(|e| e.data)
}

/// O que o FRX sabe agora sobre o sistema e o Proton do jogo.
fn known_conditions(ctx: &AppCtx, app_id: Option<&str>) -> Conditions {
    let profile = crate::system::profile();
    let gpu = profile.primary_gpu.clone();
    let proton = app_id.and_then(|id| {
        let lib = library(ctx)?;
        let game = lib.games.into_iter().find(|g| g.app_id == id)?;
        let tool = game.compatibility.tool?;
        Some(match tool.version {
            Some(v) if !tool.display_name.contains(&v) => format!(
                "{} ({v})",
                crate::fixes::short_tool_name(&tool.display_name)
            ),
            _ => crate::fixes::short_tool_name(&tool.display_name),
        })
    });
    Conditions {
        proton,
        gpu: gpu.as_ref().and_then(|g| g.name.clone()),
        driver: gpu
            .as_ref()
            .and_then(|g| match (&g.driver, &g.driver_version) {
                (Some(d), Some(v)) => Some(format!("{d} {v}")),
                (Some(d), None) => Some(d.clone()),
                _ => None,
            }),
        kernel: profile.kernel,
        os: profile.distro,
        cpu: profile.cpu,
        ..Default::default()
    }
}

fn steam_flatpak(ctx: &AppCtx) -> bool {
    library(ctx)
        .map(|l| {
            l.steam_installs
                .iter()
                .any(|i| i.kind == InstallKind::Flatpak)
        })
        .unwrap_or(false)
}

#[tauri::command]
pub async fn detect_mangohud(ctx: State<'_, AppCtx>) -> Result<MangoHudStatus, String> {
    let flatpak = steam_flatpak(&ctx);
    tauri::async_runtime::spawn_blocking(move || {
        detection::detect(&detection::Env::from_system(flatpak))
    })
    .await
    .map_err(|e| e.to_string())
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ImportProgress {
    stage: &'static str,
}

/// Importa um log escolhido pelo usuário (Modo A). Funciona sem o jogo e sem o FRX controlar nada.
#[tauri::command]
pub async fn import_benchmark(
    app: AppHandle,
    ctx: State<'_, AppCtx>,
    path: String,
    input: SessionInput,
) -> Result<SessionMeta, String> {
    let _ = app.emit(IMPORT_EVENT, ImportProgress { stage: "reading" });
    let known = known_conditions(&ctx, input.app_id.as_deref());
    let store = ctx.perf.store.clone();
    let app2 = app.clone();
    let meta = tauri::async_runtime::spawn_blocking(move || {
        let raw = performance::read_log_file(Path::new(&path))?;
        if let Some(existing) = store.find_by_hash(&performance::persistence::content_hash(&raw)) {
            return Err(format!(
                "Este log já foi importado como “{}”.",
                existing.label
            ));
        }
        let _ = app2.emit(IMPORT_EVENT, ImportProgress { stage: "analyzing" });
        let session = performance::process(
            &raw,
            ProcessArgs {
                input,
                source: Some(SessionSource::Import),
                original_path: Some(path),
                known,
            },
        )?;
        let _ = app2.emit(IMPORT_EVENT, ImportProgress { stage: "saving" });
        store.save(&session, &raw)?;
        Ok::<_, String>(session.meta)
    })
    .await
    .map_err(|e| e.to_string())??;
    let _ = app.emit(IMPORT_EVENT, ImportProgress { stage: "done" });
    Ok(meta)
}

#[tauri::command]
pub async fn list_benchmark_sessions(
    ctx: State<'_, AppCtx>,
    app_id: Option<String>,
) -> Result<Vec<SessionMeta>, String> {
    let store = ctx.perf.store.clone();
    let list = tauri::async_runtime::spawn_blocking(move || store.list())
        .await
        .map_err(|e| e.to_string())?;
    Ok(list
        .into_iter()
        .filter(|m| {
            app_id
                .as_deref()
                .is_none_or(|a| m.app_id.as_deref() == Some(a))
        })
        .collect())
}

#[tauri::command]
pub async fn get_benchmark_session(ctx: State<'_, AppCtx>, id: String) -> Result<Session, String> {
    let store = ctx.perf.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        store
            .get(&id)
            .ok_or_else(|| "Sessão não encontrada.".to_owned())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn compare_benchmark_sessions(
    ctx: State<'_, AppCtx>,
    before_id: String,
    after_id: String,
) -> Result<ComparisonResult, String> {
    if before_id == after_id {
        return Err("Escolha duas sessões diferentes.".into());
    }
    let store = ctx.perf.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let b = store
            .get(&before_id)
            .ok_or("Sessão “antes” não encontrada.")?;
        let a = store
            .get(&after_id)
            .ok_or("Sessão “depois” não encontrada.")?;
        Ok(comparison::compare(&b.meta, &a.meta))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn update_benchmark_session(
    ctx: State<'_, AppCtx>,
    id: String,
    patch: SessionPatch,
) -> Result<SessionMeta, String> {
    ctx.perf.store.update(&id, patch)
}

#[tauri::command]
pub fn delete_benchmark_session(ctx: State<'_, AppCtx>, id: String) -> Result<(), String> {
    ctx.perf.store.delete(&id)
}

/// Exporta para um caminho escolhido no diálogo "Salvar" do sistema.
#[tauri::command]
pub fn export_benchmark_session(
    ctx: State<'_, AppCtx>,
    id: String,
    path: String,
) -> Result<(), String> {
    let s = ctx.perf.store.get(&id).ok_or("Sessão não encontrada.")?;
    performance::export(&s, Path::new(&path))
}

// ---------------------------------------------------------------------------
// Captura assistida (Modo B)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_capture_status(ctx: State<'_, AppCtx>) -> CaptureStatus {
    ctx.perf.capture.status()
}

/// Prepara a sessão: verifica o MangoHud, grava o config por jogo e monta as opções de inicialização.
#[tauri::command]
pub async fn prepare_benchmark(
    app: AppHandle,
    ctx: State<'_, AppCtx>,
    request: CaptureRequest,
) -> Result<CaptureStatus, String> {
    let mgr = ctx.perf.capture.clone();
    let mut s = CaptureStatus::idle();
    s.state = CaptureState::CheckingDependencies;
    let generation = {
        let _guard = mgr.transition.lock().map_err(|_| "estado indisponível")?;
        if mgr.status().state.is_active() {
            return Err("Já existe uma captura em andamento.".into());
        }
        mgr.stop_control();
        if let Some(old) = mgr.status().plan {
            capture::write_config(&old, false)?;
        }
        let generation = mgr.generation.fetch_add(1, Ordering::SeqCst) + 1;
        mgr.set(&app, s.clone());
        generation
    };

    let flatpak = steam_flatpak(&ctx);
    let detected = tauri::async_runtime::spawn_blocking(move || {
        detection::detect(&detection::Env::from_system(flatpak))
    })
    .await
    .map_err(|e| e.to_string());
    let _guard = mgr.transition.lock().map_err(|_| "estado indisponível")?;
    if mgr.generation.load(Ordering::SeqCst) != generation {
        return Ok(mgr.status());
    }
    let existing = library(&ctx).and_then(|l| {
        l.steam_installs.iter().find_map(|i| {
            crate::steam::userdata::launch_options(Path::new(&i.path), &request.app_id)
        })
    });
    let prepared = detected.and_then(|mh| {
        if !mh.logging_supported && !(flatpak && !mh.flatpak_layer.is_empty()) {
            return Err("MangoHud não encontrado (ou antigo demais). Instale-o e tente de novo — o FRX não instala nada sozinho.".into());
        }
        let plan = capture::build_plan(&request, &ctx.perf.data_dir, existing, flatpak)?;
        // Só arma ao começar a observar; fechar o assistente aqui não deixa autostart pendente.
        capture::write_config(&plan, false)?;
        Ok(plan)
    });
    match prepared {
        Ok(plan) => {
            s.state = CaptureState::Ready;
            s.message = Some("Configuração pronta. Adicione a linha às opções de inicialização do jogo na Steam.".into());
            s.plan = Some(plan);
        }
        Err(e) => {
            s.state = CaptureState::Failed;
            s.message = Some(e);
        }
    }
    mgr.set(&app, s.clone());
    Ok(s)
}

/// Começa a observar: a partir daqui o FRX espera o jogo abrir com o MangoHud.
#[tauri::command]
pub fn start_capture(app: AppHandle, ctx: State<'_, AppCtx>) -> Result<CaptureStatus, String> {
    let mgr = ctx.perf.capture.clone();
    let guard = mgr.transition.lock().map_err(|_| "estado indisponível")?;
    let mut s = mgr.status();
    if s.state != CaptureState::Ready {
        return Err("Prepare a sessão antes de começar.".into());
    }
    let plan = s.plan.clone().ok_or("sessão sem plano")?;
    // Tira a fotografia da pasta antes de armar a gravação.
    let ignore: Vec<PathBuf> = capture::scan_logs(Path::new(&plan.output_dir), &[])
        .into_iter()
        .map(|l| l.path)
        .collect();
    mgr.stop_control();
    mgr.finish_now.store(0, Ordering::SeqCst);
    capture::write_config(&plan, true)?;
    s.state = CaptureState::WaitingForGame;
    s.message = Some("Aguardando o jogo abrir com o MangoHud…".into());
    mgr.set(&app, s.clone());

    let generation = mgr.generation.fetch_add(1, Ordering::SeqCst) + 1;
    let known = known_conditions(&ctx, Some(&plan.app_id));
    let out = Output {
        store: ctx.perf.store.clone(),
        engine: ctx.engine.clone(),
    };
    drop(guard);
    tauri::async_runtime::spawn(monitor(app, mgr, generation, plan, ignore, out, known));
    Ok(s)
}

/// Onde a sessão termina: o histórico e o Fix Engine (para registrar as correções ativas).
struct Output {
    store: Store,
    engine: Arc<crate::fix_engine::FixEngine>,
}

async fn monitor(
    app: AppHandle,
    mgr: Arc<CaptureManager>,
    generation: u64,
    plan: capture::CapturePlan,
    ignore: Vec<PathBuf>,
    out: Output,
    known: Conditions,
) {
    let store = out.store;
    let mut tail = Tail::default();
    let mut current: Option<PathBuf> = None;
    let mut last_size = 0u64;
    let mut last_growth = Instant::now();
    let mut ever_attached = false;
    let mut connected_at: Option<Instant> = None;
    let waiting_since = Instant::now();
    let mut finish_since: Option<Instant> = None;
    let mut attached_at: Option<Instant> = None;
    let mut auto_started = false;
    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let action = {
            let _guard = match mgr.transition.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            if mgr.generation.load(Ordering::SeqCst) != generation {
                return;
            }
            // Socket de controle.
            let socket = {
                let mut guard = match mgr.control.lock() {
                    Ok(g) => g,
                    Err(_) => return,
                };
                if guard.is_none() && !ever_attached {
                    *guard = ControlClient::connect(&plan.socket_name);
                }
                match guard.as_mut() {
                    Some(c) => {
                        connected_at.get_or_insert_with(Instant::now);
                        c.poll();
                        if c.closed {
                            SocketObs::Closed
                        } else if c.messages.version.is_none() {
                            // connect() pode entrar no backlog de um socket que o jogo não atende.
                            // Só o handshake do MangoHud confirma que podemos controlá-lo.
                            SocketObs::Absent
                        } else {
                            ever_attached = true;
                            SocketObs::Attached {
                                version: c.messages.version.clone(),
                                device: c.messages.device.clone(),
                                logging_finished: c.messages.logging_finished,
                            }
                        }
                    }
                    None if ever_attached => SocketObs::Closed,
                    None => SocketObs::Absent,
                }
            };
            // Log na pasta do FRX.
            let log = capture::scan_logs(Path::new(&plan.output_dir), &ignore)
                .into_iter()
                .next();
            if let Some(l) = &log {
                if current.as_ref() != Some(&l.path) {
                    current = Some(l.path.clone());
                    tail = Tail::default();
                    last_size = 0;
                }
                if l.size != last_size {
                    last_size = l.size;
                    last_growth = Instant::now();
                }
                tail.update(&l.path);
            }
            // Início automático conduzido pelo FRX (HUD oculto): espera o atraso a partir da conexão.
            if matches!(socket, SocketObs::Attached { .. }) {
                let since = *attached_at.get_or_insert_with(Instant::now);
                if capture::frx_starts_logging(&plan)
                    && !auto_started
                    && tail.data_rows == 0
                    && since.elapsed() >= Duration::from_secs(plan.start_delay_s as u64)
                {
                    if let Ok(mut g) = mgr.control.lock() {
                        if let Some(c) = g.as_mut() {
                            auto_started = c.set_logging(true);
                        }
                    }
                }
            }
            let mut status = mgr.status();
            let forced = mgr.finish_now.load(Ordering::SeqCst) > 0;
            if forced {
                finish_since.get_or_insert_with(Instant::now);
            }
            let obs = Observation {
                socket,
                log: log.clone(),
                data_rows: tail.data_rows,
                recorded_s: tail.recorded_s(),
                idle_s: last_growth.elapsed().as_secs_f64(),
                waiting_s: waiting_since.elapsed().as_secs_f64(),
            };
            let mut action = capture::step(&mut status, &obs);
            if status.state == CaptureState::WaitingForGame
                && !ever_attached
                && connected_at.is_some_and(|t| t.elapsed() > Duration::from_secs(30))
            {
                status.state = CaptureState::Failed;
                status.message = Some("O MangoHud não confirmou a conexão de controle. Feche o jogo, prepare uma nova sessão e abra-o de novo para carregar a configuração.".into());
                action = Action::Fail(String::new());
            }
            if forced && action == Action::None && obs.idle_s >= 3.0 {
                action = match &log {
                    Some(l) if tail.data_rows > 0 => {
                        status.state = CaptureState::Processing;
                        status.interrupted = true;
                        status.message =
                            Some("Encerrada por você. Analisando o que foi gravado…".into());
                        Action::Process {
                            path: l.path.clone(),
                            interrupted: true,
                        }
                    }
                    _ => {
                        status.state = CaptureState::Failed;
                        status.message =
                            Some("Encerrada antes de qualquer dado ser gravado.".into());
                        Action::Fail(String::new())
                    }
                };
            }
            if forced
                && action == Action::None
                && finish_since.is_some_and(|t| t.elapsed() > Duration::from_secs(15))
            {
                status.state = CaptureState::Failed;
                status.message = Some("O MangoHud não encerrou o log. Pare a gravação com Shift esquerdo + F2 e importe o CSV depois.".into());
                action = Action::Fail(String::new());
            }
            if matches!(action, Action::Fail(_)) {
                mgr.stop_control();
                let _ = capture::write_config(&plan, false);
                status.controllable = false;
            }
            mgr.set(&app, status.clone());
            action
        };
        match action {
            Action::None => continue,
            Action::Fail(_) => break,
            Action::Process { path, interrupted } => {
                let input = SessionInput {
                    app_id: Some(plan.app_id.clone()),
                    game_name: Some(plan.game_name.clone()),
                    ..plan.input.clone()
                };
                let known = known.clone();
                let engine = out.engine.clone();
                let app_id = plan.app_id.clone();
                let result = tauri::async_runtime::spawn_blocking(move || {
                    let raw = performance::read_log_file(&path)?;
                    let mut session = performance::process(&raw, ProcessArgs { input, source: Some(SessionSource::Assisted), original_path: Some(path.to_string_lossy().into_owned()), known })?;
                    // Correções do FRX confirmadas no disco no fim da captura (associação automática ao Smart Pack).
                    crate::smart_packs::benchmark::tag_assisted(&engine, &app_id, &mut session.meta);
                    if interrupted {
                        session.meta.quality.issues.push(Issue::new(Severity::Warning, "interrupted", "A captura foi interrompida antes do fim planejado; a duração pode ser menor que a escolhida."));
                        if session.meta.quality.level == QualityLevel::Good {
                            session.meta.quality.level = QualityLevel::Usable;
                        }
                    }
                    Ok::<_, String>((session, raw))
                })
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r);
                let _guard = match mgr.transition.lock() {
                    Ok(g) => g,
                    Err(_) => return,
                };
                if mgr.generation.load(Ordering::SeqCst) != generation {
                    return;
                }
                let result = result.and_then(|(session, raw)| {
                    store.save(&session, &raw)?;
                    Ok(session.meta)
                });
                let mut s = mgr.status();
                mgr.stop_control();
                let _ = capture::write_config(&plan, false);
                s.controllable = false;
                match result {
                    Ok(meta) => {
                        s.state = CaptureState::Completed;
                        s.session_id = Some(meta.id.clone());
                        s.message = Some(match meta.stats.fps.avg {
                            Some(f) => format!(
                                "Sessão salva: {} FPS médio em {:.0} s.",
                                format!("{f:.1}").replace('.', ","),
                                meta.stats.duration_s
                            ),
                            None => "Sessão salva.".into(),
                        });
                    }
                    Err(e) => {
                        s.state = CaptureState::Failed;
                        s.message = Some(format!("Não foi possível analisar o log: {e}"));
                    }
                }
                mgr.set(&app, s);
                break;
            }
        }
    }
    // Depois da captura, o config volta ao repouso: nada grava sozinho na próxima vez.
    // Os recursos já foram liberados antes de publicar o estado terminal.
}

/// Inicia/para a gravação pelo socket de controle do MangoHud — só quando conectado.
#[tauri::command]
pub fn capture_logging(ctx: State<'_, AppCtx>, on: bool) -> Result<(), String> {
    let mgr = &ctx.perf.capture;
    let _guard = mgr.transition.lock().map_err(|_| "estado indisponível")?;
    if !matches!(
        mgr.status().state,
        CaptureState::WaitingForGame | CaptureState::Recording
    ) {
        return Err("Nenhuma captura está aguardando ou gravando.".into());
    }
    let mut guard = mgr.control.lock().map_err(|_| "estado indisponível")?;
    let client = guard.as_mut().filter(|c| !c.closed).ok_or("O MangoHud do jogo não está conectado ao FRX. Use o atalho Shift esquerdo + F2 dentro do jogo.")?;
    if client.set_logging(on) {
        Ok(())
    } else {
        Err("O jogo não respondeu ao comando.".into())
    }
}

/// Encerra agora e analisa o que já foi gravado.
#[tauri::command]
pub fn finish_capture(ctx: State<'_, AppCtx>) -> Result<(), String> {
    let mgr = &ctx.perf.capture;
    let _guard = mgr.transition.lock().map_err(|_| "estado indisponível")?;
    if !matches!(
        mgr.status().state,
        CaptureState::WaitingForGame | CaptureState::Recording
    ) {
        return Err("Nenhuma captura em andamento.".into());
    }
    let mut control = mgr.control.lock().map_err(|_| "estado indisponível")?;
    if !control
        .as_mut()
        .is_some_and(|c| !c.closed && c.set_logging(false))
    {
        return Err("Não foi possível parar pelo socket. Use Shift esquerdo + F2 no jogo; o FRX analisará o log quando ele terminar.".into());
    }
    mgr.finish_now.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub fn cancel_capture(app: AppHandle, ctx: State<'_, AppCtx>) -> Result<CaptureStatus, String> {
    let mgr = &ctx.perf.capture;
    let _guard = mgr.transition.lock().map_err(|_| "estado indisponível")?;
    if mgr.status().state == CaptureState::Completed {
        return Err("Esta sessão já foi salva. Exclua-a pelo histórico se desejar.".into());
    }
    mgr.generation.fetch_add(1, Ordering::SeqCst);
    let mut s = mgr.status();
    mgr.stop_control();
    if let Some(plan) = &s.plan {
        let _ = capture::write_config(plan, false);
    }
    s.controllable = false;
    s.state = CaptureState::Cancelled;
    s.message = Some("Captura cancelada. Nenhuma sessão foi salva.".into());
    mgr.set(&app, s.clone());
    Ok(s)
}

#[tauri::command]
pub fn reset_capture(app: AppHandle, ctx: State<'_, AppCtx>) -> Result<CaptureStatus, String> {
    let mgr = &ctx.perf.capture;
    let _guard = mgr.transition.lock().map_err(|_| "estado indisponível")?;
    if mgr.status().state.is_active() {
        return Err("Cancele a captura em andamento primeiro.".into());
    }
    mgr.generation.fetch_add(1, Ordering::SeqCst);
    mgr.stop_control();
    if let Some(plan) = mgr.status().plan {
        capture::write_config(&plan, false)?;
    }
    let s = CaptureStatus::idle();
    mgr.set(&app, s.clone());
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::os::linux::net::SocketAddrExt;
    use std::os::unix::net::{SocketAddr, UnixListener};

    #[test]
    fn shutdown_stops_the_logger_closes_the_socket_and_disarms_only_its_config() {
        let tmp = tempfile::tempdir().unwrap();
        let plan = capture::build_plan(
            &CaptureRequest {
                app_id: "480".into(),
                game_name: "teste".into(),
                duration_s: 10,
                start: capture::StartMode::Auto,
                start_delay_s: 1,
                show_hud: true,
                input: SessionInput::default(),
            },
            tmp.path(),
            None,
            false,
        )
        .unwrap();
        capture::write_config(&plan, true).unwrap();
        let addr = SocketAddr::from_abstract_name(
            format!("frx-shutdown-{}", std::process::id()).as_bytes(),
        )
        .unwrap();
        let server = UnixListener::bind_addr(&addr).unwrap();
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = server.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut received = String::new();
            stream.read_to_string(&mut received).unwrap();
            received
        });
        let mgr = CaptureManager::default();
        *mgr.control.lock().unwrap() =
            ControlClient::connect(&format!("frx-shutdown-{}", std::process::id()));
        let mut status = CaptureStatus::idle();
        status.state = CaptureState::Recording;
        status.plan = Some(plan.clone());
        *mgr.status.lock().unwrap() = Some(status);
        mgr.shutdown();
        assert_eq!(worker.join().unwrap(), ":logging=0;");
        assert!(mgr.control.lock().unwrap().is_none());
        let config = std::fs::read_to_string(plan.config_path).unwrap();
        assert!(!config.contains("autostart_log"));
        assert!(config.contains("no_display"));
        assert_eq!(mgr.generation.load(Ordering::SeqCst), 1);
    }
}
