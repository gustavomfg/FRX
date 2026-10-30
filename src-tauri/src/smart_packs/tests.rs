//! Testes do compositor, do grafo de conflitos, do ranking e da
//! persistência. **Todas as sessões aqui são sintéticas** (`synthetic-*`),
//! com valores escolhidos para dar resultados matemáticos conhecidos.

use std::collections::{BTreeMap, HashMap};

use super::benchmark::{baseline_id, pack_id};
use super::composition::{admissible, make};
use super::conflicts::{edge, graph, launch_notes, validate_combination};
use super::evidence::{group_in_context, stats};
use super::models::*;
use crate::fix_engine::dynamic::EvidenceLevel;
use crate::fix_engine::models::{Purpose, RecipeSource, Risk, Trust};
use crate::fixes::models::GpuReq;
use crate::performance::comparison::tests::session;
use crate::performance::models::{QualityLevel, SessionMeta};

const APP: &str = "200260";

pub fn env(pairs: &[(&str, &str)]) -> Vec<EnvPair> {
    let mut v: Vec<EnvPair> = pairs
        .iter()
        .map(|(k, v)| EnvPair {
            key: (*k).into(),
            value: (*v).into(),
        })
        .collect();
    v.sort();
    v
}

pub fn unit(id: &str, pairs: &[(&str, &str)], ev: EvidenceLevel) -> Unit {
    let e = env(pairs);
    Unit {
        preview_files: None,
        hardware: Default::default(),
        file_actions: vec![],
        app_id: Some(APP.into()),
        performance_evidence: Default::default(),
        recipe_id: id.into(),
        version: "1.0.0".into(),
        fingerprint: format!("{:0>64}", id.len()),
        trust: Trust::Generated,
        title: id.into(),
        perf: e
            .iter()
            .map(|p| super::conflicts::perf_hint(&p.key))
            .find(|h| *h != PerfHint::Unknown)
            .unwrap_or(PerfHint::Unknown),
        env: e,
        purpose: Purpose::Compatibility,
        risk: Risk::Low,
        evidence: ev,
        evidence_note: format!("{ev:?}"),
        finding_ids: vec![format!("f-{id}")],
        sources: vec![RecipeSource {
            label: "s".into(),
            url: "https://github.com/x".into(),
        }],
        proton_versions: vec![],
        discovered_at: None,
        gpu: None,
        already_included: false,
        effects: vec![],
        side_effects: vec![],
    }
}

// ── Composição e conflitos ────────────────────────────────────────────────

#[test]
fn partially_unsafe_combinations_are_rejected_entirely() {
    let units = [
        unit("a", &[("PROTON_NO_ESYNC", "1")], EvidenceLevel::Documented),
        unit(
            "b",
            &[("PROTON_ENABLE_NVAPI", "1")],
            EvidenceLevel::Documented,
        ),
        unit(
            "c",
            &[("PROTON_DISABLE_NVAPI", "1")],
            EvidenceLevel::Documented,
        ),
    ];
    let bad = validate_combination(&units, 4).unwrap_err();
    assert_eq!(bad.len(), 1);
    assert!(validate_combination(&units[..2], 4).is_ok());
    assert!(
        validate_combination(&units[..1], 4).is_err(),
        "pacote composto tem 2+ receitas"
    );
    assert_eq!(graph(&units).len(), 3);
}

#[test]
fn launch_line_interactions_are_reported() {
    let caps = env(&[("DXVK_FRAME_RATE", "60")]);
    let n = launch_notes(&caps, Some("gamescope -W 1920 -r 60 -- mangohud %command%"));
    assert!(n.iter().any(|x| x.contains("gamescope já limita")));
    assert!(n.iter().any(|x| x.contains("MangoHud")));
    let n = launch_notes(
        &env(&[("PROTON_NO_ESYNC", "1")]),
        Some("PROTON_NO_ESYNC=0 %command%"),
    );
    assert!(n[0].contains("já definem PROTON_NO_ESYNC=0"));
    assert!(launch_notes(&caps, None).is_empty());
}

// ── Ranking (sessões sintéticas) ──────────────────────────────────────────

/// Sessão **sintética** com métricas exatas.
pub fn synthetic(id: &str, e: &[EnvPair], avg: f64, low1: f64, p99: f64) -> SessionMeta {
    let mut m = session(&format!("synthetic-{id}"), &[10.0; 200]);
    m.stats.fps.avg = Some(avg);
    m.stats.fps.low1 = Some(low1);
    m.stats.fps.low01 = None;
    m.stats.frametime.p99 = Some(p99);
    m.stats.stability.spikes_per_min = None;
    m.stats.stability.per_second_fps_stdev = Some(1.0);
    m.stats.stability.seconds = 60;
    m.stats.duration_s = 60.0;
    m.quality.level = QualityLevel::Good;
    m.created_at = format!("2026-10-26T10:00:{:0>2}Z", id.len());
    m.smart_pack = Some(PackSnapshot {
        file_fixes: vec![],
        app_id: Some(APP.into()),
        fingerprint: None,
        pack_id: pack_id(APP, e),
        env: e.to_vec(),
        recipes: vec![],
        operation_ids: vec![],
        verified: true,
        source: SnapshotSource::Assisted,
        recorded_at: m.created_at.clone(),
    });
    m
}

