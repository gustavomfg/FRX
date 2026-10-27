//! UMU-Protonfixes e UMU Database (Open Wine Components).
//!
//! Os scripts são baixados como **texto** do repositório público e passam
//! apenas pela análise estática de `pyscan`. Nenhum código é executado.

use std::path::Path;

use super::super::aggregation::finding_id;
use super::super::http::{Ctx, Origin, Outcome};
use super::super::models::*;
use super::super::pyscan;
use super::{summarize, GameQuery, ProviderResult};
use crate::net::{Fetcher, HttpRequest};
use crate::steam::models::ToolKind;

const REPO: &str = "Open-Wine-Components/umu-protonfixes";
const BRANCH: &str = "master";
const DB_URL: &str =
    "https://raw.githubusercontent.com/Open-Wine-Components/umu-database/main/umu-database.csv";
const MAX_SCRIPT: usize = 256 * 1024;
const MAX_CSV: usize = 4 * 1024 * 1024;
const DAY: u64 = 86_400;

pub fn raw_url(path: &str) -> String {
    format!("https://raw.githubusercontent.com/{REPO}/{BRANCH}/{path}")
}

pub fn blob_url(path: &str) -> String {
    format!("https://github.com/{REPO}/blob/{BRANCH}/{path}")
}

/// Um arquivo que é só um link simbólico do git chega como o caminho de destino.
fn symlink_target(base: &str, body: &str) -> Option<String> {
    let t = body.trim();
    if t.len() > 120
        || t.contains(char::is_whitespace)
        || !t.ends_with(".py")
        || !t.starts_with("../")
    {
        return None;
    }
    let dir = base.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let mut parts: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();
    for seg in t.split('/') {
        match seg {
            ".." => {
                parts.pop()?;
            }
            "." | "" => {}
            s if s
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) =>
            {
                parts.push(s)
            }
            _ => return None,
        }
    }
    let p = parts.join("/");
    p.starts_with("gamefixes-").then_some(p)
}

fn tidy_title(doc: &str) -> String {
    let d = doc.trim().trim_end_matches('.');
    let short: String = d.chars().take(90).collect();
    if d.chars().count() > 90 {
        format!("{short}…")
    } else {
        short
    }
}

pub async fn protonfixes<F: Fetcher>(ctx: &Ctx<'_, F>, q: &GameQuery) -> ProviderResult {
    let candidates = [
        format!("gamefixes-steam/{}.py", q.app_id),
        format!("gamefixes-umu/umu-{}.py", q.app_id),
    ];
    let mut outcomes: Vec<Outcome> = Vec::new();
    let mut files: Vec<(String, String, String)> = Vec::new(); // (AppID mapping path, resolved path, content)
    for path in &candidates {
        let o = ctx
            .get(
                HttpRequest::new(raw_url(path), "text/plain", MAX_SCRIPT),
                None,
                ctx.ttl_secs,
            )
            .await;
        let body = o.body.clone();
        outcomes.push(o);
        let Some(body) = body else { continue };
        let (resolved, content) = match symlink_target(path, &body) {
            Some(target) => {
                let o2 = ctx
                    .get(
                        HttpRequest::new(raw_url(&target), "text/plain", MAX_SCRIPT),
                        None,
                        ctx.ttl_secs,
                    )
                    .await;
                let c = o2.body.clone();
                outcomes.push(o2);
                match c {
                    Some(c) => (target, c),
                    None => continue,
                }
            }
            None => (path.clone(), body),
        };
        if !files.iter().any(|(_, p, _)| *p == resolved) {
            files.push((path.clone(), resolved, content));
        }
    }

    let mut findings = Vec::new();
    let mut artifact_documents = Vec::new();
    for (mapping, path, content) in files {
        // Data do último commit do arquivo (opcional; consome cota da API).
        let commits_url =
            format!("https://api.github.com/repos/{REPO}/commits?path={path}&per_page=1");
        let c = ctx
            .get(
                HttpRequest::new(commits_url, "application/vnd.github+json", 256 * 1024),
                Some("core"),
                ctx.ttl_secs.max(7 * DAY),
            )
            .await;
        let published = c.body.as_deref().and_then(last_commit_date);
        let mut finding = build_finding(q, &path, &content, published);
        finding.scope = DiscoveryScope::verified(
            &q.app_id,
            format!("UMU-Protonfixes: vínculo exato por AppID em {mapping}; destino {path}"),
        );
        if mapping != path {
            finding.sources.insert(
                0,
                FindingSource {
                    provider: ProviderId::UmuProtonfixes,
                    label: format!("umu-protonfixes · vínculo por AppID em {mapping}"),
                    url: blob_url(&mapping),
                    reference_id: Some(mapping),
                    snippet: Some(format!("Git symlink → {path}")),
                    published_at: None,
                },
            );
        }
        if let Some(source) = finding.sources.first() {
            artifact_documents.push(super::artifacts::Document {
                source: source.clone(),
                scope: finding.scope.clone(),
                text: content.clone(),
            });
        }
        findings.push(finding);
    }
    let (artifact_findings, artifact_outcomes) =
        super::artifacts::discover(ctx, q, &artifact_documents).await;
    findings.extend(artifact_findings);
    outcomes.extend(artifact_outcomes);
    let refs: Vec<&Outcome> = outcomes.iter().collect();
    ProviderResult {
        status: summarize(
            ProviderId::UmuProtonfixes,
            "UMU-Protonfixes",
            &refs,
            findings.len(),
        ),
        findings,
        identity: None,
    }
}

