//! Descoberta de correções — **somente leitura**.
//!
//! Consulta os provedores em paralelo, agrupa descobertas que citam o mesmo
//! ajuste, avalia a relação com o hardware e guarda o relatório em cache.
//! Nada aqui aplica correções, executa scripts ou altera arquivos do sistema.

pub mod aggregation;
pub mod compat;
pub mod http;
pub mod models;
pub mod providers;
pub mod pyscan;
pub mod reports;
pub mod tweaks;

use std::collections::HashMap;

use crate::cache::{Cache, Entry};
use crate::net::Fetcher;
use crate::steam::models::CompatTool;
use crate::time::now_iso;
use compat::HardwareContext;
pub use http::EngineState;
use models::*;
use providers::GameQuery;

pub struct SearchRequest {
    pub app_id: String,
    pub name: String,
    pub force: bool,
    pub ttl_secs: u64,
}

/// "proton-cachyos-11.0-20261015 (steam linux runtime)" → "proton-cachyos-11.0-20261015".
pub fn short_tool_name(display: &str) -> String {
    display.split(" (").next().unwrap_or(display).to_owned()
}

pub fn valid_app_id(id: &str) -> bool {
    crate::art::valid_app_id(id)
}

/// Pertinência calculada por `ai::ranker`, com o mesmo embedder lexical usado sem
/// modelo. Não há sintoma nesta busca, então esse termo do score fica zerado. Só
/// ordena: nunca muda a confiança da descoberta.
fn relevance_by_finding(
    findings: &[FixFinding],
    query: &crate::ai::models::ResearchQuery,
) -> HashMap<String, f32> {
    use crate::ai::models::{Candidate, SourceKind};
    let now = crate::time::now_unix();
    findings
        .iter()
        .map(|f| {
            let source = match (f.kind, f.sources.first().map(|s| s.provider)) {
                (FindingKind::Protonfix, _) => SourceKind::Umu,
                (FindingKind::ReleaseNote, _) => SourceKind::ReleaseNotes,
                (_, Some(ProviderId::Reddit)) => SourceKind::Reddit,
                _ => SourceKind::GithubComment,
            };
            let snippets: Vec<&str> = f
                .sources
                .iter()
                .filter_map(|s| s.snippet.as_deref())
                .collect();
            let candidate = Candidate {
                source,
                url: f.sources.first().map(|s| s.url.clone()).unwrap_or_default(),
                title: f.title.clone(),
                snippet: f.description.clone(),
                raw_text: snippets.join("\n"),
                published_at: f.published_at.clone(),
            };
            let score = crate::ai::ranker::rank(
                query,
                &[candidate],
                &crate::ai::embedder::LexicalEmbedder,
                now,
            )
            .ok()
            .and_then(|scored| scored.first().map(|s| s.relevance))
            .unwrap_or(0.0);
            (f.id.clone(), score)
        })
        .collect()
}

fn sort_findings(f: &mut [FixFinding], relevance: &HashMap<String, f32>) {
    let kind_rank = |k: FindingKind| match k {
        FindingKind::Protonfix => 0,
        FindingKind::ReleaseNote => 1,
        FindingKind::Tweak => 2,
        FindingKind::Report => 3,
    };
    // O que vale para o hardware do usuário vem antes do que depende de confirmação.
    let hardware_rank = |c: Compatibility| match c {
        Compatibility::Compatible => 0,
        Compatibility::AlreadyIncluded | Compatibility::Unknown => 1,
        Compatibility::Incompatible => 2,
    };
    f.sort_by(|a, b| {
        hardware_rank(a.compatibility)
            .cmp(&hardware_rank(b.compatibility))
            .then_with(|| {
                let score = |x: &FixFinding| relevance.get(&x.id).copied().unwrap_or(0.0);
                score(b).total_cmp(&score(a))
            })
            .then(a.confidence.cmp(&b.confidence))
            .then(kind_rank(a.kind).cmp(&kind_rank(b.kind)))
            .then(b.evidence.positive.cmp(&a.evidence.positive))
    });
}