fn d3d11() -> Vec<EnvPair> {
    env(&[("PROTON_NO_D3D11", "1")])
}
#[test]
fn statistics_are_exact() {
    let s = stats(&[100.0, 110.0, 90.0, 105.0]).unwrap();
    assert_eq!((s.n, s.median, s.min, s.max), (4, 102.5, 90.0, 110.0));
    assert!((s.spread_pct.unwrap() - 20.0 / 102.5 * 100.0).abs() < 1e-12);
    assert!(stats(&[42.0]).unwrap().spread_pct.is_none());
    assert!(stats(&[]).is_none());
}

// ── Persistência ──────────────────────────────────────────────────────────

#[test]
fn old_session_json_without_smart_pack_still_loads() {
    let mut m = session("old", &[10.0; 200]);
    m.smart_pack = None;
    let json = serde_json::to_string(&m).unwrap();
    assert!(!json.contains("smartPack"));
    let back: SessionMeta = serde_json::from_str(&json).unwrap();
    assert!(back.smart_pack.is_none());
    let _ = BTreeMap::<String, ()>::new();
}

// ── Avaliação automática: cenário de aceitação do Batman ─────────────────
//
// FIXTURE da observação informal do usuário (não é benchmark registrado):
// baseline com FPS típico entre 90 e 100; `PROTON_NO_D3D11=1` com FPS
// frequentemente abaixo de 60.

mod acceptance {
    use super::*;
    use crate::smart_packs::evaluation::{evaluate, Ctx};

    /// Sessão sintética com variação por segundo realista (desvio de 25 FPS).
    fn obs(id: &str, e: &[EnvPair], avg: f64, low1: f64) -> SessionMeta {
        let mut m = synthetic(id, e, avg, low1, 1000.0 / low1);
        m.stats.stability.per_second_fps_stdev = Some(25.0);
        m
    }

    fn eval(sessions: &[SessionMeta], id: &str) -> PackEvaluation {
        let g = group_in_context(
            APP,
            sessions,
            |m| m.smart_pack.clone(),
            &baseline_id(APP),
            None,
        );
        let metas: HashMap<&str, &SessionMeta> =
            sessions.iter().map(|m| (m.id.as_str(), m)).collect();
        let name_of = |_: &str| None;
        let ctx = Ctx {
            app_id: APP,
            baseline_id: &baseline_id(APP),
            grouped: &g,
            metas: &metas,
            name_of: &name_of,
        };
        let m = sessions.iter().find(|m| m.id == id).unwrap();
        evaluate(&ctx, m, m.smart_pack.as_ref())
    }

    #[test]
    fn one_capture_each_gives_a_preliminary_regression_with_absolute_and_relative_values() {
        let s = vec![obs("b1", &[], 95.0, 70.0), obs("d1", &d3d11(), 55.0, 38.0)];
        let e = eval(&s, "synthetic-d1");
        assert_eq!(e.verdict, EvalVerdict::Regression);
        assert!(e.preliminary, "uma captura não comprova regressão");
        assert_eq!(
            e.headline,
            "Indício de regressão — recomendado repetir o teste"
        );
        assert_eq!(
            e.pack_name, "PROTON_NO_D3D11=1",
            "sem pacote registrado, o nome são as variáveis"
        );
        let avg = e.deltas.iter().find(|d| d.metric == "fpsAvg").unwrap();
        assert_eq!(
            (avg.before, avg.after, avg.unit.as_str()),
            (95.0, 55.0, "FPS")
        );
        assert!((avg.delta_pct - (55.0 - 95.0) / 95.0 * 100.0).abs() < 1e-9);
        // Limiar preliminar = variação natural dentro das sessões (2·25/√60 ≈ 6,5% de 95; combinada ≈ 13,4%).
        assert!(
            avg.threshold_pct > 3.0 && avg.threshold_pct < 20.0,
            "{}",
            avg.threshold_pct
        );
        assert!(e.summary.contains("FPS médio 95,0 → 55,0 FPS (−42,1%"));
        assert!(e
            .next_step
            .as_deref()
            .unwrap()
            .contains("faltam 1 sessão(ões) do baseline e 1 do pacote"));
        assert!(e.limitations.iter().any(|l| l.contains("mesmo trecho")));
        assert!(e.ranking_effect.starts_with("Preliminar"));
    }

