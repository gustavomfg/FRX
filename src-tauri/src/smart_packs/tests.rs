//! Testes do compositor, do grafo de conflitos, do ranking e da
//! persistência. **Todas as sessões aqui são sintéticas** (`synthetic-*`),
//! com valores escolhidos para dar resultados matemáticos conhecidos.

use std::collections::{BTreeMap, HashMap};

use super::benchmark::{baseline_id, pack_id};
use super::composition::{compose, SingleResults};
use super::conflicts::{edge, graph, launch_notes, validate_combination};
use super::evidence::{compare, group, stats};
use super::models::*;
use super::persistence::SmartStore;
use super::ranking::{rank, Input};
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
fn one_recipe_gives_baseline_plus_single() {
    let c = compose(
        APP,
        &[unit(
            "gen.200260.aaaaaaaaaaaaaaaa",
            &[("PROTON_NO_D3D11", "1")],
            EvidenceLevel::Single,
        )],
        &SingleResults::new(),
        Limits::default(),
    );
    assert_eq!(c.packs.len(), 2);
    assert_eq!(c.packs[0].kind, PackKind::Baseline);
    assert_eq!(c.packs[0].id, baseline_id(APP));
    assert!(
        c.packs[0].apply_recipe_id.is_none(),
        "baseline não se aplica"
    );
    let s = &c.packs[1];
    assert_eq!(s.kind, PackKind::Single);
    assert_eq!(
        s.apply_recipe_id.as_deref(),
        Some("gen.200260.aaaaaaaaaaaaaaaa")
    );
    assert_eq!(s.id, pack_id(APP, &env(&[("PROTON_NO_D3D11", "1")])));
    assert!(c.notes.iter().any(|n| n.contains("só aparece sozinha")));
}

#[test]
fn preventive_risk_separates_known_costs_without_requesting_any_benchmark() {
    let wined3d = unit(
        "wine",
        &[("PROTON_USE_WINED3D", "1")],
        EvidenceLevel::Documented,
    );
    let heap = unit(
        "heap",
        &[("PROTON_HEAP_DELAY_FREE", "1")],
        EvidenceLevel::Documented,
    );
    let c = compose(
        APP,
        &[wined3d.clone(), heap],
        &SingleResults::new(),
        Limits::default(),
    );
    assert!(
        c.packs.iter().all(|p| p.kind != PackKind::Combination),
        "evidência documental de compatibilidade não prova ganho de FPS"
    );
    let only_wine = compose(APP, &[wined3d], &SingleResults::new(), Limits::default());
    for profile in [
        Profile::Performance,
        Profile::Balanced,
        Profile::Compatibility,
    ] {
        let r = rank(Input {
            profile,
            packs: &only_wine.packs,
            baseline_id: &baseline_id(APP),
            measured: &BTreeMap::new(),
            vs: &HashMap::new(),
            excluded: &[],
        });
        let wine = r
            .entries
            .iter()
            .find(|e| e.pack_id != baseline_id(APP))
            .unwrap();
        assert_eq!(wine.tier, Tier::PerformanceRisk);
        assert_eq!(wine.outcome, Outcome::NotMeasured);
        assert!(wine.position.is_none() && r.winner.is_none());
        assert!(
            r.next_test.is_none(),
            "não manda o usuário testar uma alternativa que já tem risco conhecido"
        );
        assert!(wine.reasons.iter().any(|r| r.contains("OpenGL")));
    }
}

#[test]
fn preventive_risks_survive_restart_without_breaking_legacy_history_tiers() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SmartStore::new(tmp.path().join("smart-packs"));
    let w = world(vec![]);
    let r = ranked(&w, Profile::Performance);
    let f = store.record(APP, &w.packs, &r, 0).unwrap();
    assert!(!f.history[0].preventive_risks.is_empty());
    assert!(f.history[0]
        .entries
        .iter()
        .all(|(_, _, tier, _)| *tier != Tier::PerformanceRisk));
    let restarted = SmartStore::new(tmp.path().join("smart-packs"));
    let again = restarted.record(APP, &w.packs, &r, 0).unwrap();
    assert_eq!(
        again.history.len(),
        1,
        "não registra a mesma classificação a cada abertura"
    );
    assert_eq!(
        again.history[0].preventive_risks,
        f.history[0].preventive_risks
    );
}

#[test]
fn compatibility_reports_do_not_become_fps_promises_or_mandatory_tests() {
    let c = compose(
        APP,
        &[unit(
            "memory",
            &[("PROTON_FORCE_LARGE_ADDRESS_AWARE", "1")],
            EvidenceLevel::Documented,
        )],
        &SingleResults::new(),
        Limits::default(),
    );
    let r = rank(Input {
        profile: Profile::Performance,
        packs: &c.packs,
        baseline_id: &baseline_id(APP),
        measured: &BTreeMap::new(),
        vs: &HashMap::new(),
        excluded: &[],
    });
    let entry = r
        .entries
        .iter()
        .find(|e| e.pack_id != baseline_id(APP))
        .unwrap();
    assert_eq!(entry.tier, Tier::Candidate);
    assert_eq!(entry.outcome, Outcome::NotMeasured);
    assert!(entry.headline.contains("impacto ainda desconhecido"));
    assert!(r.next_test.is_none() && r.winner.is_none());
}

#[test]
fn compatible_recipes_with_strong_evidence_are_combined_once() {
    let units = [
        unit(
            "a",
            &[("PROTON_FORCE_LARGE_ADDRESS_AWARE", "1")],
            EvidenceLevel::Corroborated,
        ),
        unit(
            "b",
            &[("PROTON_HEAP_DELAY_FREE", "1")],
            EvidenceLevel::Documented,
        ),
    ];
    let c = compose(APP, &units, &SingleResults::new(), Limits::default());
    let combos: Vec<&SmartPack> = c
        .packs
        .iter()
        .filter(|p| p.kind == PackKind::Combination)
        .collect();
    assert_eq!(
        combos.len(),
        1,
        "pack de compatibilidade e par têm as mesmas variáveis: um só"
    );
    assert_eq!(combos[0].name, "Pack de compatibilidade");
    assert_eq!(combos[0].env.len(), 2);
    assert_eq!(
        combos[0].apply_recipe_id.as_deref(),
        Some(combos[0].id.as_str())
    );
    assert_eq!(combos[0].members.len(), 2);
}

