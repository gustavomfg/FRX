use crate::fix_engine::availability::AvailabilityState as State;
use crate::fixes::models::ActionKind;
fn finding_game(fx: &Fx) -> GameCtx {
    let mut g = fx.game();
    g.findings = vec![crate::fix_engine::dynamic::tests::finding(
        "synthetic-applicability",
        ActionKind::Env,
        "PROTON_HEAP_DELAY_FREE=1",
        2,
        0,
    )];
    g
}
#[test]
fn list_is_backed_by_real_preview_and_revalidates_current_disk() {
    let fx = fixture();
    let e = fx.engine();
    let g = finding_game(&fx);
    let before = fx.line();
    let views = e.analyze(&g).unwrap();
    let a = &views[0];
    assert_eq!(a.availability.state, State::Available);
    let p = e
        .prepare(a.assessment.recipe_id.as_ref().unwrap(), &g)
        .unwrap();
    assert_eq!(
        a.availability.plan_fingerprint.as_ref(),
        Some(&p.fingerprint)
    );
    assert_eq!(fx.line(), before);
    fx.set_line("PROTON_HEAP_DELAY_FREE=0 %command%");
    let changed = e.analyze(&g).unwrap();
    assert_ne!(changed[0].availability.state, State::Available);
    assert_eq!(
        changed[0].availability.reasons[0].code,
        "launch-options-conflict"
    );
    assert!(e.apply(&p.plan_id, &p.fingerprint, &g, &|_| {}).is_err());
    assert_eq!(
        fx.line().as_deref(),
        Some("PROTON_HEAP_DELAY_FREE=0 %command%")
    );
}
#[test]
fn steam_and_game_are_temporary_not_false_ready_or_permanently_unsupported() {
    let fx = fixture();
    let e = fx.engine();
    let g = finding_game(&fx);
    fx.proc("11", "steam", "SteamAppId=0\0");
    let a = e.analyze(&g).unwrap().remove(0);
    assert_eq!(a.availability.state, State::Temporary);
    assert!(a
        .availability
        .reasons
        .iter()
        .any(|r| r.code == "steam-running"));
    fx.proc("12", "synthetic-game", "SteamAppId=200260\0");
    let a = e.analyze(&g).unwrap().remove(0);
    assert_eq!(a.availability.state, State::Temporary);
    assert!(a
        .availability
        .reasons
        .iter()
        .any(|r| r.code == "game-running"));
}
#[test]
fn missing_recipe_or_changed_source_cannot_reuse_old_cached_availability() {
    let fx = fixture();
    let e = fx.engine();
    let mut g = finding_game(&fx);
    let initial = e.analyze(&g).unwrap().remove(0);
    assert_eq!(initial.availability.state, State::Available);
    let id = initial.assessment.recipe_id.unwrap();
    g.findings[0].actions[0].value = "UNTRUSTED_UNKNOWN_OPTION=1".into();
    assert_eq!(
        e.analyze(&g).unwrap()[0].availability.state,
        State::Investigation
    );
    assert_eq!(e.availability(&id, &g).state, State::Investigation);
    assert_eq!(
        e.availability("recipe-does-not-exist", &g).reasons[0].code,
        "recipe-revalidation"
    );
}
#[test]
fn unsupported_dependencies_and_foreign_scope_have_explicit_reasons() {
    let fx = fixture();
    let e = fx.engine();
    let mut g = finding_game(&fx);
    g.findings[0].actions[0].kind = ActionKind::Winetricks;
    g.findings[0].actions[0].value = "vcrun2022".into();
    let a = e.analyze(&g).unwrap().remove(0);
    assert_eq!(a.availability.state, State::Investigation);
    assert_eq!(a.availability.reasons[0].code, "dependency-unsupported");
    g.findings[0].game_app_id = "999".into();
    assert_ne!(
        e.analyze(&g).unwrap()[0].availability.state,
        State::Available
    );
}
#[test]
fn proton_changes_and_applied_drift_refresh_without_discovery_refresh() {
    let fx = fixture();
    let e = fx.engine();
    let mut g = finding_game(&fx);
    g.uses_proton = Some(false);
    assert_eq!(
        e.analyze(&g).unwrap()[0].availability.state,
        State::Incompatible
    );
    g.uses_proton = Some(true);
    let a = e.analyze(&g).unwrap().remove(0);
    let p = e
        .prepare(a.assessment.recipe_id.as_ref().unwrap(), &g)
        .unwrap();
    e.apply(&p.plan_id, &p.fingerprint, &g, &|_| {}).unwrap();
    assert_eq!(e.analyze(&g).unwrap()[0].availability.state, State::Applied);
    fx.set_line("%command%");
    let a = e.analyze(&g).unwrap().remove(0);
    assert_eq!(a.availability.state, State::Temporary);
    assert_eq!(a.availability.reasons[0].code, "applied-state-drift");
}
#[test]
fn invalid_automatic_path_is_not_advertised_despite_a_generated_recipe() {
    let fx = fixture();
    let e = fx.engine();
    let g = finding_game(&fx);
    fx.set_line("echo arbitrary && %command%");
    let a = e.analyze(&g).unwrap().remove(0);
    assert!(a.assessment.recipe_id.is_some());
    assert_eq!(a.availability.state, State::Investigation);
}

