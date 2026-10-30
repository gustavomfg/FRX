// Controlled, synthetic publisher metadata and payloads. No real game, release,
// benchmark, network write or external executable is used by these tests.
use crate::fix_engine::{artifacts, confined};
use crate::fixes::models::*;
use crate::fixes::providers::{
    artifacts::{candidates_from_release, Document},
    GameQuery,
};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

const APP: &str = "200260";
const REPO: &str = "Open-Wine-Components/umu-protonfixes";
const META: &str =
    "https://api.github.com/repos/Open-Wine-Components/umu-protonfixes/releases/424242";

struct ControlledTransport {
    responses: HashMap<String, Vec<u8>>,
    fail: bool,
    mutate: Option<PathBuf>,
}
impl artifacts::Transport for ControlledTransport {
    fn get(
        &self,
        url: &str,
        limit: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<Vec<u8>, String> {
        artifacts::cancelled(cancel)?;
        if url != META {
            if let Some(target) = &self.mutate {
                std::fs::write(target, b"changed externally=1\n").unwrap();
            }
            if self.fail {
                return Err("Download interrompido pela fixture controlada.".into());
            }
        }
        let bytes = self
            .responses
            .get(url)
            .ok_or("URL ausente na fixture")?
            .clone();
        if bytes.len() > limit {
            return Err("Limite de download".into());
        }
        progress(bytes.len() as u64, Some(bytes.len() as u64));
        artifacts::cancelled(cancel)?;
        Ok(bytes)
    }
}

fn release(
    bytes: &[u8],
    file_specs: Vec<ArtifactFile>,
    format: ArtifactFormat,
) -> (ArtifactCandidate, ControlledTransport) {
    let name = if format == ArtifactFormat::Zip {
        "controlled-fixture.zip"
    } else {
        "controlled-fixture.ini"
    };
    let manifest = serde_json::json!({"schema":1,"appId":APP,"asset":name,"platform":"any","architecture":"any","files":file_specs});
    let url = format!("https://github.com/{REPO}/releases/download/v-fixture-1/{name}");
    let body=serde_json::json!({"id":424242,"tag_name":"v-fixture-1","html_url":format!("https://github.com/{REPO}/releases/tag/v-fixture-1"),"draft":false,"prerelease":false,"author":{"login":"synthetic-fixture-publisher"},"published_at":"2026-10-27T00:00:00Z","body":format!("```frx-file-fix\n{manifest}\n```"),"assets":[{"id":7,"name":name,"browser_download_url":url,"size":bytes.len(),"digest":format!("sha256:{}",sha256_hex(bytes)),"content_type":if format==ArtifactFormat::Zip{"application/zip"}else{"text/plain"},"state":"uploaded"}]}).to_string();
    let source = FindingSource {
        provider: ProviderId::Github,
        label: "Synthetic test fixture, not an actual release".into(),
        url: format!("https://github.com/{REPO}/issues/999999"),
        reference_id: None,
        snippet: None,
        published_at: None,
    };
    let q = GameQuery {
        app_id: APP.into(),
        ..Default::default()
    };
    let doc = Document {
        source,
        scope: DiscoveryScope::verified(APP, "Synthetic fixture: explicitly scoped publisher"),
        text: String::new(),
    };
    let mut candidates = candidates_from_release(&q, &doc, REPO, META, &body);
    assert_eq!(candidates.len(), 1);
    let candidate = candidates.remove(0);
    let transport = ControlledTransport {
        responses: HashMap::from([(META.into(), body.into_bytes()), (url, bytes.to_vec())]),
        fail: false,
        mutate: None,
    };
    (candidate, transport)
}

fn direct(bytes: &[u8], previous: Option<&[u8]>) -> (ArtifactCandidate, ControlledTransport) {
    release(
        bytes,
        vec![ArtifactFile {
            source_path: None,
            target_path: "Config/frx.ini".into(),
            sha256: Some(sha256_hex(bytes)),
            replace: previous.is_some(),
            expected_original_sha256: previous.map(sha256_hex),
        }],
        ArtifactFormat::File,
    )
}

fn game(fx: &Fx, candidate: ArtifactCandidate) -> GameCtx {
    let mut f = crate::fix_engine::dynamic::tests::finding(
        "synthetic-file",
        ActionKind::ConfigFile,
        "fixture",
        1,
        0,
    );
    f.actions.clear();
    f.artifacts = vec![candidate];
    f.confidence = Confidence::Curated;
    GameCtx {
        findings: vec![f],
        ..fx.game()
    }
}
fn setup(
    fx: &Fx,
    candidate: ArtifactCandidate,
    transport: ControlledTransport,
) -> (FixEngine, GameCtx, String) {
    let mut engine = fx.engine();
    engine.artifact_transport = Arc::new(transport);
    let game = game(fx, candidate);
    let views = engine.analyze(&game).unwrap();
    let id = views[0]
        .assessment
        .recipe_id
        .clone()
        .unwrap_or_else(|| panic!("{:?}", views[0].assessment));
    (engine, game, id)
}

#[test]
fn download_install_history_restart_backup_rollback_and_idempotency() {
    for original in [None, Some(&b"[fixture]\nvalue=old\n"[..])] {
        let fx = fixture();
        let target = fx.library.join("Config/frx.ini");
        if let Some(old) = original {
            std::fs::write(&target, old).unwrap();
        }
        let payload = b"[fixture]\nvalue=new\n";
        let (candidate, transport) = direct(payload, original);
        let (e, g, id) = setup(&fx, candidate, transport);
        let plan = e.prepare(&id, &g).unwrap();
        assert_eq!(plan.status, PlanStatus::Ready, "{:?}", plan.reasons);
        assert_eq!(plan.artifacts.len(), 1);
        assert_eq!(std::fs::read(&fx.config).unwrap(), LOCALCONFIG.as_bytes());
        let op = e
            .apply(&plan.plan_id, &plan.fingerprint, &g, &noop)
            .unwrap();
        assert_eq!(op.state, OpState::Applied, "{:?}", op.message);
        assert_eq!(std::fs::read(&target).unwrap(), payload);
        assert_eq!(
            op.steps[0].backup.as_ref().unwrap().existed,
            original.is_some()
        );
        assert_eq!(
            e.prepare(&id, &g).unwrap().status,
            PlanStatus::AlreadyApplied
        );
        assert!(!fx.data.join("staging").join(&op.id).exists());
        let restarted = fx.engine();
        assert_eq!(restarted.operation(&op.id).unwrap().artifacts, op.artifacts);
        let rollback = restarted.prepare_rollback(&op.id).unwrap();
        assert_eq!(
            restarted
                .rollback(&op.id, &rollback.fingerprint, &noop)
                .unwrap()
                .state,
            OpState::RolledBack
        );
        assert_eq!(std::fs::read(&target).ok().as_deref(), original);
        assert_eq!(std::fs::read(&fx.config).unwrap(), LOCALCONFIG.as_bytes());
    }
}

#[test]
fn checksum_interruption_cancellation_and_external_edits_never_install_partial_data() {
    for scenario in ["checksum", "interrupted", "cancelled", "changed"] {
        let fx = fixture();
        let target = fx.library.join("Config/frx.ini");
        let original = b"value=old\n";
        std::fs::write(&target, original).unwrap();
        let (candidate, mut transport) = direct(b"value=new\n", Some(original));
        if scenario == "checksum" {
            transport
                .responses
                .insert(candidate.asset_url.clone(), b"value=bad\n".to_vec());
        }
        if scenario == "interrupted" {
            transport.fail = true;
        }
        if scenario == "changed" {
            transport.mutate = Some(target.clone());
        }
        let (e, g, id) = setup(&fx, candidate, transport);
        let plan = e.prepare(&id, &g).unwrap();
        let op = e
            .apply(&plan.plan_id, &plan.fingerprint, &g, &|op| {
                if scenario == "cancelled" && op.state == OpState::Downloading {
                    assert!(e.cancel_artifact_operation(&op.id));
                }
            })
            .unwrap();
        assert_eq!(
            op.state,
            if scenario == "cancelled" {
                OpState::Cancelled
            } else if scenario == "changed" {
                OpState::Conflict
            } else {
                OpState::Failed
            },
            "{scenario}: {:?}",
            op.message
        );
        assert_eq!(
            std::fs::read(&target).unwrap(),
            if scenario == "changed" {
                &b"changed externally=1\n"[..]
            } else {
                original
            }
        );
        assert!(op
            .steps
            .iter()
            .all(|s| !matches!(s.status, StepStatus::Written | StepStatus::Verified)));
        assert!(!fx.data.join("staging").join(&op.id).exists());
    }
}

#[test]
fn changed_publisher_destinations_or_unsafe_binary_cannot_be_installed() {
    let fx = fixture();
    let (candidate, mut transport) = direct(b"value=new\n", None);
    let metadata = String::from_utf8(transport.responses[META].clone())
        .unwrap()
        .replace("Config/frx.ini", "Config/other.ini");
    transport
        .responses
        .insert(META.into(), metadata.into_bytes());
    let (e, g, id) = setup(&fx, candidate, transport);
    let plan = e.prepare(&id, &g).unwrap();
    let op = e
        .apply(&plan.plan_id, &plan.fingerprint, &g, &noop)
        .unwrap();
    assert_eq!(op.state, OpState::Failed);
    assert!(
        !fx.library.join("Config/frx.ini").exists()
            && !fx.library.join("Config/other.ini").exists()
    );
    for bytes in [
        &b"MZ untrusted executable"[..],
        &b"#!/bin/sh\nanything"[..],
        &b"command\t= run-stuff\n"[..],
    ] {
        assert!(artifacts::validate_text("Config/frx.ini", bytes).is_err());
    }
    let (mut dll, _) = direct(b"value=new\n", None);
    dll.files[0].target_path = "Binaries/untrusted.dll".into();
    assert!(artifacts::validate(&dll, APP).is_err());
    let (mut foreign, _) = direct(b"value=new\n", None);
    foreign.app_id = "620".into();
    assert!(artifacts::validate(&foreign, APP).is_err());
}

fn zip_bytes(name: &str, bytes: &[u8], symlink: bool) -> Vec<u8> {
    use std::io::Write;
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    if symlink {
        zip.add_symlink(name, "../../outside", options).unwrap();
    } else {
        zip.start_file(name, options).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

#[test]
fn selected_zip_files_are_checked_without_extracting_paths_or_links() {
    let content = b"[fixture]\nvalue=7\n";
    for (name, bytes, symlink, works) in [
        ("nested/frx.ini", content.to_vec(), false, true),
        ("../outside.ini", content.to_vec(), false, false),
        ("/outside.ini", content.to_vec(), false, false),
        ("nested/link.ini", content.to_vec(), true, false),
        ("bomb.ini", vec![b'a'; 300_000], false, false),
    ] {
        let zip = zip_bytes(name, &bytes, symlink);
        let (a, _) = release(
            &zip,
            vec![ArtifactFile {
                source_path: Some(name.into()),
                target_path: "Config/frx.ini".into(),
                sha256: Some(sha256_hex(&bytes)),
                replace: false,
                expected_original_sha256: None,
            }],
            ArtifactFormat::Zip,
        );
        let result = artifacts::contents(&a, &zip, &AtomicBool::new(false));
        assert_eq!(result.is_ok(), works, "{name}: {result:?}");
    }
    let (mut a, _) = direct(content, None);
    a.format = ArtifactFormat::Zip;
    assert!(artifacts::contents(&a, content, &AtomicBool::new(false)).is_err());
}

#[test]
fn destination_symlinks_swapped_parents_and_other_games_are_refused() {
    let fx = fixture();
    let outside = fx._tmp.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("frx.ini"), b"keep=1\n").unwrap();
    let (candidate, transport) = direct(b"value=new\n", None);
    let (e, g, id) = setup(&fx, candidate, transport);
    let plan = e.prepare(&id, &g).unwrap();
    std::fs::rename(fx.library.join("Config"), fx.library.join("OriginalConfig")).unwrap();
    std::os::unix::fs::symlink(&outside, fx.library.join("Config")).unwrap();
    assert!(e
        .apply(&plan.plan_id, &plan.fingerprint, &g, &noop)
        .is_err());
    assert_eq!(std::fs::read(outside.join("frx.ini")).unwrap(), b"keep=1\n");
    for bad in ["../OtherGame/frx.ini", "/tmp/frx.ini", "C:\\outside.ini"] {
        assert!(confined::Target::open(&fx.library, bad, None).is_err());
    }
}

#[test]
fn external_file_change_blocks_rollback_and_writing_intent_survives_restart() {
    let fx = fixture();
    let (a, t) = direct(b"value=new\n", None);
    let (e, g, id) = setup(&fx, a, t);
    let p = e.prepare(&id, &g).unwrap();
    let op = e.apply(&p.plan_id, &p.fingerprint, &g, &noop).unwrap();
    let target = fx.library.join("Config/frx.ini");
    std::fs::write(&target, b"user=edited\n").unwrap();
    assert_eq!(
        e.prepare_rollback(&op.id).unwrap().status,
        PlanStatus::Conflict
    );
    assert_eq!(std::fs::read(&target).unwrap(), b"user=edited\n");
    std::fs::write(&target, b"value=new\n").unwrap();
    let mut interrupted = op.clone();
    interrupted.state = OpState::Applying;
    interrupted.steps[0].status = StepStatus::Writing;
    e.journal.save(&interrupted).unwrap();
    let restarted = fx.engine();
    assert_eq!(
        restarted.operation(&op.id).unwrap().state,
        OpState::RecoveryRequired
    );
    let rb = restarted.prepare_rollback(&op.id).unwrap();
    assert_eq!(
        restarted
            .rollback(&op.id, &rb.fingerprint, &noop)
            .unwrap()
            .state,
        OpState::RolledBack
    );
    assert!(!target.exists());
}

#[test]
fn urls_privileges_and_missing_original_preconditions_are_refused() {
    for url in [
        "http://github.com/a/b",
        "https://user:password@github.com/a",
        "https://github.com:8443/a",
        "file:///etc/passwd",
        "https://evil.example/a",
        "https://github.com.evil.example/a",
    ] {
        assert!(!artifacts::allowed_url(url));
    }
    let fx = fixture();
    std::fs::write(fx.library.join("Config/frx.ini"), b"user=keep\n").unwrap();
    let (a, t) = direct(b"value=new\n", None);
    let (e, g, id) = setup(&fx, a, t);
    assert_eq!(
        e.prepare(&id, &g).unwrap().status,
        PlanStatus::NotExecutable
    );
}

#[test]
fn concurrent_replacement_preserves_displaced_external_content() {
    let fx = fixture();
    let target = fx.library.join("Config/race.ini");
    std::fs::write(&target, b"value=old\n").unwrap();
    let opened = confined::Target::open(&fx.library, "Config/race.ini", None).unwrap();
    let expected = sha256_hex(b"value=old\n");
    let result = opened.write_with_race(b"value=new\n", Some(&expected), || {
        std::fs::write(&target, b"external=preserve\n").unwrap()
    });
    assert!(result.is_err());
    assert_eq!(std::fs::read(&target).unwrap(), b"external=preserve\n");
}

#[test]
fn mixed_smart_pack_installs_files_and_launch_options_as_one_operation() {
    use crate::smart_packs::{benchmark, experience};
    let fx = fixture();
    let (candidate, transport) = direct(b"value=new\n", None);
    let (e, mut g, _) = setup(&fx, candidate, transport);
    g.findings.push(crate::fix_engine::dynamic::tests::finding(
        "synthetic-env",
        ActionKind::Env,
        "PROTON_HEAP_DELAY_FREE=1",
        2,
        0,
    ));
    let combined = experience::build(&e, &g, fx.line().as_deref(), &[]).unwrap();
    assert!(
        combined.actions.len() >= 2 && !combined.file_fixes.is_empty(),
        "synthetic mixed candidate: {:?}",
        combined.left_out
    );
    let id = combined.apply_recipe_id.as_ref().unwrap();
    let plan = e.prepare(id, &g).unwrap();
    assert_eq!(plan.status, PlanStatus::Ready, "{:?}", plan.reasons);
    let op = e
        .apply(&plan.plan_id, &plan.fingerprint, &g, &noop)
        .unwrap();
    assert_eq!(op.state, OpState::Applied, "{:?}", op.message);
    assert_eq!(op.steps.len(), 2);
    assert!(fx.line().unwrap().contains("PROTON_HEAP_DELAY_FREE=1"));
    let capture = benchmark::snapshot(&e, APP);
    assert!(capture.verified);
    assert_eq!(capture.file_fixes, combined.file_fixes);
    assert_eq!(Some(capture.pack_id), combined.pack_id);
    let restarted = fx.engine();
    assert_eq!(
        benchmark::snapshot(&restarted, APP).file_fixes,
        capture.file_fixes
    );
    let rb = restarted.prepare_rollback(&op.id).unwrap();
    assert_eq!(
        restarted
            .rollback(&op.id, &rb.fingerprint, &noop)
            .unwrap()
            .state,
        OpState::RolledBack
    );
    assert!(!fx.library.join("Config/frx.ini").exists());
    assert_eq!(fx.line().as_deref(), Some(ORIGINAL));
}

#[test]
fn partial_existing_artifact_records_all_members_and_keeps_existing_content_on_rollback() {
    use crate::smart_packs::benchmark;
    use std::io::Write;
    let fx = fixture();
    let body = b"value=fixture\n";
    std::fs::write(fx.library.join("Config/first.ini"), body).unwrap();
    let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for name in ["first.ini", "second.ini"] {
        archive
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(body).unwrap();
    }
    let zip = archive.finish().unwrap().into_inner();
    let files = ["first.ini", "second.ini"]
        .iter()
        .map(|n| ArtifactFile {
            source_path: Some((*n).into()),
            target_path: format!("Config/{n}"),
            sha256: Some(sha256_hex(body)),
            replace: false,
            expected_original_sha256: None,
        })
        .collect();
    let (candidate, transport) = release(&zip, files, ArtifactFormat::Zip);
    let (e, g, id) = setup(&fx, candidate, transport);
    let plan = e.prepare(&id, &g).unwrap();
    assert_eq!(plan.status, PlanStatus::Ready);
    let op = e
        .apply(&plan.plan_id, &plan.fingerprint, &g, &noop)
        .unwrap();
    assert_eq!(op.state, OpState::Applied, "{:?}", op.message);
    assert_eq!(benchmark::snapshot(&e, APP).file_fixes.len(), 2);
    assert_eq!(
        e.prepare(&id, &g).unwrap().status,
        PlanStatus::AlreadyApplied
    );
    let rb = e.prepare_rollback(&op.id).unwrap();
    assert_eq!(
        e.rollback(&op.id, &rb.fingerprint, &noop).unwrap().state,
        OpState::RolledBack
    );
    assert_eq!(
        std::fs::read(fx.library.join("Config/first.ini")).unwrap(),
        body
    );
    assert!(!fx.library.join("Config/second.ini").exists());
}

#[test]
fn root_symlinks_and_symlinked_ancestors_cannot_escape_the_game() {
    let fx = fixture();
    let other = fx._tmp.path().join("not-a-game");
    std::fs::create_dir_all(other.join("Config")).unwrap();
    std::fs::write(other.join("Config/frx.ini"), b"external=keep\n").unwrap();
    std::fs::rename(&fx.library, fx.library.with_file_name("saved-game")).unwrap();
    std::os::unix::fs::symlink(&other, &fx.library).unwrap();
    assert!(confined::Target::open(&fx.library, "Config/frx.ini", None).is_err());
    std::fs::remove_file(&fx.library).unwrap();
    std::fs::rename(fx.library.with_file_name("saved-game"), &fx.library).unwrap();
    let parent = fx.library.parent().unwrap();
    let saved = parent.with_file_name("saved-common");
    std::fs::rename(parent, &saved).unwrap();
    std::os::unix::fs::symlink(&saved, parent).unwrap();
    assert!(confined::Target::open(&fx.library, "Config/frx.ini", None).is_err());
    assert!(confined::Target::open(&fx.library.join("../Batman"), "Config/frx.ini", None).is_err());
    assert_eq!(
        std::fs::read(other.join("Config/frx.ini")).unwrap(),
        b"external=keep\n"
    );
}

#[test]
fn another_change_during_exchange_recovery_is_kept_for_manual_recovery() {
    let fx = fixture();
    let path = fx.library.join("Config/race.ini");
    std::fs::write(&path, b"value=original\n").unwrap();
    let opened = confined::Target::open(&fx.library, "Config/race.ini", None).unwrap();
    let result = opened.write_with_two_races(
        b"value=ours\n",
        Some(&sha256_hex(b"value=original\n")),
        || std::fs::write(&path, b"external=first\n").unwrap(),
        || std::fs::write(&path, b"external=second\n").unwrap(),
    );
    assert!(result.unwrap_err().starts_with("RECUPERAÇÃO:"));
    assert_eq!(std::fs::read(&path).unwrap(), b"external=first\n");
    let retained: Vec<_> = std::fs::read_dir(path.parent().unwrap())
        .unwrap()
        .filter_map(|entry| {
            let p = entry.unwrap().path();
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".frx-")
                .then_some(p)
        })
        .collect();
    assert_eq!(retained.len(), 1);
    assert_eq!(std::fs::read(&retained[0]).unwrap(), b"external=second\n");
}

#[test]
fn manual_recovery_is_persisted_and_presence_cannot_falsely_complete_it() {
    let fx = fixture();
    let (candidate, transport) = direct(b"value=new\n", None);
    let (e, g, id) = setup(&fx, candidate, transport);
    let plan = e.prepare(&id, &g).unwrap();
    let mut op = e
        .apply(&plan.plan_id, &plan.fingerprint, &g, &noop)
        .unwrap();
    op.manual_recovery_required = true;
    let retained="RECUPERAÇÃO: conteúdo externo preservado em .frx-fixture-retained.tmp no diretório do destino.";
    e.journal
        .transition(&mut op, OpState::RecoveryRequired, retained)
        .unwrap();
    let restarted = fx.engine();
    let verified = restarted.verify(&op.id, &noop).unwrap();
    assert_eq!(verified.state, OpState::RecoveryRequired);
    assert_eq!(verified.message.as_deref(), Some(retained));
    assert!(restarted
        .prepare_rollback(&op.id)
        .unwrap_err()
        .contains(".frx-fixture-retained.tmp"));
    assert_eq!(
        std::fs::read(fx.library.join("Config/frx.ini")).unwrap(),
        b"value=new\n"
    );
    let mut old = serde_json::to_value(&op).unwrap();
    old.as_object_mut()
        .unwrap()
        .remove("manualRecoveryRequired");
    assert!(
        !serde_json::from_value::<Operation>(old)
            .unwrap()
            .manual_recovery_required
    );
}

#[test]
fn downloaded_configuration_dxvk_and_environment_execute_and_rollback_as_one_solution() {
    let fx = fixture();
    let payload = b"[fixture]\nvalue=controlled\n";
    let (candidate, transport) = direct(payload, None);
    let (engine, mut g, _) = setup(&fx, candidate, transport);
    g.findings[0].actions = vec![
        FixAction {
            kind: ActionKind::ConfigFile,
            label: "Synthetic DXVK config step".into(),
            value: "dxvk.conf: dxgi.maxFrameLatency=1".into(),
            conditional: false,
        },
        FixAction {
            kind: ActionKind::Env,
            label: "Synthetic Proton step".into(),
            value: "PROTON_NO_ESYNC=1".into(),
            conditional: false,
        },
    ];
    let launch = fx.line();
    let a = engine.analyze(&g).unwrap().remove(0).assessment;
    let p = engine.prepare(a.recipe_id.as_ref().unwrap(), &g).unwrap();
    assert_eq!(p.status, PlanStatus::Ready, "{p:#?}");
    assert_eq!(p.changes.len(), 3);
    let op = engine
        .apply(&p.plan_id, &p.fingerprint, &g, &|_| {})
        .unwrap();
    assert_eq!(op.state, OpState::Applied);
    assert_eq!(
        std::fs::read(fx.library.join("Config/frx.ini")).unwrap(),
        payload
    );
    assert!(std::fs::read_to_string(fx.library.join("dxvk.conf"))
        .unwrap()
        .contains("dxgi.maxFrameLatency = 1"));
    assert!(fx.line().unwrap().contains("PROTON_NO_ESYNC=1"));
    let restarted = fx.engine();
    let rb = restarted.prepare_rollback(&op.id).unwrap();
    restarted
        .rollback(&op.id, &rb.fingerprint, &|_| {})
        .unwrap();
    assert!(!fx.library.join("Config/frx.ini").exists());
    assert!(!fx.library.join("dxvk.conf").exists());
    assert_eq!(fx.line(), launch);
}

#[test]
fn artifact_plus_unsupported_dependency_is_not_partially_generated() {
    let fx = fixture();
    let (candidate, transport) = direct(b"value=1\n", None);
    let (engine, mut g, _) = setup(&fx, candidate, transport);
    g.findings[0].actions = vec![FixAction {
        kind: ActionKind::Winetricks,
        label: "Dependency needs installer".into(),
        value: "physx".into(),
        conditional: false,
    }];
    assert!(engine.analyze(&g).unwrap()[0]
        .assessment
        .recipe_id
        .is_none());
    assert!(!fx.library.join("Config/frx.ini").exists());
    assert!(engine.operations(None).is_empty());
}
