//! Correções de sistema do laptop: opções de módulo (`/etc/modprobe.d`),
//! serviços do systemd (`/etc/systemd/system`) e parâmetros do kernel no GRUB
//! (`/etc/default/grub`).
//!
//! As ações formam um conjunto **fechado e tipado**: módulo, opção e
//! parâmetro passam por validação estrita, e o serviço do systemd vem de um
//! modelo fixo — não existe `ExecStart` livre. O conteúdo final é gerado aqui;
//! a interface só envia IDs.
//!
//! O FRX nunca roda `sudo`. Com a raiz real (`/`), a aplicação é **manual**:
//! o núcleo faz o backup, grava o arquivo novo numa pasta privada e mostra o
//! comando `sudo install …`; depois confere o hash em `/etc`. O rollback é o
//! mesmo: comando para restaurar/remover e conferência no disco. Só com uma
//! raiz de sistema fictícia (testes) o núcleo grava sozinho, pelo
//! [`confined::Target::open_system`].

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::confined::{self, Target};
use super::files::sha256_hex;
use super::models::{Check, CheckState, ChangeKind, PlanStatus, PlannedChange, Risk};

/// `appId` das operações de sistema no diário (não é um AppID da Steam).
pub const SYSTEM_APP_ID: &str = "system";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
// Os nomes são os da especificação das ações (AddModprobeOption…).
#[allow(clippy::enum_variant_names)]
pub enum SystemAction {
    /// `options <módulo> <chave=valor>…` em `/etc/modprobe.d/frx-<módulo>.conf`.
    AddModprobeOption { module: String, options: Vec<String> },
    /// Serviço gerado de um modelo fixo em `/etc/systemd/system/frx-*.service`.
    AddSystemdService { unit: SystemdUnit },
    /// Parâmetros acrescentados a `GRUB_CMDLINE_LINUX_DEFAULT`.
    AddKernelParameters { params: Vec<String> },
}

/// Modelos de serviço. Cada um gera um arquivo inteiro e conhecido.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "template", rename_all = "snake_case", deny_unknown_fields)]
pub enum SystemdUnit {
    /// Descarrega o módulo antes de suspender e o carrega de novo ao voltar
    /// (o contorno clássico para Wi-Fi que não volta da suspensão).
    ReloadModuleAroundSleep { module: String },
}

/// O que a interface pediu, já resolvido no núcleo a partir do cache da busca.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SystemFixRequest {
    pub action: SystemAction,
    pub title: String,
    /// Frase da fonte em volta da instrução (condições como "só em Intel").
    pub context: String,
    pub source_label: String,
    pub source_url: String,
    /// Nome do laptop mostrado no diário.
    pub laptop: String,
}

/// Parâmetros que mudam o alvo do boot ou desligam proteções do kernel.
/// Mesmo que uma página sugira, o FRX não os aplica.
const DENIED_PARAMS: &[&str] = &[
    "init", "rdinit", "root", "rootflags", "ro", "rw", "single", "emergency", "rescue", "s", "1",
    "rd.break", "rd.shell", "systemd.unit", "systemd.debug_shell", "systemd.debug-shell",
    "security", "selinux", "apparmor", "enforcing", "lockdown", "mitigations", "nokaslr",
    "module.sig_enforce", "module_blacklist", "modprobe.blacklist", "rd.driver.blacklist",
    "panic", "sysrq_always_enabled", "nomodeset",
];

fn ident(s: &str, max: usize, extra: &[u8]) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || extra.contains(&b))
}

fn value(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.,:+-".contains(&b))
}

pub fn valid_module(m: &str) -> bool {
    ident(m, 40, b"_-") && !m.starts_with('-')
}

/// `chave=valor` de `options`. Sem espaços, aspas, `$` ou barras.
pub fn valid_module_option(o: &str) -> bool {
    o.split_once('=')
        .is_some_and(|(k, v)| ident(k, 40, b"_") && value(v))
}

/// `nome` ou `nome=valor`, fora da lista de parâmetros negados.
pub fn valid_kernel_param(p: &str) -> bool {
    let (key, val) = match p.split_once('=') {
        Some((k, v)) => (k, Some(v)),
        None => (p, None),
    };
    ident(key, 48, b"_.-")
        && !key.starts_with(['-', '.'])
        && val.is_none_or(value)
        && !DENIED_PARAMS.contains(&key)
}

impl SystemAction {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::AddModprobeOption { module, options } => {
                if !valid_module(module) {
                    return Err(format!("Nome de módulo inválido: {module}"));
                }
                if options.is_empty() || options.len() > 8 {
                    return Err("Informe de 1 a 8 opções de módulo.".into());
                }
                if let Some(o) = options.iter().find(|o| !valid_module_option(o)) {
                    return Err(format!("Opção de módulo inválida: {o}"));
                }
            }
            Self::AddSystemdService {
                unit: SystemdUnit::ReloadModuleAroundSleep { module },
            } => {
                if !valid_module(module) {
                    return Err(format!("Nome de módulo inválido: {module}"));
                }
            }
            Self::AddKernelParameters { params } => {
                if params.is_empty() || params.len() > 4 {
                    return Err("Informe de 1 a 4 parâmetros do kernel.".into());
                }
                if let Some(p) = params.iter().find(|p| !valid_kernel_param(p)) {
                    return Err(format!("O FRX não aplica o parâmetro do kernel “{p}”."));
                }
            }
        }
        Ok(())
    }

    /// Caminho absoluto do destino (já dentro da allowlist do `confined`).
    pub fn path(&self) -> String {
        let slug = |m: &str| m.replace('_', "-");
        match self {
            Self::AddModprobeOption { module, .. } => {
                format!("/etc/modprobe.d/frx-{}.conf", slug(module))
            }
            Self::AddSystemdService {
                unit: SystemdUnit::ReloadModuleAroundSleep { module },
            } => format!("/etc/systemd/system/frx-{}-sleep.service", slug(module)),
            Self::AddKernelParameters { .. } => "/etc/default/grub".into(),
        }
    }

    fn unit_name(&self) -> Option<String> {
        matches!(self, Self::AddSystemdService { .. })
            .then(|| self.path().rsplit('/').next().unwrap_or_default().to_owned())
    }

    pub fn risk(&self) -> Risk {
        match self {
            Self::AddModprobeOption { .. } => Risk::Moderate,
            Self::AddSystemdService { .. } => Risk::Moderate,
            // Um parâmetro errado pode impedir o boot.
            Self::AddKernelParameters { .. } => Risk::High,
        }
    }

    /// Chave estável da ação (para IDs de receita e conflitos).
    pub fn key(&self) -> String {
        sha256_hex(serde_json::to_string(self).unwrap_or_default().as_bytes())[..12].to_owned()
    }
}

/// Resultado do cálculo de uma ação contra o estado atual do sistema.
#[derive(Debug, Clone, PartialEq)]
pub struct Rendered {
    pub path: String,
    pub before: Option<String>,
    pub before_sha: Option<String>,
    pub content: String,
    pub identity: Option<confined::FileIdentity>,
    pub status: PlanStatus,
    pub reasons: Vec<String>,
    pub checks: Vec<Check>,
    pub change: PlannedChange,
    /// Comandos depois da instalação (ex.: `systemctl enable`). Só exibidos.
    pub post_apply: Vec<String>,
    /// Comandos antes de remover/restaurar no rollback. Só exibidos.
    pub pre_rollback: Vec<String>,
    /// Comandos depois do rollback. Só exibidos.
    pub post_rollback: Vec<String>,
    pub side_effects: Vec<String>,
}

fn check(label: &str, state: CheckState, detail: impl Into<String>) -> Check {
    Check {
        label: label.into(),
        state,
        detail: detail.into(),
    }
}

/// Aspas simples de shell para caminhos exibidos nos comandos.
pub fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

fn header(req: &SystemFixRequest) -> String {
    // A URL vem da busca (wiki.archlinux.org); ainda assim, nada de quebra de linha.
    let url: String = req.source_url.chars().filter(|c| !c.is_control()).collect();
    format!("# Gerado pelo FRX Hardware Fixer. Fonte: {url}\n# Para desfazer, use o FRX ou remova este arquivo.\n")
}

const NO_GRUB: &str = "Este sistema não usa GRUB (não há /etc/default/grub). Adicione o parâmetro pela configuração do seu carregador de boot.";

/// Resultado da checagem leve mostrada no card, antes de qualquer prévia.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuickCheck {
    pub applicable: bool,
    pub detail: String,
}

