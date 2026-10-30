//! Fluxo completo contra uma raiz da Steam falsa (tempdir) — nunca contra a instalação real.

use std::path::PathBuf;

use super::catalog::Catalog;
use super::files::sha256_hex;
use super::models::*;
use super::FixEngine;
use crate::steam::localconfig::{self, fixtures::LOCALCONFIG};

struct Fx {
    _tmp: tempfile::TempDir,
    steam: PathBuf,
    proc_root: PathBuf,
    data: PathBuf,
    config: PathBuf,
    library: PathBuf,
}

const ACCOUNT: u64 = 12345;

#[test]
fn reviewed_wined3d_is_executable_but_warned_before_any_test_or_write() {
    let fx = fixture();
    let engine = fx.engine();
    let game = fx.game();
    let before = std::fs::read(&fx.config).unwrap();
    let preview = engine.prepare("frx.compat.wined3d", &game).unwrap();
    assert_eq!(preview.status, PlanStatus::Ready);
    assert!(preview
        .performance_warnings
        .iter()
        .any(|w| w.contains("OpenGL")));
    assert!(engine.operations(None).is_empty());
    assert_eq!(
        std::fs::read(&fx.config).unwrap(),
        before,
        "triagem é automática e somente leitura"
    );
}

#[test]
fn capture_setup_has_backup_and_selective_rollback_without_becoming_a_fix() {
    use crate::smart_packs::benchmark;
    let fx = fixture();
    let e = fx.engine();
    let g = fx.game();
    let id = "frx.capture.200260";
    let original =
        localconfig::launch_options(&std::fs::read_to_string(&fx.config).unwrap(), "200260")
            .unwrap();
    let preview = e.prepare(id, &g).unwrap();
    assert_eq!(preview.status, PlanStatus::Ready);
    let op = e
        .apply(&preview.plan_id, &preview.fingerprint, &g, &|_| {})
        .unwrap();
    assert_eq!(op.state, OpState::Applied);
    assert!(op
        .steps
        .iter()
        .all(|s| s.backup.as_ref().is_some_and(|b| b.path.is_file())));
    assert!(e.active_for("200260").is_empty());
    let baseline = benchmark::snapshot(&e, "200260");
    assert!(baseline.env.is_empty() && baseline.verified);
    let text = std::fs::read_to_string(&fx.config).unwrap();
    let line = localconfig::launch_options(&text, "200260")
        .unwrap()
        .unwrap();
    assert!(line.contains("MANGOHUD=1") && line.contains("MANGOHUD_CONFIGFILE="));
    // Mudança legítima posterior: só os campos que a operação controla importam.
    let changed =
        localconfig::set_launch_options(&text, "200260", &format!("{line} -new-option")).unwrap();
    std::fs::write(&fx.config, changed).unwrap();
    let restarted = fx.engine();
    let rb = restarted.prepare_rollback(&op.id).unwrap();
    let undone = restarted
        .rollback(&op.id, &rb.fingerprint, &|_| {})
        .unwrap();
    assert_eq!(undone.state, OpState::RolledBack);
    let line = localconfig::launch_options(&std::fs::read_to_string(&fx.config).unwrap(), "200260")
        .unwrap()
        .unwrap();
    assert!(!line.contains("MANGOHUD_CONFIGFILE=") && !line.contains("MANGOHUD=1"));
    assert_eq!(line, format!("{} -new-option", original.unwrap()));
    assert!(super::security::check_env("MANGOHUD_CONFIGFILE", "/tmp/hostile.conf").is_err());
}

#[test]
fn capture_configuration_changes_and_drift_cannot_be_ranked_as_baseline() {
    use crate::smart_packs::benchmark;
    let fx = fixture();
    let e = fx.engine();
    let g = fx.game();
    let started = benchmark::snapshot(&e, "200260");
    let p = e.prepare("frx.diagnostics.proton-log", &g).unwrap();
    let op = e.apply(&p.plan_id, &p.fingerprint, &g, &|_| {}).unwrap();
    let mut meta = crate::smart_packs::tests::synthetic("changed", &[], 100.0, 50.0, 20.0);
    benchmark::tag_capture(&e, "200260", &started, &mut meta);
    assert!(!meta.smart_pack.as_ref().unwrap().verified);
    assert!(meta.quality.issues.iter().any(|i| i.code == "pack-changed"));
    let text = std::fs::read_to_string(&fx.config).unwrap();
    let changed = localconfig::set_launch_options(&text, "200260", "%command%").unwrap();
    std::fs::write(&fx.config, changed).unwrap();
    assert!(!benchmark::snapshot(&e, "200260").verified);
    assert_eq!(e.operation(&op.id).unwrap().state, OpState::Applied);
}

#[test]
fn pending_manual_apply_makes_the_capture_configuration_unknown() {
    let fx = fixture();
    let process = fx.proc_root.join("42");
    std::fs::create_dir_all(&process).unwrap();
    std::fs::write(process.join("comm"), "steam\n").unwrap();
    let e = fx.engine();
    let g = fx.game();
    let p = e.prepare("frx.diagnostics.proton-log", &g).unwrap();
    let op = e.apply(&p.plan_id, &p.fingerprint, &g, &|_| {}).unwrap();
    assert_eq!(op.state, OpState::RequiresManualStep);
    assert!(!crate::smart_packs::benchmark::snapshot(&e, "200260").verified);
}

fn fixture() -> Fx {
    let tmp = tempfile::tempdir().unwrap();
    let steam = tmp.path().join("Steam");
    std::fs::create_dir_all(steam.join("config")).unwrap();
    std::fs::write(
        steam.join("config/loginusers.vdf"),
        format!(
            "\"users\" {{ \"{}\" {{ \"MostRecent\" \"1\" }} }}",
            76561197960265728u64 + ACCOUNT
        ),
    )
    .unwrap();
    let cfg_dir = steam.join(format!("userdata/{ACCOUNT}/config"));
    std::fs::create_dir_all(&cfg_dir).unwrap();
    let config = cfg_dir.join("localconfig.vdf");
    std::fs::write(&config, LOCALCONFIG).unwrap();
    let proc_root = tmp.path().join("proc");
    std::fs::create_dir_all(&proc_root).unwrap();
    let library = tmp.path().join("lib/steamapps/common/Batman");
    std::fs::create_dir_all(library.join("Config")).unwrap();
    Fx {
        data: tmp.path().join("data/fix-engine"),
        steam,
        proc_root,
        config,
        library,
        _tmp: tmp,
    }
}

impl Fx {
    fn engine(&self) -> FixEngine {
        FixEngine::with(
            self.data.clone(),
            Catalog::bundled(),
            self.proc_root.clone(),
        )
    }
    fn engine_with(&self, c: Catalog) -> FixEngine {
        FixEngine::with(self.data.clone(), c, self.proc_root.clone())
    }
    fn game(&self) -> GameCtx {
        GameCtx {
            app_id: "200260".into(),
            name: "Batman: Arkham City GOTY".into(),
            steam_root: Some(self.steam.clone()),
            install_path: Some(self.library.clone()),
            uses_proton: Some(true),
            tool_name: Some("proton-cachyos-slr".into()),
            tool_includes_protonfixes: true,
            gpu: None,
            ..Default::default()
        }
    }
    fn line(&self) -> Option<String> {
        localconfig::launch_options(&std::fs::read_to_string(&self.config).unwrap(), "200260")
            .unwrap()
    }
    fn set_line(&self, v: &str) {
        let t = std::fs::read_to_string(&self.config).unwrap();
        std::fs::write(
            &self.config,
            localconfig::set_launch_options(&t, "200260", v).unwrap(),
        )
        .unwrap();
    }
    fn proc(&self, pid: &str, comm: &str, environ: &str) {
        let d = self.proc_root.join(pid);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("comm"), comm).unwrap();
        std::fs::write(d.join("cmdline"), "x\0").unwrap();
        std::fs::write(d.join("environ"), environ).unwrap();
    }
    fn kill(&self, pid: &str) {
        std::fs::remove_dir_all(self.proc_root.join(pid)).unwrap();
    }
}