#[test]
fn duplicated_recipes_collapse_into_one_pack() {
    let units = [
        unit(
            "a",
            &[("PROTON_NO_ESYNC", "1")],
            EvidenceLevel::Corroborated,
        ),
        unit(
            "b",
            &[("PROTON_NO_ESYNC", "1")],
            EvidenceLevel::Corroborated,
        ),
    ];
    let c = compose(APP, &units, &SingleResults::new(), Limits::default());
    assert_eq!(c.packs.len(), 2, "baseline + uma receita");
    assert_eq!(
        edge(&units[0], &units[1]).kind,
        Some(ConflictKind::Redundant)
    );
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
    // Nada disso é combinado; combinações sem conflito conhecido continuam possíveis.
    let c = compose(
        APP,
        &[on, off, wined3d, nod3d11, old, new],
        &SingleResults::new(),
        Limits::default(),
    );
    let has = |p: &SmartPack, k: &str| p.env.iter().any(|e| e.key == k);
    assert!(!c
        .packs
        .iter()
        .any(|p| has(p, "PROTON_ENABLE_NVAPI") && has(p, "PROTON_DISABLE_NVAPI")));
    assert!(!c
        .packs
        .iter()
        .any(|p| has(p, "PROTON_USE_WINED3D") && has(p, "PROTON_NO_D3D11")));
    assert!(!c
        .packs
        .iter()
        .any(|p| has(p, "PROTON_NO_ESYNC") && has(p, "PROTON_NO_FSYNC")));
    assert!(
        !c.packs
            .iter()
            .any(|p| has(p, "PROTON_USE_WINED3D") && has(p, "PROTON_NO_ESYNC")),
        "ausência de conflito não elimina risco de custo de desempenho"
    );
    let single = c
        .packs
        .iter()
        .find(|p| p.kind == PackKind::Single && has(p, "PROTON_USE_WINED3D"))
        .unwrap();
    assert!(single
        .conflicts_avoided
        .iter()
        .any(|e| e.kind == Some(ConflictKind::MutuallyExclusive)));
}

#[test]
fn already_included_frame_caps_and_weak_evidence_are_never_combined() {
    let mut inc = unit("a", &[("PROTON_NO_ESYNC", "1")], EvidenceLevel::Documented);
    inc.already_included = true;
    let cap = unit("b", &[("DXVK_FRAME_RATE", "60")], EvidenceLevel::Documented);
    let weak = unit("c", &[("PROTON_NO_D3D11", "1")], EvidenceLevel::Single);
    let good = unit(
        "d",
        &[("PROTON_HEAP_DELAY_FREE", "1")],
        EvidenceLevel::Documented,
    );
    let c = compose(
        APP,
        &[inc, cap, weak, good],
        &SingleResults::new(),
        Limits::default(),
    );
    assert!(
        c.packs.iter().all(|p| p.kind != PackKind::Combination),
        "{:?}",
        c.packs.iter().map(|p| &p.name).collect::<Vec<_>>()
    );
    assert_eq!(c.packs.len(), 5);
}

#[test]
fn locally_measured_non_regression_makes_a_single_report_combinable() {
    let a = unit("a", &[("PROTON_NO_D3D11", "1")], EvidenceLevel::Single);
    let b = unit(
        "b",
        &[("PROTON_HEAP_DELAY_FREE", "1")],
        EvidenceLevel::Single,
    );
    let mut singles = SingleResults::new();
    singles.insert("a".into(), (Outcome::Improvement, Outcome::NoDifference));
    singles.insert("b".into(), (Outcome::Improvement, Outcome::Improvement));
    let c = compose(APP, &[a, b], &singles, Limits::default());
    let names: Vec<&str> = c.packs.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"Pack de desempenho"), "{names:?}");
    assert_eq!(
        c.packs
            .iter()
            .filter(|p| p.kind == PackKind::Combination)
            .count(),
        1,
        "desempenho, equilibrado e par têm as mesmas variáveis"
    );
}

#[test]
fn composition_is_bounded_with_dozens_of_recipes() {
    let keys = [
        "PROTON_NO_ESYNC",
        "PROTON_NO_FSYNC",
        "PROTON_NO_NTSYNC",
        "PROTON_HEAP_DELAY_FREE",
        "PROTON_OLD_GL_STRING",
        "PROTON_FORCE_LARGE_ADDRESS_AWARE",
        "PROTON_NO_D3D10",
        "PROTON_USE_XALIA",
    ];
    let mut units = Vec::new();
    for i in 0..40 {
        let k = keys[i % keys.len()];
        let v = if i < keys.len() {
            "1".to_owned()
        } else {
            format!("{}", i)
        };
        units.push(Unit {
            env: vec![EnvPair {
                key: k.into(),
                value: v,
            }],
            ..unit(&format!("u{i:02}"), &[], EvidenceLevel::Corroborated)
        });
    }
    let t = std::time::Instant::now();
    let c = compose(APP, &units, &SingleResults::new(), Limits::default());
    assert!(t.elapsed().as_millis() < 500);
    assert!(c.packs.len() <= Limits::default().max_packs);
    assert_eq!(c.excluded.len(), 40 - Limits::default().max_units);
    assert!(c.notes.iter().any(|n| n.starts_with("Limite")));
    let limits = Limits {
        max_units: 3,
        max_packs: 4,
        max_pack_size: 2,
    };
    let c = compose(APP, &units, &SingleResults::new(), limits);
    assert_eq!(c.packs.len(), 4);
    assert!(c.packs.iter().all(|p| p.members.len() <= 2));
}

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

struct World {
    packs: Vec<SmartPack>,
    sessions: Vec<SessionMeta>,
}

fn d3d11() -> Vec<EnvPair> {
    env(&[("PROTON_NO_D3D11", "1")])
}
fn esync() -> Vec<EnvPair> {
    env(&[("PROTON_NO_ESYNC", "1")])
}