fn with_config(fx: &Fx) -> GameCtx {
    let mut game = finding_game(fx);
    game.findings
        .push(crate::fix_engine::dynamic::tests::finding(
            "synthetic-dxvk-config",
            ActionKind::ConfigFile,
            "Config/dxvk.conf: dxgi.maxFrameLatency=1",
            2,
            0,
        ));
    game
}
fn packs(engine: &FixEngine, game: &GameCtx) -> crate::smart_packs::models::SmartPackReport {
    crate::smart_packs::build(
        engine,
        crate::smart_packs::BuildInput {
            game,
            sessions: &[],
            profile: crate::smart_packs::models::Profile::Balanced,
            limits: Default::default(),
            launch_line: None,
        },
    )
    .unwrap()
}
#[test]
fn smart_pack_config_and_variable_use_actual_merged_bytes_and_persist_rollback() {
    use crate::smart_packs::{benchmark, models::PackKind};
    let fx = fixture();
    let e = fx.engine();
    let g = with_config(&fx);
    let path = fx.library.join("Config/dxvk.conf");
    let original = "# synthetic fixture\ndxgi.maxFrameLatency = 3\ndxgi.enableHDR = False\n";
    std::fs::write(&path, original).unwrap();
    let launch = fx.line();
    let report = packs(&e, &g);
    let combo = report
        .packs
        .iter()
        .find(|p| p.kind == PackKind::Combination)
        .expect("supported env and config compose");
    assert_eq!(combo.availability.state, State::Available);
    assert_eq!(combo.file_fixes.len(), 1);
    let plan = e
        .prepare(combo.apply_recipe_id.as_ref().unwrap(), &g)
        .unwrap();
    assert_eq!(plan.status, PlanStatus::Ready);
    assert_eq!(plan.changes.len(), 2);
    let op = e
        .apply(&plan.plan_id, &plan.fingerprint, &g, &|_| {})
        .unwrap();
    assert_eq!(op.state, OpState::Applied);
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        combo.file_fixes[0].sha256,
        crate::fix_engine::sha256_hex(&bytes)
    );
    let restarted = fx.engine();
    let snapshot = benchmark::snapshot(&restarted, "200260");
    assert_eq!(snapshot.pack_id, combo.id);
    assert!(snapshot.verified);
    assert!(
        packs(&restarted, &g).packs.iter().any(|p| p.id == combo.id),
        "applied composed config remains visible after restart"
    );
    let undo = restarted.prepare_rollback(&op.id).unwrap();
    restarted
        .rollback(&op.id, &undo.fingerprint, &|_| {})
        .unwrap();
    assert_eq!(std::fs::read_to_string(path).unwrap(), original);
    assert_eq!(fx.line(), launch);
}
#[test]
fn current_config_changes_invalidate_pack_identity_without_discarding_other_fixes() {
    use crate::smart_packs::models::PackKind;
    let fx = fixture();
    let e = fx.engine();
    let mut g = with_config(&fx);
    let path = fx.library.join("Config/dxvk.conf");
    std::fs::write(&path, "# before\n").unwrap();
    let report = packs(&e, &g);
    let old = report
        .packs
        .iter()
        .find(|p| p.kind == PackKind::Combination)
        .unwrap();
    std::fs::write(&path, "# externally changed\n").unwrap();
    assert!(e.check(old.apply_recipe_id.as_ref().unwrap(), &g).is_err());
    let new = packs(&e, &g);
    let changed = new
        .packs
        .iter()
        .find(|p| p.kind == PackKind::Combination)
        .unwrap();
    assert_ne!(old.id, changed.id);
    g.findings.push(crate::fix_engine::dynamic::tests::finding(
        "synthetic-unsupported",
        ActionKind::Winetricks,
        "physx",
        2,
        0,
    ));
    std::fs::write(&path, "[game.exe]\ndxgi.maxFrameLatency=3\n").unwrap();
    let a = e.analyze(&g).unwrap();
    let config = a
        .iter()
        .find(|a| a.assessment.finding_id == "synthetic-dxvk-config")
        .unwrap();
    assert_eq!(config.availability.state, State::Investigation);
    assert!(config
        .availability
        .reasons
        .iter()
        .any(|r| r.code == "configuration-mismatch"));
    let report = packs(&e, &g);
    assert!(report
        .packs
        .iter()
        .any(|p| p.kind == PackKind::Single && !p.env.is_empty()));
    assert!(!report.packs.iter().any(|p| !p.file_fixes.is_empty()));
    assert!(report.excluded.len() >= 2);
}
#[test]
fn smart_pack_operational_blocker_is_explicit_and_never_ready() {
    use crate::smart_packs::models::{PackKind, PackState};
    let fx = fixture();
    let e = fx.engine();
    let g = finding_game(&fx);
    fx.proc("11", "steam", "SteamAppId=0\0");
    let report = packs(&e, &g);
    let fix = report
        .packs
        .iter()
        .find(|p| p.kind == PackKind::Single)
        .unwrap();
    assert_eq!(fix.availability.state, State::Temporary);
    assert_eq!(fix.state, PackState::Conflict);
    assert!(fix
        .availability
        .reasons
        .iter()
        .any(|r| r.code == "steam-running"));
}