fn noop(_: &Operation) {}

const ORIGINAL: &str = "PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command%";

#[test]
fn apply_verify_and_rollback_restore_the_exact_file() {
    let fx = fixture();
    let e = fx.engine();
    let plan = e.prepare("frx.diagnostics.proton-log", &fx.game()).unwrap();
    assert_eq!(plan.status, PlanStatus::Ready, "{:?}", plan.reasons);
    assert_eq!(plan.mode, Some(ExecMode::Automatic));
    assert_eq!(plan.changes[0].before.as_deref(), Some(ORIGINAL));
    assert_eq!(
        plan.changes[0].after.as_deref(),
        Some("PROTON_LOG=1 PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command%")
    );
    // Dry-run não grava nada.
    assert_eq!(std::fs::read_to_string(&fx.config).unwrap(), LOCALCONFIG);

    let events = std::cell::RefCell::new(vec![]);
    let op = e
        .apply(&plan.plan_id, &plan.fingerprint, &fx.game(), &|o| {
            events.borrow_mut().push(o.state)
        })
        .unwrap();
    assert_eq!(op.state, OpState::Applied, "{:?}", op.message);
    assert_eq!(
        *events.borrow(),
        vec![
            OpState::Checking,
            OpState::BackingUp,
            OpState::Applying,
            OpState::Verifying,
            OpState::Applied
        ]
    );
    assert_eq!(
        fx.line().as_deref(),
        Some("PROTON_LOG=1 PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command%")
    );
    // Backup verificado do arquivo original.
    let b = op.steps[0].backup.as_ref().unwrap();
    assert_eq!(std::fs::read_to_string(&b.path).unwrap(), LOCALCONFIG);
    assert_eq!(
        b.sha256.as_deref(),
        Some(sha256_hex(LOCALCONFIG.as_bytes()).as_str())
    );
    // Pastas privadas.
    use std::os::unix::fs::PermissionsExt;
    for d in ["", "backups", "operations"] {
        assert_eq!(
            std::fs::metadata(fx.data.join(d))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700,
            "{d}"
        );
    }
    // Diário persistente.
    assert_eq!(
        fx.engine().operation(&op.id).unwrap().state,
        OpState::Applied
    );
    let applied = e.applied(Some("200260"));
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0].effective, Effective::Active);
    assert_eq!(
        e.active_for("200260")[0].recipe_id,
        "frx.diagnostics.proton-log"
    );
    // O mesmo plano não pode ser usado duas vezes.
    assert!(e
        .apply(&plan.plan_id, &plan.fingerprint, &fx.game(), &noop)
        .is_err());
    assert_eq!(
        e.check("frx.diagnostics.proton-log", &fx.game())
            .unwrap()
            .status,
        PlanStatus::AlreadyApplied
    );

    let rb = e.prepare_rollback(&op.id).unwrap();
    assert_eq!(rb.status, PlanStatus::Ready, "{:?}", rb.reasons);
    let op = e.rollback(&op.id, &rb.fingerprint, &noop).unwrap();
    assert_eq!(op.state, OpState::RolledBack, "{:?}", op.message);
    assert_eq!(
        std::fs::read_to_string(&fx.config).unwrap(),
        LOCALCONFIG,
        "rollback devolve o arquivo byte a byte"
    );
    assert!(e.applied(None).is_empty());
}

#[test]
fn stale_preview_is_refused_without_writing() {
    let fx = fixture();
    let e = fx.engine();
    let plan = e.prepare("frx.diagnostics.proton-log", &fx.game()).unwrap();
    fx.set_line("mangohud %command%");
    let before = std::fs::read_to_string(&fx.config).unwrap();
    let err = e
        .apply(&plan.plan_id, &plan.fingerprint, &fx.game(), &noop)
        .unwrap_err();
    assert!(err.contains("mudou"), "{err}");
    assert_eq!(std::fs::read_to_string(&fx.config).unwrap(), before);
    // Impressão digital forjada também não passa.
    let plan = e.prepare("frx.diagnostics.proton-log", &fx.game()).unwrap();
    assert!(e.apply(&plan.plan_id, "0000", &fx.game(), &noop).is_err());
    assert!(e.operations(None).is_empty());
}

#[test]
fn rollback_preserves_third_party_changes() {
    let fx = fixture();
    let e = fx.engine();
    let plan = e.prepare("frx.compat.no-esync-fsync", &fx.game()).unwrap();
    let op = e
        .apply(&plan.plan_id, &plan.fingerprint, &fx.game(), &noop)
        .unwrap();
    assert_eq!(op.state, OpState::Applied);
    // O usuário acrescenta MangoHud e a Steam muda outro jogo.
    let cur = fx.line().unwrap();
    fx.set_line(&cur.replace("%command%", "mangohud %command%"));
    let t = std::fs::read_to_string(&fx.config)
        .unwrap()
        .replace("\"Playtime\"\t\t\"10\"", "\"Playtime\"\t\t\"11\"");
    std::fs::write(&fx.config, &t).unwrap();
    let rb = e.prepare_rollback(&op.id).unwrap();
    let op = e.rollback(&op.id, &rb.fingerprint, &noop).unwrap();
    assert_eq!(op.state, OpState::RolledBack);
    assert_eq!(
        fx.line().as_deref(),
        Some("PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 mangohud %command%")
    );
    assert!(std::fs::read_to_string(&fx.config)
        .unwrap()
        .contains("\"Playtime\"\t\t\"11\""));
}

#[test]
fn rollback_refuses_to_overwrite_a_changed_value() {
    let fx = fixture();
    let e = fx.engine();
    let plan = e.prepare("frx.diagnostics.proton-log", &fx.game()).unwrap();
    let op = e
        .apply(&plan.plan_id, &plan.fingerprint, &fx.game(), &noop)
        .unwrap();
    fx.set_line("PROTON_LOG=0 PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command%");
    let before = std::fs::read_to_string(&fx.config).unwrap();
    let rb = e.prepare_rollback(&op.id).unwrap();
    assert_eq!(rb.status, PlanStatus::Conflict);
    assert!(rb.reasons.iter().any(|r| r.contains("não sobrescreve")));
    assert!(e.rollback(&op.id, &rb.fingerprint, &noop).is_err());
    assert_eq!(std::fs::read_to_string(&fx.config).unwrap(), before);
    assert_eq!(e.applied(None)[0].effective, Effective::Drifted);
}

#[test]
fn steam_open_means_manual_step_and_real_verification() {
    let fx = fixture();
    fx.proc("100", "steam\n", "");
    let e = fx.engine();
    let plan = e.prepare("frx.diagnostics.proton-log", &fx.game()).unwrap();
    assert_eq!(plan.status, PlanStatus::RequiresManualStep);
    assert_eq!(
        plan.manual_line.as_deref(),
        Some("PROTON_LOG=1 PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command%")
    );
    let op = e
        .apply(&plan.plan_id, &plan.fingerprint, &fx.game(), &noop)
        .unwrap();
    assert_eq!(op.state, OpState::RequiresManualStep);
    assert_eq!(
        std::fs::read_to_string(&fx.config).unwrap(),
        LOCALCONFIG,
        "nada é gravado com a Steam aberta"
    );
    assert!(
        e.applied(None).is_empty(),
        "não conta como aplicada antes de conferir"
    );
    // Ainda não colado.
    assert_eq!(
        e.verify(&op.id, &noop).unwrap().state,
        OpState::RequiresManualStep
    );
    // O usuário cola a linha e a Steam grava ao fechar.
    fx.set_line(plan.manual_line.as_deref().unwrap());
    fx.kill("100");
    let op = e.verify(&op.id, &noop).unwrap();
    assert_eq!(op.state, OpState::Applied);
    assert_eq!(op.mode, ExecMode::Manual);
    // Rollback com a Steam aberta de novo também é manual.
    fx.proc("101", "steam\n", "");
    let rb = e.prepare_rollback(&op.id).unwrap();
    assert_eq!(rb.status, PlanStatus::RequiresManualStep);
    assert_eq!(rb.manual_line.as_deref(), Some(ORIGINAL));
    let op = e.rollback(&op.id, &rb.fingerprint, &noop).unwrap();
    assert_eq!(op.state, OpState::RequiresManualStep);
    fx.set_line(ORIGINAL);
    assert_eq!(e.verify(&op.id, &noop).unwrap().state, OpState::RolledBack);
}