fn last_commit_date(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let d = v.get(0)?.pointer("/commit/committer/date")?.as_str()?;
    (d.len() <= 40).then(|| d.to_owned())
}

pub fn build_finding(
    q: &GameQuery,
    path: &str,
    content: &str,
    published: Option<String>,
) -> FixFinding {
    let a = pyscan::analyze(content);
    let state = a.state.unwrap_or(AnalysisState::NotVerified);
    let docs: Vec<&String> = a
        .func_docs
        .iter()
        .filter(|d| !d.starts_with("http"))
        .collect();
    let module = a.module_doc.clone().filter(|m| {
        !m.to_ascii_lowercase().starts_with("game fix for") && !m.eq_ignore_ascii_case(&q.name)
    });
    let title = docs
        .first()
        .map(|d| tidy_title(d))
        .or_else(|| module.as_deref().map(tidy_title))
        .unwrap_or_else(|| "Correção automática do UMU-Protonfixes".into());

    let mut description = String::new();
    for d in docs
        .iter()
        .skip(1)
        .map(|s| s.as_str())
        .chain(module.as_deref())
    {
        description.push_str(d.trim_end_matches('.'));
        description.push_str(". ");
    }
    match state {
        AnalysisState::NotVerified => description.push_str(
            "Não foi possível interpretar o script com segurança; consulte o arquivo original.",
        ),
        _ => {
            let n = a.actions.len();
            description.push_str(&format!(
                "Script do UMU-Protonfixes para este AppID com {n} aç{} identificada{} por análise estática",
                if n == 1 { "ão" } else { "ões" },
                if n == 1 { "" } else { "s" }
            ));
            if a.actions.iter().any(|x| x.conditional) {
                description.push_str(" (algumas só ocorrem sob condições)");
            }
            description.push('.');
        }
    }
    if a.downloads {
        description.push_str(" O script baixa arquivos externos quando executado pelo Proton.");
    }
    let external: Vec<&String> = a
        .urls
        .iter()
        .filter(|u| !u.starts_with("https://github.com/"))
        .take(3)
        .collect();
    if !external.is_empty() {
        description.push_str(&format!(
            " Referências citadas no script: {}.",
            external
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let mut sources = vec![FindingSource {
        provider: ProviderId::UmuProtonfixes,
        label: format!("umu-protonfixes · {path}"),
        url: blob_url(path),
        reference_id: Some(path.to_owned()),
        snippet: a
            .actions
            .iter()
            .take(3)
            .map(|x| x.value.clone())
            .reduce(|acc, v| format!("{acc} · {v}")),
        published_at: published.clone(),
    }];
    for u in a
        .urls
        .iter()
        .filter(|u| u.starts_with("https://github.com/"))
        .take(3)
    {
        sources.push(FindingSource {
            provider: ProviderId::Github,
            label: "Referência citada no script".into(),
            url: u.clone(),
            reference_id: None,
            snippet: None,
            published_at: None,
        });
    }

    let (compatibility, note) = bundled_state(q, path);
    FixFinding {
        artifacts: vec![],
        scope: if path == format!("gamefixes-steam/{}.py", q.app_id)
            || path == format!("gamefixes-umu/umu-{}.py", q.app_id)
        {
            DiscoveryScope::verified(
                &q.app_id,
                format!("UMU-Protonfixes: arquivo específico do AppID em {path}"),
            )
        } else {
            DiscoveryScope::default()
        },
        performance_evidence: PerformanceEvidence::default(),
        id: finding_id("umu", &q.app_id, path),
        game_app_id: q.app_id.clone(),
        title,
        description,
        category: a.category.unwrap_or(Category::Compatibility),
        kind: FindingKind::Protonfix,
        actions: a.actions,
        sources,
        requirements: Requirements {
            hardware: Default::default(),
            gpu_vendor: a.gpu,
            proton_versions: vec![],
            game_version: None,
        },
        // Repositório mantido e revisado ≠ testado no seu sistema.
        confidence: if state == AnalysisState::NotVerified {
            Confidence::Unverified
        } else {
            Confidence::Curated
        },
        compatibility,
        compatibility_note: note,
        experimental: false,
        analysis: state,
        evidence: Evidence::default(),
        published_at: published,
        retrieved_at: q.retrieved_at.clone(),
        keys: a.keys,
    }
}

/// A ferramenta do jogo já traz este arquivo? Então a correção já é aplicada sozinha.
fn bundled_state(q: &GameQuery, path: &str) -> (Compatibility, String) {
    let Some(tool) = &q.tool else {
        return (Compatibility::Unknown, "Ferramenta de compatibilidade do jogo não identificada. GE-Proton e derivados aplicam o UMU-Protonfixes automaticamente.".into());
    };
    let name = crate::fixes::short_tool_name(&tool.display_name);
    if tool.includes_protonfixes {
        let included = tool
            .install_path
            .as_deref()
            .is_some_and(|ip| Path::new(ip).join("protonfixes").join(path).is_file());
        if included {
            return (Compatibility::AlreadyIncluded, format!("{name} já inclui esta correção e a aplica automaticamente ao iniciar o jogo. Não é preciso fazer nada."));
        }
        return (Compatibility::Unknown, format!("{name} usa UMU-Protonfixes, mas a cópia instalada ainda não tem este arquivo — uma versão mais nova da ferramenta pode incluí-lo."));
    }
    let official = matches!(
        tool.kind,
        ToolKind::Official | ToolKind::Experimental | ToolKind::Hotfix
    );
    let note = if official {
        format!("{name} (Proton oficial) não executa o UMU-Protonfixes. GE-Proton, proton-cachyos e o umu-launcher aplicam esta correção automaticamente.")
    } else {
        format!("Não foi possível confirmar se {name} executa o UMU-Protonfixes.")
    };
    (Compatibility::Unknown, note)
}

/// Leitor de CSV (RFC 4180): aspas, aspas escapadas e quebras de linha em campos.
pub fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        match c {
            '"' if field.is_empty() => quoted = true,
            ',' => row.push(std::mem::take(&mut field)),
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            _ => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

pub async fn database<F: Fetcher>(ctx: &Ctx<'_, F>, q: &GameQuery) -> ProviderResult {
    let o = ctx
        .get(
            HttpRequest::new(DB_URL, "text/csv", MAX_CSV),
            None,
            ctx.ttl_secs.max(3 * DAY),
        )
        .await;
    let umu_id = format!("umu-{}", q.app_id);
    let mut identity = Identity::default();
    if let Some(body) = &o.body {
        let rows = parse_csv(body);
        let header = rows.first().cloned().unwrap_or_default();
        let col = |name: &str| {
            header
                .iter()
                .position(|h| h.trim().eq_ignore_ascii_case(name))
        };
        if let (Some(t), Some(s), Some(u)) = (col("TITLE"), col("STORE"), col("UMU_ID")) {
            for r in rows
                .iter()
                .skip(1)
                .filter(|r| r.get(u).is_some_and(|v| v.trim() == umu_id))
            {
                identity.umu_id = Some(umu_id.clone());
                let alias = format!(
                    "{} ({})",
                    r.get(t).map(|s| s.trim()).unwrap_or(""),
                    r.get(s).map(|s| s.trim()).unwrap_or("")
                );
                if !identity.aliases.contains(&alias) && identity.aliases.len() < 8 {
                    identity.aliases.push(alias);
                }
            }
        }
    }
    let found = identity.umu_id.is_some() as usize;
    let mut status = summarize(ProviderId::UmuDatabase, "UMU Database", &[&o], found);
    if found == 0 && matches!(o.origin, Origin::Network | Origin::Cache) {
        status.message =
            Some("Sem registro para este AppID — jogos da Steam usam o UMU ID padrão.".into());
    }
    ProviderResult {
        status,
        findings: vec![],
        identity: Some(identity),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;
    use crate::fixes::http::EngineState;
    use crate::net::mock::MockFetcher;
    use crate::steam::models::{CompatTool, ToolOrigin};

    fn q(tool: Option<CompatTool>) -> GameQuery {
        GameQuery {
            app_id: "1112400".into(),
            name: "Project Torque".into(),
            tool,
            retrieved_at: "2026-10-24T00:00:00Z".into(),
        }
    }

    const SCRIPT: &str = "\"\"\"Game fix for Project Torque\"\"\"\nfrom protonfixes import util\n\ndef main() -> None:\n    \"\"\"Game needs dotnet35 to launch\"\"\"\n    util.protontricks('dotnet35sp1')\n    util.set_environment('PROTON_USE_XALIA', '0')\n";

    #[test]
    fn resolves_git_symlinks_safely() {
        assert_eq!(
            symlink_target("gamefixes-umu/umu-1.py", "../gamefixes-steam/1.py\n").as_deref(),
            Some("gamefixes-steam/1.py")
        );
        assert_eq!(
            symlink_target("gamefixes-umu/umu-1.py", "../../etc/passwd.py"),
            None
        );
        assert_eq!(
            symlink_target("gamefixes-umu/umu-1.py", "import os\nprint(1).py"),
            None
        );
    }

    #[test]
    fn parses_quoted_csv() {
        let rows =
            parse_csv("TITLE,STORE,UMU_ID\n\"Batman™, City\",egs,umu-200260\nX,gog,\"umu-1\"\n");
        assert_eq!(rows[1], vec!["Batman™, City", "egs", "umu-200260"]);
        assert_eq!(rows[2][2], "umu-1");
    }

    #[tokio::test]
    async fn finds_fix_via_umu_symlink_and_marks_bundled_tool() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path().join("c"));
        let st = EngineState::default();
        let f = MockFetcher::default();
        f.on(
            &raw_url("gamefixes-umu/umu-1112400.py"),
            200,
            "../gamefixes-steam/1112400.py",
        );
        f.on(&raw_url("gamefixes-steam/1112400.py"), 200, SCRIPT);
        f.on(
            "https://api.github.com/repos/Open-Wine-Components/umu-protonfixes/commits?path=gamefixes-steam/1112400.py&per_page=1",
            200,
            r#"[{"commit":{"committer":{"date":"2025-01-02T03:04:05Z"}}}]"#,
        );
        // Ferramenta com o mesmo arquivo embutido.
        let tool_dir = tmp.path().join("GE-Proton");
        std::fs::create_dir_all(tool_dir.join("protonfixes/gamefixes-steam")).unwrap();
        std::fs::write(
            tool_dir.join("protonfixes/gamefixes-steam/1112400.py"),
            SCRIPT,
        )
        .unwrap();
        let tool = CompatTool {
            internal_name: Some("GE-Proton10-1".into()),
            display_name: "GE-Proton10-1".into(),
            kind: ToolKind::GeProton,
            version: None,
            install_path: Some(tool_dir.to_string_lossy().into_owned()),
            includes_protonfixes: true,
            origin: ToolOrigin::CompatibilityToolsD,
        };
        let ctx = Ctx {
            fetcher: &f,
            cache: &cache,
            state: &st,
            ttl_secs: 3600,
            force: false,
        };
        let r = protonfixes(&ctx, &q(Some(tool))).await;
        assert_eq!(
            r.findings.len(),
            1,
            "steam e umu apontam para o mesmo arquivo"
        );
        let fx = &r.findings[0];
        assert_eq!(fx.title, "Game needs dotnet35 to launch");
        assert_eq!(fx.confidence, Confidence::Curated);
        assert_eq!(fx.compatibility, Compatibility::AlreadyIncluded);
        assert_eq!(fx.published_at.as_deref(), Some("2025-01-02T03:04:05Z"));
        assert_eq!(fx.sources[0].url, "https://github.com/Open-Wine-Components/umu-protonfixes/blob/master/gamefixes-steam/1112400.py");
        assert_eq!(r.status.state, ProviderState::Ok);
    }

    #[tokio::test]
    async fn no_fix_is_no_data_and_offline_is_unavailable() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let st = EngineState::default();
        let f = MockFetcher::default();
        let ctx = Ctx {
            fetcher: &f,
            cache: &cache,
            state: &st,
            ttl_secs: 3600,
            force: false,
        };
        let r = protonfixes(&ctx, &q(None)).await;
        assert!(r.findings.is_empty());
        assert_eq!(r.status.state, ProviderState::NoData);

        let f2 = MockFetcher::default();
        f2.fail(
            &raw_url("gamefixes-steam/1112400.py"),
            crate::net::NetError::Offline("sem rede".into()),
        );
        f2.fail(
            &raw_url("gamefixes-umu/umu-1112400.py"),
            crate::net::NetError::Offline("sem rede".into()),
        );
        let cache2 = Cache::new(tmp.path().join("2"));
        let ctx2 = Ctx {
            fetcher: &f2,
            cache: &cache2,
            state: &st,
            ttl_secs: 3600,
            force: false,
        };
        let r = protonfixes(&ctx2, &q(None)).await;
        assert_eq!(r.status.state, ProviderState::Unavailable);
    }

    #[tokio::test]
    async fn database_resolves_aliases() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let st = EngineState::default();
        let f = MockFetcher::default();
        f.on(DB_URL, 200, "TITLE,STORE,CODENAME,UMU_ID\nProject Torque,egs,Torque,umu-1112400\nOutro,gog,1,umu-9\n");
        let ctx = Ctx {
            fetcher: &f,
            cache: &cache,
            state: &st,
            ttl_secs: 3600,
            force: false,
        };
        let r = database(&ctx, &q(None)).await;
        let id = r.identity.unwrap();
        assert_eq!(id.umu_id.as_deref(), Some("umu-1112400"));
        assert_eq!(id.aliases, vec!["Project Torque (egs)"]);
    }
}