pub async fn search<F: Fetcher>(
    fetcher: &F,
    cache: &Cache,
    state: &EngineState,
    req: SearchRequest,
    tool: Option<CompatTool>,
    hw: HardwareContext,
) -> Result<FixReport, String> {
    if !valid_app_id(&req.app_id) {
        return Err("AppID inválido".into());
    }
    let name: String = req
        .name
        .chars()
        .filter(|c| !c.is_control())
        .take(200)
        .collect();
    let q = GameQuery {
        app_id: req.app_id.clone(),
        name: name.clone(),
        tool: tool.clone(),
        retrieved_at: now_iso(),
    };
    let ctx = http::Ctx {
        fetcher,
        cache,
        state,
        ttl_secs: req.ttl_secs,
        force: req.force,
    };

    let (umu, db, gh) = tokio::join!(
        providers::umu::protonfixes(&ctx, &q),
        providers::umu::database(&ctx, &q),
        providers::github::search(&ctx, &q)
    );

    let mut findings = aggregation::merge(umu.findings, gh.findings);
    findings.retain(|f| {
        f.scope.kind != DiscoveryScopeKind::VerifiedGame || f.scope.matches_game(&req.app_id)
    });
    for f in &mut findings {
        compat::assess(f, &hw);
    }
    let research = crate::ai::models::ResearchQuery {
        symptom: String::new(),
        appid: req.app_id.parse().unwrap_or(0),
        system: crate::ai::models::SystemSnapshot {
            gpus: hw.gpus.clone(),
            primary_gpu: hw.primary.clone(),
            vulkan_drivers: vec![],
            mesa_version: None,
        },
        game: crate::ai::models::GameInfo {
            name: name.clone(),
            proton_tool: tool.as_ref().map(|t| short_tool_name(&t.display_name)),
        },
    };
    let relevance = relevance_by_finding(&findings, &research);
    sort_findings(&mut findings, &relevance);

    let providers = vec![
        umu.status,
        db.status,
        gh.status,
        providers::protondb::status(),
        providers::reddit::status(),
        providers::winehq::status(),
        providers::steam_guides::status(),
    ];
    let from_cache = providers
        .iter()
        .any(|p| matches!(p.state, ProviderState::Cached | ProviderState::Stale));
    let stale = providers.iter().any(|p| p.state == ProviderState::Stale);
    let report = FixReport {
        app_id: req.app_id.clone(),
        game_name: name,
        findings,
        providers,
        identity: db.identity.unwrap_or_default(),
        context: ReportContext {
            gpu_vendor: hw.primary_req(),
            tool_name: tool.as_ref().map(|t| short_tool_name(&t.display_name)),
            tool_includes_protonfixes: tool.as_ref().is_some_and(|t| t.includes_protonfixes),
        },
        retrieved_at: q.retrieved_at.clone(),
        from_cache,
        stale,
    };
    let _ = cache.put("fixes", &req.app_id, &Entry::new(report.clone(), None));
    Ok(report)
}