#[test]
fn pending_manual_apply_can_be_cancelled_without_changes() {
    let fx = fixture();
    fx.proc("100", "steam\n", "");
    let e = fx.engine();
    let plan = e.prepare("frx.diagnostics.proton-log", &fx.game()).unwrap();
    let op = e
        .apply(&plan.plan_id, &plan.fingerprint, &fx.game(), &noop)
        .unwrap();
    let rb = e.prepare_rollback(&op.id).unwrap();
    assert_eq!(rb.status, PlanStatus::Ready);
    assert!(rb.changes.is_empty());
    let op = e.rollback(&op.id, &rb.fingerprint, &noop).unwrap();
    assert_eq!(op.state, OpState::Failed);
    assert_eq!(std::fs::read_to_string(&fx.config).unwrap(), LOCALCONFIG);
}

#[test]
fn compatibility_rules() {
    let fx = fixture();
    let e = fx.engine();
    let mut g = fx.game();
    // Candidata: só consulta.
    assert_eq!(
        e.check("frx.compat.no-d3d11", &g).unwrap().status,
        PlanStatus::NotExecutable
    );
    // Jogo nativo / Proton desconhecido.
    g.uses_proton = Some(false);
    assert_eq!(
        e.check("frx.diagnostics.proton-log", &g).unwrap().status,
        PlanStatus::Incompatible
    );
    g.uses_proton = None;
    assert_eq!(
        e.check("frx.diagnostics.proton-log", &g).unwrap().status,
        PlanStatus::Incompatible
    );
    g.uses_proton = Some(true);
    // Jogo aberto.
    fx.proc("200", "BatmanAC.exe\n", "SteamAppId=200260\0");
    assert_eq!(
        e.check("frx.diagnostics.proton-log", &g).unwrap().status,
        PlanStatus::Conflict
    );
    fx.kill("200");
    // Mesmo valor já presente = nada a fazer; valor diferente = conflito.
    fx.set_line("PROTON_LOG=1 %command%");
    assert_eq!(
        e.check("frx.diagnostics.proton-log", &g).unwrap().status,
        PlanStatus::AlreadyApplied
    );
    fx.set_line("PROTON_LOG=0 %command%");
    assert_eq!(
        e.check("frx.diagnostics.proton-log", &g).unwrap().status,
        PlanStatus::Conflict
    );
    // Linha com operadores: o FRX não edita nem sugere colar.
    fx.set_line("PROTON_LOG=0 %command%; rm -rf ~");
    let p = e.check("frx.compat.wined3d", &g).unwrap();
    assert_eq!(p.status, PlanStatus::Conflict);
    assert!(p.manual_line.is_none());
    // Sem localconfig.
    g.steam_root = Some(fx.steam.join("nao-existe"));
    assert_eq!(
        e.check("frx.compat.wined3d", &g).unwrap().status,
        PlanStatus::Incompatible
    );
    assert!(e.check("frx.nao.existe", &g).is_err());
}

fn test_catalog(recipes: &[&str]) -> Catalog {
    let files: Vec<(String, String)> = recipes
        .iter()
        .enumerate()
        .map(|(i, r)| (format!("r{i}.json"), r.to_string()))
        .collect();
    let manifest = format!(
        r#"{{"schema":1,"catalogVersion":"t","recipes":[{}],"revoked":[],"signature":null}}"#,
        files
            .iter()
            .map(|(n, b)| format!(
                r#"{{"file":"{n}","sha256":"{}"}}"#,
                sha256_hex(b.as_bytes())
            ))
            .collect::<Vec<_>>()
            .join(",")
    );
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_str()))
        .collect();
    let c = Catalog::load(&manifest, &refs);
    assert!(c.issues.is_empty(), "{:?}", c.issues);
    c
}

fn recipe_json(id: &str, actions: &str, extra: &str) -> String {
    format!(
        r#"{{"schema":1,"id":"{id}","version":"1.0.0","trust":"reviewed","title":"{id}","summary":"s","description":"d","category":"other","purpose":"compatibility","risk":"low","scope":{{"appIds":[],"requiresProton":true,"gpuVendor":null}},"actions":[{actions}],"sources":[{{"label":"x","url":"https://example.org"}}],"review":{{"by":"t","at":"t","notes":"t"}}{extra}}}"#
    )
}

#[test]
fn conflicts_between_recipes_and_shared_keys() {
    let fx = fixture();
    let a = recipe_json(
        "frx.t.a",
        r#"{"type":"set_launch_env","key":"PROTON_LOG","value":"1"}"#,
        r#","conflictsWith":["frx.t.b"]"#,
    );
    let b = recipe_json(
        "frx.t.b",
        r#"{"type":"set_launch_env","key":"PROTON_NO_ESYNC","value":"1"}"#,
        "",
    );
    let c = recipe_json(
        "frx.t.c",
        r#"{"type":"set_launch_env","key":"PROTON_LOG","value":"1"},{"type":"add_launch_args","args":["-dx11"]}"#,
        "",
    );
    let e = fx.engine_with(test_catalog(&[&a, &b, &c]));
    let p = e.prepare("frx.t.a", &fx.game()).unwrap();
    e.apply(&p.plan_id, &p.fingerprint, &fx.game(), &noop)
        .unwrap();
    let pb = e.check("frx.t.b", &fx.game()).unwrap();
    assert_eq!(
        pb.status,
        PlanStatus::Conflict,
        "conflictsWith vale nos dois sentidos"
    );
    let pc = e.check("frx.t.c", &fx.game()).unwrap();
    assert_eq!(
        pc.status,
        PlanStatus::Conflict,
        "mesma variável controlada por outra receita"
    );
}

#[test]
fn launch_args_are_added_and_removed_exactly() {
    let fx = fixture();
    let r = recipe_json(
        "frx.t.args",
        r#"{"type":"add_launch_args","args":["-dx11","-nostartupmovies"]}"#,
        "",
    );
    let e = fx.engine_with(test_catalog(&[&r]));
    let p = e.prepare("frx.t.args", &fx.game()).unwrap();
    let op = e
        .apply(&p.plan_id, &p.fingerprint, &fx.game(), &noop)
        .unwrap();
    assert_eq!(
        fx.line().as_deref(),
        Some("PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command% -dx11 -nostartupmovies")
    );
    let rb = e.prepare_rollback(&op.id).unwrap();
    e.rollback(&op.id, &rb.fingerprint, &noop).unwrap();
    assert_eq!(fx.line().as_deref(), Some(ORIGINAL));
}

