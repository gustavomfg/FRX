// Explicit synthetic sources and installations; never the real Steam.
use crate::fixes::{
    aggregation::{group_mentions, Mention},
    models::*,
    tweaks::scan_text,
};
fn discovered(text: &str) -> FixFinding {
    let signals = scan_text(text);
    let mentions = signals
        .tweaks
        .into_iter()
        .map(|(tweak, snippet)| Mention {
            scope: DiscoveryScope::verified("200260", "Synthetic fixture source confirms AppID"),
            performance_reports: vec![],
            tweak,
            source: FindingSource {
                provider: ProviderId::Github,
                label: "Synthetic test source".into(),
                url: "https://github.com/ValveSoftware/Proton/issues/99999#issuecomment-111".into(),
                reference_id: None,
                snippet: Some(snippet),
                published_at: None,
            },
            positive: true,
            negative: false,
            gpus: vec![],
            proton_versions: vec![],
        })
        .collect();
    let findings = group_mentions("200260", mentions, &crate::time::now_iso());
    assert_eq!(findings.len(), 1);
    findings[0].clone()
}
#[test]
fn documented_dxvk_and_environment_are_one_transaction_preserving_unrelated_lines() {
    let fx = fixture();
    let e = fx.engine();
    let mut g = fx.game();
    let target = fx.library.join("dxvk.conf");
    let old = "# personal setting\ndxgi.maxFrameLatency = 3\ndxgi.enableHDR = False\n";
    std::fs::write(&target, old).unwrap();
    let launch = fx.line();
    g.findings = vec![discovered(
        "dxvk.conf: dxgi.maxFrameLatency = 1\nPROTON_NO_ESYNC=1",
    )];
    let views = e.analyze(&g).unwrap();
    let a = &views[0].assessment;
    assert_eq!(
        a.status,
        super::super::dynamic::DynStatus::Generated,
        "{a:#?}"
    );
    let plan = e.prepare(a.recipe_id.as_ref().unwrap(), &g).unwrap();
    assert_eq!(plan.status, PlanStatus::Ready, "{plan:#?}");
    assert_eq!(plan.changes.len(), 2);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), old);
    let op = e
        .apply(&plan.plan_id, &plan.fingerprint, &g, &|_| {})
        .unwrap();
    assert_eq!(op.state, OpState::Applied);
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "# personal setting\ndxgi.maxFrameLatency = 1\ndxgi.enableHDR = False\n"
    );
    assert!(fx.line().unwrap().contains("PROTON_NO_ESYNC=1"));
    let restarted = fx.engine();
    let undo = restarted.prepare_rollback(&op.id).unwrap();
    restarted
        .rollback(&op.id, &undo.fingerprint, &|_| {})
        .unwrap();
    assert_eq!(std::fs::read_to_string(target).unwrap(), old);
    assert_eq!(fx.line(), launch);
}
#[test]
fn dxvk_creation_and_multiple_keys_roll_back_by_removing_only_added_file() {
    let fx = fixture();
    let e = fx.engine();
    let mut g = fx.game();
    g.findings = vec![discovered(
        "Config/dxvk.conf: dxgi.enableHDR=True\nConfig/dxvk.conf: dxgi.maxFrameLatency=2",
    )];
    let a = e.analyze(&g).unwrap().remove(0).assessment;
    let plan = e.prepare(a.recipe_id.as_ref().unwrap(), &g).unwrap();
    assert_eq!(plan.status, PlanStatus::Ready);
    assert_eq!(plan.changes.len(), 1);
    let op = e
        .apply(&plan.plan_id, &plan.fingerprint, &g, &|_| {})
        .unwrap();
    let content = std::fs::read_to_string(fx.library.join("Config/dxvk.conf")).unwrap();
    assert!(content.contains("dxgi.enableHDR = True"));
    assert!(content.contains("dxgi.maxFrameLatency = 2"));
    let rb = e.prepare_rollback(&op.id).unwrap();
    e.rollback(&op.id, &rb.fingerprint, &|_| {}).unwrap();
    assert!(!fx.library.join("Config/dxvk.conf").exists());
}
#[test]
fn unknown_steps_and_invalid_config_are_never_partially_generated() {
    for source in [
        "dxvk.conf: dxgi.enableHDR=True\nwinetricks physx",
        "dxvk.conf: dxgi.enableHDR=True\ndownload a DLL",
        "dxvk.conf: dxgi.enableHDR=True\ndxvk.conf: arbitrary.command=evil",
        "../dxvk.conf: dxgi.enableHDR=True",
    ] {
        let fx = fixture();
        let e = fx.engine();
        let mut g = fx.game();
        g.findings = vec![discovered(source)];
        let a = e.analyze(&g).unwrap().remove(0).assessment;
        assert!(a.recipe_id.is_none(), "{source}: {a:#?}");
        assert!(e.operations(None).is_empty());
    }
}