#[test]
fn config_requirements_and_hardware_changes_refresh_current_availability_and_packs() {
    use crate::fixes::models::GpuReq;
    use crate::system::{context::EnvironmentContext, GpuInfo, GpuVendor};
    let fx = fixture();
    let e = fx.engine();
    let mut g = with_config(&fx);
    g.findings.remove(0);
    g.findings[0].requirements.hardware.proton_versions = vec!["GE-Proton10-4".into()];
    g.findings[0].requirements.hardware.gpu_vendor = Some(GpuReq::Nvidia);
    let gpu = GpuInfo {
        pci_address: None,
        gpu_family: None,
        vram_bytes: None,
        vendor: GpuVendor::Nvidia,
        vendor_id: "10de".into(),
        device_id: "2882".into(),
        name: Some("Synthetic RTX 4060".into()),
        driver: Some("nvidia".into()),
        driver_version: Some("synthetic-1".into()),
        boot_vga: true,
    };
    assert_ne!(
        e.analyze(&g).unwrap()[0].availability.state,
        State::Available
    );
    g.performance_conditions = Some(crate::performance::models::Conditions {
        environment: Some(EnvironmentContext {
            detected_gpus: vec![gpu.clone()],
            proton_version: Some("GE-Proton10-4".into()),
            ..Default::default()
        }),
        ..Default::default()
    });
    let ready = e.analyze(&g).unwrap().remove(0);
    assert_eq!(ready.availability.state, State::Available, "{ready:#?}");
    assert_eq!(packs(&e, &g).packs.len(), 2);
    g.performance_conditions
        .as_mut()
        .unwrap()
        .environment
        .as_mut()
        .unwrap()
        .detected_gpus[0]
        .vendor = GpuVendor::Amd;
    assert_eq!(
        e.analyze(&g).unwrap()[0].availability.state,
        State::Incompatible
    );
    assert_eq!(packs(&e, &g).packs.len(), 1);
    g.performance_conditions
        .as_mut()
        .unwrap()
        .environment
        .as_mut()
        .unwrap()
        .detected_gpus = vec![gpu];
    g.performance_conditions
        .as_mut()
        .unwrap()
        .environment
        .as_mut()
        .unwrap()
        .proton_version = Some("Valve Proton 9".into());
    assert_eq!(
        e.analyze(&g).unwrap()[0].availability.state,
        State::Incompatible
    );
    g.performance_conditions
        .as_mut()
        .unwrap()
        .environment
        .as_mut()
        .unwrap()
        .proton_version = None;
    assert_ne!(
        e.analyze(&g).unwrap()[0].availability.state,
        State::Available
    );
}