#[test]
fn game_files_are_confined_backed_up_and_restored() {
    let fx = fixture();
    let r = recipe_json(
        "frx.t.file",
        r#"{"type":"write_game_file","path":"Config/frx.ini","content":"[x]\na=1\n"}"#,
        "",
    );
    let e = fx.engine_with(test_catalog(&[&r]));
    // Criação de arquivo novo e rollback que o remove.
    let p = e.prepare("frx.t.file", &fx.game()).unwrap();
    assert_eq!(p.status, PlanStatus::Ready, "{:?}", p.reasons);
    let op = e
        .apply(&p.plan_id, &p.fingerprint, &fx.game(), &noop)
        .unwrap();
    assert_eq!(op.state, OpState::Applied, "{:?}", op.message);
    let target = fx.library.join("Config/frx.ini");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "[x]\na=1\n");
    let rb = e.prepare_rollback(&op.id).unwrap();
    e.rollback(&op.id, &rb.fingerprint, &noop).unwrap();
    assert!(!target.exists());

    // Substituição de arquivo existente; mudança de terceiros bloqueia o rollback.
    std::fs::write(&target, "orig").unwrap();
    let p = e.prepare("frx.t.file", &fx.game()).unwrap();
    let op = e
        .apply(&p.plan_id, &p.fingerprint, &fx.game(), &noop)
        .unwrap();
    std::fs::write(&target, "editado pelo usuário").unwrap();
    assert_eq!(
        e.prepare_rollback(&op.id).unwrap().status,
        PlanStatus::Conflict
    );
    std::fs::write(&target, "[x]\na=1\n").unwrap();
    let rb = e.prepare_rollback(&op.id).unwrap();
    e.rollback(&op.id, &rb.fingerprint, &noop).unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "orig");

    // Link simbólico para fora da pasta do jogo é recusado.
    std::fs::remove_file(&target).unwrap();
    std::os::unix::fs::symlink(fx.steam.join("config/loginusers.vdf"), &target).unwrap();
    assert_eq!(
        e.check("frx.t.file", &fx.game()).unwrap().status,
        PlanStatus::Conflict
    );
    // Pasta fora de steamapps/common também.
    let mut g = fx.game();
    g.install_path = Some(fx.steam.clone());
    assert_eq!(
        e.check("frx.t.file", &g).unwrap().status,
        PlanStatus::Conflict
    );
}

#[test]
fn concurrent_operations_are_serialized() {
    let fx = fixture();
    let e = fx.engine();
    let p = e.prepare("frx.diagnostics.proton-log", &fx.game()).unwrap();
    let guard = e.exec.lock().unwrap();
    assert!(e.status().busy);
    let err = e
        .apply(&p.plan_id, &p.fingerprint, &fx.game(), &noop)
        .unwrap_err();
    assert!(err.contains("Aguarde"));
    drop(guard);
    assert_eq!(
        e.apply(&p.plan_id, &p.fingerprint, &fx.game(), &noop)
            .unwrap()
            .state,
        OpState::Applied
    );
}

#[test]
fn steam_opening_mid_apply_aborts_before_writing() {
    let fx = fixture();
    let e = fx.engine();
    let p = e.prepare("frx.diagnostics.proton-log", &fx.game()).unwrap();
    // A Steam abre entre a prévia e a gravação: o plano recalculado muda.
    fx.proc("300", "steam\n", "");
    assert!(e
        .apply(&p.plan_id, &p.fingerprint, &fx.game(), &noop)
        .is_err());
    assert_eq!(std::fs::read_to_string(&fx.config).unwrap(), LOCALCONFIG);
}

fn seed_inflight(fx: &Fx, status: StepStatus, write: bool) -> String {
    let e = fx.engine();
    let p = e.prepare("frx.diagnostics.proton-log", &fx.game()).unwrap();
    let op = e
        .apply(&p.plan_id, &p.fingerprint, &fx.game(), &noop)
        .unwrap();
    // Simula um fechamento no meio: volta o diário para "applying".
    let mut op = e.operation(&op.id).unwrap();
    op.state = OpState::Applying;
    op.steps[0].status = status;
    if !write {
        std::fs::write(&fx.config, LOCALCONFIG).unwrap();
    }
    e.journal.save(&op).unwrap();
    op.id
}

#[test]
fn interrupted_operations_are_never_reported_as_done() {
    let fx = fixture();
    let id = seed_inflight(&fx, StepStatus::Written, true);
    let e = fx.engine(); // "reabre o app"
    let op = e.operation(&id).unwrap();
    assert_eq!(op.state, OpState::RecoveryRequired);
    assert_eq!(e.status().recovery_required, 1);
    assert!(e.applied(None).is_empty());
    // Conferir o estado real confirma a gravação.
    assert_eq!(e.verify(&id, &noop).unwrap().state, OpState::Applied);

    let fx2 = fixture();
    let id = seed_inflight(&fx2, StepStatus::BackedUp, false);
    let op = fx2.engine().operation(&id).unwrap();
    assert_eq!(op.state, OpState::Failed);
    assert!(op.message.unwrap().contains("Nada mudou"));
}

// ── Receitas geradas dinamicamente (sem cadastro no catálogo) ───────────

mod generated {
    use super::*;
    use crate::fix_engine::dynamic::tests::{batman_no_d3d11, finding};
    use crate::fix_engine::dynamic::{DynStatus, EvidenceLevel};
    use crate::fixes::models::ActionKind;

    fn game_with(fx: &Fx, findings: Vec<crate::fixes::models::FixFinding>) -> GameCtx {
        GameCtx {
            findings,
            ..fx.game()
        }
    }

    /// Descoberta → gerador → validação → plano → backup → aplicação →
    /// verificação → histórico → Performance Lab → rollback.
    #[test]
    fn discovery_to_rollback_without_any_catalog_entry() {
        let fx = fixture();
        let e = fx.engine();
        let game = game_with(&fx, vec![batman_no_d3d11()]);
        let views = e.analyze(&game).unwrap();
        assert_eq!(views[0].assessment.status, DynStatus::Generated);
        let id = views[0].assessment.recipe_id.clone().unwrap();
        assert!(e.recipe(&id).is_err(), "não existe no catálogo");

        let plan = e.prepare(&id, &game).unwrap();
        assert_eq!(plan.status, PlanStatus::Ready, "{:?}", plan.reasons);
        assert_eq!(plan.trust, Trust::Generated);
        assert!(plan
            .checks
            .iter()
            .any(|c| c.label == "Receita gerada" && c.state == CheckState::Warn));
        assert_eq!(
            plan.generated.as_ref().unwrap().evidence,
            EvidenceLevel::Single
        );
        assert!(plan
            .sources
            .iter()
            .any(|s| s.url.ends_with("issuecomment-435132818")));
        assert_eq!(
            plan.changes[0].after.as_deref(),
            Some("PROTON_NO_D3D11=1 PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command%")
        );
        assert_eq!(
            std::fs::read_to_string(&fx.config).unwrap(),
            LOCALCONFIG,
            "prévia não grava"
        );

        let op = e
            .apply(&plan.plan_id, &plan.fingerprint, &game, &noop)
            .unwrap();
        assert_eq!(op.state, OpState::Applied, "{:?}", op.message);
        assert_eq!(op.recipe_trust, Some(Trust::Generated));
        assert_eq!(op.finding_ids, vec!["github-200260-0751180507"]);
        let b = op.steps[0].backup.as_ref().unwrap();
        assert_eq!(
            b.sha256.as_deref(),
            Some(sha256_hex(LOCALCONFIG.as_bytes()).as_str())
        );
        assert_eq!(
            fx.line().as_deref(),
            Some("PROTON_NO_D3D11=1 PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command%")
        );

        // Reinício do app: histórico, estado real e Performance Lab continuam.
        let e = fx.engine();
        assert_eq!(
            e.operations(Some("200260"))[0].recipe_trust,
            Some(Trust::Generated)
        );
        assert_eq!(e.applied(Some("200260"))[0].effective, Effective::Active);
        assert_eq!(e.active_for("200260")[0].recipe_id, id);
        assert_eq!(
            e.prepare(&id, &game).unwrap().status,
            PlanStatus::AlreadyApplied
        );

        let rb = e.prepare_rollback(&op.id).unwrap();
        let done = e.rollback(&op.id, &rb.fingerprint, &noop).unwrap();
        assert_eq!(done.state, OpState::RolledBack, "{:?}", done.message);
        assert_eq!(
            std::fs::read_to_string(&fx.config).unwrap(),
            LOCALCONFIG,
            "arquivo idêntico ao original"
        );
        assert!(e.active_for("200260").is_empty());
    }