/// Checagem leve, sem prévia: só existência de arquivo, sem ler conteúdo
/// nem calcular hash. Hoje só para parâmetros do kernel (precisa de GRUB);
/// a verificação completa continua na prévia.
pub fn quick_check(system_root: &Path, action: &SystemAction) -> Option<QuickCheck> {
    match action {
        SystemAction::AddKernelParameters { .. } => Some(if system_root.join("etc/default/grub").is_file() {
            QuickCheck {
                applicable: true,
                detail: "/etc/default/grub encontrado. A prévia confere o conteúdo antes de liberar o comando.".into(),
            }
        } else {
            QuickCheck {
                applicable: false,
                detail: NO_GRUB.into(),
            }
        }),
        _ => None,
    }
}

/// Calcula o conteúdo novo e o estado do plano para a raiz de sistema dada.
pub fn render(system_root: &Path, req: &SystemFixRequest) -> Result<Rendered, String> {
    req.action.validate()?;
    let path = req.action.path();
    let target = Target::open_system(system_root, &path, None)?;
    let current = target
        .read()?
        .map(|b| String::from_utf8(b).map_err(|_| format!("{path} não é texto UTF-8.")))
        .transpose()?;
    let before_sha = current.as_deref().map(|t| sha256_hex(t.as_bytes()));
    let mut checks = vec![check(
        "Destino",
        CheckState::Ok,
        format!("{path} está na lista de arquivos de sistema permitidos."),
    )];
    let mut reasons = Vec::new();
    let mut status = PlanStatus::Ready;
    let mut post_apply = Vec::new();
    let mut pre_rollback = Vec::new();
    let mut post_rollback = Vec::new();
    let mut side_effects = Vec::new();

    let (content, change) = match &req.action {
        SystemAction::AddModprobeOption { module, options } => {
            let content = format!("{}options {module} {}\n", header(req), options.join(" "));
            others_configure_module(system_root, module, &path, &mut checks);
            side_effects.push(format!(
                "Vale depois de reiniciar ou de recarregar o módulo {module}. Se ele é carregado no initramfs, regenere a imagem (no Arch: sudo mkinitcpio -P)."
            ));
            let change = PlannedChange {
                kind: ChangeKind::SystemFile,
                target: path.clone(),
                before: None,
                after: Some(content.clone()),
                summary: format!("cria {path} com options {module} {}", options.join(" ")),
            };
            (content, change)
        }
        SystemAction::AddSystemdService {
            unit: SystemdUnit::ReloadModuleAroundSleep { module },
        } => {
            let unit = req.action.unit_name().unwrap_or_default();
            let content = format!(
                "{}[Unit]\nDescription=FRX: recarrega o módulo {module} ao suspender e retomar\nBefore=sleep.target\nStopWhenUnneeded=yes\n\n[Service]\nType=oneshot\nRemainAfterExit=yes\nExecStart=/usr/bin/modprobe -r {module}\nExecStop=/usr/bin/modprobe {module}\n\n[Install]\nWantedBy=sleep.target\n",
                header(req)
            );
            post_apply.push("sudo systemctl daemon-reload".into());
            post_apply.push(format!("sudo systemctl enable {unit}"));
            pre_rollback.push(format!("sudo systemctl disable {unit}"));
            post_rollback.push("sudo systemctl daemon-reload".into());
            side_effects.push(format!(
                "O módulo {module} é descarregado a cada suspensão e carregado de novo ao voltar."
            ));
            let change = PlannedChange {
                kind: ChangeKind::SystemFile,
                target: path.clone(),
                before: None,
                after: Some(content.clone()),
                summary: format!("cria e habilita o serviço {unit}"),
            };
            (content, change)
        }
        SystemAction::AddKernelParameters { params } => {
            let Some(text) = current.as_deref() else {
                return Ok(incompatible(
                    req,
                    &path,
                    target.identity.clone(),
                    checks,
                    NO_GRUB,
                ));
            };
            match add_kernel_params(text, params) {
                Ok(GrubEdit::Unchanged) => {
                    status = PlanStatus::AlreadyApplied;
                    reasons.push("Os parâmetros já estão em GRUB_CMDLINE_LINUX_DEFAULT.".into());
                    (text.to_owned(), grub_change(&path, None, None, params))
                }
                Ok(GrubEdit::Changed { content, old, new }) => {
                    let grub_cfg = system_root.join("boot/grub/grub.cfg");
                    if grub_cfg.exists() {
                        checks.push(check("GRUB", CheckState::Ok, "/boot/grub/grub.cfg encontrado."));
                    } else {
                        checks.push(check("GRUB", CheckState::Warn, "/boot/grub/grub.cfg não encontrado: confira onde a sua distribuição guarda o grub.cfg antes de rodar o grub-mkconfig."));
                    }
                    post_apply.push("sudo grub-mkconfig -o /boot/grub/grub.cfg".into());
                    post_rollback.push("sudo grub-mkconfig -o /boot/grub/grub.cfg".into());
                    side_effects.push("Vale depois de reiniciar. Se o sistema não iniciar, escolha a entrada no menu do GRUB, aperte “e” e remova o parâmetro da linha linux para iniciar uma vez.".into());
                    (content, grub_change(&path, Some(old), Some(new), params))
                }
                Err(e) => {
                    status = PlanStatus::Conflict;
                    reasons.push(e);
                    (text.to_owned(), grub_change(&path, None, None, params))
                }
            }
        }
    };

    // Arquivo criado pelo FRX que já existe: igual = aplicado; diferente = conflito.
    if !matches!(req.action, SystemAction::AddKernelParameters { .. }) {
        match current.as_deref() {
            Some(t) if t == content => {
                status = PlanStatus::AlreadyApplied;
                reasons.push(format!("{path} já tem exatamente este conteúdo."));
            }
            Some(_) => {
                status = PlanStatus::Conflict;
                reasons.push(format!("{path} já existe com outro conteúdo. Desfaça a correção anterior no FRX antes de aplicar esta."));
            }
            None => checks.push(check("Arquivo", CheckState::Ok, "Ainda não existe; será criado.")),
        }
    }

    Ok(Rendered {
        path,
        before: current,
        before_sha,
        content,
        identity: Some(target.identity.clone()),
        status,
        reasons,
        checks,
        change,
        post_apply,
        pre_rollback,
        post_rollback,
        side_effects,
    })
}

fn incompatible(
    req: &SystemFixRequest,
    path: &str,
    identity: confined::FileIdentity,
    mut checks: Vec<Check>,
    reason: &str,
) -> Rendered {
    checks.push(check("Carregador de boot", CheckState::Fail, reason));
    let params = match &req.action {
        SystemAction::AddKernelParameters { params } => params.as_slice(),
        _ => &[],
    };
    Rendered {
        path: path.into(),
        before: None,
        before_sha: None,
        content: String::new(),
        identity: Some(identity),
        status: PlanStatus::Incompatible,
        reasons: vec![reason.into()],
        checks,
        change: grub_change(path, None, None, params),
        post_apply: vec![],
        pre_rollback: vec![],
        post_rollback: vec![],
        side_effects: vec![],
    }
}

fn grub_change(path: &str, old: Option<String>, new: Option<String>, params: &[String]) -> PlannedChange {
    PlannedChange {
        kind: ChangeKind::SystemFile,
        target: path.into(),
        before: old,
        after: new,
        summary: format!("acrescenta {} a GRUB_CMDLINE_LINUX_DEFAULT", params.join(" ")),
    }
}

/// Avisa quando outro arquivo de `/etc/modprobe.d` já define opções do módulo.
fn others_configure_module(system_root: &Path, module: &str, own: &str, checks: &mut Vec<Check>) {
    let norm = |m: &str| m.replace('-', "_");
    let dir = system_root.join("etc/modprobe.d");
    let own_name = own.rsplit('/').next().unwrap_or_default();
    let mut found = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name == own_name || !name.ends_with(".conf") {
                continue;
            }
            // Leitura simples só para o aviso; nada é escrito fora do destino.
            let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
            if text.lines().any(|l| {
                let mut w = l.split_whitespace();
                w.next() == Some("options") && w.next().is_some_and(|m| norm(m) == norm(module))
            }) {
                found.push(name);
            }
        }
    }
    if found.is_empty() {
        checks.push(check("Outras opções", CheckState::Ok, format!("Nenhum outro arquivo em /etc/modprobe.d define opções para {module}.")));
    } else {
        found.sort();
        checks.push(check(
            "Outras opções",
            CheckState::Warn,
            format!("{} já define opções para {module}; confira se não se contradizem.", found.join(", ")),
        ));
    }
}

#[derive(Debug, PartialEq)]
enum GrubEdit {
    Unchanged,
    Changed { content: String, old: String, new: String },
}