    #[test]
    fn repeated_captures_prove_the_regression_and_move_it_to_not_recommended() {
        let s = vec![
            obs("b1", &[], 95.0, 70.0),
            obs("b2", &[], 92.0, 68.0),
            obs("d1", &d3d11(), 55.0, 38.0),
            obs("d2", &d3d11(), 58.0, 40.0),
        ];
        let e = eval(&s, "synthetic-d2");
        assert_eq!((e.verdict, e.preliminary), (EvalVerdict::Regression, false));
        assert_eq!(e.headline, "Regressão de desempenho medida");
        assert!(e.ranking_effect.contains("não recomendados"));
    }

    #[test]
    fn other_outcomes_are_named_honestly() {
        // Referência.
        let s = vec![obs("b1", &[], 95.0, 70.0)];
        let e = eval(&s, "synthetic-b1");
        assert_eq!(e.verdict, EvalVerdict::Reference);
        assert!(e.next_step.is_some());
        // Sem baseline.
        let s = vec![obs("d1", &d3d11(), 55.0, 38.0)];
        assert_eq!(eval(&s, "synthetic-d1").verdict, EvalVerdict::NoBaseline);
        // Condições diferentes: não dá para determinar.
        let mut other = obs("d1", &d3d11(), 55.0, 38.0);
        other.conditions.resolution = Some("2560x1440".into());
        let s = vec![obs("b1", &[], 95.0, 70.0), other];
        let e = eval(&s, "synthetic-d1");
        assert_eq!(e.verdict, EvalVerdict::NotComparable);
        assert_eq!(e.headline, "Não foi possível determinar a diferença");
        // Dentro da variação: sem diferença (preliminar).
        let s = vec![obs("b1", &[], 95.0, 70.0), obs("d1", &d3d11(), 94.0, 69.0)];
        let e = eval(&s, "synthetic-d1");
        assert_eq!(e.verdict, EvalVerdict::NoDifference);
        assert_eq!(e.headline, "Sem diferença relevante detectada (preliminar)");
        // Sinais mistos: inconclusivo.
        let s = vec![obs("b1", &[], 95.0, 40.0), obs("d1", &d3d11(), 60.0, 70.0)];
        assert_eq!(eval(&s, "synthetic-d1").verdict, EvalVerdict::Inconclusive);
        // Configuração desconhecida.
        let mut old = obs("old", &[], 95.0, 70.0);
        old.smart_pack = None;
        assert_eq!(
            eval(&[old], "synthetic-old").verdict,
            EvalVerdict::NotComparable
        );
    }

    #[test]
    fn synthetic_interrupted_and_unverified_sessions_never_supply_real_evidence() {
        use crate::performance::models::{Issue, SessionSource, Severity};
        for variant in 0..3 {
            let mut bad = obs("d1", &d3d11(), 180.0, 100.0);
            match variant {
                0 => bad.source = SessionSource::Synthetic,
                1 => bad.quality.issues.push(Issue::new(
                    Severity::Warning,
                    "interrupted",
                    "interrompida",
                )),
                _ => bad.smart_pack.as_mut().unwrap().verified = false,
            }
            let sessions = vec![obs("b1", &[], 95.0, 70.0), obs("b2", &[], 95.0, 70.0), bad];
            let e = eval(&sessions, "synthetic-d1");
            assert_eq!(e.verdict, EvalVerdict::NotComparable, "variant {variant}");
            assert_eq!(e.pack_sessions, 0);
        }
    }

    #[test]
    fn software_context_changes_exclude_sessions_without_destroying_the_evidence() {
        for field in ["proton", "driver", "kernel"] {
            let mut baseline = obs("b1", &[], 95.0, 70.0);
            baseline.conditions.proton = Some("Proton A".into());
            baseline.conditions.driver = Some("Driver A".into());
            baseline.conditions.kernel = Some("Kernel A".into());
            let mut pack = obs("d1", &d3d11(), 150.0, 100.0);
            pack.conditions = baseline.conditions.clone();
            match field {
                "proton" => pack.conditions.proton = Some("Proton B".into()),
                "driver" => pack.conditions.driver = Some("Driver B".into()),
                _ => pack.conditions.kernel = Some("Kernel B".into()),
            }
            let sessions = vec![baseline, pack];
            let e = eval(&sessions, "synthetic-d1");
            assert_eq!(e.verdict, EvalVerdict::NotComparable, "{field}");
            assert_eq!(sessions.len(), 2);
        }
    }