    #[test]
    fn findings_or_config_changing_after_the_preview_block_the_apply() {
        let fx = fixture();
        let e = fx.engine();
        let game = game_with(&fx, vec![batman_no_d3d11()]);
        let id = e.analyze(&game).unwrap()[0]
            .assessment
            .recipe_id
            .clone()
            .unwrap();

        // A pesquisa foi refeita e a descoberta sumiu.
        let p = e.prepare(&id, &game).unwrap();
        assert!(e
            .apply(&p.plan_id, &p.fingerprint, &game_with(&fx, vec![]), &noop)
            .is_err());
        // Agora há relatos contrários.
        let p = e.prepare(&id, &game).unwrap();
        let mut contested = batman_no_d3d11();
        contested.evidence.negative = 2;
        assert!(e
            .apply(
                &p.plan_id,
                &p.fingerprint,
                &game_with(&fx, vec![contested]),
                &noop
            )
            .is_err());
        // Mais um relato favorável: a receita muda (evidência) → nova prévia exigida.
        let p = e.prepare(&id, &game).unwrap();
        let mut stronger = batman_no_d3d11();
        stronger.evidence.positive = 3;
        assert!(e
            .apply(
                &p.plan_id,
                &p.fingerprint,
                &game_with(&fx, vec![stronger]),
                &noop
            )
            .is_err());
        // As opções de inicialização mudaram.
        let p = e.prepare(&id, &game).unwrap();
        fx.set_line("PROTON_NVIDIA_LIBS=1 %command%");
        assert!(e.apply(&p.plan_id, &p.fingerprint, &game, &noop).is_err());
        assert_eq!(fx.line().as_deref(), Some("PROTON_NVIDIA_LIBS=1 %command%"));
        assert!(
            e.operations(None).is_empty(),
            "nada foi registrado como aplicado"
        );
    }

    #[test]
    fn cancel_conflicts_and_already_included() {
        let fx = fixture();
        let e = fx.engine();
        let esync = finding("1", ActionKind::Env, "PROTON_NO_ESYNC=1", 2, 0);
        let game = game_with(&fx, vec![esync]);
        let id = e.analyze(&game).unwrap()[0]
            .assessment
            .recipe_id
            .clone()
            .unwrap();
        // Cancelar = não confirmar: a prévia expira sem gravar nada.
        let _ = e.prepare(&id, &game).unwrap();
        assert_eq!(std::fs::read_to_string(&fx.config).unwrap(), LOCALCONFIG);

        let p = e.prepare(&id, &game).unwrap();
        assert_eq!(
            e.apply(&p.plan_id, &p.fingerprint, &game, &noop)
                .unwrap()
                .state,
            OpState::Applied
        );
        // A receita revisada mexe na mesma variável: conflito.
        let other = e.prepare("frx.compat.no-esync-fsync", &game).unwrap();
        assert_eq!(other.status, PlanStatus::Conflict, "{:?}", other.reasons);

        // Script do UMU que a ferramenta do jogo já aplica sozinha.
        let mut pf = finding("2", ActionKind::Env, "PROTON_HEAP_DELAY_FREE=1", 0, 0);
        pf.kind = crate::fixes::models::FindingKind::Protonfix;
        pf.confidence = crate::fixes::models::Confidence::Curated;
        let g2 = game_with(&fx, vec![pf]);
        let id2 = e.analyze(&g2).unwrap()[0]
            .assessment
            .recipe_id
            .clone()
            .unwrap();
        assert_eq!(
            e.prepare(&id2, &g2).unwrap().status,
            PlanStatus::AlreadyIncluded
        );
    }

    #[test]
    fn steam_open_means_manual_step_for_generated_recipes_too() {
        let fx = fixture();
        let e = fx.engine();
        let game = game_with(&fx, vec![batman_no_d3d11()]);
        let id = e.analyze(&game).unwrap()[0]
            .assessment
            .recipe_id
            .clone()
            .unwrap();
        fx.proc("300", "steam\n", "");
        let p = e.prepare(&id, &game).unwrap();
        assert_eq!(p.status, PlanStatus::RequiresManualStep);
        let op = e.apply(&p.plan_id, &p.fingerprint, &game, &noop).unwrap();
        assert_eq!(op.state, OpState::RequiresManualStep);
        assert_eq!(
            std::fs::read_to_string(&fx.config).unwrap(),
            LOCALCONFIG,
            "nada gravado com a Steam aberta"
        );
        assert!(e.applied(None).is_empty(), "não conta como aplicada");
        // O usuário cola a linha e fecha a Steam; o FRX confere no disco.
        fx.set_line(&op.pending.as_ref().unwrap().line);
        fx.kill("300");
        assert_eq!(e.verify(&op.id, &noop).unwrap().state, OpState::Applied);
    }

    #[test]
    fn interrupted_generated_operation_requires_recovery_and_can_be_undone() {
        let fx = fixture();
        let e = fx.engine();
        let game = game_with(&fx, vec![batman_no_d3d11()]);
        let id = e.analyze(&game).unwrap()[0]
            .assessment
            .recipe_id
            .clone()
            .unwrap();
        let p = e.prepare(&id, &game).unwrap();
        let op = e.apply(&p.plan_id, &p.fingerprint, &game, &noop).unwrap();
        // Falha parcial: o app fecha depois de gravar, antes de verificar.
        let mut op = e.operation(&op.id).unwrap();
        op.state = OpState::Applying;
        op.steps[0].status = StepStatus::Written;
        e.journal.save(&op).unwrap();
        let e = fx.engine();
        assert_eq!(
            e.operation(&op.id).unwrap().state,
            OpState::RecoveryRequired
        );
        assert!(e.active_for("200260").is_empty());
        let rb = e.prepare_rollback(&op.id).unwrap();
        assert_eq!(
            e.rollback(&op.id, &rb.fingerprint, &noop).unwrap().state,
            OpState::RolledBack
        );
        assert_eq!(std::fs::read_to_string(&fx.config).unwrap(), LOCALCONFIG);
    }

    #[test]
    fn analysis_is_persisted_in_a_private_registry() {
        use std::os::unix::fs::PermissionsExt;
        let fx = fixture();
        let game = game_with(&fx, vec![batman_no_d3d11()]);
        let first = fx.engine().analyze(&game).unwrap();
        let again = fx.engine().analyze(&game).unwrap();
        assert_eq!(first, again, "mesma análise depois de reiniciar");
        let dir = fx.data.join("generated");
        assert_eq!(
            std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert!(dir.join("200260.json").is_file());
    }
}

/// Validação de ponta a ponta com **dados reais, sem tocar na instalação**:
/// lê (só leitura) o relatório de descobertas em cache e uma cópia do
/// localconfig.vdf real, e aplica/desfaz numa Steam isolada em tempdir.
/// `cargo test real_dynamic_end_to_end -- --ignored --nocapture`
#[test]
#[ignore]
fn real_dynamic_end_to_end() {
    use crate::fixes::models::FixReport;
    let home = std::env::var("HOME").unwrap();
    let cache = PathBuf::from(&home).join(".cache/io.frx.FRX/fixes/200260.json");
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&cache).unwrap()).unwrap();
    let report: FixReport = serde_json::from_value(v["data"].clone()).unwrap();

    let fx = fixture();
    // Cópia do localconfig.vdf real da conta mais recente (o original não é aberto para escrita).
    let real_steam = PathBuf::from(&home).join(".local/share/Steam");
    let real_cfg = localconfig::path_for(&real_steam).unwrap().1;
    let original = std::fs::read(&real_cfg).unwrap();
    std::fs::write(&fx.config, &original).unwrap();
    let before_line = fx.line();

    let e = fx.engine();
    let game = GameCtx {
        findings: report.findings.clone(),
        gpu: report.context.gpu_vendor,
        tool_name: report.context.tool_name.clone(),
        ..fx.game()
    };
    let views = e.analyze(&game).unwrap();
    for a in &views {
        println!(
            "{:<28} {:<18} {:<14} {:?}",
            a.assessment.finding_id,
            format!("{:?}", a.assessment.status),
            format!("{:?}", a.assessment.evidence),
            a.assessment.reasons.first()
        );
    }
    let gen = views
        .iter()
        .find(|a| a.assessment.status == super::dynamic::DynStatus::Generated)
        .expect("nenhuma descoberta real gerou receita");
    let id = gen.assessment.recipe_id.clone().unwrap();
    println!(
        "receita: {id} · {}",
        gen.assessment.recipe.as_ref().unwrap().summary
    );

    let plan = e.prepare(&id, &game).unwrap();
    println!("plano: {:?} · {:?}", plan.status, plan.changes[0].after);
    assert_eq!(plan.status, PlanStatus::Ready, "{:?}", plan.reasons);
    let op = e
        .apply(&plan.plan_id, &plan.fingerprint, &game, &noop)
        .unwrap();
    assert_eq!(op.state, OpState::Applied, "{:?}", op.message);
    println!("aplicada: {:?}", fx.line());
    assert_ne!(fx.line(), before_line);
    let rb = e.prepare_rollback(&op.id).unwrap();
    assert_eq!(
        e.rollback(&op.id, &rb.fingerprint, &noop).unwrap().state,
        OpState::RolledBack
    );
    assert_eq!(
        std::fs::read(&fx.config).unwrap(),
        original,
        "byte a byte igual à cópia do original"
    );
    assert_eq!(std::fs::read(&real_cfg).unwrap().len(), original.len());
    println!(
        "rollback: arquivo idêntico ao original ({} bytes)",
        original.len()
    );
}