/// Acrescenta parâmetros à única linha `GRUB_CMDLINE_LINUX_DEFAULT="…"`.
/// Recusa linhas com `$`, barras invertidas ou aspas internas, e chaves que já
/// existem com outro valor — nesses casos a edição não é segura.
fn add_kernel_params(text: &str, params: &[String]) -> Result<GrubEdit, String> {
    const KEY: &str = "GRUB_CMDLINE_LINUX_DEFAULT=";
    let hits: Vec<usize> = text
        .lines()
        .enumerate()
        .filter(|(_, l)| l.starts_with(KEY))
        .map(|(i, _)| i)
        .collect();
    let [idx] = hits[..] else {
        return Err(format!(
            "/etc/default/grub precisa ter exatamente uma linha {KEY}\"…\" (encontradas: {}).",
            hits.len()
        ));
    };
    let line = text.lines().nth(idx).unwrap_or_default();
    let quoted = &line[KEY.len()..];
    let inner = quoted
        .strip_prefix('"')
        .and_then(|q| q.strip_suffix('"'))
        .filter(|i| !i.contains(['"', '$', '\\', '`', '\'']))
        .ok_or("A linha GRUB_CMDLINE_LINUX_DEFAULT não está no formato simples \"…\"; o FRX não a edita.")?;
    let mut words: Vec<String> = inner.split_whitespace().map(str::to_owned).collect();
    let key_of = |p: &str| p.split_once('=').map_or(p, |(k, _)| k).to_owned();
    let mut added = false;
    for p in params {
        match words.iter().find(|w| key_of(w) == key_of(p)) {
            Some(w) if w == p => {}
            Some(w) => {
                return Err(format!(
                    "{} já está na linha como “{w}”. O FRX não troca um valor existente.",
                    key_of(p)
                ))
            }
            None => {
                words.push(p.clone());
                added = true;
            }
        }
    }
    if !added {
        return Ok(GrubEdit::Unchanged);
    }
    let new_line = format!("{KEY}\"{}\"", words.join(" "));
    let mut out: Vec<&str> = text.lines().collect();
    out[idx] = &new_line;
    let mut content = out.join("\n");
    if text.ends_with('\n') {
        content.push('\n');
    }
    Ok(GrubEdit::Changed {
        content,
        old: line.to_owned(),
        new: new_line,
    })
}

/// Pasta privada com os arquivos prontos para o `sudo install`.
pub fn staged_dir(engine_root: &Path, op_id: &str) -> PathBuf {
    engine_root.join("system-staged").join(op_id)
}

/// Conferência feita **pelo próprio comando**, como root e depois da senha:
/// o destino precisa ter exatamente o hash que o FRX leu. É o único ponto que
/// fecha a janela entre mostrar o comando e o usuário rodá-lo — o FRX não
/// executa o comando, então não pode conferir nesse instante.
/// `$1` = hash esperado, `$2` = destino, `$3` = arquivo de origem.
const GUARD: &str = r#"echo "$1  $2" | sha256sum --check --status || { echo "FRX: $2 mudou desde a prévia; nada foi alterado. Refaça a prévia no FRX." >&2; exit 1; };"#;

/// Instala `staged` em `path`. Com `expected`, o destino existente só é
/// sobrescrito se ainda tiver esse hash; sem ele (arquivo novo do FRX), é
/// um `install` simples. Só exibido ao usuário; o FRX nunca executa.
pub fn install_commands(
    staged: &Path,
    path: &str,
    mode: u32,
    expected: Option<&str>,
    post: &[String],
) -> Vec<String> {
    let src = sh_quote(&staged.to_string_lossy());
    let install = format!("install -m {mode:04o} -o root -g root --");
    let mut cmds = vec![match expected {
        Some(sha) => format!(
            "sudo sh -c '{GUARD} {install} \"$3\" \"$2\"' frx {sha} {} {src}",
            sh_quote(path)
        ),
        None => format!("sudo {install} {src} {}", sh_quote(path)),
    }];
    cmds.extend(post.iter().cloned());
    cmds
}

/// Remove `path` só se ainda tiver o hash do que o FRX escreveu.
pub fn remove_command(path: &str, expected: &str) -> String {
    format!("sudo sh -c '{GUARD} rm -- \"$2\"' frx {expected} {}", sh_quote(path))
}

// ── Fix Engine ───────────────────────────────────────────────────────────

use std::sync::TryLockError;

use super::journal::{new_op_id, valid_op_id};
use super::models::{
    BackupRef, ExecMode, FixPlan, OpState, Operation, Pending, PendingKind, Purpose,
    RecipeSource, RollbackPlan, Step, StepRecord, StepStatus, Trust, JOURNAL_SCHEMA,
};
use super::planner::PLAN_TTL_SECS;
use super::{is_system_op, open_game_file, step_sha, FixEngine, RollbackWrites};
use crate::time::{now_iso, now_unix};

/// Recusa na confirmação quando o destino mudou depois da prévia.
pub const FILE_CHANGED: &str = "O arquivo foi alterado desde a prévia. Refaça a prévia para garantir que a alteração não seja perdida. Nada foi alterado.";

const MANUAL_NOTE: &str = "Gravar em /etc exige root. O FRX não usa sudo: ele faz o backup, prepara o arquivo numa pasta privada e mostra o comando para você rodar no terminal. Depois, confere o resultado em /etc.";

impl FixEngine {
    /// Checagem leve de uma ação contra a raiz de sistema deste núcleo.
    pub fn quick_check(&self, action: &SystemAction) -> Option<QuickCheck> {
        quick_check(&self.system_root, action)
    }

    /// Modo manual sempre que a raiz é o sistema real.
    fn system_manual(&self) -> bool {
        // Na raiz real o modo automático não existe, seja qual for a configuração.
        self.system_root == Path::new("/") || self.system_force_manual
    }

    /// Prévia de uma correção de sistema. Nada é escrito.
    pub fn prepare_system(&self, req: SystemFixRequest) -> Result<FixPlan, String> {
        let plan_id = new_op_id();
        let (plan, _) = self.system_plan(&req, &plan_id)?;
        let mut plans = self.system_plans.lock().map_err(|_| "estado indisponível")?;
        let now = now_unix();
        plans.retain(|_, (p, _)| p.expires_at > now);
        plans.insert(plan_id, (plan.clone(), req));
        Ok(plan)
    }

