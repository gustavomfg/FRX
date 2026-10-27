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
    let view = engine
        .recipes(&[])
        .into_iter()
        .find(|r| r.recipe.id == "frx.compat.wined3d")
        .unwrap();
    assert!(view.executable);
    assert!(view
        .performance_warnings
        .iter()
        .any(|w| w.contains("OpenGL")));
    let preview = engine.prepare("frx.compat.wined3d", &game).unwrap();
    assert_eq!(preview.status, PlanStatus::Ready);
    assert_eq!(preview.performance_warnings, view.performance_warnings);
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

#[test]
fn related_findings_are_linked_by_action() {
    use crate::fixes::models::*;
    let e = fixture().engine();
    let f = |kind, value: &str| FixFinding {
        artifacts: vec![],
        scope: DiscoveryScope::verified("200260", "Fixture: verified game source"),
        performance_evidence: Default::default(),
        id: format!("f-{value}"),
        game_app_id: "200260".into(),
        title: "t".into(),
        description: "d".into(),
        category: Category::Graphics,
        kind: FindingKind::Tweak,
        actions: vec![FixAction {
            kind,
            label: "l".into(),
            value: value.into(),
            conditional: false,
        }],
        sources: vec![],
        requirements: Default::default(),
        confidence: Confidence::CommunityReported,
        compatibility: Compatibility::Unknown,
        compatibility_note: String::new(),
        experimental: false,
        analysis: AnalysisState::Parsed,
        evidence: Default::default(),
        published_at: None,
        retrieved_at: "t".into(),
        keys: vec![],
    };
    let findings = vec![
        f(ActionKind::Env, "PROTON_USE_WINED3D=1"),
        f(ActionKind::LaunchOption, "PROTON_NO_D3D11=1 %command%"),
        f(ActionKind::Winetricks, "d3dx9_43"),
    ];
    let views = e.recipes(&findings);
    let by = |id: &str| views.iter().find(|v| v.recipe.id == id).unwrap();
    assert_eq!(
        by("frx.compat.wined3d").related_findings,
        vec!["f-PROTON_USE_WINED3D=1"]
    );
    assert_eq!(
        by("frx.compat.no-d3d11").related_findings,
        vec!["f-PROTON_NO_D3D11=1 %command%"]
    );
    assert!(!by("frx.compat.no-d3d11").executable);
    assert!(by("frx.diagnostics.proton-log").related_findings.is_empty());
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
    use crate::smart_packs::models::*;
    use crate::smart_packs::{self, benchmark, BuildInput};

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

    fn report(fx: &Fx, e: &FixEngine, g: &GameCtx) -> SmartPackReport {
        smart_packs::build(
            e,
            BuildInput {
                game: g,
                sessions: &[],
                profile: Profile::Performance,
                limits: Limits::default(),
                launch_line: fx.line(),
            },
        )
        .unwrap()
    }

    fn combo(r: &SmartPackReport) -> SmartPack {
        r.packs
            .iter()
            .find(|p| p.kind == PackKind::Combination)
            .cloned()
            .unwrap()
    }

    fn single(r: &SmartPackReport, key: &str) -> SmartPack {
        r.packs
            .iter()
            .find(|p| p.kind == PackKind::Single && p.env[0].key == key)
            .cloned()
            .unwrap()
    }

    #[test]
    fn multi_recipe_pack_is_one_operation_with_one_backup_and_exact_rollback() {
        let fx = fixture();
        let e = fx.engine();
        let g = game(&fx, findings());
        let r = report(&fx, &e, &g);
        let p = combo(&r);
        assert_eq!(p.state, PackState::Ready, "{}", p.state_detail);
        assert_eq!(r.ranking.winner, None, "sem benchmark, sem vencedor");
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
        // Estado, configuração atual e snapshot de benchmark apontam para o pacote.
        let r = report(&fx, &fx.engine(), &g);
        let p2 = r.packs.iter().find(|x| x.id == p.id).unwrap();
        assert_eq!(p2.state, PackState::Verified, "{}", p2.state_detail);
        assert_eq!(r.current_pack_id.as_deref(), Some(p.id.as_str()));
        assert_eq!(benchmark::snapshot(&e, "200260").pack_id, p.id);
        let snapshot = benchmark::snapshot(&e, "200260");
        assert_eq!(snapshot.recipes[0].fingerprint, op.recipe_fingerprint);
        assert!(benchmark::recipe_context_matches(&e, &g, &snapshot));
        let mut changed_game = g.clone();
        changed_game.findings[0].sources[0].snippet =
            Some("Synthetic fixture: source changed after capture".into());
        assert!(
            !benchmark::recipe_context_matches(&e, &changed_game, &snapshot),
            "the new recipe fingerprint cannot inherit old benchmark evidence"
        );
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
        let r = report(&fx, &e, &g);
        assert_eq!(
            r.packs.iter().find(|x| x.id == p.id).unwrap().state,
            PackState::RolledBack
        );
        assert_eq!(
            r.packs[0].state,
            PackState::Verified,
            "baseline ativo de novo"
        );
    }

    #[test]
    fn switching_packs_is_planned_step_by_step_and_preserves_shared_keys() {
        let fx = fixture();
        let e = fx.engine();
        let g = game(&fx, findings());
        let r = report(&fx, &e, &g);
        let a = single(&r, "PROTON_FORCE_LARGE_ADDRESS_AWARE");
        let b = combo(&r);
        let pa = e
            .prepare(a.apply_recipe_id.as_deref().unwrap(), &g)
            .unwrap();
        let opa = e.apply(&pa.plan_id, &pa.fingerprint, &g, &noop).unwrap();
        assert_eq!(opa.state, OpState::Applied);
        // Aplicar B por cima: conflito na variável compartilhada.
        assert_eq!(
            e.prepare(b.apply_recipe_id.as_deref().unwrap(), &g)
                .unwrap()
                .status,
            PlanStatus::Conflict
        );
        let r = report(&fx, &e, &g);
        assert_eq!(
            r.packs.iter().find(|x| x.id == b.id).unwrap().state,
            PackState::Conflict
        );

        let sw = smart_packs::switch_plan(&e, "200260", &b);
        assert!(sw.possible);
        assert_eq!(sw.keep.len(), 1);
        assert_eq!(
            sw.add,
            vec![EnvPair {
                key: "PROTON_HEAP_DELAY_FREE".into(),
                value: "1".into()
            }]
        );
        assert_eq!(
            sw.steps.iter().map(|s| s.kind.as_str()).collect::<Vec<_>>(),
            vec!["rollback", "apply"]
        );
        // Executa os passos com as mesmas funções da interface.
        let rb = e
            .prepare_rollback(sw.steps[0].operation_id.as_deref().unwrap())
            .unwrap();
        e.rollback(&opa.id, &rb.fingerprint, &noop).unwrap();
        let pb = e
            .prepare(sw.steps[1].recipe_id.as_deref().unwrap(), &g)
            .unwrap();
        let opb = e.apply(&pb.plan_id, &pb.fingerprint, &g, &noop).unwrap();
        assert_eq!(opb.state, OpState::Applied);
        assert_eq!(benchmark::snapshot(&e, "200260").pack_id, b.id);
        // Voltar ao baseline.
        let base = r
            .packs
            .iter()
            .find(|p| p.kind == PackKind::Baseline)
            .unwrap();
        let back = smart_packs::switch_plan(&e, "200260", base);
        assert_eq!(back.steps.len(), 1);
        let rb = e.prepare_rollback(&opb.id).unwrap();
        e.rollback(&opb.id, &rb.fingerprint, &noop).unwrap();
        assert_eq!(std::fs::read_to_string(&fx.config).unwrap(), LOCALCONFIG);
        assert!(
            !smart_packs::switch_plan(&e, "200260", base).possible,
            "já é a configuração ativa"
        );
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
        let r = report(&fx, &e, &g);
        assert_eq!(
            r.packs.iter().find(|x| x.id == p.id).unwrap().state,
            PackState::PartiallyRecovered
        );
        assert!(!smart_packs::switch_plan(&e, "200260", &p).possible);
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
        let r = report(&fx, &fx.engine(), &g);
        assert_eq!(
            r.packs.iter().find(|x| x.id == p.id).unwrap().state,
            PackState::Testing
        );
        assert_eq!(std::fs::read_to_string(&fx.config).unwrap(), LOCALCONFIG);
    }

    #[test]
    fn sessions_are_linked_to_packs_and_only_unrecorded_ones_can_be_declared() {
        use crate::performance::persistence::Store;
        let fx = fixture();
        let e = fx.engine();
        let g = game(&fx, findings());
        let p = combo(&report(&fx, &e, &g));
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(tmp.path());
        let mk = |id: &str| crate::smart_packs::tests::synthetic(id, &[], 100.0, 50.0, 20.0);
        let mut old = mk("old");
        old.smart_pack = None;
        let mut assisted = mk("assisted");
        assisted.active_fixes = Some(vec![]);
        assisted.smart_pack = None;
        for m in [&old, &assisted] {
            store
                .save(
                    &crate::performance::models::Session {
                        meta: m.clone(),
                        series: vec![],
                    },
                    m.id.as_bytes(),
                )
                .unwrap();
        }
        let declared = store
            .set_smart_pack(
                "synthetic-old",
                Some(benchmark::declared("200260", p.env.clone())),
            )
            .unwrap();
        assert_eq!(declared.smart_pack.as_ref().unwrap().pack_id, p.id);
        assert!(store
            .set_smart_pack(
                "synthetic-assisted",
                Some(benchmark::declared("200260", vec![]))
            )
            .is_err());
        // active_fixes = [] (v0.4) conta como baseline registrado pelo FRX.
        let cfg = benchmark::session_config(&e, &assisted).unwrap();
        assert_eq!(cfg.pack_id, benchmark::baseline_id("200260"));
        let sessions = store.list();
        let r = smart_packs::build(
            &e,
            BuildInput {
                game: &g,
                sessions: &sessions,
                profile: Profile::Balanced,
                limits: Limits::default(),
                launch_line: None,
            },
        )
        .unwrap();
        let m = r.measurements.iter().find(|m| m.pack_id == p.id).unwrap();
        assert!(m.sessions[0].notes.iter().any(|n| n.contains("declarada")));
        assert!(r.unassigned_sessions.is_empty());
        assert!(r.windows.is_none(), "sem referência Windows integrada");
    }
}

