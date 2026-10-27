// Additional security and recovery tests, included after artifact_tests.rs.
// All publishers, payloads, processes and Steam installations are synthetic.
// No network request, real Steam installation or executable is used.

#[test]
fn readonly_original_is_backed_up_but_never_replaced() {
    use std::os::unix::fs::PermissionsExt;
    let fx = fixture();
    let target = fx.library.join("Config/frx.ini");
    let original = b"[fixture]\nvalue=old\n";
    std::fs::write(&target, original).unwrap();
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o444)).unwrap();
    let (candidate, transport) = direct(b"[fixture]\nvalue=new\n", Some(original));
    let (engine, game, id) = setup(&fx, candidate, transport);
    let preview = engine.prepare(&id, &game).unwrap();
    assert_eq!(preview.status, PlanStatus::Ready);
    let operation = engine
        .apply(&preview.plan_id, &preview.fingerprint, &game, &noop)
        .unwrap();
    assert_eq!(operation.state, OpState::Failed, "{:?}", operation.message);
    assert_eq!(std::fs::read(&target).unwrap(), original);
    assert_eq!(
        std::fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o444
    );
    let backup = operation.steps[0].backup.as_ref().unwrap();
    assert_eq!(std::fs::read(&backup.path).unwrap(), original);
    assert_eq!(backup.sha256, Some(sha256_hex(original)));
    assert_eq!(std::fs::read(&fx.config).unwrap(), LOCALCONFIG.as_bytes());
}