    fn system_plan(&self, req: &SystemFixRequest, plan_id: &str) -> Result<(FixPlan, Step), String> {
        let r = render(&self.system_root, req)?;
        let manual = self.system_manual();
        let mut status = r.status;
        let mut reasons = r.reasons.clone();
        let mut checks = r.checks.clone();

        // Outra operação do FRX já cuida deste arquivo.
        if status == PlanStatus::Ready {
            let holder = self.journal.list().into_iter().find(|o| {
                is_system_op(o)
                    && (o.holds_changes()
                        || (o.state == OpState::RequiresManualStep
                            && o.pending.as_ref().is_some_and(|p| p.kind == PendingKind::Apply)))
                    && o.steps.iter().any(|s| matches!(&s.step, Step::SystemFile { path, .. } if *path == r.path))
            });
            if let Some(o) = holder {
                status = PlanStatus::Conflict;
                reasons.push(format!("A correção “{}” já cuida de {}. Desfaça-a antes de aplicar esta.", o.recipe_title, r.path));
            }
        }

        if let (Some(sha), PlanStatus::Ready) = (&r.before_sha, status) {
            checks.push(check(
                "Arquivo existente",
                CheckState::Ok,
                format!("Lido agora (SHA-256 {}…). O comando só sobrescreve {} se ele ainda tiver exatamente este conteúdo.", &sha[..12], r.path),
            ));
        }

        let staged = staged_dir(&self.root, plan_id).join(r.path.rsplit('/').next().unwrap_or("arquivo"));
        let mode = Target::open_system(&self.system_root, &r.path, None)?
            .mode()?
            .unwrap_or(0o644);
        let mut manual_line = None;
        if status == PlanStatus::Ready && manual {
            status = PlanStatus::RequiresManualStep;
            checks.push(check("Permissão", CheckState::Warn, MANUAL_NOTE));
            manual_line = Some(
                install_commands(&staged, &r.path, mode, r.before_sha.as_deref(), &r.post_apply)
                    .join(" && "),
            );
        }
        let exec_mode = match status {
            PlanStatus::Ready => Some(ExecMode::Automatic),
            PlanStatus::RequiresManualStep => Some(ExecMode::Manual),
            _ => None,
        };
        let fingerprint = sha256_hex(
            serde_json::to_string(&(
                req,
                &r.before_sha,
                &r.content,
                format!("{status:?}{manual}"),
                self.system_root.to_string_lossy(),
            ))
            .unwrap_or_default()
            .as_bytes(),
        )[..32]
            .to_owned();
        let backup_note = Some(match r.before {
            Some(_) => format!("Antes de mudar, o FRX guarda uma cópia verificada de {}. Desfazer restaura essa cópia.", r.path),
            None => format!("{} ainda não existe. Desfazer remove o arquivo criado.", r.path),
        });
        let kind = match &req.action {
            SystemAction::AddModprobeOption { .. } => "modprobe",
            SystemAction::AddSystemdService { .. } => "systemd",
            SystemAction::AddKernelParameters { .. } => "kernel-params",
        };
        let plan = FixPlan {
            requires_explicit_user_consent: false,
            artifacts: vec![],
            environment: None,
            hardware_match: None,
            performance_warnings: vec![],
            plan_id: plan_id.to_owned(),
            fingerprint,
            recipe_id: format!("system.{kind}.{}", req.action.key()),
            recipe_version: "1".into(),
            recipe_title: req.title.clone(),
            app_id: SYSTEM_APP_ID.into(),
            game_name: req.laptop.clone(),
            status,
            mode: exec_mode,
            reasons,
            checks,
            changes: vec![r.change.clone()],
            manual_line,
            backup_note,
            steam_running: false,
            game_running: false,
            risk: req.action.risk(),
            purpose: Purpose::Compatibility,
            trust: Trust::Generated,
            effects: vec![req.context.clone()],
            side_effects: r.side_effects.clone(),
            sources: vec![RecipeSource {
                label: req.source_label.clone(),
                url: req.source_url.clone(),
            }],
            generated: None,
            pack: None,
            created_at: now_iso(),
            expires_at: now_unix() + PLAN_TTL_SECS,
        };
        let step = Step::SystemFile {
            system_root: self.system_root.clone(),
            path: r.path.clone(),
            target: PathBuf::from(&r.path),
            identity: r.identity.clone(),
            before_sha: r.before_sha.clone(),
            content: r.content.clone(),
            post_apply: r.post_apply.clone(),
            pre_rollback: r.pre_rollback.clone(),
            post_rollback: r.post_rollback.clone(),
        };
        Ok((plan, step))
    }

    /// Confirma uma prévia de sistema. Com a raiz real: backup + arquivo
    /// preparado + comando `sudo` pendente. Com raiz fictícia: grava sozinho.
    pub fn apply_system(
        &self,
        plan_id: &str,
        fingerprint: &str,
        emit: &dyn Fn(&Operation),
    ) -> Result<Operation, String> {
        let _guard = match self.exec.try_lock() {
            Ok(g) => g,
            Err(TryLockError::WouldBlock) => {
                return Err("Outra correção está sendo aplicada ou desfeita. Aguarde.".into())
            }
            Err(TryLockError::Poisoned(_)) => return Err("estado indisponível".into()),
        };
        let (stored, req) = self
            .system_plans
            .lock()
            .map_err(|_| "estado indisponível")?
            .remove(plan_id)
            .ok_or("Prévia não encontrada ou já usada. Revise a correção de novo.")?;
        if stored.expires_at <= now_unix() {
            return Err("A prévia expirou. Revise a correção de novo.".into());
        }
        if stored.fingerprint != fingerprint || !valid_op_id(plan_id) {
            return Err("A confirmação não corresponde à prévia mostrada.".into());
        }
        let (plan, step) = self.system_plan(&req, plan_id)?;
        if plan.fingerprint != stored.fingerprint {
            return Err(FILE_CHANGED.into());
        }
        if !plan.executable() {
            return Err(plan.reasons.first().cloned().unwrap_or_else(|| "Esta correção não pode ser aplicada agora.".into()));
        }
        let now = now_iso();
        let mut op = Operation {
            requires_explicit_user_consent: false,
            manual_recovery_required: false,
            artifacts: vec![],
            artifact_progress: None,
            recipe_fingerprint: Some(plan.fingerprint.clone()),
            schema: JOURNAL_SCHEMA,
            id: plan_id.to_owned(),
            recipe_id: plan.recipe_id.clone(),
            recipe_version: plan.recipe_version.clone(),
            recipe_title: plan.recipe_title.clone(),
            app_id: SYSTEM_APP_ID.into(),
            game_name: req.laptop.clone(),
            state: OpState::Checking,
            mode: plan.mode.unwrap_or(ExecMode::Manual),
            steps: vec![StepRecord {
                step,
                status: StepStatus::Pending,
                backup: None,
                after_sha: None,
                rollback_backup: None,
            }],
            plan_fingerprint: plan.fingerprint.clone(),
            created_at: now.clone(),
            updated_at: now,
            applied_at: None,
            rolled_back_at: None,
            message: None,
            pending: None,
            events: vec![],
            recipe_trust: Some(Trust::Generated),
            finding_ids: vec![],
        };
        if op.mode == ExecMode::Automatic {
            self.journal.transition(&mut op, OpState::Checking, "Plano confirmado; conferindo o estado atual.")?;
            emit(&op);
            self.run_apply(&mut op, emit);
            return Ok(op);
        }

        // Manual: backup verificado e arquivo preparado. Nada em /etc muda aqui.
        self.journal.transition(&mut op, OpState::BackingUp, "Criando backup verificado.")?;
        emit(&op);
        if let Err(e) = self.stage_system(&mut op) {
            let _ = std::fs::remove_dir_all(staged_dir(&self.root, &op.id));
            let state = if e.starts_with("CONFLITO:") { OpState::Conflict } else { OpState::Failed };
            self.journal.transition(&mut op, state, format!("{} Nenhum arquivo do sistema foi alterado.", e.trim_start_matches("CONFLITO:").trim()))?;
            emit(&op);
            return Ok(op);
        }
        op.pending = Some(Pending {
            kind: PendingKind::Apply,
            line: plan.manual_line.unwrap_or_default(),
        });
        self.journal.transition(&mut op, OpState::RequiresManualStep, "Backup feito e arquivo preparado. Rode o comando com sudo no terminal; a correção só conta como aplicada depois que o FRX conferir o arquivo em /etc.")?;
        emit(&op);
        Ok(op)
    }