// ── Smart Packs aplicados pelo Fix Engine (Steam falsa) ─────────────────

mod packs {
    use super::*;
    use crate::fix_engine::dynamic::tests::finding;
    use crate::fixes::models::{ActionKind, FixFinding};
    use crate::smart_packs::benchmark;
    use crate::smart_packs::experience::{self, ExperiencePack};
    use crate::smart_packs::models::*;

    fn findings() -> Vec<FixFinding> {
        vec![
            finding(
                "address",
                ActionKind::Env,
                "PROTON_FORCE_LARGE_ADDRESS_AWARE=1",
                2,
                0,
            ),
            finding("heap", ActionKind::Env, "PROTON_HEAP_DELAY_FREE=1", 2, 0),
        ]
    }

    fn game(fx: &Fx, f: Vec<FixFinding>) -> GameCtx {
        GameCtx {
            findings: f,
            ..fx.game()
        }
    }

    fn report(fx: &Fx, e: &FixEngine, g: &GameCtx) -> ExperiencePack {
        experience::build(e, g, fx.line().as_deref(), &[]).unwrap()
    }

    /// O pacote do jogo com as duas receitas: uma receita composta.
    fn combo(r: &ExperiencePack) -> ExperiencePack {
        assert_eq!(r.actions.len(), 2, "{:?}", r.left_out);
        assert!(r
            .apply_recipe_id
            .as_deref()
            .unwrap()
            .starts_with("pack.200260."));
        r.clone()
    }

    #[test]
    fn multi_recipe_pack_is_one_operation_with_one_backup_and_exact_rollback() {
        let fx = fixture();
        let e = fx.engine();
        let g = game(&fx, findings());
        let p = combo(&report(&fx, &e, &g));
        assert_eq!(p.state, PackState::Ready, "{}", p.state_detail);
        let plan = e
            .prepare(p.apply_recipe_id.as_deref().unwrap(), &g)
            .unwrap();
        assert_eq!(plan.status, PlanStatus::Ready, "{:?}", plan.reasons);
        assert_eq!(plan.trust, Trust::Pack);
        assert_eq!(plan.pack.as_ref().unwrap().members.len(), 2);
        assert_eq!(plan.changes.len(), 1, "dry run consolidado: uma alteração");
        let after = plan.changes[0].after.clone().unwrap();
        assert!(
            after.contains("PROTON_FORCE_LARGE_ADDRESS_AWARE=1")
                && after.contains("PROTON_HEAP_DELAY_FREE=1")
        );
        assert_eq!(std::fs::read_to_string(&fx.config).unwrap(), LOCALCONFIG);

        let op = e
            .apply(&plan.plan_id, &plan.fingerprint, &g, &noop)
            .unwrap();
        assert_eq!(op.state, OpState::Applied, "{:?}", op.message);
        assert_eq!(op.recipe_trust, Some(Trust::Pack));
        assert_eq!(op.steps.len(), 1);
        assert_eq!(
            op.steps[0].backup.as_ref().unwrap().sha256.as_deref(),
            Some(sha256_hex(LOCALCONFIG.as_bytes()).as_str())
        );
        // Estado e snapshot de benchmark apontam para o pacote.
        let p2 = report(&fx, &fx.engine(), &g);
        assert_eq!(p2.state, PackState::Verified, "{}", p2.state_detail);
        assert_eq!(p2.operation_id.as_deref(), Some(op.id.as_str()));
        let snapshot = benchmark::snapshot(&e, "200260");
        assert_eq!(Some(snapshot.pack_id), p.pack_id);
        assert_eq!(snapshot.recipes[0].fingerprint, op.recipe_fingerprint);
        assert_eq!(
            e.operation(&op.id).unwrap().recipe_fingerprint,
            op.recipe_fingerprint,
            "original operation retained"
        );
        assert_eq!(
            e.prepare(p.apply_recipe_id.as_deref().unwrap(), &g)
                .unwrap()
                .status,
            PlanStatus::AlreadyApplied
        );

        let rb = e.prepare_rollback(&op.id).unwrap();
        assert_eq!(
            e.rollback(&op.id, &rb.fingerprint, &noop).unwrap().state,
            OpState::RolledBack
        );
        assert_eq!(std::fs::read_to_string(&fx.config).unwrap(), LOCALCONFIG);
        assert_eq!(report(&fx, &e, &g).state, PackState::RolledBack);
    }

    #[test]
    fn changes_between_preview_and_confirmation_or_tampering_block_the_pack() {
        let fx = fixture();
        let e = fx.engine();
        let g = game(&fx, findings());
        let p = combo(&report(&fx, &e, &g));
        let id = p.apply_recipe_id.clone().unwrap();
        // Cancelar = não confirmar.
        let _ = e.prepare(&id, &g).unwrap();
        assert!(e.operations(None).is_empty());
        // Uma das descobertas sumiu da pesquisa.
        let plan = e.prepare(&id, &g).unwrap();
        let fewer = game(&fx, findings()[..1].to_vec());
        assert!(e
            .apply(&plan.plan_id, &plan.fingerprint, &fewer, &noop)
            .is_err());
        // Agora há relatos contrários a uma das receitas.
        let plan = e.prepare(&id, &g).unwrap();
        let mut contested = findings();
        contested[1].evidence.negative = 5;
        assert!(e
            .apply(
                &plan.plan_id,
                &plan.fingerprint,
                &game(&fx, contested),
                &noop
            )
            .is_err());
        // Registro adulterado: membro inexistente.
        let path = fx.data.join("smart-packs/200260.json");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, text.replace("gen.200260.", "gen.200260.f")).unwrap();
        assert!(e.prepare(&id, &g).is_err());
        assert_eq!(std::fs::read_to_string(&fx.config).unwrap(), LOCALCONFIG);
        assert!(
            e.operations(None).is_empty(),
            "nada registrado como aplicado"
        );
    }

    #[test]
    fn interrupted_pack_requires_recovery_and_manual_steps_survive_restart() {
        let fx = fixture();
        let e = fx.engine();
        let g = game(&fx, findings());
        let p = combo(&report(&fx, &e, &g));
        let plan = e
            .prepare(p.apply_recipe_id.as_deref().unwrap(), &g)
            .unwrap();
        let op = e
            .apply(&plan.plan_id, &plan.fingerprint, &g, &noop)
            .unwrap();
        let mut op = e.operation(&op.id).unwrap();
        op.state = OpState::Applying;
        op.steps[0].status = StepStatus::Written;
        e.journal.save(&op).unwrap();
        let e = fx.engine(); // reinício
        assert_eq!(report(&fx, &e, &g).state, PackState::PartiallyRecovered);
        let rb = e.prepare_rollback(&op.id).unwrap();
        assert_eq!(
            e.rollback(&op.id, &rb.fingerprint, &noop).unwrap().state,
            OpState::RolledBack
        );
        assert_eq!(std::fs::read_to_string(&fx.config).unwrap(), LOCALCONFIG);

        // Steam aberta: passo manual, que continua pendente depois de reiniciar.
        fx.proc("300", "steam\n", "");
        let plan = e
            .prepare(p.apply_recipe_id.as_deref().unwrap(), &g)
            .unwrap();
        assert_eq!(plan.status, PlanStatus::RequiresManualStep);
        let op = e
            .apply(&plan.plan_id, &plan.fingerprint, &g, &noop)
            .unwrap();
        assert_eq!(op.state, OpState::RequiresManualStep);
        assert_eq!(report(&fx, &fx.engine(), &g).state, PackState::Testing);
        assert_eq!(std::fs::read_to_string(&fx.config).unwrap(), LOCALCONFIG);
    }
}