    #[test]
    fn measured_improvement_and_frametime_tradeoffs_are_reported_honestly() {
        let mut sessions = vec![
            obs("b1", &[], 100.0, 50.0),
            obs("b2", &[], 100.0, 50.0),
            obs("d1", &d3d11(), 120.0, 60.0),
            obs("d2", &d3d11(), 120.0, 60.0),
        ];
        let e = eval(&sessions, "synthetic-d2");
        assert_eq!(e.verdict, EvalVerdict::Improvement);
        assert!(!e.preliminary);
        for m in &mut sessions[2..] {
            m.stats.frametime.p99 = Some(60.0);
        }
        let e = eval(&sessions, "synthetic-d2");
        assert_eq!(e.verdict, EvalVerdict::Inconclusive);
        assert!(e
            .deltas
            .iter()
            .any(|d| d.metric == "ftP99" && d.outcome == Outcome::Regression));
        assert!(!e.ranking_effect.contains("sem diferença"));
    }

    #[test]
    fn stutters_from_zero_are_not_hidden_by_an_undefined_percentage() {
        let mut sessions = vec![
            obs("b1", &[], 100.0, 50.0),
            obs("b2", &[], 100.0, 50.0),
            obs("d1", &d3d11(), 120.0, 60.0),
            obs("d2", &d3d11(), 120.0, 60.0),
        ];
        for (i, m) in sessions.iter_mut().enumerate() {
            m.stats.stability.spikes_per_min = Some(if i < 2 { 0.0 } else { 5.0 });
        }
        let e = eval(&sessions, "synthetic-d2");
        assert_eq!(e.verdict, EvalVerdict::Inconclusive);
        let spikes = e
            .deltas
            .iter()
            .find(|d| d.metric == "spikesPerMin")
            .unwrap();
        assert_eq!(
            (spikes.before, spikes.after, spikes.percentage_available),
            (0.0, 5.0, false)
        );
        assert_eq!(spikes.outcome, Outcome::Regression);
    }
}

// ── Game-specific intelligence: synthetic fixtures only ──────────────────

fn game_fixture(app: &str, id: &str, e: &[EnvPair], fps: f64) -> SessionMeta {
    let mut m = synthetic(id, e, fps, fps * 0.8, 1000.0 / (fps * 0.8));
    m.app_id = Some(app.into());
    m.game_name = Some(format!("Fixture game {app}"));
    let snap = m.smart_pack.as_mut().unwrap();
    snap.app_id = Some(app.into());
    snap.pack_id = pack_id(app, e);
    m
}

#[test]
fn evaluate_requested_session_uses_its_gpu_cohort_even_when_another_gpu_is_newer() {
    use crate::fix_engine::{catalog::Catalog, FixEngine};

    // These imported-session records are explicit test fixtures, never saved
    // to the user's Performance Lab. Declared snapshots intentionally exercise
    // the production validation path without inventing assisted operations.
    let tmp = tempfile::tempdir().unwrap();
    let engine = FixEngine::with(
        tmp.path().join("fixture-data"),
        Catalog::bundled(),
        tmp.path().join("fixture-proc"),
    );
    let mut samples = Vec::new();
    for (gpu, driver, hour, fps) in [
        (
            "NVIDIA GeForce RTX 4060",
            "fixture-nvidia-driver",
            10,
            130.0,
        ),
        ("AMD Radeon RX 7600", "fixture-amd-driver", 11, 55.0),
    ] {
        for n in 0..2 {
            for (kind, changes, avg) in [("base", Vec::new(), 100.0), ("pack", d3d11(), fps)] {
                let id = format!("gpu-{hour}-{kind}-{n}");
                let mut sample = game_fixture(APP, &id, &changes, avg);
                sample.conditions.gpu = Some(gpu.into());
                sample.conditions.driver = Some(driver.into());
                sample.conditions.proton = Some("fixture-proton".into());
                sample.captured_at = Some(format!("2026-10-27T{hour}:00:0{n}Z"));
                sample.smart_pack = Some(super::benchmark::declared(APP, changes));
                samples.push(sample);
            }
        }
    }
    let mut intel = game_fixture(APP, "gpu-intel-pack", &d3d11(), 90.0);
    intel.conditions.gpu = Some("Intel Arc A750".into());
    intel.conditions.driver = Some("fixture-intel-driver".into());
    intel.conditions.proton = Some("fixture-proton".into());
    intel.captured_at = Some("2026-10-27T12:00:00Z".into());
    intel.smart_pack = Some(super::benchmark::declared(APP, d3d11()));
    samples.push(intel);

    let nvidia = super::evaluate_session(&engine, &samples, "synthetic-gpu-10-pack-1").unwrap();
    let amd = super::evaluate_session(&engine, &samples, "synthetic-gpu-11-pack-1").unwrap();
    let intel = super::evaluate_session(&engine, &samples, "synthetic-gpu-intel-pack").unwrap();
    assert_eq!(
        nvidia.verdict,
        EvalVerdict::Improvement,
        "{}",
        nvidia.summary
    );
    assert_eq!(amd.verdict, EvalVerdict::Regression, "{}", amd.summary);
    assert_eq!(intel.verdict, EvalVerdict::NoBaseline, "{}", intel.summary);
    assert_eq!(
        nvidia.pack_id, amd.pack_id,
        "The action and AppID are identical"
    );
    assert_eq!(amd.pack_id, intel.pack_id);
    assert!(!nvidia.preliminary && !amd.preliminary);
    assert_eq!((nvidia.baseline_sessions, nvidia.pack_sessions), (2, 2));
    assert_eq!((amd.baseline_sessions, amd.pack_sessions), (2, 2));
    assert_eq!((intel.baseline_sessions, intel.pack_sessions), (0, 1));
    let fps_delta = |result: &PackEvaluation| {
        result
            .deltas
            .iter()
            .find(|d| d.metric == "fpsAvg")
            .unwrap()
            .delta_pct
    };
    assert!((fps_delta(&nvidia) - 30.0).abs() < 1e-6);
    assert!((fps_delta(&amd) + 45.0).abs() < 1e-6);
    assert!(
        intel.deltas.is_empty(),
        "Other GPUs cannot supply Intel's baseline"
    );
    assert_eq!(samples.len(), 9, "Historical sessions are retained");
    assert!(
        engine.operations(Some(APP)).is_empty(),
        "Evaluation never applies a fix"
    );
}