    fn stage_system(&self, op: &mut Operation) -> Result<(), String> {
        let dir = self.backups_dir(&op.id);
        let staged_root = staged_dir(&self.root, &op.id);
        super::files::private_dir(&self.root.join("system-staged"))?;
        super::files::private_dir(&staged_root)?;
        for (i, rec) in op.steps.iter_mut().enumerate() {
            let Step::SystemFile { path, before_sha, content, .. } = &rec.step else {
                return Err("Passo inesperado.".into());
            };
            let name = path.rsplit('/').next().unwrap_or("arquivo").to_owned();
            let backup: BackupRef = open_game_file(&rec.step)?.backup(&dir, &format!("{i}-{name}"))?;
            if backup.sha256 != *before_sha {
                return Err(format!("CONFLITO: {path} mudou desde a prévia."));
            }
            rec.backup = Some(backup);
            rec.status = StepStatus::BackedUp;
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let staged = staged_root.join(&name);
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW)
                .open(&staged)
                .map_err(|e| format!("Não foi possível preparar o arquivo: {e}"))?;
            f.write_all(content.as_bytes())
                .and_then(|_| f.sync_all())
                .map_err(|e| e.to_string())?;
            let sha = sha256_hex(content.as_bytes());
            if super::files::sha256_file(&staged)?.as_deref() != Some(sha.as_str()) {
                return Err("O arquivo preparado não confere.".into());
            }
            rec.after_sha = Some(sha);
        }
        self.journal.save(op)
    }

    /// Confere em `/etc` o passo manual (aplicar ou desfazer).
    pub(super) fn verify_system(&self, mut op: Operation, emit: &dyn Fn(&Operation)) -> Result<Operation, String> {
        let mut current = Vec::new();
        for rec in &op.steps {
            current.push(step_sha(&rec.step)?);
        }
        let path = op.steps.iter().find_map(|s| match &s.step {
            Step::SystemFile { path, .. } => Some(path.clone()),
            _ => None,
        }).unwrap_or_default();
        let all_after = op.steps.iter().zip(&current).all(|(s, c)| s.after_sha.is_some() && *c == s.after_sha);
        let all_original = op.steps.iter().zip(&current).all(|(s, c)| {
            s.backup.as_ref().is_some_and(|b| *c == b.sha256)
        });
        match (op.state, op.pending.as_ref().map(|p| p.kind)) {
            (OpState::RequiresManualStep, Some(PendingKind::Apply)) | (OpState::RecoveryRequired, _) => {
                if all_after {
                    for s in &mut op.steps {
                        s.status = StepStatus::Verified;
                    }
                    op.applied_at.get_or_insert_with(now_iso);
                    op.pending = None;
                    self.journal.transition(&mut op, OpState::Applied, format!("Confirmado em {path}: o arquivo tem exatamente o conteúdo preparado."))?;
                    let _ = std::fs::remove_dir_all(staged_dir(&self.root, &op.id));
                } else if all_original && op.state == OpState::RecoveryRequired {
                    self.journal.transition(&mut op, OpState::Failed, "Conferido: a alteração não ficou gravada; nada está aplicado.")?;
                } else if all_original {
                    op.message = Some(format!("{path} ainda está como antes. Rode o comando no terminal e confira de novo."));
                    self.journal.save(&op)?;
                } else {
                    op.message = Some(format!("{path} não corresponde nem ao original nem ao arquivo preparado: ele mudou depois da prévia (o comando do FRX recusa sobrescrever nesse caso). Cancele esta correção e refaça a prévia; o backup foi preservado."));
                    self.journal.save(&op)?;
                }
            }
            (OpState::RequiresManualStep, Some(PendingKind::Rollback)) => {
                if all_original {
                    for s in &mut op.steps {
                        s.status = StepStatus::Reverted;
                    }
                    op.rolled_back_at = Some(now_iso());
                    op.pending = None;
                    self.journal.transition(&mut op, OpState::RolledBack, format!("Confirmado: {path} voltou ao estado anterior à correção."))?;
                    let _ = std::fs::remove_dir_all(staged_dir(&self.root, &op.id));
                } else {
                    op.message = Some(format!("{path} ainda não voltou ao estado anterior. Rode o comando no terminal e confira de novo."));
                    self.journal.save(&op)?;
                }
            }
            _ => return Err("Esta operação não tem nada a conferir.".into()),
        }
        emit(&op);
        Ok(op)
    }

    /// Rollback de sistema: restaura o backup ou remove o arquivo criado.
    /// Com a raiz real, vira um comando `sudo` e uma conferência depois.
    pub(super) fn system_rollback_plan(
        &self,
        op: &Operation,
        pending_apply: bool,
    ) -> Result<(RollbackPlan, RollbackWrites), String> {
        let manual = self.system_manual();
        let mut reasons = Vec::new();
        let mut changes = Vec::new();
        let mut writes = Vec::new();
        let mut commands = Vec::new();
        let mut status = PlanStatus::Ready;
        let mut fp: Vec<String> = vec![op.id.clone(), format!("{:?}", op.state)];
        for rec in &op.steps {
            let Step::SystemFile { path, pre_rollback, post_rollback, .. } = &rec.step else {
                return Err("Passo inesperado.".into());
            };
            let cur = step_sha(&rec.step)?;
            fp.push(cur.clone().unwrap_or_default());
            let backup = rec.backup.as_ref();
            if rec.after_sha.is_some() && cur == rec.after_sha {
                let restore = backup.is_some_and(|b| b.existed);
                changes.push(PlannedChange {
                    kind: ChangeKind::SystemFile,
                    target: path.clone(),
                    before: None,
                    after: None,
                    summary: if restore {
                        format!("restaura {path} do backup")
                    } else {
                        format!("remove {path}, criado pelo FRX")
                    },
                });
                commands.extend(pre_rollback.iter().cloned());
                // Só desfaz se o arquivo ainda for exatamente o que o FRX escreveu.
                let after = rec.after_sha.as_deref().unwrap_or_default();
                match backup {
                    Some(b) if b.existed => commands.extend(install_commands(&b.path, path, b.mode.unwrap_or(0o644), Some(after), &[])),
                    _ => commands.push(remove_command(path, after)),
                }
                commands.extend(post_rollback.iter().cloned());
                writes.push(Some(vec![]));
            } else if pending_apply || backup.is_some_and(|b| cur == b.sha256) {
                // Antes do comando (ou se ele recusou), nada do FRX está no disco: cancelar não mexe no arquivo.
                writes.push(None);
            } else {
                status = PlanStatus::Conflict;
                reasons.push(format!("{path} foi alterado depois da aplicação; o FRX não o substitui pelo backup antigo."));
                writes.push(None);
            }
        }
        let mut manual_line = None;
        if status == PlanStatus::Ready && !changes.is_empty() && manual {
            status = PlanStatus::RequiresManualStep;
            reasons.push("Desfazer em /etc também exige root: rode o comando abaixo no terminal. O FRX confere depois.".into());
            manual_line = Some(commands.join(" && "));
        }
        if changes.is_empty() && status == PlanStatus::Ready {
            reasons.push(if pending_apply {
                "O comando ainda não foi rodado; cancelar não altera nada no sistema.".into()
            } else {
                "Nada a desfazer no disco: as alterações já não estão lá.".into()
            });
        }
        fp.push(format!("{status:?}{manual}"));
        let mode = match status {
            PlanStatus::Ready => Some(ExecMode::Automatic),
            PlanStatus::RequiresManualStep => Some(ExecMode::Manual),
            _ => None,
        };
        Ok((
            RollbackPlan {
                operation_id: op.id.clone(),
                fingerprint: sha256_hex(fp.join("|").as_bytes())[..32].to_owned(),
                status,
                mode,
                reasons,
                changes,
                manual_line,
                steam_running: false,
                game_running: false,
            },
            writes,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(action: SystemAction) -> SystemFixRequest {
        SystemFixRequest {
            action,
            title: "Teste".into(),
            context: "Contexto".into(),
            source_label: "Arch Wiki".into(),
            source_url: "https://wiki.archlinux.org/title/Exemplo".into(),
            laptop: "Laptop de teste".into(),
        }
    }

    fn sysroot() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        for d in ["etc/modprobe.d", "etc/systemd/system", "etc/default"] {
            std::fs::create_dir_all(tmp.path().join(d)).unwrap();
        }
        tmp
    }

    #[test]
    fn validation_rejects_anything_outside_the_closed_grammar() {
        assert!(valid_module("snd_hda_intel") && valid_module("snd-hda-intel"));
        for bad in ["", "-x", "Snd", "a b", "a;b", "a/b", "a$b"] {
            assert!(!valid_module(bad), "{bad}");
        }
        assert!(valid_module_option("model=dell-headset-multi"));
        assert!(valid_module_option("disable_11ax=Y"));
        for bad in ["model", "=x", "a=", "a=b c", "a=$(id)", "a=\"x\"", "a=b;rm", "a=/etc"] {
            assert!(!valid_module_option(bad), "{bad}");
        }
        assert!(valid_kernel_param("nvme.noacpi=1") && valid_kernel_param("iommu=pt"));
        assert!(valid_kernel_param("i915.enable_psr=0"));
        for bad in [
            "init=/bin/sh", "rd.break", "single", "mitigations=off", "selinux=0",
            "memmap=12M$20M", "acpi_osi=\"!Windows 2020\"", "a b", "-x",
        ] {
            assert!(!valid_kernel_param(bad), "{bad}");
        }
        assert!(SystemAction::AddModprobeOption { module: "x".into(), options: vec![] }
            .validate()
            .is_err());
    }

    #[test]
    fn modprobe_renders_a_new_frx_file_and_detects_existing_state() {
        let root = sysroot();
        std::fs::write(
            root.path().join("etc/modprobe.d/alsa-base.conf"),
            "options snd-hda-intel power_save=1\n",
        )
        .unwrap();
        let r = req(SystemAction::AddModprobeOption {
            module: "snd_hda_intel".into(),
            options: vec!["model=dell-headset-multi".into()],
        });
        let out = render(root.path(), &r).unwrap();
        assert_eq!(out.path, "/etc/modprobe.d/frx-snd-hda-intel.conf");
        assert_eq!(out.status, PlanStatus::Ready);
        assert!(out.content.ends_with("options snd_hda_intel model=dell-headset-multi\n"));
        assert!(out.content.starts_with("# Gerado pelo FRX Hardware Fixer. Fonte: https://wiki.archlinux.org/"));
        // snd-hda-intel e snd_hda_intel são o mesmo módulo para o modprobe.
        assert!(out.checks.iter().any(|c| c.state == CheckState::Warn && c.detail.contains("alsa-base.conf")));

        std::fs::write(root.path().join("etc/modprobe.d/frx-snd-hda-intel.conf"), &out.content).unwrap();
        assert_eq!(render(root.path(), &r).unwrap().status, PlanStatus::AlreadyApplied);
        std::fs::write(root.path().join("etc/modprobe.d/frx-snd-hda-intel.conf"), "options x y=1\n").unwrap();
        assert_eq!(render(root.path(), &r).unwrap().status, PlanStatus::Conflict);
    }

    #[test]
    fn systemd_service_comes_from_the_fixed_template() {
        let root = sysroot();
        let r = req(SystemAction::AddSystemdService {
            unit: SystemdUnit::ReloadModuleAroundSleep { module: "iwlwifi".into() },
        });
        let out = render(root.path(), &r).unwrap();
        assert_eq!(out.path, "/etc/systemd/system/frx-iwlwifi-sleep.service");
        assert!(out.content.contains("ExecStart=/usr/bin/modprobe -r iwlwifi\n"));
        assert!(out.content.contains("WantedBy=sleep.target\n"));
        assert_eq!(out.post_apply, ["sudo systemctl daemon-reload", "sudo systemctl enable frx-iwlwifi-sleep.service"]);
        assert_eq!(out.pre_rollback, ["sudo systemctl disable frx-iwlwifi-sleep.service"]);
    }

    #[test]
    fn grub_edits_only_the_default_cmdline_and_refuses_unsafe_lines() {
        let root = sysroot();
        let grub = root.path().join("etc/default/grub");
        let r = req(SystemAction::AddKernelParameters { params: vec!["iommu=pt".into()] });

        // Sem GRUB: incompatível, nada a escrever.
        assert_eq!(render(root.path(), &r).unwrap().status, PlanStatus::Incompatible);

        std::fs::write(&grub, "GRUB_DEFAULT=0\nGRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3 quiet\"\nGRUB_CMDLINE_LINUX=\"\"\n").unwrap();
        let out = render(root.path(), &r).unwrap();
        assert_eq!(out.status, PlanStatus::Ready);
        assert_eq!(
            out.content,
            "GRUB_DEFAULT=0\nGRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3 quiet iommu=pt\"\nGRUB_CMDLINE_LINUX=\"\"\n"
        );
        assert_eq!(out.change.before.as_deref(), Some("GRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3 quiet\""));
        assert!(out.post_apply[0].starts_with("sudo grub-mkconfig"));

        std::fs::write(&grub, &out.content).unwrap();
        assert_eq!(render(root.path(), &r).unwrap().status, PlanStatus::AlreadyApplied);

        for unsafe_text in [
            "GRUB_CMDLINE_LINUX_DEFAULT=\"quiet $extra\"\n",
            "GRUB_CMDLINE_LINUX_DEFAULT='quiet'\n",
            "GRUB_CMDLINE_LINUX_DEFAULT=\"quiet\"\nGRUB_CMDLINE_LINUX_DEFAULT=\"splash\"\n",
            "GRUB_CMDLINE_LINUX_DEFAULT=\"iommu=on\"\n",
        ] {
            std::fs::write(&grub, unsafe_text).unwrap();
            assert_eq!(render(root.path(), &r).unwrap().status, PlanStatus::Conflict, "{unsafe_text}");
        }
    }

    #[test]
    fn install_commands_quote_paths() {
        let cmds = install_commands(
            Path::new("/home/a b/it's/frx-x.conf"),
            "/etc/modprobe.d/frx-x.conf",
            0o644,
            None,
            &[],
        );
        assert_eq!(
            cmds[0],
            r"sudo install -m 0644 -o root -g root -- '/home/a b/it'\''s/frx-x.conf' '/etc/modprobe.d/frx-x.conf'"
        );
    }

    // ── Fix Engine com raiz de sistema fictícia ─────────────────────────

    use crate::fix_engine::catalog::Catalog;

    struct Env {
        _tmp: tempfile::TempDir,
        sys: PathBuf,
        engine: FixEngine,
    }

    fn env(manual: bool) -> Env {
        let tmp = tempfile::tempdir().unwrap();
        let sys = tmp.path().join("sys");
        for d in ["etc/modprobe.d", "etc/systemd/system", "etc/default"] {
            std::fs::create_dir_all(sys.join(d)).unwrap();
        }
        // /proc fictício com a Steam aberta: correções de sistema não dependem dela.
        let proc_root = tmp.path().join("proc");
        std::fs::create_dir_all(proc_root.join("4242")).unwrap();
        std::fs::write(proc_root.join("4242/comm"), "steam\n").unwrap();
        std::fs::write(proc_root.join("4242/cmdline"), "/usr/bin/steam\0").unwrap();
        let engine = FixEngine::with(tmp.path().join("data"), Catalog::bundled(), proc_root)
            .with_system_root(sys.clone(), manual);
        Env { _tmp: tmp, sys, engine }
    }

    fn audio() -> SystemFixRequest {
        req(SystemAction::AddModprobeOption {
            module: "snd_hda_intel".into(),
            options: vec!["model=dell-headset-multi".into()],
        })
    }

    fn noop(_: &Operation) {}

    #[test]
    fn modprobe_option_is_applied_verified_and_rolled_back_with_a_fake_root() {
        let e = env(false);
        let file = e.sys.join("etc/modprobe.d/frx-snd-hda-intel.conf");
        let plan = e.engine.prepare_system(audio()).unwrap();
        assert_eq!(plan.status, PlanStatus::Ready, "{:?}", plan.reasons);
        assert_eq!(plan.mode, Some(ExecMode::Automatic));
        assert!(!file.exists(), "a prévia não escreve");

        let op = e.engine.apply_system(&plan.plan_id, &plan.fingerprint, &noop).unwrap();
        assert_eq!(op.state, OpState::Applied, "{:?}", op.message);
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.ends_with("options snd_hda_intel model=dell-headset-multi\n"));
        assert!(op.steps[0].backup.as_ref().is_some_and(|b| !b.existed));
        // A mesma prévia não pode ser usada duas vezes.
        assert!(e.engine.apply_system(&plan.plan_id, &plan.fingerprint, &noop).is_err());
        // Aplicar de novo: o núcleo reconhece que já está aplicado.
        assert_eq!(e.engine.prepare_system(audio()).unwrap().status, PlanStatus::AlreadyApplied);

        let rb = e.engine.prepare_rollback(&op.id).unwrap();
        assert_eq!(rb.status, PlanStatus::Ready, "{:?}", rb.reasons);
        assert_eq!(rb.changes[0].summary, "remove /etc/modprobe.d/frx-snd-hda-intel.conf, criado pelo FRX");
        let op = e.engine.rollback(&op.id, &rb.fingerprint, &noop).unwrap();
        assert_eq!(op.state, OpState::RolledBack, "{:?}", op.message);
        assert!(!file.exists());
        assert_eq!(std::fs::read_dir(e.sys.join("etc/modprobe.d")).unwrap().count(), 0, "nenhum resto no diretório");
    }

    #[test]
    fn manual_mode_stages_the_file_and_only_counts_after_verification() {
        let e = env(true);
        let file = e.sys.join("etc/modprobe.d/frx-snd-hda-intel.conf");
        let plan = e.engine.prepare_system(audio()).unwrap();
        assert_eq!(plan.status, PlanStatus::RequiresManualStep);
        let line = plan.manual_line.clone().unwrap();
        assert!(line.starts_with("sudo install -m 0644 -o root -g root -- '"), "{line}");
        assert!(line.ends_with("'/etc/modprobe.d/frx-snd-hda-intel.conf'"), "{line}");

        let op = e.engine.apply_system(&plan.plan_id, &plan.fingerprint, &noop).unwrap();
        assert_eq!(op.state, OpState::RequiresManualStep);
        assert!(!file.exists(), "o FRX nunca grava no sistema em modo manual");
        let staged = staged_dir(e.engine.root(), &op.id).join("frx-snd-hda-intel.conf");
        let staged_text = std::fs::read_to_string(&staged).unwrap();

        // Antes do comando: a conferência não aceita.
        let op = e.engine.verify(&op.id, &noop).unwrap();
        assert_eq!(op.state, OpState::RequiresManualStep);
        assert!(op.message.unwrap().contains("ainda está como antes"));

        // Arquivo adulterado depois da prévia: não conta como aplicado.
        std::fs::write(&file, "options snd_hda_intel model=outra\n").unwrap();
        let op = e.engine.verify(&op.id, &noop).unwrap();
        assert_eq!(op.state, OpState::RequiresManualStep);
        std::fs::remove_file(&file).unwrap();

        // O usuário roda o `sudo install` (simulado com uma cópia).
        std::fs::write(&file, &staged_text).unwrap();
        let op = e.engine.verify(&op.id, &noop).unwrap();
        assert_eq!(op.state, OpState::Applied, "{:?}", op.message);
        assert!(!staged.exists(), "o arquivo preparado é removido depois de conferido");

        // Desfazer também é manual: comando de remoção e conferência.
        let rb = e.engine.prepare_rollback(&op.id).unwrap();
        assert_eq!(rb.status, PlanStatus::RequiresManualStep);
        let rm = rb.manual_line.clone().unwrap();
        assert!(rm.starts_with("sudo sh -c '"), "{rm}");
        assert!(rm.ends_with(&format!(" rm -- \"$2\"' frx {} '/etc/modprobe.d/frx-snd-hda-intel.conf'", sha256_hex(staged_text.as_bytes()))), "{rm}");
        let op = e.engine.rollback(&op.id, &rb.fingerprint, &noop).unwrap();
        assert_eq!(op.state, OpState::RequiresManualStep);
        assert!(file.exists(), "o FRX não remove sozinho");
        std::fs::remove_file(&file).unwrap();
        let op = e.engine.verify(&op.id, &noop).unwrap();
        assert_eq!(op.state, OpState::RolledBack, "{:?}", op.message);
    }

    #[test]
    fn a_pending_manual_apply_can_be_cancelled_without_touching_the_system() {
        let e = env(true);
        let plan = e.engine.prepare_system(audio()).unwrap();
        let op = e.engine.apply_system(&plan.plan_id, &plan.fingerprint, &noop).unwrap();
        // Enquanto pendente, outra prévia do mesmo arquivo é conflito.
        assert_eq!(e.engine.prepare_system(audio()).unwrap().status, PlanStatus::Conflict);
        let rb = e.engine.prepare_rollback(&op.id).unwrap();
        assert!(rb.changes.is_empty());
        let op = e.engine.rollback(&op.id, &rb.fingerprint, &noop).unwrap();
        assert_eq!(op.state, OpState::Failed);
        assert!(!staged_dir(e.engine.root(), &op.id).exists());
        assert_eq!(e.engine.prepare_system(audio()).unwrap().status, PlanStatus::RequiresManualStep);
    }

    #[test]
    fn rollback_refuses_to_replace_a_file_changed_after_apply() {
        let e = env(false);
        let plan = e.engine.prepare_system(audio()).unwrap();
        let op = e.engine.apply_system(&plan.plan_id, &plan.fingerprint, &noop).unwrap();
        let file = e.sys.join("etc/modprobe.d/frx-snd-hda-intel.conf");
        std::fs::write(&file, "# editado pelo usuário\n").unwrap();
        let rb = e.engine.prepare_rollback(&op.id).unwrap();
        assert_eq!(rb.status, PlanStatus::Conflict);
        assert!(e.engine.rollback(&op.id, &rb.fingerprint, &noop).is_err());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "# editado pelo usuário\n");
    }

    const GRUB: &str = "# config\nGRUB_TIMEOUT=5\nGRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3 quiet\"\n";

    fn suspend() -> SystemFixRequest {
        req(SystemAction::AddKernelParameters { params: vec!["nvme.noacpi=1".into()] })
    }

    #[test]
    fn grub_edited_between_preview_and_confirmation_is_refused() {
        let e = env(true);
        let grub = e.sys.join("etc/default/grub");
        std::fs::write(&grub, GRUB).unwrap();
        let plan = e.engine.prepare_system(suspend()).unwrap();
        assert_eq!(plan.status, PlanStatus::RequiresManualStep);
        // Outro programa edita o grub enquanto a prévia está aberta.
        let edited = GRUB.replace("GRUB_TIMEOUT=5", "GRUB_TIMEOUT=10");
        std::fs::write(&grub, &edited).unwrap();
        let err = e.engine.apply_system(&plan.plan_id, &plan.fingerprint, &noop).unwrap_err();
        assert_eq!(err, FILE_CHANGED);
        assert_eq!(std::fs::read_to_string(&grub).unwrap(), edited, "a edição externa foi preservada");
        assert!(!e.engine.root().join("system-staged").exists(), "nenhum arquivo preparado");
        assert!(e.engine.operations(Some(SYSTEM_APP_ID)).is_empty(), "nenhum comando liberado");
        // Refazer a prévia lê o arquivo novo e mantém a edição externa.
        let again = e.engine.prepare_system(suspend()).unwrap();
        assert!(again.changes[0].after.as_deref().unwrap().contains("nvme.noacpi=1"));
    }

    #[test]
    fn same_hash_releases_a_guarded_install_and_the_line_is_verified() {
        let e = env(true);
        let grub = e.sys.join("etc/default/grub");
        std::fs::write(&grub, GRUB).unwrap();
        let plan = e.engine.prepare_system(suspend()).unwrap();
        assert_eq!(plan.status, PlanStatus::RequiresManualStep, "{:?}", plan.reasons);
        assert_eq!(plan.risk, Risk::High);
        let line = plan.manual_line.clone().unwrap();
        let original_sha = sha256_hex(GRUB.as_bytes());
        assert!(line.starts_with("sudo sh -c 'echo \"$1  $2\" | sha256sum --check --status ||"), "{line}");
        assert!(line.contains(&format!("' frx {original_sha} '/etc/default/grub' '")), "{line}");
        assert!(line.ends_with(" && sudo grub-mkconfig -o /boot/grub/grub.cfg"), "{line}");
        assert!(!line.contains("sed"), "sem sed -i");

        let op = e.engine.apply_system(&plan.plan_id, &plan.fingerprint, &noop).unwrap();
        assert_eq!(op.state, OpState::RequiresManualStep);
        assert_eq!(std::fs::read_to_string(&grub).unwrap(), GRUB, "o FRX não grava o grub");
        let staged = std::fs::read_to_string(staged_dir(e.engine.root(), &op.id).join("grub")).unwrap();
        assert_eq!(staged, GRUB.replace("\"loglevel=3 quiet\"", "\"loglevel=3 quiet nvme.noacpi=1\""), "só a linha muda");

        // O usuário roda o comando (simulado) e o FRX confere o hash final.
        std::fs::write(&grub, &staged).unwrap();
        let op = e.engine.verify(&op.id, &noop).unwrap();
        assert_eq!(op.state, OpState::Applied, "{:?}", op.message);

        // Desfazer: restaura o backup só se o grub ainda for o que o FRX escreveu.
        let rb = e.engine.prepare_rollback(&op.id).unwrap();
        let undo = rb.manual_line.clone().unwrap();
        assert!(undo.contains(&format!("' frx {} '/etc/default/grub' '", sha256_hex(staged.as_bytes()))), "{undo}");
        let op = e.engine.rollback(&op.id, &rb.fingerprint, &noop).unwrap();
        let backup = op.steps[0].backup.clone().unwrap();
        assert_eq!(std::fs::read_to_string(&backup.path).unwrap(), GRUB);
        std::fs::write(&grub, GRUB).unwrap();
        assert_eq!(e.engine.verify(&op.id, &noop).unwrap().state, OpState::RolledBack);
    }

    #[test]
    fn rollback_refuses_when_grub_is_no_longer_what_frx_wrote() {
        let e = env(false);
        let grub = e.sys.join("etc/default/grub");
        std::fs::write(&grub, GRUB).unwrap();
        let plan = e.engine.prepare_system(suspend()).unwrap();
        let op = e.engine.apply_system(&plan.plan_id, &plan.fingerprint, &noop).unwrap();
        assert_eq!(op.state, OpState::Applied, "{:?}", op.message);
        let mut after = std::fs::read_to_string(&grub).unwrap();
        after.push_str("GRUB_DISABLE_OS_PROBER=false\n");
        std::fs::write(&grub, &after).unwrap();
        let rb = e.engine.prepare_rollback(&op.id).unwrap();
        assert_eq!(rb.status, PlanStatus::Conflict);
        assert!(e.engine.rollback(&op.id, &rb.fingerprint, &noop).is_err());
        assert_eq!(std::fs::read_to_string(&grub).unwrap(), after);
    }

    #[test]
    fn a_pending_grub_apply_can_be_cancelled_after_the_command_refused() {
        let e = env(true);
        let grub = e.sys.join("etc/default/grub");
        std::fs::write(&grub, GRUB).unwrap();
        let plan = e.engine.prepare_system(suspend()).unwrap();
        let op = e.engine.apply_system(&plan.plan_id, &plan.fingerprint, &noop).unwrap();
        // O grub mudou depois do comando aparecer; o comando recusaria.
        std::fs::write(&grub, "GRUB_CMDLINE_LINUX_DEFAULT=\"quiet\"\n").unwrap();
        let op = e.engine.verify(&op.id, &noop).unwrap();
        assert!(op.message.unwrap().contains("Cancele esta correção e refaça a prévia"));
        let rb = e.engine.prepare_rollback(&op.id).unwrap();
        assert!(rb.changes.is_empty(), "{:?}", rb.reasons);
        assert_eq!(e.engine.rollback(&op.id, &rb.fingerprint, &noop).unwrap().state, OpState::Failed);
        assert_eq!(std::fs::read_to_string(&grub).unwrap(), "GRUB_CMDLINE_LINUX_DEFAULT=\"quiet\"\n");
    }

    #[test]
    fn grub_is_restored_byte_for_byte_with_a_fake_root() {
        let e = env(false);
        let grub = e.sys.join("etc/default/grub");
        std::fs::write(&grub, GRUB).unwrap();
        let plan = e.engine.prepare_system(suspend()).unwrap();
        let op = e.engine.apply_system(&plan.plan_id, &plan.fingerprint, &noop).unwrap();
        assert_eq!(op.state, OpState::Applied, "{:?}", op.message);
        let rb = e.engine.prepare_rollback(&op.id).unwrap();
        e.engine.rollback(&op.id, &rb.fingerprint, &noop).unwrap();
        assert_eq!(std::fs::read_to_string(&grub).unwrap(), GRUB);
    }

    /// Roda o comando gerado num shell de verdade (sem `sudo`).
    fn run_without_sudo(cmd: &str) -> std::process::Output {
        let script = cmd.strip_prefix("sudo ").expect("comando começa com sudo");
        std::process::Command::new("sh").arg("-c").arg(script).output().unwrap()
    }

    #[test]
    fn the_guard_runs_in_a_real_shell_and_refuses_a_changed_file() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("it's a file.conf");
        std::fs::write(&target, "conteúdo do FRX\n").unwrap();
        let sha = sha256_hex(b"conteudo do FRX\n"); // hash de outra versão
        let out = run_without_sudo(&remove_command(&target.to_string_lossy(), &sha));
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("mudou desde a prévia; nada foi alterado"));
        assert!(target.exists(), "arquivo alterado não é removido");

        let sha = sha256_hex("conteúdo do FRX\n".as_bytes());
        let out = run_without_sudo(&remove_command(&target.to_string_lossy(), &sha));
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert!(!target.exists(), "com o hash certo, o comando age");

        // install: a guarda aborta antes de tocar no destino.
        let dst = tmp.path().join("grub");
        let src = tmp.path().join("staged grub");
        std::fs::write(&dst, "editado por outro programa\n").unwrap();
        std::fs::write(&src, "versão do FRX\n").unwrap();
        let cmd = &install_commands(&src, &dst.to_string_lossy(), 0o644, Some(&sha256_hex(b"original\n")), &[])[0];
        let out = run_without_sudo(cmd);
        assert!(!out.status.success());
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "editado por outro programa\n");
        // Com o hash certo a guarda passa (o install falha só por não ser root aqui).
        let cmd = &install_commands(&src, &dst.to_string_lossy(), 0o644, Some(&sha256_hex(b"editado por outro programa\n")), &[])[0];
        let out = run_without_sudo(cmd);
        assert!(!String::from_utf8_lossy(&out.stderr).contains("mudou desde a prévia"));
    }

    #[test]
    fn a_stale_preview_is_refused_without_writing() {
        let e = env(false);
        let plan = e.engine.prepare_system(audio()).unwrap();
        let file = e.sys.join("etc/modprobe.d/frx-snd-hda-intel.conf");
        std::fs::write(&file, "options outro x=1\n").unwrap();
        assert!(e.engine.apply_system(&plan.plan_id, &plan.fingerprint, &noop).is_err());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "options outro x=1\n");
    }

    /// Lê o `/etc` de verdade (sem escrever). `cargo test -- --ignored real_root`
    #[test]
    #[ignore = "lê o /etc real"]
    fn real_root_is_always_manual_and_never_writes_etc() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = FixEngine::with(tmp.path().join("data"), Catalog::bundled(), PathBuf::from("/proc"));
        let r = req(SystemAction::AddModprobeOption {
            module: "frx_teste_inexistente".into(),
            options: vec!["x=1".into()],
        });
        let path = Path::new("/etc/modprobe.d/frx-frx-teste-inexistente.conf");
        assert!(!path.exists());
        let plan = engine.prepare_system(r).unwrap();
        assert_eq!(plan.status, PlanStatus::RequiresManualStep, "{:?}", plan.reasons);
        assert_eq!(plan.mode, Some(ExecMode::Manual));
        let op = engine.apply_system(&plan.plan_id, &plan.fingerprint, &noop).unwrap();
        assert_eq!(op.state, OpState::RequiresManualStep);
        assert!(!path.exists(), "o FRX não grava em /etc");
        println!("{}", op.pending.unwrap().line);
        // Cancelar antes do comando: nada a desfazer no sistema.
        let rb = engine.prepare_rollback(&op.id).unwrap();
        assert_eq!(engine.rollback(&op.id, &rb.fingerprint, &noop).unwrap().state, OpState::Failed);
    }

    /// Busca real (Framework 13, Suspensão) → prévia com grub fictício e com o
    /// `/` real. `cargo test -- --ignored framework_suspend_live --nocapture`
    #[tokio::test]
    #[ignore = "rede e /etc real"]
    async fn framework_suspend_live() {
        use crate::fixes::providers::arch_wiki::{search, HardwareSymptom};
        let tmp = tempfile::tempdir().unwrap();
        let cache = crate::cache::Cache::new(tmp.path());
        let st = crate::fixes::http::EngineState::default();
        let f = crate::net::ReqwestFetcher::new().unwrap();
        let ctx = crate::fixes::http::Ctx { fetcher: &f, cache: &cache, state: &st, ttl_secs: 3600, force: true };
        let q = crate::fixes::providers::LaptopQuery {
            identity: Some(crate::system::LaptopIdentity {
                vendor: Some("Framework".into()),
                model: Some("Laptop (13th Gen Intel Core)".into()),
                board_name: None,
                product_version: None,
                display_name: Some("Framework Laptop 13".into()),
            }),
        };
        let r = search(&ctx, &q, HardwareSymptom::Suspend).await;
        let (s, a) = r.suggestions.iter().find_map(|s| s.actions.first().map(|a| (s, a))).expect("ação");
        println!("seção: {:?} · {} · condicional={}", s.section, a.snippet, a.conditional);
        let request = SystemFixRequest {
            action: a.action.clone(),
            title: a.snippet.clone(),
            context: a.context.clone(),
            source_label: "Arch Wiki".into(),
            source_url: s.url.clone(),
            laptop: "Framework Laptop 13".into(),
        };
        // Máquina com GRUB (raiz fictícia, modo manual como no sistema real).
        let e = env(true);
        std::fs::write(e.sys.join("etc/default/grub"), GRUB).unwrap();
        let plan = e.engine.prepare_system(request.clone()).unwrap();
        println!("com grub: {:?}\n  {}", plan.status, plan.manual_line.clone().unwrap_or_default());
        assert_eq!(plan.status, PlanStatus::RequiresManualStep);
        // Esta máquina, sem /etc/default/grub.
        let real = FixEngine::with(tmp.path().join("data"), Catalog::bundled(), PathBuf::from("/proc"));
        let plan = real.prepare_system(request.clone()).unwrap();
        println!("/ real: {:?} · {:?}", plan.status, plan.reasons);
        let check = real.quick_check(&request.action).unwrap();
        println!("checagem leve no / real: {check:?}");
        if !Path::new("/etc/default/grub").exists() {
            assert_eq!(plan.status, PlanStatus::Incompatible);
            assert!(!check.applicable);
        }
    }

    #[test]
    fn quick_check_only_looks_for_grub_and_only_for_kernel_params() {
        let root = sysroot();
        let kernel = SystemAction::AddKernelParameters { params: vec!["nvme.noacpi=1".into()] };
        let c = quick_check(root.path(), &kernel).unwrap();
        assert!(!c.applicable);
        assert!(c.detail.starts_with("Este sistema não usa GRUB"));
        std::fs::write(root.path().join("etc/default/grub"), "GRUB_CMDLINE_LINUX_DEFAULT=\"quiet\"\n").unwrap();
        assert!(quick_check(root.path(), &kernel).unwrap().applicable);
        let modprobe = SystemAction::AddModprobeOption { module: "iwlwifi".into(), options: vec!["disable_11ax=Y".into()] };
        assert_eq!(quick_check(root.path(), &modprobe), None, "modprobe/systemd não mudam");
    }
}