/// Relatórios salvos, com o selo de desatualizado recalculado pela validade atual.
pub fn cached_reports(cache: &Cache, ttl_secs: u64) -> Vec<FixReport> {
    let mut out: Vec<FixReport> = cache
        .list::<FixReport>("fixes")
        .into_iter()
        .filter(|(key, e)| valid_app_id(key) && e.data.app_id == *key)
        .map(|(_, e)| {
            let expired = !e.is_fresh(ttl_secs);
            let mut r = e.data;
            r.from_cache = true;
            r.stale = r.stale
                || expired
                || r.findings.iter().any(|f| {
                    f.scope.kind == DiscoveryScopeKind::UnknownScope
                        || (f.scope.kind == DiscoveryScopeKind::VerifiedGame
                            && (!f.scope.matches_game(&r.app_id) || f.game_app_id != r.app_id))
                });
            // Return a safe view. Keep the original cache file intact for audit;
            // an entry claiming a different game is never shown for this report.
            r.findings.retain(|f| {
                (f.game_app_id.is_empty() || f.game_app_id == r.app_id)
                    && (f.scope.kind != DiscoveryScopeKind::VerifiedGame
                        || f.scope.matches_game(&r.app_id))
            });
            r
        })
        .collect();
    out.sort_by(|a, b| b.retrieved_at.cmp(&a.retrieved_at));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::mock::MockFetcher;

    #[tokio::test]
    async fn full_search_offline_without_cache_still_returns_a_report() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let st = EngineState::default();
        let f = MockFetcher::default(); // tudo 404 = nada encontrado
        let r = search(
            &f,
            &cache,
            &st,
            SearchRequest {
                app_id: "620".into(),
                name: "Portal 2".into(),
                force: false,
                ttl_secs: 3600,
            },
            None,
            HardwareContext::default(),
        )
        .await
        .unwrap();
        assert!(r.findings.is_empty());
        assert_eq!(r.providers.len(), 7);
        assert!(r.providers[3..]
            .iter()
            .all(|p| p.state == ProviderState::Disabled));
        // Desativado não consulta a rede.
        assert_eq!(f.count("https://www.reddit.com/"), 0);
        assert_eq!(f.count("https://bugs.winehq.org/"), 0);
        assert_eq!(cached_reports(&cache, 3600).len(), 1);
        // Expiração: com validade zero, o relatório salvo aparece desatualizado.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert!(cached_reports(&cache, 0)[0].stale);
    }

    #[test]
    fn cached_reports_preserve_legacy_data_without_assigning_scope_or_mixing_keys() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let query = GameQuery {
            app_id: "200260".into(),
            name: "Synthetic fixture A".into(),
            retrieved_at: "fixture".into(),
            tool: None,
        };
        let finding = providers::umu::build_finding(
            &query,
            "gamefixes-steam/200260.py",
            r#"def main():
    util.set_environment("PROTON_NO_D3D11", "1")"#,
            None,
        );
        assert!(finding.scope.matches_game("200260"));
        let report = FixReport {
            app_id: query.app_id.clone(),
            game_name: query.name.clone(),
            findings: vec![finding],
            providers: vec![],
            identity: Identity::default(),
            context: ReportContext::default(),
            retrieved_at: "fixture".into(),
            from_cache: false,
            stale: false,
        };
        let mut legacy = serde_json::to_value(&report).unwrap();
        legacy["findings"][0]
            .as_object_mut()
            .unwrap()
            .remove("scope");
        legacy["findings"][0]
            .as_object_mut()
            .unwrap()
            .remove("performanceEvidence");
        cache
            .put("fixes", "200260", &Entry::new(legacy, None))
            .unwrap();
        let before = std::fs::read(cache.ns_dir("fixes").join("200260.json")).unwrap();
        let loaded = cached_reports(&cache, 3600);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].findings.len(), 1);
        assert_eq!(
            loaded[0].findings[0].scope.kind,
            DiscoveryScopeKind::UnknownScope
        );
        assert_eq!(
            loaded[0].findings[0].performance_evidence,
            PerformanceEvidence::default()
        );
        assert!(loaded[0].stale, "legacy source attribution needs refresh");
        assert_eq!(
            before,
            std::fs::read(cache.ns_dir("fixes").join("200260.json")).unwrap()
        );
        cache
            .put("fixes", "570", &Entry::new(report.clone(), None))
            .unwrap();
        assert_eq!(
            cached_reports(&cache, 3600).len(),
            1,
            "file key must match embedded AppID"
        );
        let mut wrong_finding = report.clone();
        wrong_finding.findings[0].scope =
            DiscoveryScope::verified("570", "fixture: wrong-game source");
        cache
            .put("fixes", "200260", &Entry::new(wrong_finding, None))
            .unwrap();
        let loaded = cached_reports(&cache, 3600);
        assert!(loaded[0].findings.is_empty());
        assert!(
            cache
                .get::<FixReport>("fixes", "200260")
                .unwrap()
                .data
                .findings
                .len()
                == 1,
            "unsafe cached entries retained on disk for audit, not returned as game evidence"
        );
        let cache_after_restart = Cache::new(tmp.path());
        assert!(cached_reports(&cache_after_restart, 3600)[0]
            .findings
            .is_empty());
    }

    #[test]
    fn umu_variable_keeps_its_vendor_and_is_assessed_against_the_gpu() {
        let query = GameQuery {
            app_id: "200260".into(),
            name: "Synthetic fixture A".into(),
            retrieved_at: "fixture".into(),
            tool: None,
        };
        let mut finding = providers::umu::build_finding(
            &query,
            "gamefixes-steam/200260.py",
            r#"def main():
    util.set_environment("PROTON_ENABLE_NVAPI", "1")"#,
            None,
        );
        assert_eq!(finding.requirements.gpu_vendor, Some(GpuReq::Nvidia));
        let gpu = |vendor: &str| -> crate::system::GpuInfo {
            serde_json::from_value(serde_json::json!({
                "vendor": vendor, "vendorId": "0", "deviceId": "0", "name": "fixture",
                "driver": null, "driverVersion": null, "bootVga": true
            }))
            .unwrap()
        };
        for (vendor, expected) in [
            ("nvidia", Compatibility::Compatible),
            ("amd", Compatibility::Incompatible),
        ] {
            let hw = HardwareContext {
                gpus: vec![gpu(vendor)],
                primary: Some(gpu(vendor)),
            };
            compat::assess(&mut finding, &hw);
            assert_eq!(finding.compatibility, expected, "{vendor}");
        }
    }

    #[test]
    fn hardware_match_sorts_before_confidence() {
        let finding = |id: &str, compatibility, confidence| {
            let mut f = providers::umu::build_finding(
                &GameQuery {
                    app_id: "200260".into(),
                    name: "fixture".into(),
                    retrieved_at: "fixture".into(),
                    tool: None,
                },
                "gamefixes-steam/200260.py",
                "def main():\n    pass",
                None,
            );
            f.id = id.into();
            f.compatibility = compatibility;
            f.confidence = confidence;
            f
        };
        let mut list = vec![
            finding(
                "wrong-gpu",
                Compatibility::Incompatible,
                Confidence::Verified,
            ),
            finding("unknown", Compatibility::Unknown, Confidence::Verified),
            finding("match", Compatibility::Compatible, Confidence::Unverified),
        ];
        sort_findings(&mut list, &HashMap::new());
        let ids: Vec<&str> = list.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, ["match", "unknown", "wrong-gpu"]);
    }

    #[test]
    fn ranker_orders_findings_inside_the_same_hardware_tier() {
        let finding = |id: &str, snippet: &str| {
            let mut f = providers::umu::build_finding(
                &GameQuery {
                    app_id: "200260".into(),
                    name: "fixture".into(),
                    retrieved_at: "fixture".into(),
                    tool: None,
                },
                "gamefixes-steam/200260.py",
                "def main():\n    pass",
                None,
            );
            f.id = id.into();
            f.kind = FindingKind::Tweak;
            f.compatibility = Compatibility::Unknown;
            f.confidence = Confidence::Verified;
            f.published_at = None;
            f.sources[0].snippet = Some(snippet.into());
            f
        };
        let primary: crate::system::GpuInfo = serde_json::from_value(serde_json::json!({
            "vendor": "nvidia", "vendorId": "0", "deviceId": "0", "name": "NVIDIA GeForce RTX 4060",
            "driver": null, "driverVersion": null, "bootVga": true
        }))
        .unwrap();
        let query = crate::ai::models::ResearchQuery {
            symptom: String::new(),
            appid: 200260,
            system: crate::ai::models::SystemSnapshot {
                gpus: vec![],
                primary_gpu: Some(primary),
                vulkan_drivers: vec![],
                mesa_version: None,
            },
            game: Default::default(),
        };
        let mut list = vec![
            finding("radeon", "works on my Radeon RX 7800 XT"),
            finding("plain", "worked for me"),
            finding("rtx", "works on my RTX 4060"),
        ];
        let relevance = relevance_by_finding(&list, &query);
        sort_findings(&mut list, &relevance);
        let ids: Vec<&str> = list.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, ["rtx", "plain", "radeon"]);
    }

    #[tokio::test]
    async fn rejects_invalid_app_ids() {
        let tmp = tempfile::tempdir().unwrap();
        let f = MockFetcher::default();
        let r = search(
            &f,
            &Cache::new(tmp.path()),
            &EngineState::default(),
            SearchRequest {
                app_id: "../x".into(),
                name: "x".into(),
                force: false,
                ttl_secs: 1,
            },
            None,
            HardwareContext::default(),
        )
        .await;
        assert!(r.is_err());
    }
}