#[test]
fn unavailable_backup_or_destination_permissions_refuse_real_preview_without_writes() {
    use std::os::unix::fs::PermissionsExt;
    let fx = fixture();
    let e = fx.engine();
    let g = with_config(&fx);
    let before = fx.line();
    let views = e.analyze(&g).unwrap();
    let id = views
        .iter()
        .find(|v| v.assessment.finding_id == "synthetic-dxvk-config")
        .unwrap()
        .assessment
        .recipe_id
        .as_ref()
        .unwrap();
    for (directory, code) in [
        (e.root().join("backups"), "recovery-storage-unavailable"),
        (fx.library.join("Config"), "destination-permission"),
    ] {
        let permissions = std::fs::metadata(&directory).unwrap().permissions();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o500)).unwrap();
        let available = e.availability(id, &g);
        let plan = e.prepare(id, &g).unwrap();
        std::fs::set_permissions(&directory, permissions).unwrap();
        assert_eq!(available.state, State::Investigation);
        assert!(
            available.reasons.iter().any(|r| r.code == code),
            "{available:#?}"
        );
        assert!(!plan.executable());
        assert_eq!(fx.line(), before);
        assert!(!fx.library.join("Config/dxvk.conf").exists());
    }
}

#[test]
fn legacy_cached_assessments_and_packs_never_invent_current_availability() {
    let fx = fixture();
    let e = fx.engine();
    let g = finding_game(&fx);
    let assessment = e.analyze(&g).unwrap().remove(0);
    let mut json = serde_json::to_value(assessment).unwrap();
    json.as_object_mut().unwrap().remove("availability");
    let legacy: crate::fix_engine::dynamic::AssessmentView = serde_json::from_value(json).unwrap();
    assert_eq!(legacy.availability.state, State::Investigation);
    assert_eq!(legacy.availability.reasons[0].code, "not-validated");
    let pack = packs(&e, &g)
        .packs
        .into_iter()
        .find(|p| p.apply_recipe_id.is_some())
        .unwrap();
    let mut json = serde_json::to_value(pack).unwrap();
    json.as_object_mut().unwrap().remove("availability");
    let legacy: crate::smart_packs::models::SmartPack = serde_json::from_value(json).unwrap();
    assert_eq!(legacy.availability.state, State::Investigation);
}

#[test]
fn pending_nonoverlapping_operation_after_listing_blocks_preview_and_apply_but_not_recovery() {
    for state in [OpState::RequiresManualStep, OpState::RecoveryRequired] {
        let fx = fixture();
        let e = fx.engine();
        let g = finding_game(&fx);
        let a = e.analyze(&g).unwrap().remove(0);
        let id = a.assessment.recipe_id.as_ref().unwrap();
        let old = e.prepare(id, &g).unwrap();
        let original = fx.line();
        fx.proc("15", "steam", "SteamAppId=0\0");
        let other = e.prepare("frx.diagnostics.proton-log", &g).unwrap();
        let mut op = e
            .apply(&other.plan_id, &other.fingerprint, &g, &|_| {})
            .unwrap();
        assert_eq!(op.state, OpState::RequiresManualStep);
        op.state = state;
        e.journal.save(&op).unwrap();
        fx.kill("15");
        let availability = e.availability(id, &g);
        assert_eq!(availability.state, State::Temporary);
        assert!(availability
            .reasons
            .iter()
            .any(|r| r.code == "pending-operation"));
        let fresh = e.prepare(id, &g).unwrap();
        assert!(!fresh.executable());
        assert_ne!(old.fingerprint, fresh.fingerprint);
        assert!(e
            .apply(&old.plan_id, &old.fingerprint, &g, &|_| {})
            .is_err());
        assert_eq!(fx.line(), original);
        let undo = e.prepare_rollback(&op.id).unwrap();
        assert_eq!(undo.status, PlanStatus::Ready);
        let restored = e.rollback(&op.id, &undo.fingerprint, &|_| {}).unwrap();
        assert_eq!(
            restored.state,
            if state == OpState::RequiresManualStep {
                OpState::Failed // Existing terminal state for cancellation before any write.
            } else {
                OpState::RolledBack
            }
        );
        assert_eq!(e.availability(id, &g).state, State::Available);
    }
}