fn world(sessions: Vec<SessionMeta>) -> World {
    let c = compose(
        APP,
        &[
            unit(
                "gen.200260.aaaaaaaaaaaaaaaa",
                &[("PROTON_NO_D3D11", "1")],
                EvidenceLevel::Single,
            ),
            unit(
                "frx.compat.esync",
                &[("PROTON_NO_ESYNC", "1")],
                EvidenceLevel::Corroborated,
            ),
        ],
        &SingleResults::new(),
        Limits::default(),
    );
    World {
        packs: c.packs,
        sessions,
    }
}

fn ranked(w: &World, profile: Profile) -> Ranking {
    let g = group(
        APP,
        &w.sessions,
        |m| m.smart_pack.clone(),
        &baseline_id(APP),
    );
    let metas: HashMap<&str, &SessionMeta> =
        w.sessions.iter().map(|m| (m.id.as_str(), m)).collect();
    let base = g.measurements.get(&baseline_id(APP));
    let mut vs = HashMap::new();
    for p in w.packs.iter().filter(|p| p.kind != PackKind::Baseline) {
        if let (Some(b), Some(m)) = (base, g.measurements.get(&p.id)) {
            vs.insert(p.id.clone(), compare(b, m, &metas));
        }
    }
    rank(Input {
        profile,
        packs: &w.packs,
        baseline_id: &baseline_id(APP),
        measured: &g.measurements,
        vs: &vs,
        excluded: &[],
    })
}

fn entry<'a>(r: &'a Ranking, e: &[EnvPair]) -> &'a RankEntry {
    r.entries
        .iter()
        .find(|x| x.pack_id == pack_id(APP, e))
        .unwrap()
}

fn base(n: usize, avg: f64) -> Vec<SessionMeta> {
    (0..n)
        .map(|i| synthetic(&format!("b{i}"), &[], avg + i as f64 * 0.5, 50.0, 20.0))
        .collect()
}

#[test]
fn without_benchmarks_there_is_no_winner_and_baseline_comes_first() {
    let r = ranked(&world(vec![]), Profile::Performance);
    assert!(r.winner.is_none());
    assert!(r
        .summary
        .contains("Nenhuma otimização de desempenho identificada"));
    assert!(
        r.next_test.is_none(),
        "riscos conhecidos não exigem que o usuário meça todos os candidatos"
    );
    assert_eq!(r.entries[0].pack_id, baseline_id(APP));
    let e = entry(&r, &d3d11());
    assert_eq!(e.tier, Tier::PerformanceRisk);
    assert_eq!(
        e.headline, "Risco potencial — impacto neste jogo desconhecido",
        "PROTON_NO_D3D11 não sobe sem benchmark"
    );
    assert!(e.position.is_none());
    assert_eq!(
        entry(&r, &esync()).headline,
        "Risco potencial — impacto neste jogo desconhecido"
    );
}

#[test]
fn a_single_benchmark_each_is_only_indicative() {
    let mut s = base(1, 100.0);
    s.push(synthetic("p0", &d3d11(), 130.0, 60.0, 18.0));
    let r = ranked(&world(s), Profile::Performance);
    let e = entry(&r, &d3d11());
    assert_eq!(e.tier, Tier::PerformanceRisk);
    assert!(e.headline.contains("dados preliminares"));
    assert!(e.vs_baseline.as_ref().unwrap().limited);
    assert!(r.winner.is_none(), "uma captura não elege vencedor");
    assert!(r.next_test.is_none());
}

#[test]
fn clear_improvement_with_repetitions_wins_with_known_percentages() {
    let mut s = base(2, 100.0); // medianas: 100,25 ; dispersão 0,5%
    s.push(synthetic("p0", &d3d11(), 120.0, 55.0, 19.0));
    s.push(synthetic("p1", &d3d11(), 121.0, 55.0, 19.0));
    let r = ranked(&world(s), Profile::Performance);
    let e = entry(&r, &d3d11());
    assert_eq!(e.tier, Tier::Measured);
    assert_eq!(e.position, Some(1));
    let avg = e.vs_baseline.as_ref().unwrap().get("fpsAvg").unwrap();
    assert!((avg.delta_pct - (120.5 - 100.25) / 100.25 * 100.0).abs() < 1e-9);
    assert_eq!(avg.threshold_pct, 3.0);
    assert_eq!(r.winner.as_deref(), Some(pack_id(APP, &d3d11()).as_str()));
    assert!(r.summary.starts_with("#1 — Melhor desempenho medido"));
}

#[test]
fn higher_average_but_lower_one_percent_low_depends_on_profile() {
    let mut s = base(2, 100.0);
    s.push(synthetic("p0", &d3d11(), 120.0, 40.0, 30.0));
    s.push(synthetic("p1", &d3d11(), 120.0, 40.0, 30.0));
    let w = world(s);
    let perf = ranked(&w, Profile::Performance);
    assert_eq!(entry(&perf, &d3d11()).outcome, Outcome::Mixed);
    assert!(perf.winner.is_none(), "troca não vira vencedor");
    assert!(perf.summary.contains("Sem vencedor claro"));
    let bal = ranked(&w, Profile::Balanced);
    assert_eq!(entry(&bal, &d3d11()).tier, Tier::NotRecommended);
    let compat = ranked(&w, Profile::Compatibility);
    assert_eq!(
        entry(&compat, &d3d11()).tier,
        Tier::NotRecommended,
        "P99 pior = estabilidade pior"
    );
}

#[test]
fn unstable_frametime_is_a_regression_for_balanced() {
    let mut s = base(2, 100.0);
    s.push(synthetic("p0", &d3d11(), 101.0, 50.0, 26.0));
    s.push(synthetic("p1", &d3d11(), 101.0, 50.0, 26.0));
    let r = ranked(&world(s), Profile::Balanced);
    let e = entry(&r, &d3d11());
    assert_eq!(e.tier, Tier::NotRecommended);
    assert!(e.reasons.iter().any(|x| x.contains("Frametime P99")));
}

#[test]
fn similar_performance_is_no_difference_and_no_winner() {
    let mut s = base(2, 100.0);
    s.push(synthetic("p0", &d3d11(), 101.0, 50.0, 20.0));
    s.push(synthetic("p1", &d3d11(), 100.5, 50.0, 20.0));
    s.push(synthetic("q0", &esync(), 100.0, 50.0, 20.0));
    s.push(synthetic("q1", &esync(), 100.8, 50.0, 20.0));
    let r = ranked(&world(s), Profile::Performance);
    let (a, b) = (entry(&r, &d3d11()), entry(&r, &esync()));
    assert_eq!(a.outcome, Outcome::NoDifference);
    assert_eq!(a.position, b.position, "empate técnico");
    assert!(a.reasons[0].starts_with("Empate técnico"));
    assert!(r.winner.is_none());
    assert!(r.summary.starts_with("Resultado inconclusivo"));
}