mod file_artifacts {
    use super::*;
    include!("artifact_tests.rs");
    include!("file_security_tests.rs");
    include!("https_fixture_test.rs");
}

mod availability_acceptance {
    use super::*;
    include!("availability_tests.rs");
}

mod structured_config_cases {
    use super::*;
    include!("structured_config_tests.rs");
}

/// PhysX por GPU: diagnóstico real da cadeia → plano aplicável → prévia →
/// aplicação → rollback byte a byte → após a execução, "já resolvido".
#[test]
fn gpu_physx_plan_applies_the_reviewed_recipe_and_rolls_back() {
    use crate::fix_engine::availability::AvailabilityState;
    use crate::resolution::physx;
    let fx = fixture();
    let root = fx.library.parent().unwrap().parent().unwrap().to_path_buf(); // .../steamapps
    let put = |p: std::path::PathBuf, b: &[u8]| {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b).unwrap();
    };
    let mut pe = vec![0u8; 256];
    pe[..2].copy_from_slice(b"MZ");
    pe[0x3c] = 0x80;
    pe[0x80..0x84].copy_from_slice(b"PE\0\0");
    pe[0x84..0x86].copy_from_slice(&0x14cu16.to_le_bytes());
    put(fx.library.join("Binaries/Win32/PhysXLoader.dll"), &pe);
    let tool = root.parent().unwrap().join("tool");
    put(tool.join("proton"), b"PROTON_NVIDIA_LIBS");
    put(
        tool.join("files/lib/wine/nvidia-libs/nvcuda/i386-windows/nvcuda.dll.so"),
        b"real-32",
    );
    let pfx = root.join("compatdata/200260/pfx/drive_c");
    put(
        pfx.join("Program Files (x86)/NVIDIA Corporation/PhysX/Common/PhysXLoader.dll"),
        &pe,
    );
    let sys = root.parent().unwrap().join("sys");
    put(sys.join("usr/lib32/libcuda.so.1"), b"elf");
    fx.set_line("mangohud %command%");
    let original = std::fs::read(&fx.config).unwrap();
    let e = fx.engine();
    let g = GameCtx {
        gpu: Some(crate::fixes::models::GpuReq::Nvidia),
        tool_path: Some(tool.clone()),
        ..fx.game()
    };
    let diag = |line: Option<String>| {
        physx::add(&e, &g, line.as_deref(), &sys, &mut [])
            .unwrap()
            .1
    };

    let plan = diag(fx.line());
    assert_eq!(plan.state, "applicable", "{:?}", plan.availability.reasons);
    assert_eq!(plan.availability.state, AvailabilityState::Available);
    assert_eq!(plan.recipe_id.as_deref(), Some(physx::RECIPE));
    let p = e.prepare(physx::RECIPE, &g).unwrap();
    assert_eq!(
        p.changes[0].after.as_deref(),
        Some("PROTON_USE_WOW64=0 PROTON_NVIDIA_LIBS=1 mangohud %command%")
    );
    assert_eq!(
        std::fs::read(&fx.config).unwrap(),
        original,
        "prévia não grava"
    );
    let op = e.apply(&p.plan_id, &p.fingerprint, &g, &noop).unwrap();
    assert_eq!(op.state, OpState::Applied, "{:?}", op.message);
    assert_eq!(
        op.steps[0].backup.as_ref().unwrap().sha256.as_deref(),
        Some(sha256_hex(&original).as_str())
    );

    // Antes da execução: configurado, aguardando o Proton instalar o nvcuda.
    assert_eq!(
        diag(fx.line()).availability.state,
        AvailabilityState::Temporary
    );
    // "Execução": o Proton copia o nvcuda do nvidia-libs para o prefixo.
    put(pfx.join("windows/syswow64/nvcuda.dll"), b"real-32");
    let done = diag(fx.line());
    assert_eq!(
        (done.state.as_str(), done.availability.state),
        ("already-present", AvailabilityState::Applied)
    );

    let rb = e.prepare_rollback(&op.id).unwrap();
    assert_eq!(
        e.rollback(&op.id, &rb.fingerprint, &noop).unwrap().state,
        OpState::RolledBack
    );
    assert_eq!(
        std::fs::read(&fx.config).unwrap(),
        original,
        "restaurado byte a byte"
    );

    // Ferramenta sem nvidia-libs: nada aplicável, motivo específico.
    std::fs::remove_dir_all(tool.join("files/lib/wine/nvidia-libs")).unwrap();
    let blocked = diag(fx.line());
    assert!(blocked.recipe_id.is_none());
    assert!(blocked
        .availability
        .reasons
        .iter()
        .any(|r| r.code == "tool" && r.message.contains("stub")));
}

/// Pacote único: dez relatos do mesmo ajuste viram uma ação; relatos
/// negativos, alternativas conflitantes e evidência fraca ficam de fora com
/// o motivo; o pacote composto aplica tudo numa operação e desfaz tudo.
#[test]
fn experience_pack_merges_duplicates_skips_conflicts_and_rolls_back_as_one() {
    use crate::fix_engine::dynamic::tests::finding;
    use crate::fixes::models::ActionKind;
    use crate::smart_packs::experience::{self, LeftOutKind};
    let fx = fixture();
    let e = fx.engine();
    let mut findings: Vec<_> = (0..10)
        .map(|i| {
            finding(
                &format!("heap-{i}"),
                ActionKind::Env,
                "PROTON_HEAP_DELAY_FREE=1",
                2,
                0,
            )
        })
        .collect();
    findings.push(finding(
        "nvapi-on",
        ActionKind::Env,
        "PROTON_ENABLE_NVAPI=1",
        3,
        0,
    ));
    findings.push(finding(
        "nvapi-off",
        ActionKind::Env,
        "PROTON_DISABLE_NVAPI=1",
        2,
        0,
    ));
    findings.push(finding("bad", ActionKind::Env, "PROTON_USE_XALIA=0", 1, 2));
    findings.push(finding(
        "weak",
        ActionKind::Env,
        "PROTON_ENABLE_WAYLAND=1",
        0,
        0,
    ));
    let g = GameCtx {
        findings,
        gpu: Some(crate::fixes::models::GpuReq::Nvidia),
        ..fx.game()
    };
    let p = experience::build(&e, &g, fx.line().as_deref(), &[]).unwrap();
    let changes: Vec<&str> = p
        .actions
        .iter()
        .flat_map(|a| a.changes.iter().map(String::as_str))
        .collect();
    assert_eq!(
        changes
            .iter()
            .filter(|c| **c == "PROTON_HEAP_DELAY_FREE=1")
            .count(),
        1,
        "dez relatos do mesmo ajuste são uma ação só: {changes:?}"
    );
    let heap = p
        .actions
        .iter()
        .find(|a| a.changes == ["PROTON_HEAP_DELAY_FREE=1"])
        .unwrap();
    assert!(
        heap.sources.len() >= 10,
        "todas as fontes ficam como evidência"
    );
    assert!(
        !(changes.contains(&"PROTON_ENABLE_NVAPI=1")
            && changes.contains(&"PROTON_DISABLE_NVAPI=1")),
        "alternativas incompatíveis não entram juntas"
    );
    let out = |k: LeftOutKind| p.left_out.iter().filter(move |l| l.kind == k);
    assert!(out(LeftOutKind::Conflict).any(|l| l.changes.iter().any(|c| c.contains("NVAPI"))));
    assert!(
        out(LeftOutKind::Negative).any(|l| l.changes == ["PROTON_USE_XALIA=0"]),
        "relatos negativos: {:?}",
        p.left_out
    );
    assert!(
        !changes.contains(&"PROTON_ENABLE_WAYLAND=1"),
        "um comentário sem resultado não entra"
    );
    assert!(p.actions.len() >= 2, "{:?}", p.left_out);
    let rid = p.apply_recipe_id.clone().unwrap();
    assert!(
        rid.starts_with("pack.200260."),
        "várias ações = uma receita composta"
    );

    // Desligar uma ação recompõe o pacote sem ela.
    let without = experience::build(
        &e,
        &g,
        fx.line().as_deref(),
        std::slice::from_ref(&heap.recipe_id),
    )
    .unwrap();
    assert!(without
        .actions
        .iter()
        .any(|a| a.recipe_id == heap.recipe_id && !a.selected));
    assert_ne!(without.apply_recipe_id, p.apply_recipe_id);

    let before = std::fs::read(&fx.config).unwrap();
    let plan = e.prepare(&rid, &g).unwrap();
    assert_eq!(plan.status, PlanStatus::Ready);
    let op = e
        .apply(&plan.plan_id, &plan.fingerprint, &g, &|_| {})
        .unwrap();
    assert_eq!(op.state, OpState::Applied);
    let line = fx.line().unwrap();
    for c in &changes {
        assert!(line.contains(c), "{c} aplicado: {line}");
    }
    let applied = experience::build(&e, &g, fx.line().as_deref(), &[]).unwrap();
    assert_eq!(applied.operation_id.as_deref(), Some(op.id.as_str()));
    assert!(matches!(
        applied.state,
        crate::smart_packs::models::PackState::Verified
    ));

    let rb = e.prepare_rollback(&op.id).unwrap();
    e.rollback(&rb.operation_id, &rb.fingerprint, &|_| {})
        .unwrap();
    assert_eq!(
        std::fs::read(&fx.config).unwrap(),
        before,
        "o pacote inteiro é desfeito"
    );
}

