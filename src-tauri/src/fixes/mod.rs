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
pub mod tweaks;

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

fn sort_findings(f: &mut [FixFinding]) {
    let kind_rank = |k: FindingKind| match k {
        FindingKind::Protonfix => 0,
        FindingKind::ReleaseNote => 1,
        FindingKind::Tweak => 2,
        FindingKind::Report => 3,
    };
    f.sort_by(|a, b| {
        (a.compatibility == Compatibility::Incompatible)
            .cmp(&(b.compatibility == Compatibility::Incompatible))
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
    for f in &mut findings {
        compat::assess(f, &hw);
    }
    sort_findings(&mut findings);

    let providers = vec![
        umu.status,
        db.status,
        gh.status,
        providers::protondb::status(),
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
        .map(|(_, e)| {
            let expired = !e.is_fresh(ttl_secs);
            let mut r = e.data;
            r.from_cache = true;
            r.stale = r.stale || expired;
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
        assert_eq!(r.providers.len(), 4);
        assert_eq!(r.providers[3].state, ProviderState::Disabled);
        assert_eq!(cached_reports(&cache, 3600).len(), 1);
        // Expiração: com validade zero, o relatório salvo aparece desatualizado.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert!(cached_reports(&cache, 0)[0].stale);
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
}