#[test]
fn equivalent_gpu_labels_do_not_exclude_the_same_model_but_another_model_does() {
    let mut sample = game_fixture(APP, "gpu-normalization", &[], 100.0);
    sample.conditions.gpu = Some("NVIDIA GeForce RTX 4060/PCIe/SSE2".into());
    let mut live = crate::performance::models::Conditions {
        gpu: Some("AD107 [GeForce RTX 4060]".into()),
        ..Default::default()
    };
    let g = super::evidence::group_in_context(
        APP,
        &[sample.clone()],
        |m| m.smart_pack.clone(),
        &baseline_id(APP),
        Some(&live),
    );
    assert_eq!(g.measurements[&baseline_id(APP)].used(), 1);
    live.gpu = Some("AD104 [GeForce RTX 4070]".into());
    let g = super::evidence::group_in_context(
        APP,
        &[sample],
        |m| m.smart_pack.clone(),
        &baseline_id(APP),
        Some(&live),
    );
    assert_eq!(g.measurements[&baseline_id(APP)].used(), 0);
}

mod file_packs {
    use super::*;
    use crate::fix_engine::models::RecipeAction;
    use crate::fixes::models::{
        ArtifactCandidate, ArtifactFile, ArtifactFormat, ArtifactTrust, DiscoveryScope,
        FindingSource, ProviderId,
    };
    use crate::smart_packs::benchmark::{
        action_file_fixes, declared_with_files, pack_id_with_files, snapshot_matches_app,
    };

    /// Entirely synthetic metadata; this fixture never downloads a payload.
    fn artifact(app: &str, path: &str) -> ArtifactCandidate {
        ArtifactCandidate {
            id: format!("synthetic-config-{app}-{path}"), app_id: app.into(),
            scope: DiscoveryScope::verified(app, "Synthetic publisher fixture"),
            recommendation_source: FindingSource { provider:ProviderId::Github,label:"Synthetic fixture".into(),url:"https://github.com/ValveSoftware/Proton/issues/1".into(),reference_id:None,snippet:None,published_at:None },
            repository:"ValveSoftware/Proton".into(),release_tag:"synthetic-v1".into(),
            release_url:"https://github.com/ValveSoftware/Proton/releases/tag/synthetic-v1".into(),
            metadata_url:"https://api.github.com/repos/ValveSoftware/Proton/releases/1".into(),release_id:1,publisher:Some("synthetic-publisher".into()),asset_id:2,
            asset_name:"synthetic.cfg".into(),asset_url:"https://github.com/ValveSoftware/Proton/releases/download/synthetic-v1/synthetic.cfg".into(),size_bytes:12,content_type:Some("text/plain".into()),sha256:Some("a".repeat(64)),signature_url:None,
            format:ArtifactFormat::File,platform:Some("any".into()),architecture:Some("any".into()),
            files:vec![ArtifactFile { source_path:None,target_path:path.into(),sha256:Some("a".repeat(64)),replace:false,expected_original_sha256:None }],
            requirements:Default::default(),trust:ArtifactTrust::OriginAndIntegrity,reasons:vec!["Synthetic fixture only".into()],license:None,published_at:None,
        }
    }