/// Complete Experience Pack com dados reais: descobertas do cache, instalação,
/// ferramenta e hardware reais (só leitura). Gravação e rollback numa cópia
/// isolada da configuração da Steam. `FRX_APPID` (padrão 3321460).
/// `cargo test real_experience_pack -- --ignored --nocapture`
#[test]
#[ignore]
fn real_experience_pack() {
    use crate::fixes::models::FixReport;
    use crate::smart_packs::experience;
    use crate::steam::{self, discovery::DiscoveryEnv};
    let app = std::env::var("FRX_APPID").unwrap_or_else(|_| "3321460".into());
    let home = PathBuf::from(std::env::var("HOME").unwrap());
    let v: serde_json::Value = serde_json::from_slice(
        &std::fs::read(home.join(format!(".cache/io.frx.FRX/fixes/{app}.json"))).unwrap(),
    )
    .unwrap();
    let report: FixReport = serde_json::from_value(v["data"].clone()).unwrap();
    let scan = steam::scan(&DiscoveryEnv::from_process(), None);
    let real = scan.games.into_iter().find(|g| g.app_id == app).unwrap();
    let profile = crate::system::profile();
    let fx = fixture();
    let real_cfg = localconfig::path_for(&home.join(".local/share/Steam"))
        .unwrap()
        .1;
    let mut cfg = std::fs::read_to_string(&real_cfg).unwrap();
    if std::env::var_os("FRX_CLEAN_CONFIG").is_some() || !cfg.contains(&format!("\"{app}\"")) {
        // Entrada sem opções de inicialização (a do AppID 620 na fixture).
        cfg = LOCALCONFIG
            .replace("\"200260\"", "\"0\"")
            .replace("\"620\"", &format!("\"{app}\""));
    }
    std::fs::write(&fx.config, &cfg).unwrap();
    let e = fx.engine();
    let tool = real.compatibility.tool.as_ref();
    let g = GameCtx {
        app_id: app.clone(),
        name: real.name.clone(),
        steam_root: Some(fx.steam.clone()),
        install_path: real.install_path.clone().map(Into::into),
        uses_proton: real.compatibility.uses_proton,
        tool_name: tool.map(|t| crate::fixes::short_tool_name(&t.display_name)),
        tool_path: tool.and_then(|t| t.install_path.clone()).map(Into::into),
        tool_includes_protonfixes: tool.is_some_and(|t| t.includes_protonfixes),
        gpu: report.context.gpu_vendor,
        driver: profile
            .primary_gpu
            .as_ref()
            .and_then(|g| g.driver_version.clone()),
        findings: report.findings.clone(),
        ..Default::default()
    };
    let line = || {
        localconfig::launch_options(&std::fs::read_to_string(&fx.config).unwrap(), &app).unwrap()
    };
    println!(
        "{} · {} descobertas · {:?} · {:?}",
        g.name,
        g.findings.len(),
        g.tool_name,
        g.gpu
    );
    for id in ["frx.proton.dlss-upgrade", crate::resolution::physx::RECIPE] {
        let a = e.availability(id, &g);
        println!(
            "catálogo {id}: {:?} {:?}",
            a.state,
            a.reasons.iter().map(|r| &r.message).collect::<Vec<_>>()
        );
    }
    if let Some((_, plan)) = crate::resolution::physx::add(
        &e,
        &g,
        line().as_deref(),
        std::path::Path::new("/"),
        &mut [],
    ) {
        println!(
            "PhysX: {} · {:?} · {:?}",
            plan.state,
            plan.recipe_id,
            plan.availability
                .reasons
                .iter()
                .map(|r| &r.message)
                .collect::<Vec<_>>()
        );
    }
    let p = experience::build(&e, &g, line().as_deref(), &[]).unwrap();
    println!(
        "\n== Pacote: {:?} · {} · aplicar {:?} · parcial {}",
        p.state,
        p.availability
            .reasons
            .iter()
            .map(|r| r.message.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        p.apply_recipe_id,
        p.partial
    );
    println!("ambiente: {:?} · áreas: {:?}", p.environment, p.areas);
    for a in &p.actions {
        println!(
            "  + {} [{}] {:?} · {:?} · +{} -{} · {}",
            a.title, a.area, a.changes, a.evidence, a.positive, a.negative, a.evidence_note
        );
        for s in &a.sources {
            println!("      fonte: {} {}", s.label, s.url);
        }
    }
    for r in &p.risks {
        println!("  risco: {r}");
    }
    println!("== Fora do pacote");
    for l in &p.left_out {
        println!(
            "  - [{:?}] {} {:?} ×{} — {}",
            l.kind, l.title, l.changes, l.count, l.reason
        );
    }
    let Some(rid) = p.apply_recipe_id.clone() else {
        return;
    };
    let before = std::fs::read(&fx.config).unwrap();
    let plan = e.prepare(&rid, &g).unwrap();
    println!("\n== Prévia {:?}", plan.status);
    for c in &plan.changes {
        println!("  {c:?}");
    }
    let op = e
        .apply(&plan.plan_id, &plan.fingerprint, &g, &|_| {})
        .unwrap();
    println!("aplicado: {:?} · opções agora: {:?}", op.state, line());
    let after = experience::build(&e, &g, line().as_deref(), &[]).unwrap();
    println!(
        "estado do pacote: {:?} — {}",
        after.state, after.state_detail
    );
    let rb = e.prepare_rollback(&op.id).unwrap();
    let back = e
        .rollback(&rb.operation_id, &rb.fingerprint, &|_| {})
        .unwrap();
    println!("rollback: {:?} · opções agora: {:?}", back.state, line());
    assert_eq!(
        std::fs::read(&fx.config).unwrap(),
        before,
        "rollback restaura o arquivo byte a byte"
    );
    println!("arquivo restaurado byte a byte: sim");
}
