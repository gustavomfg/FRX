//! PhysX por GPU no Proton: verifica a cadeia real, peça por peça, e usa o
//! Fix Engine só para a parte que o FRX pode alterar com segurança.
//!
//! O PhysX legado (2.x, ex.: Batman: Arkham City) usa CUDA para rodar na GPU.
//! Dentro do Wine isso exige, para a arquitetura do jogo:
//!
//! 1. GPU NVIDIA e a `libcuda` do driver no sistema (32 bits para jogos x86);
//! 2. um `nvcuda.dll` real no prefixo — o do Proton padrão é um stub; o
//!    Proton-CachyOS traz o do nvidia-libs e o copia para o prefixo quando
//!    `PROTON_NVIDIA_LIBS` está ativo, NVAPI ativa e sem WoW64;
//! 3. o runtime PhysX legado instalado no prefixo (Steam instala o PhysX
//!    System Software do jogo).
//!
//! Tudo aqui é leitura. A única alteração possível é a receita revisada
//! `frx.nvidia.gpu-physx` (opções de inicialização), aplicada pelo Fix Engine
//! com prévia, backup e rollback. Nada de DLL é copiado pelo FRX: quem instala
//! o nvcuda no prefixo é o próprio Proton, a partir dos arquivos dele.

use std::path::{Path, PathBuf};

use super::models::*;
use crate::fix_engine::availability::{Availability, AvailabilityReason, AvailabilityState};
use crate::fix_engine::files::sha256_file;
use crate::fix_engine::models::GameCtx;
use crate::fix_engine::FixEngine;
use crate::fixes::models::GpuReq;
use crate::steam::launch_options::LaunchLine;
use crate::time::now_iso;

pub const RECIPE: &str = "frx.nvidia.gpu-physx";
const CACHYOS_README: &str =
    "https://github.com/CachyOS/proton-cachyos/blob/cachyos_main/README.md";
const GE_196: &str = "https://github.com/GloriousEggroll/proton-ge-custom/issues/196";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86,
    X64,
}