    fn file_unit(id: &str, path: &str) -> Unit {
        let mut u = unit(id, &[], EvidenceLevel::Documented);
        u.file_actions.push(RecipeAction::InstallArtifact {
            artifact: Box::new(artifact(APP, path)),
        });
        u
    }

    #[test]
    fn config_artifact_and_environment_compose_without_fictitious_variables() {
        let files = file_unit("synthetic-file", "settings.cfg");
        let vars = unit(
            "synthetic-env",
            &[("PROTON_HEAP_DELAY_FREE", "1")],
            EvidenceLevel::Documented,
        );
        assert!(admissible(APP, &files).is_ok() && admissible(APP, &vars).is_ok());
        let file = make(
            APP,
            &[&files],
            PackKind::Single,
            Goal::Balanced,
            "f".into(),
            "t".into(),
            &[],
        );
        assert_ne!(
            file.id,
            baseline_id(APP),
            "a file-only pack is never the baseline"
        );
        let mixed = make(
            APP,
            &[&files, &vars],
            PackKind::Combination,
            Goal::Balanced,
            "m".into(),
            "t".into(),
            &[],
        );
        assert_eq!(mixed.env, env(&[("PROTON_HEAP_DELAY_FREE", "1")]));
        assert_eq!(mixed.file_fixes.len(), 1);
        assert_eq!(mixed.artifacts.len(), 1);
        assert_eq!(
            mixed
                .members
                .iter()
                .map(|m| m.file_actions.len())
                .sum::<usize>(),
            1
        );
        assert!(mixed.apply_recipe_id.as_deref() == Some(mixed.id.as_str()));
    }

    #[test]
    fn invalid_artifact_or_foreign_game_never_leaves_a_partial_env_pack() {
        let mut invalid = file_unit("synthetic-invalid", "settings.cfg");
        invalid.env = env(&[("PROTON_HEAP_DELAY_FREE", "1")]);
        let RecipeAction::InstallArtifact { artifact: a } = &mut invalid.file_actions[0] else {
            unreachable!()
        };
        a.sha256 = None;
        let mut foreign = file_unit("synthetic-foreign", "other.cfg");
        let RecipeAction::InstallArtifact { artifact: a } = &mut foreign.file_actions[0] else {
            unreachable!()
        };
        a.app_id = "480".into();
        a.scope = DiscoveryScope::verified("480", "Synthetic other game");
        let invalid = admissible(APP, &invalid).unwrap_err();
        assert!(admissible(APP, &foreign).is_err());
        assert!(invalid.contains("SHA-256"), "{invalid}");
    }

    #[test]
    fn same_file_targets_conflict_even_when_environment_actions_differ() {
        let a = file_unit("synthetic-one", "settings.cfg");
        let b = file_unit("synthetic-two", "SETTINGS.cfg");
        assert_eq!(edge(&a, &b).kind, Some(ConflictKind::SameFile));
        assert!(validate_combination(&[a.clone(), b], 4).is_err());
        let c = file_unit("synthetic-other", "other.cfg");
        assert!(validate_combination(&[a, c], 4).is_ok());
    }

    #[test]
    fn release_and_content_changes_invalidate_pack_identity_and_persisted_snapshot() {
        let u = file_unit("synthetic-release", "settings.cfg");
        let old = action_file_fixes(APP, &u.file_actions).unwrap();
        let mut changed = u.clone();
        let RecipeAction::InstallArtifact { artifact: a } = &mut changed.file_actions[0] else {
            unreachable!()
        };
        a.release_tag = "synthetic-v2".into();
        a.release_url = "https://github.com/ValveSoftware/Proton/releases/tag/synthetic-v2".into();
        a.asset_url =
            "https://github.com/ValveSoftware/Proton/releases/download/synthetic-v2/synthetic.cfg"
                .into();
        let updated = action_file_fixes(APP, &changed.file_actions).unwrap();
        assert_ne!(
            pack_id_with_files(APP, &[], &old),
            pack_id_with_files(APP, &[], &updated)
        );
        let mut content = old.clone();
        content[0].sha256 = "b".repeat(64);
        assert_ne!(
            pack_id_with_files(APP, &[], &old),
            pack_id_with_files(APP, &[], &content)
        );
        let snapshot = declared_with_files(APP, vec![], old);
        let mut loaded: PackSnapshot =
            serde_json::from_str(&serde_json::to_string(&snapshot).unwrap()).unwrap();
        assert!(snapshot_matches_app(APP, &loaded));
        loaded.file_fixes = updated;
        assert!(
            !snapshot_matches_app(APP, &loaded),
            "an old fingerprint cannot claim an updated patch"
        );
        assert!(!snapshot_matches_app("480", &snapshot));
    }