fn archive_with_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
    use std::io::Write;
    let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in entries {
        archive.start_file(*name, options).unwrap();
        archive.write_all(bytes).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

#[test]
fn failed_second_file_restores_first_and_preserves_unwritten_original() {
    use std::os::unix::fs::PermissionsExt;
    let fx = fixture();
    let first = fx.library.join("Config/first.ini");
    let second = fx.library.join("Config/second.ini");
    let old_first = b"value=old-first\n";
    let old_second = b"value=old-second\n";
    let new_first = b"value=new-first\n";
    let new_second = b"value=new-second\n";
    std::fs::write(&first, old_first).unwrap();
    std::fs::write(&second, old_second).unwrap();
    let archive = archive_with_entries(&[("first.ini", new_first), ("second.ini", new_second)]);
    let files = [
        (
            "first.ini",
            "Config/first.ini",
            &old_first[..],
            &new_first[..],
        ),
        (
            "second.ini",
            "Config/second.ini",
            &old_second[..],
            &new_second[..],
        ),
    ]
    .into_iter()
    .map(|(source, target, original, replacement)| ArtifactFile {
        source_path: Some(source.into()),
        target_path: target.into(),
        sha256: Some(sha256_hex(replacement)),
        replace: true,
        expected_original_sha256: Some(sha256_hex(original)),
    })
    .collect();
    let (candidate, transport) = release(&archive, files, ArtifactFormat::Zip);
    let (engine, game, id) = setup(&fx, candidate, transport);
    let preview = engine.prepare(&id, &game).unwrap();
    assert_eq!(preview.status, PlanStatus::Ready, "{:?}", preview.reasons);
    let operation = engine
        .apply(
            &preview.plan_id,
            &preview.fingerprint,
            &game,
            &|operation| {
                if operation.state == OpState::Applying {
                    // Deterministic permission failure after both verified backups,
                    // leaving the first write to exercise transaction recovery.
                    std::fs::set_permissions(&second, std::fs::Permissions::from_mode(0o444))
                        .unwrap();
                }
            },
        )
        .unwrap();
    assert_eq!(operation.state, OpState::Failed, "{:?}", operation.message);
    assert_eq!(std::fs::read(&first).unwrap(), old_first);
    assert_eq!(std::fs::read(&second).unwrap(), old_second);
    assert!(operation
        .steps
        .iter()
        .all(|step| step.status == StepStatus::Reverted));
    assert_eq!(
        std::fs::read(operation.steps[0].backup.as_ref().unwrap().path.clone()).unwrap(),
        old_first
    );
    assert_eq!(std::fs::read(&fx.config).unwrap(), LOCALCONFIG.as_bytes());
    let restarted = fx.engine();
    assert_eq!(
        restarted.operation(&operation.id).unwrap().state,
        OpState::Failed
    );
}

struct SwapParentDuringDownload {
    inner: ControlledTransport,
    parent: PathBuf,
    saved_parent: PathBuf,
    outside: PathBuf,
}

impl artifacts::Transport for SwapParentDuringDownload {
    fn get(
        &self,
        url: &str,
        limit: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<Vec<u8>, String> {
        if url != META {
            std::fs::rename(&self.parent, &self.saved_parent).unwrap();
            std::os::unix::fs::symlink(&self.outside, &self.parent).unwrap();
        }
        self.inner.get(url, limit, cancel, progress)
    }
}

#[test]
fn parent_swapped_to_symlink_during_staging_never_writes_outside_the_game() {
    let fx = fixture();
    let outside = fx._tmp.path().join("outside-installation");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("frx.ini"), b"outside=preserve\n").unwrap();
    let (candidate, transport) = direct(b"value=verified\n", None);
    let (mut engine, game, id) = setup(
        &fx,
        candidate,
        ControlledTransport {
            responses: transport.responses.clone(),
            fail: false,
            mutate: None,
        },
    );
    engine.artifact_transport = Arc::new(SwapParentDuringDownload {
        inner: transport,
        parent: fx.library.join("Config"),
        saved_parent: fx.library.join("OriginalConfig"),
        outside: outside.clone(),
    });
    let preview = engine.prepare(&id, &game).unwrap();
    assert_eq!(preview.status, PlanStatus::Ready);
    let operation = engine
        .apply(&preview.plan_id, &preview.fingerprint, &game, &noop)
        .unwrap();
    assert!(
        matches!(operation.state, OpState::Failed | OpState::Conflict),
        "{:?}",
        operation.message
    );
    assert_eq!(
        std::fs::read(outside.join("frx.ini")).unwrap(),
        b"outside=preserve\n"
    );
    assert!(!fx.library.join("OriginalConfig/frx.ini").exists());
    assert!(operation.steps.iter().all(|step| !matches!(
        step.status,
        StepStatus::Writing | StepStatus::Written | StepStatus::Verified
    )));
    assert!(!fx.data.join("staging").join(&operation.id).exists());
    assert_eq!(std::fs::read(&fx.config).unwrap(), LOCALCONFIG.as_bytes());
}

fn known_config_catalog(original: &[u8]) -> Catalog {
    let mut catalog = Catalog::bundled();
    let mut recipe = catalog.get("frx.diagnostics.proton-log").unwrap().clone();
    recipe.id = "frx.fixture.known-config".into();
    recipe.title = "Synthetic exact-known configuration fixture".into();
    recipe.scope.app_ids = vec![APP.into()];
    recipe.actions = vec![RecipeAction::PatchKnownConfig {
        path: "Config/frx.ini".into(),
        expected_sha256: sha256_hex(original),
        replacements: vec![ConfigReplacement {
            find: "mode=off".into(),
            replace: "mode=on".into(),
        }],
    }];
    super::super::security::validate_recipe(&recipe).unwrap();
    catalog.recipes = vec![recipe];
    catalog
}

#[test]
fn exact_known_config_patch_preserves_unrelated_values_and_rolls_back_after_restart() {
    let fx = fixture();
    let original = b"[Engine]\nmode=off\nquality=preserved\n";
    let target = fx.library.join("Config/frx.ini");
    std::fs::write(&target, original).unwrap();
    let catalog = known_config_catalog(original);
    let engine = fx.engine_with(catalog.clone());
    let game = fx.game();
    let preview = engine.prepare("frx.fixture.known-config", &game).unwrap();
    assert_eq!(preview.status, PlanStatus::Ready, "{:?}", preview.reasons);
    let operation = engine
        .apply(&preview.plan_id, &preview.fingerprint, &game, &noop)
        .unwrap();
    assert_eq!(operation.state, OpState::Applied, "{:?}", operation.message);
    assert_eq!(
        std::fs::read(&target).unwrap(),
        b"[Engine]\nmode=on\nquality=preserved\n"
    );
    assert_eq!(
        std::fs::read(&operation.steps[0].backup.as_ref().unwrap().path).unwrap(),
        original
    );
    let restarted = fx.engine_with(catalog);
    let rollback = restarted.prepare_rollback(&operation.id).unwrap();
    assert_eq!(rollback.status, PlanStatus::Ready);
    assert_eq!(
        restarted
            .rollback(&operation.id, &rollback.fingerprint, &noop)
            .unwrap()
            .state,
        OpState::RolledBack
    );
    assert_eq!(std::fs::read(target).unwrap(), original);
    assert_eq!(std::fs::read(&fx.config).unwrap(), LOCALCONFIG.as_bytes());
}

#[test]
fn known_config_patch_rejects_changed_original_and_ambiguous_literal() {
    for (original, expected) in [
        (
            &b"mode=off\nquality=user\n"[..],
            &b"mode=off\nquality=old\n"[..],
        ),
        (&b"mode=off\nmode=off\n"[..], &b"mode=off\nmode=off\n"[..]),
    ] {
        let fx = fixture();
        let target = fx.library.join("Config/frx.ini");
        std::fs::write(&target, original).unwrap();
        let engine = fx.engine_with(known_config_catalog(expected));
        let preview = engine
            .prepare("frx.fixture.known-config", &fx.game())
            .unwrap();
        assert_eq!(
            preview.status,
            PlanStatus::Conflict,
            "{:?}",
            preview.reasons
        );
        assert_eq!(std::fs::read(&target).unwrap(), original);
        assert!(engine.operations(None).is_empty());
    }
}

#[test]
fn archive_entry_count_and_total_expansion_limits_cover_unselected_entries() {
    let content = b"value=selected\n";
    let files = || {
        vec![ArtifactFile {
            source_path: Some("selected.ini".into()),
            target_path: "Config/frx.ini".into(),
            sha256: Some(sha256_hex(content)),
            replace: false,
            expected_original_sha256: None,
        }]
    };
    let names: Vec<String> = (0..128).map(|n| format!("ignored-{n}.bin")).collect();
    let mut entries = vec![("selected.ini", &content[..])];
    entries.extend(names.iter().map(|name| (name.as_str(), &b"x"[..])));
    let archive = archive_with_entries(&entries);
    let (candidate, _) = release(&archive, files(), ArtifactFormat::Zip);
    assert!(artifacts::contents(&candidate, &archive, &AtomicBool::new(false)).is_err());

    let block = vec![b'x'; 240_000];
    let mut entries = vec![("selected.ini", &content[..])];
    entries.extend(
        names
            .iter()
            .take(9)
            .map(|name| (name.as_str(), block.as_slice())),
    );
    let archive = archive_with_entries(&entries);
    let (candidate, _) = release(&archive, files(), ArtifactFormat::Zip);
    assert!(artifacts::contents(&candidate, &archive, &AtomicBool::new(false)).is_err());
}

#[test]
fn restart_before_the_intended_write_does_not_claim_an_installation() {
    let fx = fixture();
    let original = b"value=original\n";
    let target = fx.library.join("Config/frx.ini");
    std::fs::write(&target, original).unwrap();
    let (candidate, transport) = direct(b"value=installed\n", Some(original));
    let (engine, game, id) = setup(&fx, candidate, transport);
    let preview = engine.prepare(&id, &game).unwrap();
    let mut operation = engine
        .apply(&preview.plan_id, &preview.fingerprint, &game, &noop)
        .unwrap();
    assert_eq!(operation.state, OpState::Applied);
    // Reconstruct a crash after durable Writing intent but before the syscall.
    std::fs::write(&target, original).unwrap();
    operation.state = OpState::Applying;
    operation.steps[0].status = StepStatus::Writing;
    engine.journal.save(&operation).unwrap();
    let restarted = fx.engine();
    assert_eq!(
        restarted.operation(&operation.id).unwrap().state,
        OpState::RecoveryRequired
    );
    let verified = restarted.verify(&operation.id, &noop).unwrap();
    assert_eq!(verified.state, OpState::Failed);
    assert_eq!(std::fs::read(&target).unwrap(), original);
    assert_eq!(
        std::fs::read(&verified.steps[0].backup.as_ref().unwrap().path).unwrap(),
        original
    );
}

#[test]
fn steam_or_game_started_during_download_prevents_file_installation() {
    for process in ["steam", "game"] {
        let fx = fixture();
        let target = fx.library.join("Config/frx.ini");
        let original = b"value=original\n";
        std::fs::write(&target, original).unwrap();
        let (candidate, transport) = direct(b"value=installed\n", Some(original));
        let (engine, game, id) = setup(&fx, candidate, transport);
        let preview = engine.prepare(&id, &game).unwrap();
        assert_eq!(preview.status, PlanStatus::Ready);
        let operation = engine
            .apply(
                &preview.plan_id,
                &preview.fingerprint,
                &game,
                &|operation| {
                    if operation.state == OpState::Downloading {
                        if process == "steam" {
                            fx.proc("123", "steam\n", "");
                        } else {
                            fx.proc("123", "SyntheticGame.exe\n", "SteamAppId=200260\0");
                        }
                    }
                },
            )
            .unwrap();
        assert_eq!(
            operation.state,
            OpState::Conflict,
            "{process}: {:?}",
            operation.message
        );
        assert_eq!(std::fs::read(&target).unwrap(), original);
        assert!(operation
            .steps
            .iter()
            .all(|step| step.status == StepStatus::BackedUp));
        assert_eq!(std::fs::read(&fx.config).unwrap(), LOCALCONFIG.as_bytes());
        assert!(
            fx.proc_root.join("123").is_dir(),
            "FRX must not close processes"
        );
        assert!(!fx.data.join("staging").join(&operation.id).exists());
    }
}

#[test]
fn steam_or_game_started_during_rollback_preserves_installed_file() {
    for process in ["steam", "game"] {
        let fx = fixture();
        let target = fx.library.join("Config/frx.ini");
        let original = b"value=original\n";
        let installed = b"value=installed\n";
        std::fs::write(&target, original).unwrap();
        let (candidate, transport) = direct(installed, Some(original));
        let (engine, game, id) = setup(&fx, candidate, transport);
        let preview = engine.prepare(&id, &game).unwrap();
        let operation = engine
            .apply(&preview.plan_id, &preview.fingerprint, &game, &noop)
            .unwrap();
        assert_eq!(operation.state, OpState::Applied);
        let rollback = engine.prepare_rollback(&operation.id).unwrap();
        assert_eq!(rollback.status, PlanStatus::Ready);
        let interrupted = engine
            .rollback(&operation.id, &rollback.fingerprint, &|operation| {
                if operation.state == OpState::RollingBack {
                    if process == "steam" {
                        fx.proc("123", "steam\n", "");
                    } else {
                        fx.proc("123", "SyntheticGame.exe\n", "SteamAppId=200260\0");
                    }
                }
            })
            .unwrap();
        assert_eq!(
            interrupted.state,
            OpState::RecoveryRequired,
            "{process}: {:?}",
            interrupted.message
        );
        assert_eq!(std::fs::read(&target).unwrap(), installed);
        assert_eq!(
            std::fs::read(interrupted.steps[0].backup.as_ref().unwrap().path.clone()).unwrap(),
            original
        );
        assert!(
            fx.proc_root.join("123").is_dir(),
            "FRX must not close processes"
        );
        assert_eq!(std::fs::read(&fx.config).unwrap(), LOCALCONFIG.as_bytes());
    }
}

#[test]
fn rollback_after_restart_keeps_previous_attempt_backup_and_can_finish() {
    let fx = fixture();
    let target = fx.library.join("Config/frx.ini");
    let original = b"value=original\n";
    let installed = b"value=installed\n";
    std::fs::write(&target, original).unwrap();
    let (candidate, transport) = direct(installed, Some(original));
    let (engine, game, id) = setup(&fx, candidate, transport);
    let preview = engine.prepare(&id, &game).unwrap();
    let mut operation = engine
        .apply(&preview.plan_id, &preview.fingerprint, &game, &noop)
        .unwrap();
    assert_eq!(operation.state, OpState::Applied);
    // Reconstruct the durable state from a crash after a prior attempt saved
    // its rollback backup but before it restored the original. Preserve the
    // old fixed-name backup too, covering journals from the previous format.
    let old_backup = confined::Target::open(&fx.library, "Config/frx.ini", None)
        .unwrap()
        .backup(&engine.backups_dir(&operation.id), "rollback-0-frx.ini")
        .unwrap();
    operation.steps[0].rollback_backup = Some(old_backup.clone());
    operation.state = OpState::RollingBack;
    engine.journal.save(&operation).unwrap();
    let restarted = fx.engine();
    assert_eq!(
        restarted.operation(&operation.id).unwrap().state,
        OpState::RecoveryRequired
    );
    let rollback = restarted.prepare_rollback(&operation.id).unwrap();
    assert_eq!(rollback.status, PlanStatus::Ready);
    let restored = restarted
        .rollback(&operation.id, &rollback.fingerprint, &noop)
        .unwrap();
    assert_eq!(
        restored.state,
        OpState::RolledBack,
        "{:?}",
        restored.message
    );
    assert_eq!(std::fs::read(&target).unwrap(), original);
    assert_eq!(
        std::fs::read(&old_backup.path).unwrap(),
        installed,
        "previous recovery copy retained"
    );
    let new_backup = restored.steps[0].rollback_backup.as_ref().unwrap();
    assert_ne!(new_backup.path, old_backup.path);
    assert_eq!(std::fs::read(&new_backup.path).unwrap(), installed);
    assert_eq!(
        fx.engine().operation(&operation.id).unwrap().state,
        OpState::RolledBack
    );
    assert_eq!(std::fs::read(&fx.config).unwrap(), LOCALCONFIG.as_bytes());
}

#[test]
fn late_process_after_first_write_defers_compensating_rollback_until_closed() {
    for process in ["steam", "game"] {
        let fx = fixture();
        let old = b"value=old\n";
        let installed = b"value=installed\n";
        for path in ["Config/first.ini", "Config/second.ini"] {
            std::fs::write(fx.library.join(path), old).unwrap();
        }
        let archive = archive_with_entries(&[("first.ini", installed), ("second.ini", installed)]);
        let files = ["first.ini", "second.ini"]
            .into_iter()
            .map(|name| ArtifactFile {
                source_path: Some(name.into()),
                target_path: format!("Config/{name}"),
                sha256: Some(sha256_hex(installed)),
                replace: true,
                expected_original_sha256: Some(sha256_hex(old)),
            })
            .collect();
        let (candidate, transport) = release(&archive, files, ArtifactFormat::Zip);
        let (engine, game, id) = setup(&fx, candidate, transport);
        let preview = engine.prepare(&id, &game).unwrap();
        assert_eq!(preview.status, PlanStatus::Ready);
        let operation = engine
            .apply(
                &preview.plan_id,
                &preview.fingerprint,
                &game,
                &|operation| {
                    if operation.state == OpState::Applying
                        && operation.steps[0].status == StepStatus::Written
                    {
                        if process == "steam" {
                            fx.proc("123", "steam\n", "");
                        } else {
                            fx.proc("123", "SyntheticGame.exe\n", "SteamAppId=200260\0");
                        }
                    }
                },
            )
            .unwrap();
        assert_eq!(
            operation.state,
            OpState::RecoveryRequired,
            "{process}: {:?}",
            operation.message
        );
        assert_eq!(
            std::fs::read(fx.library.join("Config/first.ini")).unwrap(),
            installed
        );
        assert_eq!(
            std::fs::read(fx.library.join("Config/second.ini")).unwrap(),
            old
        );
        assert_eq!(operation.steps[0].status, StepStatus::Written);
        assert_eq!(operation.steps[1].status, StepStatus::BackedUp);
        assert!(operation.steps.iter().all(|step| std::fs::read(
            step.backup.as_ref().unwrap().path.clone()
        )
        .unwrap()
            == old));
        assert!(
            fx.proc_root.join("123").is_dir(),
            "FRX must not close processes"
        );
        let blocked = engine.prepare_rollback(&operation.id).unwrap();
        assert_eq!(blocked.status, PlanStatus::Conflict);
        fx.kill("123"); // remove only the synthetic process fixture
        let rollback = engine.prepare_rollback(&operation.id).unwrap();
        assert_eq!(rollback.status, PlanStatus::Ready);
        let restored = engine
            .rollback(&operation.id, &rollback.fingerprint, &noop)
            .unwrap();
        assert_eq!(
            restored.state,
            OpState::RolledBack,
            "{:?}",
            restored.message
        );
        for path in ["Config/first.ini", "Config/second.ini"] {
            assert_eq!(std::fs::read(fx.library.join(path)).unwrap(), old);
        }
        assert_eq!(std::fs::read(&fx.config).unwrap(), LOCALCONFIG.as_bytes());
    }
}

#[test]
fn artifact_requirements_are_checked_independently_of_outer_recipe_hardware() {
    use crate::system::{
        context::{tests::gpu, EnvironmentContext},
        GpuVendor,
    };
    for vendor in [GpuReq::Nvidia, GpuReq::Amd] {
        let fx = fixture();
        let (mut candidate, transport) = direct(b"value=fixture\n", None);
        candidate.requirements.hardware.gpu_vendor = Some(vendor);
        let (engine, mut game, id) = setup(&fx, candidate, transport);
        assert!(
            engine.resolve(&id, &game).unwrap().hardware.is_empty(),
            "outer synthetic finding carries no hardware requirements"
        );
        game.performance_conditions = Some(crate::performance::models::Conditions {
            environment: Some(EnvironmentContext {
                detected_gpus: vec![gpu(
                    GpuVendor::Nvidia,
                    "Synthetic RTX 4060",
                    "2882",
                    "fixture-driver",
                )],
                ..Default::default()
            }),
            ..Default::default()
        });
        let preview = engine.prepare(&id, &game).unwrap();
        assert_eq!(
            preview.status,
            if vendor == GpuReq::Nvidia {
                PlanStatus::Ready
            } else {
                PlanStatus::NotExecutable
            },
            "{:?}",
            preview.reasons
        );
        assert!(engine.operations(None).is_empty());
        assert!(!fx.library.join("Config/frx.ini").exists());
    }
    let fx = fixture();
    let (mut candidate, transport) = direct(b"value=fixture\n", None);
    candidate.requirements.gpu_vendor = Some(GpuReq::Nvidia);
    let (engine, game, id) = setup(&fx, candidate, transport);
    let preview = engine.prepare(&id, &game).unwrap();
    assert_eq!(preview.status, PlanStatus::NotExecutable);
    assert!(preview
        .reasons
        .iter()
        .any(|reason| reason.contains("Fabricante da GPU")));
}

#[test]
fn artifact_game_build_must_be_known_and_equal_before_any_download() {
    for actual in [
        None,
        Some("fixture-different-build"),
        Some("fixture-required-build"),
    ] {
        let fx = fixture();
        let (mut candidate, transport) = direct(b"value=fixture\n", None);
        candidate.requirements.game_version = Some("fixture-required-build".into());
        let (engine, mut game, id) = setup(&fx, candidate, transport);
        game.performance_conditions = Some(crate::performance::models::Conditions {
            game_build_id: actual.map(str::to_owned),
            ..Default::default()
        });
        let preview = engine.prepare(&id, &game).unwrap();
        assert_eq!(
            preview.status,
            if actual == Some("fixture-required-build") {
                PlanStatus::Ready
            } else {
                PlanStatus::NotExecutable
            },
            "{:?}",
            preview.reasons
        );
        assert!(engine.operations(None).is_empty());
        assert!(!fx.library.join("Config/frx.ini").exists());
        if preview.status == PlanStatus::Ready {
            game.performance_conditions.as_mut().unwrap().game_build_id =
                Some("fixture-update-after-preview".into());
            assert!(engine
                .apply(&preview.plan_id, &preview.fingerprint, &game, &noop)
                .is_err());
            assert!(engine.operations(None).is_empty());
            assert!(!fx.library.join("Config/frx.ini").exists());
        }
    }
}

#[test]
fn artifact_legacy_proton_requirement_uses_confirmed_tool_without_guessing() {
    for actual in [
        None,
        Some("fixture-other-Proton"),
        Some("fixture-required-Proton"),
    ] {
        let fx = fixture();
        let (mut candidate, transport) = direct(b"value=fixture\n", None);
        candidate.requirements.proton_versions = vec!["fixture-required-Proton".into()];
        let (engine, mut game, id) = setup(&fx, candidate, transport);
        game.tool_name = actual.map(str::to_owned);
        let preview = engine.prepare(&id, &game).unwrap();
        assert_eq!(
            preview.status,
            if actual == Some("fixture-required-Proton") {
                PlanStatus::Ready
            } else {
                PlanStatus::NotExecutable
            },
            "{:?}",
            preview.reasons
        );
        assert!(engine.operations(None).is_empty());
        assert!(!fx.library.join("Config/frx.ini").exists());
    }
}

#[test]
fn artifact_game_architecture_is_not_inferred_from_cpu_architecture() {
    use crate::system::context::EnvironmentContext;
    for architecture in [None, Some("any"), Some("x86"), Some("x86_64")] {
        let fx = fixture();
        let (mut candidate, transport) = direct(b"value=fixture\n", None);
        candidate.architecture = architecture.map(str::to_owned);
        let (engine, mut game, id) = setup(&fx, candidate, transport);
        game.performance_conditions = Some(crate::performance::models::Conditions {
            environment: Some(EnvironmentContext {
                cpu_arch: Some("x86_64".into()),
                ..Default::default()
            }),
            ..Default::default()
        });
        let preview = engine.prepare(&id, &game).unwrap();
        assert_eq!(
            preview.status,
            if matches!(architecture, None | Some("any")) {
                PlanStatus::Ready
            } else {
                PlanStatus::NotExecutable
            },
            "{:?}",
            preview.reasons
        );
        if matches!(architecture, Some("x86" | "x86_64")) {
            assert!(preview
                .reasons
                .iter()
                .any(|reason| reason.contains("arquitetura da CPU")));
        }
        assert!(engine.operations(None).is_empty());
        assert!(!fx.library.join("Config/frx.ini").exists());
    }
}

#[test]
fn artifact_platform_requires_confirmed_matching_game_execution_mode() {
    for (platform, proton, expected) in [
        ("windows", Some(true), true),
        ("windows", Some(false), false),
        ("windows", None, false),
        ("linux", Some(false), true),
        ("linux", Some(true), false),
        ("linux", None, false),
        ("any", None, true),
    ] {
        let fx = fixture();
        let (mut candidate, transport) = direct(b"value=fixture\n", None);
        candidate.platform = Some(platform.into());
        let (engine, mut game, id) = setup(&fx, candidate, transport);
        game.uses_proton = proton;
        let preview = engine.prepare(&id, &game).unwrap();
        assert_eq!(
            preview.status,
            if expected {
                PlanStatus::Ready
            } else {
                PlanStatus::NotExecutable
            },
            "{platform} {proton:?}: {:?}",
            preview.reasons
        );
        assert!(engine.operations(None).is_empty());
        assert!(!fx.library.join("Config/frx.ini").exists());
    }
}