/// Diagnóstico manual com rede real:
/// `FRX_APPID=200260 FRX_NAME="Batman: Arkham City GOTY" cargo test real_fix_search -- --ignored --nocapture`
#[cfg(test)]
mod real {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn real_fix_search() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let fetcher = crate::net::ReqwestFetcher::new().unwrap();
        let app_id = std::env::var("FRX_APPID").unwrap_or_else(|_| "200260".into());
        let name = std::env::var("FRX_NAME").unwrap_or_else(|_| "Batman: Arkham City GOTY".into());
        let profile = crate::system::profile();
        let scan = crate::steam::scan(&crate::steam::discovery::DiscoveryEnv::from_process(), None);
        let tool = scan
            .games
            .iter()
            .find(|g| g.app_id == app_id)
            .and_then(|g| g.compatibility.tool.clone());
        let hw = HardwareContext {
            gpus: profile.gpus,
            primary: profile.primary_gpu,
        };
        let r = search(
            &fetcher,
            &cache,
            &EngineState::default(),
            SearchRequest {
                app_id,
                name,
                force: true,
                ttl_secs: 3600,
            },
            tool,
            hw,
        )
        .await
        .unwrap();
        println!("{}", serde_json::to_string_pretty(&r).unwrap());
    }

    /// Confere se os provedores desativados respondem na rede de quem roda:
    /// `FRX_NAME="Batman: Arkham City GOTY" cargo test real_disabled_sources -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn real_disabled_sources() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let fetcher = crate::net::ReqwestFetcher::new().unwrap();
        let state = EngineState::default();
        let ctx = http::Ctx {
            fetcher: &fetcher,
            cache: &cache,
            state: &state,
            ttl_secs: 3600,
            force: true,
        };
        let q = GameQuery {
            app_id: std::env::var("FRX_APPID").unwrap_or_else(|_| "200260".into()),
            name: std::env::var("FRX_NAME").unwrap_or_else(|_| "Batman: Arkham City GOTY".into()),
            tool: None,
            retrieved_at: now_iso(),
        };
        for r in [
            providers::reddit::search(&ctx, &q).await,
            providers::winehq::search(&ctx, &q).await,
        ] {
            println!("{:?}: {} descobertas", r.status, r.findings.len());
        }
    }
}