    #[test]
    fn legacy_snapshots_keep_their_identity_without_inventing_file_metadata() {
        let snapshot =
            super::super::benchmark::declared(APP, env(&[("PROTON_HEAP_DELAY_FREE", "1")]));
        let mut value = serde_json::to_value(&snapshot).unwrap();
        value.as_object_mut().unwrap().remove("fileFixes");
        let loaded: PackSnapshot = serde_json::from_value(value).unwrap();
        assert!(loaded.file_fixes.is_empty());
        assert!(snapshot_matches_app(APP, &loaded));
        assert_eq!(
            pack_id_with_files(APP, &loaded.env, &[]),
            pack_id(APP, &loaded.env)
        );
    }

    #[test]
    fn binary_and_unknown_config_actions_cannot_bypass_artifact_policy_in_a_pack() {
        let mut dll = file_unit("synthetic-dll", "physx.dll");
        dll.env = env(&[("PROTON_HEAP_DELAY_FREE", "1")]);
        let mut arbitrary = file_unit("synthetic-script", "settings.cfg");
        arbitrary.file_actions.push(RecipeAction::WriteGameFile {
            path: "manual.cfg".into(),
            content: "quality=high".into(),
        });
        assert!(admissible(APP, &dll).is_err());
        assert!(admissible(APP, &arbitrary).is_err());
    }

    #[test]
    fn joint_hardware_intersection_is_checked_for_all_members() {
        let mut a = unit(
            "a",
            &[("PROTON_HEAP_DELAY_FREE", "1")],
            EvidenceLevel::Documented,
        );
        let mut b = unit(
            "b",
            &[("PROTON_FORCE_LARGE_ADDRESS_AWARE", "1")],
            EvidenceLevel::Documented,
        );
        let mut c = file_unit("c", "settings.cfg");
        a.hardware.driver_versions = vec!["1".into(), "2".into()];
        b.hardware.driver_versions = vec!["2".into(), "3".into()];
        c.hardware.driver_versions = vec!["1".into(), "3".into()];
        assert!(
            validate_combination(&[a, b, c], 4).is_err(),
            "pairwise overlap does not guarantee one common environment"
        );
    }
    #[test]
    fn download_lifecycle_uses_intent_but_benchmark_never_uses_unwritten_hashes() {
        use super::super::benchmark::{operation_file_fixes, operation_planned_file_fixes};
        use crate::fix_engine::models::{
            ExecMode, OpState, Operation, Step, StepRecord, StepStatus,
        };
        let u = file_unit("synthetic-staging", "settings.cfg");
        let RecipeAction::InstallArtifact { artifact: a } = &u.file_actions[0] else {
            unreachable!()
        };
        let mut pack = make(
            APP,
            &[&u],
            PackKind::Single,
            Goal::Balanced,
            "s".into(),
            "t".into(),
            &[],
        );
        let op = Operation {
            manual_recovery_required: false,
            schema: 1,
            id: "synthetic-op".into(),
            recipe_id: u.recipe_id,
            recipe_version: u.version,
            recipe_title: u.title,
            app_id: APP.into(),
            game_name: "Synthetic fixture".into(),
            state: OpState::Downloading,
            mode: ExecMode::Automatic,
            steps: vec![StepRecord {
                step: Step::GameFile {
                    artifact: Some(a.clone()),
                    identity: None,
                    root: "/synthetic/game".into(),
                    rel: "settings.cfg".into(),
                    target: "/synthetic/game/settings.cfg".into(),
                    before_sha: None,
                    content: String::new(),
                },
                status: StepStatus::Pending,
                backup: None,
                after_sha: None,
                rollback_backup: None,
            }],
            plan_fingerprint: "synthetic".into(),
            created_at: String::new(),
            updated_at: String::new(),
            applied_at: None,
            rolled_back_at: None,
            message: None,
            pending: None,
            events: vec![],
            recipe_trust: Some(Trust::Generated),
            finding_ids: vec![],
            recipe_fingerprint: None,
            artifacts: vec![(**a).clone()],
            artifact_progress: None,
        };
        assert_eq!(operation_planned_file_fixes(&op), pack.file_fixes);
        let unwritten = declared_with_files(APP, vec![], operation_file_fixes(&op));
        assert!(!snapshot_matches_app(APP, &unwritten));
        super::super::lifecycle::derive(&mut pack, None, &[op], &[], &[], &[]);
        assert_eq!(pack.state, PackState::Testing);
    }
}