impl Arch {
    fn bits(self) -> &'static str {
        match self {
            Arch::X86 => "32 bits",
            Arch::X64 => "64 bits",
        }
    }
    fn wine_dir(self) -> &'static str {
        match self {
            Arch::X86 => "i386-windows",
            Arch::X64 => "x86_64-windows",
        }
    }
    fn prefix_dir(self) -> &'static str {
        match self {
            Arch::X86 => "syswow64",
            Arch::X64 => "system32",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixCuda {
    /// O nvcuda do prefixo é idêntico ao do nvidia-libs da ferramenta.
    NvidiaLibs,
    /// Idêntico ao stub do prefixo padrão da ferramenta.
    Stub,
    Missing,
    Unknown,
}

/// Estado observado de cada peça.
#[derive(Debug, Clone)]
pub struct Chain {
    pub game_dll: Option<String>,
    pub arch: Option<Arch>,
    pub nvidia: Option<bool>,
    pub host_cuda: Option<PathBuf>,
    pub tool_nvcuda: Option<PathBuf>,
    pub tool_switch: bool,
    pub env_libs: bool,
    pub env_wow64: bool,
    pub env_no32: bool,
    pub env_no_nvapi: bool,
    pub prefix: Option<PathBuf>,
    pub prefix_runtime: bool,
    pub prefix_cuda: PrefixCuda,
}

/// Arquitetura de um executável PE (campo Machine do cabeçalho COFF).
pub fn pe_arch(path: &Path) -> Option<Arch> {
    use std::io::Read;
    let mut head = [0u8; 4096];
    let n = std::fs::File::open(path).ok()?.read(&mut head).ok()?;
    let head = &head[..n];
    if head.get(..2)? != b"MZ" {
        return None;
    }
    let off = u32::from_le_bytes(head.get(0x3c..0x40)?.try_into().ok()?) as usize;
    if head.get(off..off + 4)? != b"PE\0\0" {
        return None;
    }
    match u16::from_le_bytes(head.get(off + 4..off + 6)?.try_into().ok()?) {
        0x14c => Some(Arch::X86),
        0x8664 => Some(Arch::X64),
        _ => None,
    }
}

/// Procura o carregador do PhysX na instalação (profundidade e volume limitados).
fn find_game_physx(root: &Path) -> Option<PathBuf> {
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    let mut seen = 0usize;
    let mut fallback = None;
    while let Some((dir, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            seen += 1;
            if seen > 20_000 {
                return fallback;
            }
            let Ok(ft) = e.file_type() else { continue };
            let name = e.file_name().to_string_lossy().to_ascii_lowercase();
            if ft.is_dir() && depth < 4 {
                stack.push((e.path(), depth + 1));
            } else if ft.is_file() {
                match name.as_str() {
                    "physxloader.dll" => return Some(e.path()),
                    "physxcore.dll" => fallback = fallback.or(Some(e.path())),
                    _ => {}
                }
            }
        }
    }
    fallback
}

fn host_cuda(sys_root: &Path, arch: Arch) -> Option<PathBuf> {
    let candidates: &[&str] = match arch {
        Arch::X86 => &[
            "usr/lib32/libcuda.so.1",
            "usr/lib/i386-linux-gnu/libcuda.so.1",
        ],
        Arch::X64 => &[
            "usr/lib/libcuda.so.1",
            "usr/lib64/libcuda.so.1",
            "usr/lib/x86_64-linux-gnu/libcuda.so.1",
        ],
    };
    candidates
        .iter()
        .map(|c| sys_root.join(c))
        .find(|p| p.exists())
}

/// `.../steamapps/common/<jogo>` → `.../steamapps/compatdata/<appid>/pfx`.
fn prefix_of(game: &GameCtx) -> Option<PathBuf> {
    let common = game.install_path.as_ref()?.parent()?;
    let steamapps = common.parent()?;
    Some(steamapps.join("compatdata").join(&game.app_id).join("pfx"))
}

pub fn inspect(game: &GameCtx, launch_line: Option<&str>, sys_root: &Path) -> Chain {
    let dll = game.install_path.as_deref().and_then(find_game_physx);
    let arch = dll.as_deref().and_then(pe_arch);
    let tool = game.tool_path.as_deref();
    let tool_nvcuda = match (tool, arch) {
        (Some(t), Some(a)) => {
            let p = t
                .join("files/lib/wine/nvidia-libs/nvcuda")
                .join(a.wine_dir())
                .join("nvcuda.dll.so");
            p.is_file().then_some(p)
        }
        _ => None,
    };
    let tool_switch = tool
        .and_then(|t| std::fs::read(t.join("proton")).ok())
        .filter(|b| b.len() < 4 * 1024 * 1024)
        .is_some_and(|b| String::from_utf8_lossy(&b).contains("PROTON_NVIDIA_LIBS"));
    let env: Vec<(String, String)> = launch_line
        .and_then(|l| LaunchLine::parse(l).ok())
        .map(|l| {
            l.leading_env()
                .into_iter()
                .map(|(_, k, v)| (k, v))
                .collect()
        })
        .unwrap_or_default();
    let on = |k: &str| {
        env.iter()
            .any(|(key, v)| key == k && v != "0" && !v.is_empty())
    };
    let prefix = prefix_of(game);
    let drive = prefix.as_ref().map(|p| p.join("drive_c"));
    let prefix_runtime = drive.as_ref().is_some_and(|d| {
        [
            "Program Files (x86)/NVIDIA Corporation/PhysX/Common/PhysXLoader.dll",
            "Program Files/NVIDIA Corporation/PhysX/Common/PhysXLoader64.dll",
            "Program Files/NVIDIA Corporation/PhysX/Common/PhysXLoader.dll",
        ]
        .iter()
        .any(|rel| d.join(rel).is_file())
    });
    let prefix_cuda = match (&drive, arch) {
        (Some(d), Some(a)) => {
            let installed = d.join("windows").join(a.prefix_dir()).join("nvcuda.dll");
            match sha256_file(&installed).ok().flatten() {
                None => PrefixCuda::Missing,
                Some(h) => {
                    let libs = tool_nvcuda
                        .as_deref()
                        .and_then(|p| sha256_file(p).ok().flatten());
                    let stub = tool.and_then(|t| {
                        sha256_file(
                            &t.join("files/share/default_pfx/drive_c/windows")
                                .join(a.prefix_dir())
                                .join("nvcuda.dll"),
                        )
                        .ok()
                        .flatten()
                    });
                    if libs.as_deref() == Some(h.as_str()) {
                        PrefixCuda::NvidiaLibs
                    } else if stub.as_deref() == Some(h.as_str()) {
                        PrefixCuda::Stub
                    } else {
                        PrefixCuda::Unknown
                    }
                }
            }
        }
        _ => PrefixCuda::Unknown,
    };
    Chain {
        game_dll: dll.and_then(|p| {
            game.install_path.as_ref().and_then(|r| {
                p.strip_prefix(r)
                    .ok()
                    .map(|x| x.to_string_lossy().into_owned())
            })
        }),
        arch,
        nvidia: game.gpu.map(|g| g == GpuReq::Nvidia),
        host_cuda: arch.and_then(|a| host_cuda(sys_root, a)),
        tool_nvcuda,
        tool_switch,
        env_libs: on("PROTON_NVIDIA_LIBS"),
        env_wow64: on("PROTON_USE_WOW64"),
        env_no32: on("PROTON_NVIDIA_LIBS_NO_32BIT"),
        env_no_nvapi: on("PROTON_DISABLE_NVAPI"),
        prefix,
        prefix_runtime,
        prefix_cuda,
    }
}

fn source(url: &str, publisher: &str, summary: &str, app: Option<&str>) -> Evidence {
    Evidence {
        id: crate::fix_engine::files::sha256_hex(url.as_bytes())[..16].into(),
        kind: if app.is_some() {
            "community-report"
        } else {
            "official-documentation"
        }
        .into(),
        summary: summary.into(),
        source_url: Some(url.into()),
        publisher: Some(publisher.into()),
        published_at: None,
        updated_at: None,
        fetched_at: now_iso(),
        version: None,
        app_id: app.map(str::to_owned),
        technology: Some("physx".into()),
        stale: false,
        limitations: vec![],
    }
}

fn observed(summary: String) -> Evidence {
    Evidence {
        id: crate::fix_engine::files::sha256_hex(summary.as_bytes())[..16].into(),
        kind: "local-inspection".into(),
        summary,
        source_url: None,
        publisher: Some("FRX (leitura local)".into()),
        published_at: None,
        updated_at: None,
        fetched_at: now_iso(),
        version: None,
        app_id: None,
        technology: Some("physx".into()),
        stale: false,
        limitations: vec![],
    }
}

/// Problema e plano de PhysX por GPU, só quando a instalação traz o PhysX legado:
/// sem essa evidência, nenhuma solução é sugerida.
pub fn add(
    engine: &FixEngine,
    game: &GameCtx,
    launch_line: Option<&str>,
    sys_root: &Path,
    technologies: &mut [Technology],
) -> Option<(Problem, ResolutionPlan)> {
    let c = inspect(game, launch_line, sys_root);
    let d = c.game_dll.clone()?;
    // A linha da tabela reflete o que foi conferido; execução segue desconhecida.
    if let Some(t) = technologies.iter_mut().find(|t| t.id == "physx") {
        t.hardware = match c.nvidia {
            Some(true) => "available",
            Some(false) => "incompatible",
            None => "unknown",
        }
        .into();
        if c.arch.is_some() {
            t.environment = if c.tool_nvcuda.is_some() && c.tool_switch && c.host_cuda.is_some() {
                "available"
            } else {
                "unavailable"
            }
            .into();
        }
        t.configured = if c.env_libs && !c.env_wow64 {
            "available"
        } else {
            "unavailable"
        }
        .into();
    }
    let tool = game
        .tool_name
        .clone()
        .unwrap_or_else(|| "a ferramenta selecionada".into());
    let mut ev = Vec::new();
    let mut ok = Vec::new();
    let mut blockers: Vec<(&str, String)> = Vec::new();
    let mut deps = Vec::new();

    match c.arch {
        Some(a) => ev.push(observed(format!(
            "O jogo traz o PhysX legado ({d}), compilado para {}.",
            a.bits()
        ))),
        None => blockers.push((
            "arch-unknown",
            format!("PhysX encontrado ({d}), mas a arquitetura do binário não foi identificada."),
        )),
    }
    match c.nvidia {
        Some(true) => ok.push("GPU NVIDIA detectada.".to_owned()),
        Some(false) => blockers.push(("hardware", "PhysX por GPU exige GPU NVIDIA; o sistema usa outro fabricante. O jogo usa o caminho de CPU.".into())),
        None => blockers.push(("hardware-unknown", "Não foi possível identificar a GPU principal.".into())),
    }
    let rtx50 = game
        .performance_conditions
        .as_ref()
        .and_then(|p| p.gpu.as_deref())
        .is_some_and(|g| g.contains("RTX 50"));
    if rtx50 && c.arch == Some(Arch::X86) {
        blockers.push(("hardware", "As GPUs RTX 50 não executam CUDA de 32 bits; o PhysX por GPU de jogos 32 bits não é suportado nelas.".into()));
    }
    if let Some(a) = c.arch {
        match &c.host_cuda {
            Some(p) => ok.push(format!("Driver com libcuda de {} ({}).", a.bits(), p.display())),
            None => blockers.push(("dependency", format!(
                "O driver NVIDIA não tem a libcuda de {} instalada no sistema. Instale o pacote de {} do driver (Arch/CachyOS: {}). O FRX não instala pacotes do sistema.",
                a.bits(),
                a.bits(),
                if a == Arch::X86 { "lib32-nvidia-utils" } else { "nvidia-utils" }
            ))),
        }
        match (&c.tool_nvcuda, c.tool_switch) {
            (Some(_), true) => ok.push(format!("{tool} traz o nvcuda do nvidia-libs para {} e a opção PROTON_NVIDIA_LIBS.", a.bits())),
            _ => blockers.push(("tool", format!(
                "{tool} não traz o nvcuda do nvidia-libs para {} — o nvcuda padrão do Proton é um stub e não funciona com PhysX por GPU. Selecione nas propriedades do jogo uma ferramenta que o inclua (ex.: Proton-CachyOS); o FRX não troca a ferramenta da Steam.",
                a.bits()
            ))),
        }
    }
    if c.env_no_nvapi {
        blockers.push(("conflict", "PROTON_DISABLE_NVAPI está ativo nas opções do jogo; o Proton só instala o nvcuda com NVAPI ativa. Remova essa variável.".into()));
    }
    if c.arch == Some(Arch::X86) && c.env_no32 {
        blockers.push(("conflict", "PROTON_NVIDIA_LIBS_NO_32BIT está ativo: o nvcuda de 32 bits não é instalado. Remova essa variável para jogos 32 bits.".into()));
    }
    let env_ready = c.env_libs && !c.env_wow64;
    if env_ready {
        ok.push("Opções do jogo já ativam PROTON_NVIDIA_LIBS sem WoW64.".into());
    }
    if c.prefix_runtime {
        ok.push("Runtime PhysX legado instalado no prefixo do jogo.".into());
    } else {
        deps.push(format!(
            "Runtime PhysX legado no prefixo: a Steam instala o PhysX System Software do jogo na primeira execução. Ainda não encontrado em {}.",
            c.prefix.as_deref().map(|p| p.display().to_string()).unwrap_or_else(|| "o prefixo do jogo".into())
        ));
    }
    match c.prefix_cuda {
        PrefixCuda::NvidiaLibs => ok.push("O nvcuda.dll do prefixo é o do nvidia-libs (SHA-256 idêntico ao da ferramenta): o Proton já instalou a CUDA real.".into()),
        PrefixCuda::Stub => ev.push(observed("O nvcuda.dll do prefixo é o stub do Proton (SHA-256 idêntico ao do prefixo padrão).".into())),
        PrefixCuda::Missing => ev.push(observed("Não há nvcuda.dll no prefixo para a arquitetura do jogo.".into())),
        PrefixCuda::Unknown => {}
    }
    ev.extend(ok.iter().cloned().map(observed));

    let hard = blockers
        .iter()
        .any(|(code, _)| !matches!(*code, "conflict"));
    let complete = env_ready
        && blockers.is_empty()
        && c.prefix_runtime
        && c.prefix_cuda == PrefixCuda::NvidiaLibs;
    let id = format!("resolution.{}.gpu-physx", game.app_id);
    let problem_id = "gpu-physx-chain".to_owned();
    let sources = vec![
        source(CACHYOS_README, "CachyOS", "PROTON_NVIDIA_LIBS ativa as bibliotecas NVIDIA alternativas (nvcuda, nvenc, nvml, nvoptix) e permite PhysX acelerado por hardware; incompatível com WoW64.", None),
        source(GE_196, "GloriousEggroll/proton-ge-custom", "Arkham City sem aceleração PhysX por GPU: o nvcuda.dll do Proton é um stub; resolvido com o nvcuda do nvidia-libs e o PhysX instalado no prefixo.", Some(&game.app_id)),
    ];

    let (state, availability, recipe_id, verification) = if complete {
        (
            "already-present",
            Availability::reason(AvailabilityState::Applied, "already-present", "A cadeia de PhysX por GPU já está completa neste ambiente; nada a instalar."),
            None,
            "Configuração e arquivos conferidos agora. Para confirmar o PhysX na GPU, use a opção de PhysX do jogo (o aviso de aceleração ausente não deve aparecer) ou compare no Performance Lab.".to_owned(),
        )
    } else if hard {
        let reasons = blockers
            .iter()
            .map(|(code, m)| AvailabilityReason {
                code: (*code).into(),
                message: m.clone(),
            })
            .collect();
        let state = if blockers.iter().any(|(code, _)| *code == "hardware") {
            AvailabilityState::Incompatible
        } else {
            AvailabilityState::Investigation
        };
        (
            "review",
            Availability {
                state,
                reasons,
                checked_at: now_iso(),
                plan_status: None,
                plan_fingerprint: None,
            },
            None,
            "Nenhuma alteração possível até resolver os itens acima.".to_owned(),
        )
    } else if !env_ready {
        (
            "applicable",
            engine.availability(RECIPE, game),
            Some(RECIPE.to_owned()),
            "Depois de aplicar, abra o jogo uma vez pela Steam e diagnostique de novo: o FRX confere se o nvcuda.dll do prefixo passou a ser o do nvidia-libs.".to_owned(),
        )
    } else {
        // Opções certas, mas o Proton ainda não instalou o nvcuda (jogo não aberto desde então) ou há conflito.
        let mut reasons: Vec<AvailabilityReason> = blockers
            .iter()
            .map(|(code, m)| AvailabilityReason {
                code: (*code).into(),
                message: m.clone(),
            })
            .collect();
        if c.prefix_cuda != PrefixCuda::NvidiaLibs {
            reasons.push(AvailabilityReason { code: "pending-launch".into(), message: "Opções já configuradas; o Proton instala o nvcuda no prefixo na próxima execução. Abra o jogo uma vez e diagnostique de novo.".into() });
        }
        (
            "review",
            Availability {
                state: AvailabilityState::Temporary,
                reasons,
                checked_at: now_iso(),
                plan_status: None,
                plan_fingerprint: None,
            },
            None,
            "Aguardando a próxima execução do jogo.".to_owned(),
        )
    };

    let plan = ResolutionPlan {
        availability,
        id: id.clone(),
        app_id: game.app_id.clone(),
        title: "Ativar CUDA real do Proton para PhysX por GPU".into(),
        objective: "Fazer o PhysX do jogo encontrar um dispositivo CUDA no Wine: o Proton instala no prefixo o nvcuda.dll do nvidia-libs no lugar do stub.".into(),
        problem_id: problem_id.clone(),
        recipe_id,
        state: state.into(),
        actions: vec!["Definir PROTON_NVIDIA_LIBS=1 e PROTON_USE_WOW64=0 nas opções de inicialização deste jogo, preservando as demais.".into(), "Na próxima execução, o próprio Proton copia o nvcuda do nvidia-libs para o prefixo; o FRX não copia DLLs.".into()],
        dependencies: ok.iter().cloned().chain(deps).collect(),
        risks: vec!["As bibliotecas alternativas valem para todo o jogo. Em RTX 4000/5000, bibliotecas de 32 bits podem causar lentidão ou falhas em alguns jogos; compare no Performance Lab.".into()],
        sources,
        reversible: true,
        verification_method: "Conferir no prefixo, após uma execução, que o nvcuda.dll é o do nvidia-libs (SHA-256); depois observar o PhysX na GPU no jogo ou comparar no Performance Lab.".into(),
        success_criterion: "nvcuda do nvidia-libs presente no prefixo confirma a instalação. PhysX na GPU e ganho de desempenho exigem observação do jogo em execução.".into(),
        operation_id: None,
        verification,
        limitations: vec!["A instalação automatizada validada não equivale a problema do jogo resolvido: isso depende de executar o jogo.".into()],
    };
    let problem = Problem {
        id: problem_id,
        technology: "physx".into(),
        symptom: if complete { "PhysX por GPU: ambiente completo".into() } else { "PhysX por GPU indisponível no Wine/Proton".into() },
        certainty: if complete || c.prefix_cuda == PrefixCuda::Stub { "observed" } else { "hypothesis" }.into(),
        evidence: ev,
        hypotheses: vec!["Sem um nvcuda.dll real no prefixo, o PhysX legado não encontra CUDA e roda na CPU (ou avisa que a aceleração está ausente), com queda de desempenho em efeitos de física.".into()],
        ruled_out: if complete { vec!["Componentes ausentes: todas as peças da cadeia foram encontradas.".into()] } else { vec![] },
        limitations: blockers.iter().map(|(_, m)| m.clone()).collect(),
        next_action: if complete {
            "Nada a instalar. Confirme no jogo ou no Performance Lab.".into()
        } else if hard {
            "Resolver os itens indicados; o FRX não instala pacotes do sistema nem troca a ferramenta da Steam.".into()
        } else if !env_ready {
            "Aplicar a correção (prévia, backup e rollback do Fix Engine) e abrir o jogo uma vez.".into()
        } else {
            "Abrir o jogo uma vez pela Steam e diagnosticar de novo.".into()
        },
        plan_ids: vec![id],
    };
    Some((problem, plan))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Instalação, ferramenta e sistema falsos com os mesmos caminhos reais.
    struct Fx {
        _tmp: tempfile::TempDir,
        sys: PathBuf,
        game: GameCtx,
        pfx: PathBuf,
        tool: PathBuf,
    }

    fn pe(arch: u16) -> Vec<u8> {
        let mut b = vec![0u8; 512];
        b[..2].copy_from_slice(b"MZ");
        b[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        b[0x80..0x84].copy_from_slice(b"PE\0\0");
        b[0x84..0x86].copy_from_slice(&arch.to_le_bytes());
        b
    }

    fn put(p: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }

    fn fx() -> Fx {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib/steamapps");
        let install = lib.join("common/Game");
        put(&install.join("Binaries/Win32/PhysXLoader.dll"), &pe(0x14c));
        let tool = tmp.path().join("tool");
        put(
            &tool.join("proton"),
            b"self.check_environment(\"PROTON_NVIDIA_LIBS\", \"nvidialibs\")",
        );
        put(
            &tool.join("files/lib/wine/nvidia-libs/nvcuda/i386-windows/nvcuda.dll.so"),
            b"real-cuda-32",
        );
        put(
            &tool.join("files/share/default_pfx/drive_c/windows/syswow64/nvcuda.dll"),
            b"stub-32",
        );
        let sys = tmp.path().join("sys");
        put(&sys.join("usr/lib32/libcuda.so.1"), b"elf");
        let pfx = lib.join("compatdata/4242/pfx");
        put(
            &pfx.join(
                "drive_c/Program Files (x86)/NVIDIA Corporation/PhysX/Common/PhysXLoader.dll",
            ),
            &pe(0x14c),
        );
        put(&pfx.join("drive_c/windows/syswow64/nvcuda.dll"), b"stub-32");
        let game = GameCtx {
            app_id: "4242".into(),
            name: "Jogo".into(),
            install_path: Some(install),
            uses_proton: Some(true),
            tool_name: Some("proton-cachyos".into()),
            tool_path: Some(tool.clone()),
            gpu: Some(GpuReq::Nvidia),
            ..Default::default()
        };
        Fx {
            sys,
            game,
            pfx,
            tool,
            _tmp: tmp,
        }
    }

    #[test]
    fn detects_each_link_of_the_chain_from_real_file_layouts() {
        let f = fx();
        let c = inspect(&f.game, Some("mangohud %command%"), &f.sys);
        assert_eq!(c.arch, Some(Arch::X86));
        assert_eq!(
            c.game_dll.as_deref(),
            Some("Binaries/Win32/PhysXLoader.dll")
        );
        assert!(
            c.host_cuda.is_some() && c.tool_nvcuda.is_some() && c.tool_switch && c.prefix_runtime
        );
        assert_eq!(c.prefix_cuda, PrefixCuda::Stub);
        assert!(!c.env_libs);
        // Depois da execução com a opção: o prefixo tem o nvcuda real.
        std::fs::copy(
            f.tool
                .join("files/lib/wine/nvidia-libs/nvcuda/i386-windows/nvcuda.dll.so"),
            f.pfx.join("drive_c/windows/syswow64/nvcuda.dll"),
        )
        .unwrap();
        let c = inspect(
            &f.game,
            Some("PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command%"),
            &f.sys,
        );
        assert_eq!(c.prefix_cuda, PrefixCuda::NvidiaLibs);
        assert!(c.env_libs && !c.env_wow64);
        // Sem libcuda de 32 bits e com ferramenta sem nvidia-libs.
        std::fs::remove_file(f.sys.join("usr/lib32/libcuda.so.1")).unwrap();
        std::fs::remove_dir_all(f.tool.join("files/lib/wine/nvidia-libs")).unwrap();
        let c = inspect(&f.game, None, &f.sys);
        assert!(c.host_cuda.is_none() && c.tool_nvcuda.is_none());
    }
}

/// Diagnóstico real, somente leitura: `FRX_APPID=200260 cargo test real_gpu_physx -- --ignored --nocapture`.
/// O Fix Engine usa um diretório temporário; nada é gravado na Steam ou no prefixo.
#[cfg(test)]
mod real {
    use super::*;

    #[test]
    #[ignore]
    fn real_gpu_physx() {
        let tmp = tempfile::tempdir().unwrap();
        let app = std::env::var("FRX_APPID").unwrap_or_else(|_| "200260".into());
        let game = crate::commands::fix_engine::game_ctx(
            &crate::cache::Cache::new(tmp.path().join("cache")),
            &app,
        )
        .unwrap();
        let engine = FixEngine::with(
            tmp.path().join("fix-engine"),
            crate::fix_engine::catalog::Catalog::bundled(),
            "/proc".into(),
        );
        let line = crate::smart_packs::launch_line(&game);
        println!(
            "jogo: {} · ferramenta: {:?} ({:?}) · GPU: {:?}",
            game.name, game.tool_name, game.tool_path, game.gpu
        );
        println!("opções: {line:?}");
        println!("{:#?}", inspect(&game, line.as_deref(), Path::new("/")));
        let (problem, p) =
            add(&engine, &game, line.as_deref(), Path::new("/"), &mut []).expect("PhysX detectado");
        println!("\nPROBLEMA: {} [{}]", problem.symptom, problem.certainty);
        for e in &problem.evidence {
            println!("  • {}", e.summary);
        }
        println!(
            "PLANO: {} · estado {} · disponibilidade {:?}",
            p.title, p.state, p.availability.state
        );
        for r in &p.availability.reasons {
            println!("  - {}", r.message);
        }
        println!("verificação: {}", p.verification);
    }
}