/// Validação de ponta a ponta dos Smart Packs **sem tocar na instalação**:
/// descobertas reais do Batman (cache, só leitura) + 2 descobertas de
/// fixture identificadas, cópia do localconfig.vdf real numa Steam em
/// tempdir, log MangoHud real reprocessado do CSV bruto como baseline
/// declarado e sessões **sintéticas** para os pacotes.
/// `cargo test real_smart_packs_end_to_end -- --ignored --nocapture`
#[test]
#[ignore]
fn real_smart_packs_end_to_end() {
    use crate::fix_engine::dynamic::tests::finding;
    use crate::fixes::models::{ActionKind, FixReport};
    use crate::performance::models::{SessionInput, SessionSource};
    use crate::performance::{process, ProcessArgs};
    use crate::smart_packs::models::*;
    use crate::smart_packs::{self, benchmark, BuildInput};
    let home = PathBuf::from(std::env::var("HOME").unwrap());
    let v: serde_json::Value = serde_json::from_slice(
        &std::fs::read(home.join(".cache/io.frx.FRX/fixes/200260.json")).unwrap(),
    )
    .unwrap();
    let report: FixReport = serde_json::from_value(v["data"].clone()).unwrap();
    let mut findings = report.findings.clone();
    findings.push(finding(
        "fixture-esync",
        ActionKind::Env,
        "PROTON_NO_ESYNC=1",
        2,
        0,
    ));
    findings.push(finding(
        "fixture-heap",
        ActionKind::Env,
        "PROTON_HEAP_DELAY_FREE=1",
        2,
        0,
    ));

    let fx = fixture();
    let real_cfg = localconfig::path_for(&home.join(".local/share/Steam"))
        .unwrap()
        .1;
    let original = std::fs::read(&real_cfg).unwrap();
    std::fs::write(&fx.config, &original).unwrap();
    let e = fx.engine();
    let g = GameCtx {
        findings,
        gpu: report.context.gpu_vendor,
        tool_name: report.context.tool_name.clone(),
        ..fx.game()
    };

    // Log real do Performance Lab (captura de 25/10, antes de qualquer correção do FRX).
    let sessions_dir = home.join(".local/share/io.frx.FRX/benchmarks/sessions");
    let real_id = std::fs::read_dir(&sessions_dir)
        .unwrap()
        .flatten()
        .next()
        .unwrap()
        .path();
    let real_meta: crate::performance::models::SessionMeta =
        serde_json::from_slice(&std::fs::read(real_id.join("meta.json")).unwrap()).unwrap();
    let raw = std::fs::read(real_id.join("raw.csv")).unwrap();
    let mut baseline_real = process(
        &raw,
        ProcessArgs {
            input: SessionInput {
                app_id: Some("200260".into()),
                game_name: real_meta.game_name.clone(),
                label: Some("real · baseline".into()),
                notes: None,
                conditions: real_meta.conditions.clone(),
            },
            source: Some(SessionSource::Import),
            original_path: None,
            known: real_meta.conditions.clone(),
        },
    )
    .unwrap()
    .meta;
    baseline_real.id = "real-baseline".into();
    baseline_real.smart_pack = Some(benchmark::declared("200260", vec![]));
    println!(
        "baseline real: FPS médio {:.2} · 1% low {:.2} · P99 {:.2} ms · {:.0} s",
        baseline_real.stats.fps.avg.unwrap(),
        baseline_real.stats.fps.low1.unwrap(),
        baseline_real.stats.frametime.p99.unwrap(),
        baseline_real.stats.duration_s
    );

    let build = |sessions: &[crate::performance::models::SessionMeta], profile| {
        smart_packs::build(
            &e,
            BuildInput {
                game: &g,
                sessions,
                profile,
                limits: Limits::default(),
                launch_line: fx.line(),
            },
        )
        .unwrap()
    };
    let r = build(std::slice::from_ref(&baseline_real), Profile::Balanced);
    println!("\n== Pacotes compostos ==");
    for p in &r.packs {
        println!(
            "  {:<44} {:?} · {:?} · {} · {:?}",
            p.name,
            p.kind,
            p.state,
            p.env
                .iter()
                .map(|x| format!("{}={}", x.key, x.value))
                .collect::<Vec<_>>()
                .join(" "),
            p.evidence
        );
    }
    println!("== Conflitos (grafo) ==");
    for x in r.graph.iter().filter(|x| x.relation == Relation::Excludes) {
        println!("  {} × {}: {:?} — {}", x.a, x.b, x.kind, x.reason);
    }
    println!("== Fora dos pacotes ==");
    for x in &r.excluded {
        println!("  {} — {}", x.title, x.reason);
    }
    println!(
        "== Ranking (1 baseline real, nenhum pacote medido) ==\n  {}",
        r.ranking.summary
    );
    for x in &r.ranking.entries {
        println!("  [{:?}] {:<44} {}", x.tier, x.title, x.headline);
    }
    let d3d = r
        .packs
        .iter()
        .find(|p| p.env.len() == 1 && p.env[0].key == "PROTON_NO_D3D11")
        .cloned()
        .unwrap();
    assert_eq!(
        r.ranking
            .entries
            .iter()
            .find(|x| x.pack_id == d3d.id)
            .unwrap()
            .tier,
        Tier::Candidate,
        "NO_D3D11 não sobe sem benchmark"
    );
    assert!(r.ranking.winner.is_none());
    let combo = r
        .packs
        .iter()
        .find(|p| p.kind == PackKind::Combination)
        .cloned()
        .unwrap();

    // Aplica o pacote A (combinação) na cópia.
    let pa = e
        .prepare(combo.apply_recipe_id.as_deref().unwrap(), &g)
        .unwrap();
    println!("\nPacote A: {:?} → {:?}", pa.status, pa.changes[0].after);
    let opa = e.apply(&pa.plan_id, &pa.fingerprint, &g, &noop).unwrap();
    assert_eq!(opa.state, OpState::Applied);
    let b = opa.steps[0].backup.as_ref().unwrap();
    assert_eq!(b.sha256.as_deref(), Some(sha256_hex(&original).as_str()));
    println!("backup verificado: {} ({} bytes)", b.path.display(), b.size);
    let snap_a = benchmark::snapshot(&e, "200260");
    assert_eq!(snap_a.pack_id, combo.id);

    // Sessões sintéticas do pacote A, com o snapshot real do FRX.
    let synth = |id: &str, snap: &PackSnapshot, f: f64| {
        let mut m = baseline_real.clone();
        m.id = format!("synthetic-{id}");
        m.label = format!("SINTÉTICA {id}");
        m.stats.fps.avg = m.stats.fps.avg.map(|v| v * f);
        m.stats.fps.low1 = m.stats.fps.low1.map(|v| v * f);
        m.smart_pack = Some(snap.clone());
        m
    };
    let mut sessions = vec![baseline_real.clone(), synth("a1", &snap_a, 1.10)];
    let r = build(&sessions, Profile::Balanced);
    let ea = r
        .ranking
        .entries
        .iter()
        .find(|x| x.pack_id == combo.id)
        .unwrap();
    println!(
        "\nRanking com 1 real × 1 sintética: [{:?}] {} — {}",
        ea.tier,
        ea.headline,
        ea.reasons.join(" ")
    );
    assert_eq!(ea.tier, Tier::Candidate);

    // Repetições (sintéticas, incluindo um segundo baseline sintético).
    let mut base2 = synth("b2", &benchmark::declared("200260", vec![]), 1.01);
    base2.smart_pack = Some(benchmark::declared("200260", vec![]));
    sessions.push(base2);
    sessions.push(synth("a2", &snap_a, 1.11));
    let r = build(&sessions, Profile::Balanced);
    let ea = r
        .ranking
        .entries
        .iter()
        .find(|x| x.pack_id == combo.id)
        .unwrap();
    println!(
        "Ranking com 2 × 2: [{:?}] #{:?} {} · {}",
        ea.tier, ea.position, ea.headline, r.ranking.summary
    );
    assert_eq!(ea.tier, Tier::Measured);
    assert!(r.history.len() >= 2);

    // Troca para o pacote B (NO_D3D11 sozinha): desfaz A, aplica B.
    let sw = smart_packs::switch_plan(&e, "200260", &d3d);
    println!(
        "\nTroca A → B: mantém {:?}, remove {:?}, adiciona {:?}, passos {:?}",
        sw.keep,
        sw.remove,
        sw.add,
        sw.steps.iter().map(|s| &s.title).collect::<Vec<_>>()
    );
    let rb = e.prepare_rollback(&opa.id).unwrap();
    e.rollback(&opa.id, &rb.fingerprint, &noop).unwrap();
    let pb = e
        .prepare(d3d.apply_recipe_id.as_deref().unwrap(), &g)
        .unwrap();
    let opb = e.apply(&pb.plan_id, &pb.fingerprint, &g, &noop).unwrap();
    assert_eq!(opb.state, OpState::Applied);
    let snap_b = benchmark::snapshot(&e, "200260");
    sessions.push(synth("d1", &snap_b, 0.80));
    sessions.push(synth("d2", &snap_b, 0.81));
    let cmp = smart_packs::compare_packs(&e, "200260", &sessions, &combo.id, &d3d.id).unwrap();
    println!(
        "A × B: {} · {:?}",
        cmp.summary,
        cmp.deltas
            .iter()
            .map(|d| format!("{} {:+.1}%", d.label, d.delta_pct))
            .collect::<Vec<_>>()
    );
    let r = build(&sessions, Profile::Performance);
    let eb = r
        .ranking
        .entries
        .iter()
        .find(|x| x.pack_id == d3d.id)
        .unwrap();
    println!(
        "Perfil desempenho: B [{:?}] {} · {}",
        eb.tier, eb.headline, r.ranking.summary
    );
    assert_eq!(eb.tier, Tier::NotRecommended);

    // Restaura o estado original.
    let rb = e.prepare_rollback(&opb.id).unwrap();
    e.rollback(&opb.id, &rb.fingerprint, &noop).unwrap();
    assert_eq!(std::fs::read(&fx.config).unwrap(), original);
    println!(
        "\nRollback final: cópia idêntica ao original ({} bytes). Histórico do ranking: {} pontos.",
        original.len(),
        build(&sessions, Profile::Balanced).history.len()
    );
}