#[test]
fn unrecognized_shell_and_file_steps_are_kept_in_review_as_a_whole() {
    for extra in [
        "rm obsolete-file.ini",
        "cp replacement.dat game.dat",
        "python3 patch.py",
        "./community-installer",
        "~/Downloads/install-helper",
        "extract the other archive first",
        "remova a configuração antiga",
        "PROTON_NO_ESYNC=1 && unknown-step",
        "only for a custom component build",
    ] {
        let fx = fixture();
        let engine = fx.engine();
        let mut g = fx.game();
        g.findings = vec![discovered(&format!(
            "dxvk.conf: dxgi.maxFrameLatency=1\n{extra}"
        ))];
        let a = engine.analyze(&g).unwrap().remove(0).assessment;
        assert!(
            a.recipe_id.is_none(),
            "Incomplete procedure offered for {extra}: {a:#?}"
        );
        assert!(!fx.library.join("dxvk.conf").exists());
        assert!(engine.operations(None).is_empty());
    }
}
#[test]
fn duplicate_and_sectioned_configs_refuse_preview_without_writes() {
    for original in [
        "dxgi.enableHDR=False\ndxgi.enableHDR=True\n",
        "[game.exe]\ndxgi.enableHDR=False\n",
        "unsupported malformed line\n",
    ] {
        let fx = fixture();
        let e = fx.engine();
        let mut g = fx.game();
        let file = fx.library.join("dxvk.conf");
        std::fs::write(&file, original).unwrap();
        g.findings = vec![discovered("dxvk.conf: dxgi.enableHDR=True")];
        let a = e.analyze(&g).unwrap().remove(0).assessment;
        let plan = e.prepare(a.recipe_id.as_ref().unwrap(), &g).unwrap();
        assert!(!plan.executable());
        assert_eq!(std::fs::read_to_string(file).unwrap(), original);
    }
}
#[test]
fn config_source_or_destination_change_after_preview_prevents_all_writes() {
    for source_change in [true, false] {
        let fx = fixture();
        let e = fx.engine();
        let mut g = fx.game();
        let path = fx.library.join("dxvk.conf");
        std::fs::write(&path, "dxgi.enableHDR=False\n").unwrap();
        g.findings = vec![discovered(
            "dxvk.conf: dxgi.enableHDR=True\nPROTON_NO_ESYNC=1",
        )];
        let before = fx.line();
        let a = e.analyze(&g).unwrap().remove(0).assessment;
        let plan = e.prepare(a.recipe_id.as_ref().unwrap(), &g).unwrap();
        if source_change {
            g.findings[0].actions[0].value = "dxvk.conf: dxgi.enableHDR=False".into();
        } else {
            std::fs::write(&path, "# externally changed\n").unwrap();
        }
        assert!(e
            .apply(&plan.plan_id, &plan.fingerprint, &g, &|_| {})
            .is_err());
        assert_eq!(fx.line(), before);
    }
}
#[test]
fn config_symlink_never_writes_another_installation() {
    let fx = fixture();
    let e = fx.engine();
    let mut g = fx.game();
    let outside = fx._tmp.path().join("other-game.conf");
    std::fs::write(&outside, "untouched=1").unwrap();
    std::os::unix::fs::symlink(&outside, fx.library.join("dxvk.conf")).unwrap();
    g.findings = vec![discovered("dxvk.conf: dxgi.enableHDR=True")];
    let a = e.analyze(&g).unwrap().remove(0).assessment;
    let plan = e.prepare(a.recipe_id.as_ref().unwrap(), &g).unwrap();
    assert!(!plan.executable());
    assert_eq!(std::fs::read_to_string(outside).unwrap(), "untouched=1");
}

#[test]
fn documented_proton_restrictions_are_required_and_preserved_for_config_only() {
    use super::super::dynamic::DynStatus;
    for (actual, expected) in [
        (None, DynStatus::NeedsValidation),
        (Some("Proton 10"), DynStatus::Incompatible),
        (Some("GE-Proton10-4"), DynStatus::Generated),
    ] {
        let fx = fixture();
        let engine = fx.engine();
        let mut g = fx.game();
        g.tool_name = actual.map(str::to_owned);
        let mut finding = discovered("dxvk.conf: dxgi.maxFrameLatency=1");
        finding.requirements.proton_versions = vec!["GE-Proton10-4".into()];
        g.findings = vec![finding];
        let a = engine.analyze(&g).unwrap().remove(0).assessment;
        assert_eq!(a.status, expected, "{a:#?}");
        if let Some(recipe) = a.recipe {
            assert_eq!(recipe.hardware.proton_versions, vec!["GE-Proton10-4"]);
            let plan = engine.prepare(&recipe.id, &g).unwrap();
            assert_eq!(plan.status, PlanStatus::Ready, "{plan:#?}");
            g.tool_name = Some("Proton 10".into());
            assert!(engine
                .apply(&plan.plan_id, &plan.fingerprint, &g, &noop)
                .is_err());
        }
        assert!(!fx.library.join("dxvk.conf").exists());
    }
}