#[test]
fn proven_regression_is_not_recommended() {
    let mut s = base(2, 100.0);
    s.push(synthetic("p0", &d3d11(), 80.0, 40.0, 25.0));
    s.push(synthetic("p1", &d3d11(), 81.0, 40.0, 25.0));
    let r = ranked(&world(s), Profile::Performance);
    let e = entry(&r, &d3d11());
    assert_eq!(
        (e.tier, e.headline.as_str()),
        (Tier::NotRecommended, "Regressão medida")
    );
}

#[test]
fn different_conditions_are_excluded_and_missing_metadata_is_noted() {
    let mut s = base(2, 100.0);
    let mut other = synthetic("p0", &d3d11(), 150.0, 60.0, 15.0);
    other.conditions.resolution = Some("1280x720".into());
    let mut nometa = synthetic("p1", &d3d11(), 120.0, 55.0, 19.0);
    nometa.conditions.preset = None;
    s.push(other);
    s.push(nometa);
    let g = group(APP, &s, |m| m.smart_pack.clone(), &baseline_id(APP));
    let m = &g.measurements[&pack_id(APP, &d3d11())];
    let excl = m.sessions.iter().find(|x| x.id == "synthetic-p0").unwrap();
    assert!(excl.excluded.as_deref().unwrap().contains("Resolução"));
    let kept = m.sessions.iter().find(|x| x.id == "synthetic-p1").unwrap();
    assert!(kept.excluded.is_none());
    assert!(kept.notes.iter().any(|n| n.contains("preset")));
    assert_eq!(m.used(), 1);
}

#[test]
fn large_spread_makes_the_result_inconclusive() {
    let mut s = vec![
        synthetic("b0", &[], 100.0, 50.0, 20.0),
        synthetic("b1", &[], 140.0, 50.0, 20.0),
    ];
    s.push(synthetic("p0", &d3d11(), 140.0, 50.0, 20.0));
    s.push(synthetic("p1", &d3d11(), 145.0, 50.0, 20.0));
    let r = ranked(&world(s), Profile::Performance);
    let avg = entry(&r, &d3d11()).vs_baseline.clone().unwrap();
    let d = avg.get("fpsAvg").unwrap();
    assert!(
        (d.threshold_pct - 40.0 / 120.0 * 100.0).abs() < 1e-9,
        "limiar = dispersão do baseline"
    );
    assert_eq!(d.outcome, Outcome::NoDifference);
    assert!(r.winner.is_none());
}

#[test]
fn ranking_updates_after_new_sessions_and_history_keeps_every_change() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SmartStore::new(tmp.path().join("smart-packs"));
    let mut s = base(1, 100.0);
    s.push(synthetic("p0", &d3d11(), 120.0, 55.0, 19.0));
    let w = world(s.clone());
    let r1 = ranked(&w, Profile::Performance);
    assert_eq!(entry(&r1, &d3d11()).tier, Tier::PerformanceRisk);
    store.record(APP, &w.packs, &r1, 2).unwrap();
    store.record(APP, &w.packs, &r1, 2).unwrap();
    assert_eq!(
        store.load(APP).unwrap().history.len(),
        1,
        "sem mudança, sem novo ponto"
    );
    s.push(synthetic("b9", &[], 100.4, 50.0, 20.0));
    s.push(synthetic("p9", &d3d11(), 121.0, 55.0, 19.0));
    let r2 = ranked(&world(s), Profile::Performance);
    assert_eq!(entry(&r2, &d3d11()).position, Some(1));
    let f = store.record(APP, &w.packs, &r2, 4).unwrap();
    assert_eq!(f.history.len(), 2);
    assert!(f.history[1].reason.contains("2 → 4"));
    assert_eq!(f.history[0].sessions, 2, "histórico antigo preservado");
}

#[test]
fn statistics_are_exact() {
    let s = stats(&[100.0, 110.0, 90.0, 105.0]).unwrap();
    assert_eq!((s.n, s.median, s.min, s.max), (4, 102.5, 90.0, 110.0));
    assert!((s.spread_pct.unwrap() - 20.0 / 102.5 * 100.0).abs() < 1e-12);
    assert!(stats(&[42.0]).unwrap().spread_pct.is_none());
    assert!(stats(&[]).is_none());
}

#[test]
fn unsuitable_and_unrecorded_sessions_never_count() {
    let mut bad = synthetic("p0", &d3d11(), 500.0, 300.0, 5.0);
    bad.quality.level = QualityLevel::Unsuitable;
    let mut old = synthetic("old", &[], 100.0, 50.0, 20.0);
    old.smart_pack = None;
    let g = group(
        APP,
        &[bad, old],
        |m| m.smart_pack.clone(),
        &baseline_id(APP),
    );
    assert_eq!(g.unassigned.len(), 1);
    assert!(g.unassigned[0]
        .excluded
        .as_deref()
        .unwrap()
        .contains("Declare"));
    assert_eq!(g.measurements[&pack_id(APP, &d3d11())].used(), 0);
}

// ── Persistência ──────────────────────────────────────────────────────────