/// Teste de aceitação do "Testar desempenho" (Batman) **sem tocar na
/// instalação**: cópia do localconfig.vdf real numa Steam em tempdir e
/// logs MangoHud de FIXTURE gerados a partir da observação informal do
/// usuário (referência ≈ 95 FPS; `PROTON_NO_D3D11=1` ≈ 55 FPS). Os logs
/// passam pelo mesmo processamento e pela mesma associação da captura
/// assistida (`tag_assisted`).
/// `cargo test acceptance_test_performance_batman -- --ignored --nocapture`
/// Com `FRX_E2E_STORE=<pasta>`, as sessões também são salvas ali (para
/// validar a interface num ambiente isolado).
#[test]
#[ignore]
fn acceptance_test_performance_batman() {
    use crate::fix_engine::dynamic::tests::batman_no_d3d11;
    use crate::performance::models::{Conditions, SessionInput, SessionMeta, SessionSource};
    use crate::performance::parser::fixtures::per_frame;
    use crate::performance::{process, ProcessArgs};
    use crate::smart_packs::models::*;
    use crate::smart_packs::{self, benchmark, BuildInput};

    let home = PathBuf::from(std::env::var("HOME").unwrap());
    let fx = fixture();
    let real_cfg = localconfig::path_for(&home.join(".local/share/Steam"))
        .unwrap()
        .1;
    let original = std::fs::read(&real_cfg).unwrap();
    std::fs::write(&fx.config, &original).unwrap();
    let e = fx.engine();
    let g = GameCtx {
        findings: vec![batman_no_d3d11()],
        ..fx.game()
    };
    let out = std::env::var_os("FRX_E2E_STORE").map(crate::performance::persistence::Store::new);

    // Log de FIXTURE: 60 s de quadros com oscilação e alguns picos.
    let log = |fps: f64, seed: f64| {
        let mut fts = Vec::new();
        let (mut t, mut i) = (0.0, 0usize);
        while t < 60_000.0 {
            let wobble = 1.0
                + 0.08 * ((i as f64 / 37.0) + seed).sin()
                + 0.05 * ((i as f64 / 5.0) * seed).cos();
            let mut ft = 1000.0 / fps * wobble;
            if i % 400 == 17 {
                ft *= 2.6;
            }
            fts.push(ft);
            t += ft;
            i += 1;
        }
        per_frame(&fts, 97.0)
    };
    let conditions = Conditions {
        resolution: Some("1920x1080".into()),
        preset: Some("Alto".into()),
        vsync: Some(false),
        gpu: Some("NVIDIA GeForce RTX 4060".into()),
        ..Default::default()
    };
    let mut sessions: Vec<SessionMeta> = Vec::new();
    let capture = |label: &str, fps: f64, seed: f64, sessions: &mut Vec<SessionMeta>| {
        let raw = log(fps, seed);
        let mut s = process(
            raw.as_bytes(),
            ProcessArgs {
                input: SessionInput {
                    app_id: Some("200260".into()),
                    game_name: Some("Batman: Arkham City GOTY".into()),
                    label: Some(format!("FIXTURE · {label}")),
                    notes: Some(
                        "Gerada a partir da observação informal do usuário; não é benchmark real."
                            .into(),
                    ),
                    conditions: conditions.clone(),
                },
                source: Some(SessionSource::Assisted),
                original_path: None,
                known: conditions.clone(),
            },
        )
        .unwrap();
        s.meta.id = format!("s-test-{}", sessions.len());
        s.meta.created_at = format!("2026-10-26T22:0{}:00Z", sessions.len());
        benchmark::tag_assisted(&e, "200260", &mut s.meta);
        if let Some(st) = &out {
            let mut exported = s.clone();
            exported.meta.source = SessionSource::Synthetic;
            st.save(&exported, raw.as_bytes()).unwrap();
        }
        sessions.push(s.meta.clone());
        s.meta.id
    };
    let report = |sessions: &[SessionMeta], profile| {
        smart_packs::build(
            &e,
            BuildInput {
                game: &g,
                sessions,
                profile,
                limits: Limits::default(),
                launch_line: fx.line(),
            },
        )
        .unwrap()
    };
    let show = |ev: &PackEvaluation| {
        println!(
            "  [{:?}{}] {}",
            ev.verdict,
            if ev.preliminary { " · preliminar" } else { "" },
            ev.headline
        );
        println!("    {}", ev.summary);
        if let Some(n) = &ev.next_step {
            println!("    próximo passo: {n}");
        }
        println!("    ranking: {}", ev.ranking_effect);
    };

    // 1. Referência (configuração normal).
    let r = report(&sessions, Profile::Performance);
    let pack = r
        .packs
        .iter()
        .find(|p| p.env.len() == 1 && p.env[0].key == "PROTON_NO_D3D11")
        .cloned()
        .unwrap();
    let b1 = capture("referência 1", 95.0, 0.3, &mut sessions);
    let ev = smart_packs::evaluate_session(&e, &sessions, &b1).unwrap();
    println!("Captura da referência:");
    show(&ev);
    assert_eq!(ev.verdict, EvalVerdict::Reference);

    // 2. Aplica o pacote (prévia → confirmação) e captura.
    let p = e
        .prepare(pack.apply_recipe_id.as_deref().unwrap(), &g)
        .unwrap();
    let op = e.apply(&p.plan_id, &p.fingerprint, &g, &noop).unwrap();
    assert_eq!(op.state, OpState::Applied);
    let d1 = capture("PROTON_NO_D3D11 1", 55.0, 1.1, &mut sessions);
    let m = sessions.iter().find(|s| s.id == d1).unwrap();
    assert_eq!(
        m.smart_pack.as_ref().unwrap().pack_id,
        pack.id,
        "associada sozinha ao pack ativo"
    );
    let ev = smart_packs::evaluate_session(&e, &sessions, &d1).unwrap();
    println!("\nPrimeira captura com o pacote:");
    show(&ev);
    assert_eq!(
        (ev.verdict, ev.preliminary),
        (EvalVerdict::Regression, true)
    );
    assert_eq!(
        ev.current_pack_id.as_deref(),
        Some(pack.id.as_str()),
        "oferece restaurar"
    );
    let r = report(&sessions, Profile::Performance);
    let en = r
        .ranking
        .entries
        .iter()
        .find(|x| x.pack_id == pack.id)
        .unwrap();
    println!("  ranking: [{:?}] {}", en.tier, en.headline);
    assert_eq!(
        en.tier,
        Tier::Candidate,
        "uma captura não comprova a regressão"
    );

    // 3. Repetição: pacote ainda ativo; depois restaura e repete a referência.
    let d2 = capture("PROTON_NO_D3D11 2", 57.0, 2.3, &mut sessions);
    let sw = smart_packs::switch_plan(&e, "200260", &r.packs[0]);
    println!(
        "\nRestaurar: remove {:?} em {} passo(s)",
        sw.remove
            .iter()
            .map(|x| format!("{}={}", x.key, x.value))
            .collect::<Vec<_>>(),
        sw.steps.len()
    );
    let rb = e.prepare_rollback(&op.id).unwrap();
    assert_eq!(
        e.rollback(&op.id, &rb.fingerprint, &noop).unwrap().state,
        OpState::RolledBack
    );
    assert_eq!(
        std::fs::read(&fx.config).unwrap(),
        original,
        "configuração anterior restaurada byte a byte"
    );
    let b2 = capture("referência 2", 92.0, 4.7, &mut sessions);
    assert_eq!(
        sessions
            .iter()
            .find(|s| s.id == b2)
            .unwrap()
            .smart_pack
            .as_ref()
            .unwrap()
            .pack_id,
        r.baseline_id
    );
    let ev = smart_packs::evaluate_session(&e, &sessions, &d2).unwrap();
    println!("\nCom 2 × 2 capturas:");
    show(&ev);
    for d in &ev.deltas {
        println!(
            "    {:<18} {:>7.1} → {:>7.1} {:<4} ({:+.1}%, limiar ±{:.1}%) {:?}",
            d.label, d.before, d.after, d.unit, d.delta_pct, d.threshold_pct, d.outcome
        );
    }
    assert_eq!(
        (ev.verdict, ev.preliminary),
        (EvalVerdict::Regression, false)
    );
    for profile in [Profile::Performance, Profile::Balanced] {
        let r = report(&sessions, profile);
        let en = r
            .ranking
            .entries
            .iter()
            .find(|x| x.pack_id == pack.id)
            .unwrap();
        println!("  ranking {:?}: [{:?}] {}", profile, en.tier, en.headline);
        assert_eq!(en.tier, Tier::NotRecommended);
        assert!(r.ranking.winner.is_none());
    }
    let _ = d1;
}

mod file_artifacts {
    use super::*;
    include!("artifact_tests.rs");
    include!("file_security_tests.rs");
    include!("https_fixture_test.rs");
}