#[test]
fn contradictions_exclusions_gpu_and_proton_versions_are_edges() {
    let fr60 = unit("a", &[("DXVK_FRAME_RATE", "60")], EvidenceLevel::Documented);
    let fr30 = unit("b", &[("DXVK_FRAME_RATE", "30")], EvidenceLevel::Documented);
    assert_eq!(
        edge(&fr60, &fr30).kind,
        Some(ConflictKind::SameKeyDifferentValue)
    );
    let on = unit(
        "c",
        &[("PROTON_ENABLE_NVAPI", "1")],
        EvidenceLevel::Documented,
    );
    let off = unit(
        "d",
        &[("PROTON_DISABLE_NVAPI", "1")],
        EvidenceLevel::Documented,
    );
    assert_eq!(edge(&on, &off).kind, Some(ConflictKind::MutuallyExclusive));
    let wined3d = unit(
        "e",
        &[("PROTON_USE_WINED3D", "1")],
        EvidenceLevel::Documented,
    );
    let nod3d11 = unit("f", &[("PROTON_NO_D3D11", "1")], EvidenceLevel::Documented);
    assert_eq!(
        edge(&wined3d, &nod3d11).kind,
        Some(ConflictKind::MutuallyExclusive)
    );
    let mut nv = unit("g", &[("PROTON_NO_ESYNC", "1")], EvidenceLevel::Documented);
    nv.gpu = Some(GpuReq::Nvidia);
    let mut amd = unit("h", &[("PROTON_NO_FSYNC", "1")], EvidenceLevel::Documented);
    amd.gpu = Some(GpuReq::Amd);
    assert_eq!(edge(&nv, &amd).kind, Some(ConflictKind::GpuMismatch));
    let mut old = unit(
        "i",
        &[("PROTON_NO_ESYNC", "1")],
        EvidenceLevel::Corroborated,
    );
    old.proton_versions = vec!["Proton 3.16".into()];
    let mut new = unit(
        "j",
        &[("PROTON_NO_FSYNC", "1")],
        EvidenceLevel::Corroborated,
    );
    new.proton_versions = vec!["Proton 9.0-4".into()];
    assert_eq!(edge(&old, &new).kind, Some(ConflictKind::ProtonVersions));
    let ok = edge(
        &unit("k", &[("PROTON_NO_ESYNC", "1")], EvidenceLevel::Documented),
        &unit("l", &[("PROTON_NO_FSYNC", "1")], EvidenceLevel::Documented),
    );
    assert_eq!(ok.relation, Relation::NoKnownConflict);
    assert!(ok.reason.contains("não prova"));
    assert!(validate_combination(&[on, off], 6).is_err());
}

#[test]
fn snapshot_identity_rejects_foreign_game_mismatched_actions_and_fingerprint() {
    let mut m = game_fixture(APP, "tampered", &d3d11(), 130.0);
    let mut snap = m.smart_pack.take().unwrap();
    assert!(super::benchmark::snapshot_matches_app(APP, &snap));
    assert!(!super::benchmark::snapshot_matches_app("480", &snap));
    snap.pack_id = pack_id("480", &snap.env);
    m.smart_pack = Some(snap.clone());
    let grouped = group_in_context(APP, &[m], |m| m.smart_pack.clone(), &baseline_id(APP), None);
    assert!(grouped.measurements.values().all(|m| m.used() == 0));
    snap.pack_id = pack_id(APP, &snap.env);
    snap.fingerprint = Some("unrelated fingerprint".into());
    assert!(!super::benchmark::snapshot_matches_app(APP, &snap));
    snap.fingerprint = Some(super::benchmark::snapshot_fingerprint(
        APP,
        &snap.env,
        &snap.recipes,
    ));
    assert!(super::benchmark::snapshot_matches_app(APP, &snap));
    snap.env.clear();
    assert!(!super::benchmark::snapshot_matches_app(APP, &snap));
}

#[test]
fn legacy_snapshot_requires_its_own_appid_identity_and_no_scope_is_invented() {
    let mut m = game_fixture(APP, "legacy", &d3d11(), 120.0);
    let s = m.smart_pack.as_mut().unwrap();
    s.app_id = None;
    s.fingerprint = None;
    assert!(super::benchmark::snapshot_matches_app(APP, s));
    assert!(!super::benchmark::snapshot_matches_app("480", s));
    m.app_id = None;
    let g = group_in_context(APP, &[m], |m| m.smart_pack.clone(), &baseline_id(APP), None);
    assert!(g.measurements.is_empty());
}

#[test]
fn pack_fingerprint_changes_when_a_recipe_changes() {
    let a = unit("x", &[("PROTON_NO_ESYNC", "1")], EvidenceLevel::Documented);
    let mut b = a.clone();
    b.version = "1.0.1".into();
    let pack = |u: &Unit| {
        make(
            APP,
            &[u],
            PackKind::Single,
            Goal::Balanced,
            "x".into(),
            "t".into(),
            &[],
        )
    };
    assert_eq!(pack(&a).id, pack(&b).id, "mesmas variáveis = mesmo pacote");
    assert_ne!(pack(&a).fingerprint, pack(&b).fingerprint);
}