#[test]
fn store_migrates_old_files_and_never_overwrites_newer_or_corrupt_ones() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("smart-packs");
    std::fs::create_dir_all(&dir).unwrap();
    let store = SmartStore::new(dir.clone());
    std::fs::write(dir.join("200260.json"), r#"{"appId":"200260","packs":[]}"#).unwrap();
    assert_eq!(
        store.load(APP).unwrap().schema,
        PACK_SCHEMA,
        "formato sem versão migrado"
    );
    std::fs::write(dir.join("200260.json"), r#"{"schema":99,"appId":"200260"}"#).unwrap();
    assert!(store.load(APP).unwrap_err().contains("mais novo"));
    let w = world(vec![]);
    assert!(
        store.remember(APP, &w.packs).is_err(),
        "não sobrescreve formato mais novo"
    );
    std::fs::write(dir.join("200260.json"), "{lixo").unwrap();
    assert!(store.load(APP).unwrap().packs.is_empty());
    assert!(
        dir.join("200260.json.corrupt").is_file(),
        "cópia guardada para inspeção"
    );
    store.remember(APP, &w.packs).unwrap();
    let id = pack_id(APP, &d3d11());
    assert_eq!(
        store.members(APP, &id).unwrap(),
        vec!["gen.200260.aaaaaaaaaaaaaaaa"]
    );
    // Pacote que deixou de ser composto fica no registro como aposentado.
    store
        .record(APP, &w.packs[..1], &ranked(&w, Profile::Performance), 0)
        .unwrap();
    let f = store.load(APP).unwrap();
    assert!(f.packs.iter().find(|p| p.pack.id == id).unwrap().retired);
}

#[test]
fn pack_fingerprint_changes_when_a_recipe_changes() {
    let a = compose(
        APP,
        &[unit(
            "x",
            &[("PROTON_NO_ESYNC", "1")],
            EvidenceLevel::Documented,
        )],
        &SingleResults::new(),
        Limits::default(),
    );
    let mut changed = unit("x", &[("PROTON_NO_ESYNC", "1")], EvidenceLevel::Documented);
    changed.version = "1.0.1".into();
    let b = compose(APP, &[changed], &SingleResults::new(), Limits::default());
    assert_eq!(
        a.packs[1].id, b.packs[1].id,
        "mesmas variáveis = mesmo pacote"
    );
    assert_ne!(a.packs[1].fingerprint, b.packs[1].fingerprint);
}

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
        let w = world(sessions.to_vec());
        let g = group(APP, sessions, |m| m.smart_pack.clone(), &baseline_id(APP));
        let metas: HashMap<&str, &SessionMeta> =
            sessions.iter().map(|m| (m.id.as_str(), m)).collect();
        let name_of = |pid: &str| w.packs.iter().find(|p| p.id == pid).map(|p| p.name.clone());
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
            e.pack_name, "gen.200260.aaaaaaaaaaaaaaaa",
            "nome do pacote de uma receita"
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
        // Ranking: não sobe, não é vencedor, fica marcado no fim dos candidatos.
        let r = ranked(&world(s.clone()), Profile::Performance);
        let entry = entry(&r, &d3d11());
        assert_eq!(entry.tier, Tier::PerformanceRisk);
        assert!(entry.headline.contains("indício de regressão neste jogo"));
        assert!(r.winner.is_none());
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
        for profile in [Profile::Performance, Profile::Balanced] {
            let r = ranked(&world(s.clone()), profile);
            assert_eq!(
                entry(&r, &d3d11()).tier,
                Tier::NotRecommended,
                "{profile:?}"
            );
            assert!(r.winner.is_none());
        }
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
    fn preliminary_regression_blocks_combinations_until_repeated() {
        let strong = unit("a", &[("PROTON_NO_ESYNC", "1")], EvidenceLevel::Documented);
        let other = unit(
            "b",
            &[("PROTON_HEAP_DELAY_FREE", "1")],
            EvidenceLevel::Documented,
        );
        let mut singles = SingleResults::new();
        singles.insert("a".into(), (Outcome::Regression, Outcome::NoDifference));
        let c = compose(APP, &[strong, other], &singles, Limits::default());
        assert!(c.packs.iter().all(|p| p.kind != PackKind::Combination));
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
            assert!(ranked(&world(sessions), Profile::Performance)
                .winner
                .is_none());
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
        assert!(ranked(&world(sessions.clone()), Profile::Performance)
            .winner
            .is_some());
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
        assert!(ranked(&world(sessions), Profile::Balanced).winner.is_none());
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
        assert!(ranked(&world(sessions.clone()), Profile::Performance)
            .winner
            .is_none());
        assert!(ranked(&world(sessions), Profile::Balanced).winner.is_none());
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

fn scoped_world(app: &str, sessions: &[SessionMeta]) -> Ranking {
    let mut u = unit(
        "shared-action",
        &[("PROTON_NO_D3D11", "1")],
        EvidenceLevel::Single,
    );
    u.app_id = Some(app.into());
    let packs = compose(app, &[u], &SingleResults::new(), Limits::default()).packs;
    let grouped = group(app, sessions, |m| m.smart_pack.clone(), &baseline_id(app));
    let metas = sessions.iter().map(|m| (m.id.as_str(), m)).collect();
    let mut vs = HashMap::new();
    if let Some(base) = grouped.measurements.get(&baseline_id(app)) {
        for p in packs.iter().filter(|p| p.kind != PackKind::Baseline) {
            if let Some(measured) = grouped.measurements.get(&p.id) {
                vs.insert(p.id.clone(), compare(base, measured, &metas));
            }
        }
    }
    rank(Input {
        profile: Profile::Performance,
        packs: &packs,
        baseline_id: &baseline_id(app),
        measured: &grouped.measurements,
        vs: &vs,
        excluded: &[],
    })
}

#[test]
fn same_action_keeps_positive_negative_and_unknown_games_independent() {
    // Emulates real metadata strictly inside an explicitly synthetic fixture.
    let mut all = Vec::new();
    for (app, after) in [(APP, 55.0), ("480", 125.0)] {
        for n in 0..2 {
            all.push(game_fixture(app, &format!("{app}-base-{n}"), &[], 100.0));
            all.push(game_fixture(
                app,
                &format!("{app}-pack-{n}"),
                &d3d11(),
                after,
            ));
        }
    }
    let batman = scoped_world(APP, &all);
    let other = scoped_world("480", &all);
    let unknown = scoped_world("570", &all);
    let pick = |r: &Ranking, app: &str| {
        r.entries
            .iter()
            .find(|e| e.pack_id == pack_id(app, &d3d11()))
            .unwrap()
            .clone()
    };
    assert_eq!(pick(&batman, APP).outcome, Outcome::Regression);
    assert_eq!(pick(&batman, APP).tier, Tier::NotRecommended);
    assert_eq!(pick(&other, "480").outcome, Outcome::Improvement);
    assert_eq!(pick(&other, "480").tier, Tier::Measured);
    assert_eq!(
        other.winner.as_deref(),
        Some(pack_id("480", &d3d11()).as_str())
    );
    assert_eq!(pick(&unknown, "570").outcome, Outcome::NotMeasured);
    assert!(unknown.winner.is_none());
}

#[test]
fn composition_and_conflicts_reject_foreign_and_unattributed_units() {
    let a = unit(
        "a",
        &[("PROTON_FORCE_LARGE_ADDRESS_AWARE", "1")],
        EvidenceLevel::Documented,
    );
    let mut b = unit(
        "b",
        &[("PROTON_HEAP_DELAY_FREE", "1")],
        EvidenceLevel::Documented,
    );
    b.app_id = Some("480".into());
    let mut legacy = b.clone();
    legacy.recipe_id = "legacy-no-app".into();
    legacy.app_id = None;
    assert_eq!(edge(&a, &b).kind, Some(ConflictKind::GameScope));
    assert!(validate_combination(&[a.clone(), b.clone()], 4).is_err());
    for app in [APP, "480"] {
        let composed = compose(
            app,
            &[a.clone(), b.clone(), legacy.clone()],
            &SingleResults::new(),
            Limits::default(),
        );
        assert_eq!(composed.packs.len(), 2);
        assert_eq!(composed.excluded.len(), 2);
        assert!(composed.packs.iter().all(|p| p.app_id == app));
        assert!(composed.packs.iter().all(|p| p.members.len() < 2));
    }
}

#[test]
fn specific_positive_reports_can_justify_testing_without_overriding_safety_or_claiming_local_gain()
{
    use crate::fix_engine::dynamic::GamePerformanceEvidence;
    let mut risky = unit(
        "risk",
        &[("PROTON_NO_D3D11", "1")],
        EvidenceLevel::Corroborated,
    );
    risky.performance_evidence = GamePerformanceEvidence {
        app_id: Some(APP.into()),
        positive: 2,
        negative: 0,
        context_compatible: true,
        source_urls: vec![
            "https://github.com/example/repo/issues/1#issuecomment-1".into(),
            "https://github.com/example/repo/issues/1#issuecomment-2".into(),
        ],
        ..Default::default()
    };
    let packs = compose(
        APP,
        &[risky.clone()],
        &SingleResults::new(),
        Limits::default(),
    )
    .packs;
    let ranked = rank(Input {
        profile: Profile::Performance,
        packs: &packs,
        baseline_id: &baseline_id(APP),
        measured: &BTreeMap::new(),
        vs: &HashMap::new(),
        excluded: &[],
    });
    let risk = &ranked.entries[1];
    assert_eq!(risk.tier, Tier::Candidate);
    assert_eq!(risk.outcome, Outcome::NotMeasured);
    assert!(risk.headline.contains("específica deste jogo"));
    assert!(ranked.winner.is_none());
    assert!(ranked.next_test.is_some());
    assert!(!packs[1].performance_warnings.is_empty());
    assert!(
        !super::composition::eligible(&risky, None),
        "relatos positivos não liberam combinação de risco sem medição local"
    );
    risky.performance_evidence.negative = 1;
    let packs = compose(
        APP,
        &[risky.clone()],
        &SingleResults::new(),
        Limits::default(),
    )
    .packs;
    let mixed = rank(Input {
        profile: Profile::Performance,
        packs: &packs,
        baseline_id: &baseline_id(APP),
        measured: &BTreeMap::new(),
        vs: &HashMap::new(),
        excluded: &[],
    });
    assert!(mixed.entries[1].headline.contains("conflitantes"));
    assert!(mixed.next_test.is_none());
    risky.performance_evidence.negative = 0;
    risky.performance_evidence.app_id = Some("480".into());
    let packs = compose(APP, &[risky], &SingleResults::new(), Limits::default()).packs;
    let foreign = rank(Input {
        profile: Profile::Performance,
        packs: &packs,
        baseline_id: &baseline_id(APP),
        measured: &BTreeMap::new(),
        vs: &HashMap::new(),
        excluded: &[],
    });
    assert_eq!(foreign.entries[1].tier, Tier::PerformanceRisk);
}

#[test]
fn snapshot_identity_rejects_foreign_game_mismatched_actions_and_fingerprint() {
    let mut m = game_fixture(APP, "tampered", &d3d11(), 130.0);
    let mut snap = m.smart_pack.take().unwrap();
    assert!(super::benchmark::snapshot_matches_app(APP, &snap));
    assert!(!super::benchmark::snapshot_matches_app("480", &snap));
    snap.pack_id = pack_id("480", &snap.env);
    m.smart_pack = Some(snap.clone());
    let grouped = group(APP, &[m], |m| m.smart_pack.clone(), &baseline_id(APP));
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
    let g = group(APP, &[m], |m| m.smart_pack.clone(), &baseline_id(APP));
    assert!(g.measurements.is_empty() && g.unassigned.is_empty());
}

#[test]
fn per_game_store_survives_switching_restart_and_preserves_invalid_legacy_records_without_executing_them(
) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("packs");
    let store = SmartStore::new(dir.clone());
    let mut ua = unit(
        "gen.200260.aaaaaaaaaaaaaaaa",
        &[("PROTON_HEAP_DELAY_FREE", "1")],
        EvidenceLevel::Single,
    );
    let a = compose(APP, &[ua.clone()], &SingleResults::new(), Limits::default()).packs;
    ua.app_id = Some("480".into());
    ua.recipe_id = "gen.480.aaaaaaaaaaaaaaaa".into();
    let b = compose("480", &[ua], &SingleResults::new(), Limits::default()).packs;
    store.remember(APP, &a).unwrap();
    store.remember("480", &b).unwrap();
    assert!(store.remember(APP, &b).is_err());
    let restarted = SmartStore::new(dir.clone());
    assert_eq!(restarted.pack(APP, &a[1].id).unwrap().app_id, APP);
    assert!(restarted.pack("480", &a[1].id).is_none());
    let mut f = restarted.load(APP).unwrap();
    f.packs[1].pack.app_id = "480".into();
    std::fs::write(
        dir.join(format!("{APP}.json")),
        serde_json::to_vec(&f).unwrap(),
    )
    .unwrap();
    assert!(restarted.pack(APP, &a[1].id).is_none());
    assert_eq!(
        restarted.load(APP).unwrap().packs.len(),
        2,
        "registro antigo preservado para auditoria"
    );
}

#[test]
fn locally_positive_wined3d_is_not_blocked_by_a_universal_performance_penalty() {
    let wine = unit(
        "wine",
        &[("PROTON_USE_WINED3D", "1")],
        EvidenceLevel::Single,
    );
    let memory = unit(
        "memory",
        &[("PROTON_FORCE_LARGE_ADDRESS_AWARE", "1")],
        EvidenceLevel::Single,
    );
    let singles = SingleResults::from([
        ("wine".into(), (Outcome::Improvement, Outcome::NoDifference)),
        (
            "memory".into(),
            (Outcome::Improvement, Outcome::NoDifference),
        ),
    ]);
    let c = compose(APP, &[wine, memory], &singles, Limits::default());
    assert!(c
        .packs
        .iter()
        .any(|p| p.kind == PackKind::Combination && p.goal == Goal::Performance));
    assert!(c
        .packs
        .iter()
        .filter(|p| p.kind == PackKind::Combination)
        .all(|p| !p.performance_warnings.is_empty()));
}

#[test]
fn old_environment_evidence_is_retained_but_not_reused_for_current_machine() {
    let mut samples = vec![];
    for n in 0..2 {
        samples.push(game_fixture(APP, &format!("env-base-{n}"), &[], 100.0));
        samples.push(game_fixture(APP, &format!("env-pack-{n}"), &d3d11(), 125.0));
    }
    for s in &mut samples {
        s.conditions.driver = Some("fixture-driver-1".into());
        s.conditions.proton = Some("fixture-proton-1".into());
    }
    let current = crate::performance::models::Conditions {
        driver: Some("fixture-driver-2".into()),
        proton: Some("fixture-proton-1".into()),
        ..Default::default()
    };
    let grouped = super::evidence::group_in_context(
        APP,
        &samples,
        |m| m.smart_pack.clone(),
        &baseline_id(APP),
        Some(&current),
    );
    assert!(grouped.reference.is_none());
    assert_eq!(
        grouped
            .measurements
            .values()
            .map(|m| m.sessions.len())
            .sum::<usize>(),
        4
    );
    assert!(grouped.measurements.values().all(|m| m.used() == 0));
    assert!(grouped
        .measurements
        .values()
        .flat_map(|m| &m.sessions)
        .all(|s| s.excluded.as_ref().unwrap().contains("ambiente atual")));
    // All historic measurements still exist for their original context.
    let historical = group(APP, &samples, |m| m.smart_pack.clone(), &baseline_id(APP));
    assert_eq!(
        historical
            .measurements
            .values()
            .map(|m| m.used())
            .sum::<usize>(),
        4
    );
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
fn recorded_recipe_hash_is_revalidated_without_inventing_a_legacy_hash() {
    use crate::fix_engine::{catalog::Catalog, models::GameCtx, FixEngine};
    let tmp = tempfile::tempdir().unwrap();
    let engine = FixEngine::with(
        tmp.path().join("data"),
        Catalog::bundled(),
        tmp.path().join("proc"),
    );
    let game = GameCtx {
        app_id: APP.into(),
        name: "Synthetic fixture".into(),
        ..Default::default()
    };
    let recipe = engine.resolve("frx.compat.wined3d", &game).unwrap();
    let mut snapshot = super::benchmark::declared(APP, env(&[("PROTON_USE_WINED3D", "1")]));
    snapshot.recipes = vec![SnapshotRecipe {
        recipe_id: recipe.id.clone(),
        version: recipe.version.clone(),
        fingerprint: Some(super::discovery::fingerprint(&recipe)),
    }];
    snapshot.fingerprint = Some(super::benchmark::snapshot_fingerprint(
        APP,
        &snapshot.env,
        &snapshot.recipes,
    ));
    assert!(super::benchmark::recipe_context_matches(
        &engine, &game, &snapshot
    ));
    snapshot.recipes[0].fingerprint = Some("outdated-recipe-content".into());
    snapshot.fingerprint = Some(super::benchmark::snapshot_fingerprint(
        APP,
        &snapshot.env,
        &snapshot.recipes,
    ));
    assert!(!super::benchmark::recipe_context_matches(
        &engine, &game, &snapshot
    ));
    snapshot.recipes[0].fingerprint = None;
    snapshot.fingerprint = None;
    assert!(super::benchmark::recipe_context_matches(
        &engine, &game, &snapshot
    ));
    assert!(
        snapshot.recipes[0].fingerprint.is_none(),
        "o conteúdo atual não é atribuído retrospectivamente"
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

#[test]
fn unknown_historic_hardware_runtime_or_build_is_not_assumed_to_match_current_environment() {
    let mut s = game_fixture(APP, "missing-critical-context", &d3d11(), 130.0);
    s.conditions = Default::default();
    let current = crate::performance::models::Conditions {
        gpu: Some("fixture-gpu".into()),
        driver: Some("fixture-driver".into()),
        proton: Some("fixture-proton".into()),
        cpu: Some("fixture-cpu".into()),
        game_build_id: Some("fixture-build".into()),
        ..Default::default()
    };
    let g = super::evidence::group_in_context(
        APP,
        &[s.clone()],
        |m| m.smart_pack.clone(),
        &baseline_id(APP),
        Some(&current),
    );
    let sample = &g.measurements[&pack_id(APP, &d3d11())];
    assert_eq!(sample.used(), 0);
    let why = sample.sessions[0].excluded.as_deref().unwrap();
    for missing in ["GPU", "driver", "Proton", "CPU", "build do jogo"] {
        assert!(why.contains(missing));
    }
    let history = group(
        APP,
        &[s.clone()],
        |m| m.smart_pack.clone(),
        &baseline_id(APP),
    );
    assert_eq!(history.measurements[&pack_id(APP, &d3d11())].used(), 1);
    s.conditions = current.clone();
    let g = super::evidence::group_in_context(
        APP,
        &[s.clone()],
        |m| m.smart_pack.clone(),
        &baseline_id(APP),
        Some(&current),
    );
    assert_eq!(g.measurements[&pack_id(APP, &d3d11())].used(), 1);
    s.conditions.game_build_id = Some("older-fixture-build".into());
    let g = super::evidence::group_in_context(
        APP,
        &[s],
        |m| m.smart_pack.clone(),
        &baseline_id(APP),
        Some(&current),
    );
    assert_eq!(g.measurements[&pack_id(APP, &d3d11())].used(), 0);
}

#[test]
fn separate_contested_report_is_not_lost_when_a_positive_recipe_exists_for_the_same_actions() {
    use crate::fix_engine::{
        catalog::Catalog,
        dynamic::{
            analyzer::{assess, Ctx},
            tests::finding,
            AssessmentView,
        },
    };
    use crate::fixes::models::*;
    let key = "PROTON_NO_D3D11";
    let claim = |id: &str, positive: bool| {
        let mut f = finding(
            id,
            ActionKind::Env,
            &format!("{key}=1"),
            if positive { 2 } else { 0 },
            if positive { 0 } else { 2 },
        );
        f.sources.clear();
        for n in 0..if positive { 2 } else { 1 } {
            let url = format!(
                "https://github.com/ValveSoftware/Proton/issues/999999#issuecomment-{id}-{n}"
            );
            f.sources.push(FindingSource {
                provider: ProviderId::Github,
                label: "Synthetic fixture claim".into(),
                url: url.clone(),
                reference_id: None,
                snippet: None,
                published_at: None,
            });
            f.performance_evidence.reports.push(PerformanceReport {
                hardware: Default::default(),
                app_id: APP.into(),
                action_key: format!("env:{key}=1"),
                source_url: url,
                author: Some(format!("{id}-{n}")),
                reported_at: None,
                statement: "Synthetic fixture only".into(),
                outcome: if positive {
                    PerformanceOutcome::Positive
                } else {
                    PerformanceOutcome::Negative
                },
                proton_version: None,
                gpu_vendor: None,
                confidence: Confidence::CommunityReported,
                limitations: vec!["Synthetic fixture".into()],
            });
        }
        f
    };
    let positive = claim("synthetic-positive", true);
    let negative = claim("synthetic-negative", false);
    let mut foreign = negative.clone();
    foreign.id = "synthetic-foreign".into();
    foreign.game_app_id = "480".into();
    foreign.scope = DiscoveryScope::verified("480", "Synthetic foreign fixture");
    let mut unsafe_finding = negative.clone();
    unsafe_finding.id = "synthetic-unsafe".into();
    unsafe_finding.actions[0].kind = ActionKind::LaunchOption;
    unsafe_finding.actions[0].value = format!("{key}=1 curl https://example.invalid %command%");
    let catalog = Catalog::bundled();
    let ctx = Ctx {
        environment: None,
        game_build_id: None,
        app_id: APP,
        gpu: None,
        tool_name: Some("Proton Experimental"),
        catalog: &catalog,
        game: None,
    };
    let view = |f: &FixFinding| AssessmentView {
        availability: crate::fix_engine::availability::Availability::reason(
            crate::fix_engine::availability::AvailabilityState::Available,
            "synthetic",
            "Synthetic planner-approved fixture",
        ),
        assessment: assess(f, &ctx),
        generated_at: "fixture".into(),
        updated_at: "fixture".into(),
        discovered_at: None,
        reevaluated: None,
    };
    let pos_view = view(&positive);
    assert!(pos_view
        .assessment
        .performance_evidence
        .contextual_candidate(APP));
    let neg_view = view(&negative);
    assert_eq!(
        neg_view.assessment.status,
        crate::fix_engine::dynamic::DynStatus::Contested
    );
    assert!(neg_view.assessment.recipe.is_none());
    let unrelated = super::discovery::discover(
        APP,
        &[pos_view.clone(), view(&foreign), view(&unsafe_finding)],
        &[positive.clone(), foreign, unsafe_finding],
        |id| catalog.get(id).cloned(),
    );
    assert_eq!(unrelated.units.len(), 1);
    assert!(
        unrelated.units[0]
            .performance_evidence
            .contextual_candidate(APP),
        "unsafe procedures and other games cannot contaminate the valid action"
    );
    let result =
        super::discovery::discover(APP, &[pos_view, neg_view], &[positive, negative], |id| {
            catalog.get(id).cloned()
        });
    assert_eq!(result.units.len(), 1);
    assert!(result.units[0].performance_evidence.conflicting());
    assert!(!result.units[0]
        .performance_evidence
        .contextual_candidate(APP));
    assert!(result.units[0]
        .finding_ids
        .iter()
        .any(|id| id == "synthetic-negative"));
    let packs = compose(APP, &result.units, &SingleResults::new(), Limits::default()).packs;
    let r = rank(Input {
        profile: Profile::Performance,
        packs: &packs,
        baseline_id: &baseline_id(APP),
        measured: &BTreeMap::new(),
        vs: &HashMap::new(),
        excluded: &[],
    });
    assert!(r.entries[1].headline.contains("conflitantes"));
    assert!(r.winner.is_none() && r.next_test.is_none());
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
        let report = compose(
            APP,
            &[files, vars],
            &SingleResults::new(),
            Limits::default(),
        );
        assert_eq!(
            report.packs.len(),
            4,
            "baseline, file, variable and whole composition"
        );
        let file = report
            .packs
            .iter()
            .find(|p| p.kind == PackKind::Single && p.env.is_empty())
            .unwrap();
        assert_ne!(
            file.id,
            baseline_id(APP),
            "a file-only pack is never the baseline"
        );
        let mixed = report
            .packs
            .iter()
            .find(|p| p.kind == PackKind::Combination)
            .unwrap();
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
        let report = compose(
            APP,
            &[invalid, foreign],
            &SingleResults::new(),
            Limits::default(),
        );
        assert_eq!(report.packs.len(), 1);
        assert_eq!(report.excluded.len(), 2);
        assert!(report.excluded.iter().any(|e| e.reason.contains("SHA-256")));
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
        let report = compose(
            APP,
            &[dll, arbitrary],
            &SingleResults::new(),
            Limits::default(),
        );
        assert_eq!(report.packs.len(), 1);
        assert_eq!(report.excluded.len(), 2);
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
        let mut pack = compose(
            APP,
            std::slice::from_ref(&u),
            &SingleResults::new(),
            Limits::default(),
        )
        .packs
        .remove(1);
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