#[test]
fn game_build_requirement_is_checked_again_after_preview() {
    use super::super::dynamic::DynStatus;
    let fx = fixture();
    let engine = fx.engine();
    let mut g = fx.game();
    let mut finding = discovered("dxvk.conf: dxgi.maxFrameLatency=1");
    finding.requirements.game_version = Some("fixture-build-1".into());
    g.findings = vec![finding];
    assert_eq!(
        engine.analyze(&g).unwrap()[0].assessment.status,
        DynStatus::NeedsValidation
    );
    g.performance_conditions = Some(crate::performance::models::Conditions {
        game_build_id: Some("different-build".into()),
        ..Default::default()
    });
    assert_eq!(
        engine.analyze(&g).unwrap()[0].assessment.status,
        DynStatus::Incompatible
    );
    g.performance_conditions.as_mut().unwrap().game_build_id = Some("fixture-build-1".into());
    let a = engine.analyze(&g).unwrap().remove(0).assessment;
    let plan = engine.prepare(a.recipe_id.as_ref().unwrap(), &g).unwrap();
    assert_eq!(plan.status, PlanStatus::Ready);
    g.performance_conditions.as_mut().unwrap().game_build_id = Some("updated-by-steam".into());
    assert!(engine
        .apply(&plan.plan_id, &plan.fingerprint, &g, &noop)
        .is_err());
    assert!(!fx.library.join("dxvk.conf").exists());
}

#[test]
fn contradictory_legacy_and_structured_requirements_do_not_broaden_scope() {
    use super::super::dynamic::DynStatus;
    let fx = fixture();
    let engine = fx.engine();
    let mut g = fx.game();
    let mut finding = discovered("dxvk.conf: dxgi.maxFrameLatency=1");
    finding.requirements.proton_versions = vec!["GE-Proton10-4".into()];
    finding.requirements.hardware.proton_versions = vec!["Proton 10".into()];
    g.tool_name = Some("Proton 10".into());
    g.findings = vec![finding];
    assert_eq!(
        engine.analyze(&g).unwrap()[0].assessment.status,
        DynStatus::NeedsValidation
    );
    g.findings[0].requirements.hardware.proton_versions =
        vec!["GE-Proton10-4".into(), "Proton 10".into()];
    g.tool_name = Some("GE-Proton10-4".into());
    let a = engine.analyze(&g).unwrap().remove(0).assessment;
    assert_eq!(a.status, DynStatus::Generated);
    assert_eq!(
        a.recipe.unwrap().hardware.proton_versions,
        vec!["GE-Proton10-4"]
    );
}

#[test]
fn same_actions_with_different_source_requirements_keep_independent_identity() {
    let fx = fixture();
    let engine = fx.engine();
    let mut g = fx.game();
    let finding = discovered("dxvk.conf: dxgi.maxFrameLatency=1");
    let mut scoped = finding.clone();
    scoped.id.push_str("-build-specific");
    scoped.requirements.game_version = Some("fixture-build-1".into());
    g.performance_conditions = Some(crate::performance::models::Conditions {
        game_build_id: Some("fixture-build-1".into()),
        ..Default::default()
    });
    g.findings = vec![finding, scoped];
    let views = engine.analyze(&g).unwrap();
    assert_ne!(views[0].assessment.recipe_id, views[1].assessment.recipe_id);
    let plan = engine
        .prepare(views[0].assessment.recipe_id.as_ref().unwrap(), &g)
        .unwrap();
    g.findings[0].requirements.game_version = Some("fixture-build-1".into());
    assert!(engine
        .apply(&plan.plan_id, &plan.fingerprint, &g, &noop)
        .is_err());
    assert!(!fx.library.join("dxvk.conf").exists());
}

#[test]
fn matching_configuration_is_idempotent_without_blocking_other_required_steps() {
    let fx = fixture();
    let engine = fx.engine();
    let mut g = fx.game();
    let file = fx.library.join("dxvk.conf");
    let original = "dxgi.maxFrameLatency = 1\n";
    std::fs::write(&file, original).unwrap();
    g.findings = vec![discovered("dxvk.conf: dxgi.maxFrameLatency=1")];
    let a = engine.analyze(&g).unwrap().remove(0).assessment;
    assert_eq!(
        engine
            .prepare(a.recipe_id.as_ref().unwrap(), &g)
            .unwrap()
            .status,
        PlanStatus::AlreadyApplied
    );
    g.findings = vec![discovered(
        "dxvk.conf: dxgi.maxFrameLatency=1\nPROTON_NO_ESYNC=1",
    )];
    let before = fx.line();
    let a = engine.analyze(&g).unwrap().remove(0).assessment;
    let plan = engine.prepare(a.recipe_id.as_ref().unwrap(), &g).unwrap();
    assert_eq!(plan.status, PlanStatus::Ready);
    let op = engine
        .apply(&plan.plan_id, &plan.fingerprint, &g, &noop)
        .unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
    assert!(fx.line().unwrap().contains("PROTON_NO_ESYNC=1"));
    let rb = engine.prepare_rollback(&op.id).unwrap();
    engine.rollback(&op.id, &rb.fingerprint, &noop).unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
    assert_eq!(fx.line(), before);
}
