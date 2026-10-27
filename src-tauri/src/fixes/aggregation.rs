//! Agrupamento de menções e deduplicação entre fontes.
//!
//! Regra: só agrupamos quando o **mesmo ajuste normalizado** aparece (mesma
//! variável com o mesmo valor, mesmo componente, mesma opção de inicialização).
//! Palavras parecidas não bastam. Toda origem continua rastreável em `sources`.

use std::collections::BTreeMap;

use super::models::*;
use super::pyscan::{category_for_env, category_for_verb, gpu_for_env, verb_label};
use super::tweaks::Tweak;

#[derive(Debug, Clone)]
pub struct Mention {
    pub scope: DiscoveryScope,
    pub performance_reports: Vec<PerformanceReport>,
    pub tweak: Tweak,
    pub source: FindingSource,
    pub positive: bool,
    pub negative: bool,
    pub gpus: Vec<GpuReq>,
    pub proton_versions: Vec<String>,
}

/// Chaves que descrevem um ajuste — inclusive os componentes de uma opção de inicialização.
pub fn component_keys(t: &Tweak) -> Vec<String> {
    match t {
        Tweak::Launch(l) => {
            let mut keys = Vec::new();
            let mut after = false;
            for tok in l.split_whitespace() {
                if tok == "%command%" {
                    after = true;
                    continue;
                }
                if !after {
                    if let Some((k, v)) = tok.split_once('=') {
                        keys.push(format!("env:{k}={v}"));
                    } else {
                        keys.push(format!("wrapper:{tok}"));
                    }
                } else {
                    keys.push(format!("arg:{tok}"));
                }
            }
            keys
        }
        other => vec![other.key()],
    }
}

fn short_hash(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

pub fn finding_id(provider: &str, app: &str, key: &str) -> String {
    format!("{provider}-{app}-{}", &short_hash(key)[..10])
}

/// Agrupa menções da comunidade por ajuste idêntico.
pub fn group_mentions(app_id: &str, mentions: Vec<Mention>, retrieved_at: &str) -> Vec<FixFinding> {
    let mut groups: BTreeMap<String, Vec<Mention>> = BTreeMap::new();
    for m in mentions {
        let identity = format!(
            "{:?}:{}:{}",
            m.scope.kind,
            m.scope.app_id.as_deref().unwrap_or(""),
            if m.scope.kind == DiscoveryScopeKind::UnknownScope {
                m.scope.basis.as_str()
            } else {
                ""
            }
        );
        groups
            .entry(format!("{identity}:{}", m.tweak.key()))
            .or_default()
            .push(m);
    }
    let mut out = Vec::new();
    for (key, mut ms) in groups {
        ms.sort_by(|a, b| b.source.published_at.cmp(&a.source.published_at));
        // Uma pessoa repetindo o ajuste no mesmo comentário conta uma vez.
        ms.dedup_by(|a, b| a.source.url == b.source.url);
        let tweak = ms[0].tweak.clone();
        let positive = ms.iter().filter(|m| m.positive && !m.negative).count() as u32;
        let negative = ms.iter().filter(|m| m.negative).count() as u32;

        let mut versions: Vec<String> = Vec::new();
        for m in &ms {
            for v in &m.proton_versions {
                if !versions.contains(v) && versions.len() < 5 {
                    versions.push(v.clone());
                }
            }
        }
        // Requisito de GPU: o nome da variável manda; senão, só se todos os relatos
        // favoráveis (pelo menos dois) citarem o mesmo fabricante e nenhum outro.
        let gpu = match &tweak {
            Tweak::Env { name, .. } => gpu_for_env(name),
            _ => None,
        }
        .or_else(|| {
            let favorable: Vec<&Mention> =
                ms.iter().filter(|m| m.positive && !m.negative).collect();
            let first = favorable.first()?.gpus.first().copied()?;
            (favorable.len() >= 2 && favorable.iter().all(|m| m.gpus == vec![first]))
                .then_some(first)
        });

        let (title, category, actions) = match &tweak {
            Tweak::Env { name, value } => (
                format!("Variável de ambiente {name}={value}"),
                category_for_env(name),
                vec![FixAction {
                    kind: ActionKind::Env,
                    label: "Define variável de ambiente".into(),
                    value: format!("{name}={value}"),
                    conditional: false,
                }],
            ),
            Tweak::Verb(v) => (
                format!("Componente {v} (winetricks)"),
                category_for_verb(v),
                vec![FixAction {
                    kind: ActionKind::Winetricks,
                    label: verb_label(v),
                    value: v.clone(),
                    conditional: false,
                }],
            ),
            Tweak::Launch(l) => {
                let env_cat = l
                    .split_whitespace()
                    .take_while(|t| *t != "%command%")
                    .find_map(|t| t.split_once('=').map(|(k, _)| category_for_env(k)));
                (
                    "Opção de inicialização sugerida".to_owned(),
                    env_cat.unwrap_or(Category::Compatibility),
                    vec![FixAction {
                        kind: ActionKind::LaunchOption,
                        label: "Opções de inicialização na Steam".into(),
                        value: l.clone(),
                        conditional: false,
                    }],
                )
            }
        };
        let distinct = ms.len();
        let confidence = if positive >= 2 && positive > negative {
            Confidence::CommunityReported
        } else {
            Confidence::Unverified
        };
        let mut description = format!(
            "Citado em {distinct} comentário{} público{} {}",
            if distinct == 1 { "" } else { "s" },
            if distinct == 1 { "" } else { "s" },
            if ms[0].scope.matches_game(app_id) {
                "sobre este jogo"
            } else {
                "sem atribuição confirmada ao jogo selecionado"
            }
        );
        if negative > 0 {
            description.push_str(&format!(", {negative} dizendo que não resolveu"));
        }
        description.push_str(". O FRX não testou este ajuste; ele é exibido como referência, não como recomendação garantida.");
        let published = ms.first().and_then(|m| m.source.published_at.clone());
        let scope = ms[0].scope.clone();
        let scoped_app = scope
            .app_id
            .as_deref()
            .filter(|_| scope.kind == DiscoveryScopeKind::VerifiedGame)
            .unwrap_or("")
            .to_owned();
        let mut performance_evidence = PerformanceEvidence::default();
        for report in ms.iter().flat_map(|m| &m.performance_reports) {
            if report.app_id != scoped_app
                || !scope.matches_game(&report.app_id)
                || performance_evidence.reports.iter().any(|r| {
                    r.source_url == report.source_url
                        && r.action_key == report.action_key
                        && r.outcome == report.outcome
                })
            {
                continue;
            }
            match report.outcome {
                PerformanceOutcome::Positive => performance_evidence.positive += 1,
                PerformanceOutcome::Negative => performance_evidence.negative += 1,
            }
            performance_evidence.reports.push(report.clone());
        }
        out.push(FixFinding {
            scope,
            performance_evidence,
            id: finding_id("github", app_id, &key),
            game_app_id: scoped_app,
            title,
            description,
            category,
            kind: FindingKind::Tweak,
            actions,
            // HTTP providers already bound the number and size of fetched
            // comments. Keep every fetched source so a later negative claim
            // cannot lose its provenance while positive claims remain valid.
            sources: ms.iter().map(|m| m.source.clone()).collect(),
            requirements: Requirements {
                gpu_vendor: gpu,
                proton_versions: versions,
                game_version: None,
            },
            experimental: confidence == Confidence::Unverified,
            confidence,
            compatibility: Compatibility::Unknown,
            compatibility_note: String::new(),
            analysis: AnalysisState::Parsed,
            evidence: Evidence { positive, negative },
            published_at: published,
            retrieved_at: retrieved_at.to_owned(),
            keys: component_keys(&tweak),
        });
    }
    out
}

/// Anexa a uma correção do UMU-Protonfixes os relatos que citam exatamente
/// os mesmos ajustes. Os demais continuam como descobertas independentes.
pub fn merge(mut primary: Vec<FixFinding>, others: Vec<FixFinding>) -> Vec<FixFinding> {
    let mut rest = Vec::new();
    for o in others {
        let target = primary.iter_mut().find(|p| {
            p.kind == FindingKind::Protonfix
                && p.game_app_id == o.game_app_id
                && p.scope.matches_game(&p.game_app_id)
                && o.scope.matches_game(&p.game_app_id)
                && !o.keys.is_empty()
                && o.keys.iter().all(|k| p.keys.contains(k))
        });
        match target {
            Some(p) => {
                for s in o.sources {
                    if !p.sources.iter().any(|x| x.url == s.url) {
                        p.sources.push(s);
                    }
                }
                p.evidence.positive += o.evidence.positive;
                p.evidence.negative += o.evidence.negative;
                for report in o.performance_evidence.reports {
                    if !p.performance_evidence.reports.iter().any(|r| {
                        r.source_url == report.source_url
                            && r.action_key == report.action_key
                            && r.outcome == report.outcome
                    }) {
                        match report.outcome {
                            PerformanceOutcome::Positive => p.performance_evidence.positive += 1,
                            PerformanceOutcome::Negative => p.performance_evidence.negative += 1,
                        }
                        p.performance_evidence.reports.push(report);
                    }
                }
                for v in o.requirements.proton_versions {
                    if !p.requirements.proton_versions.contains(&v) {
                        p.requirements.proton_versions.push(v);
                    }
                }
            }
            None => rest.push(o),
        }
    }
    primary.extend(rest);
    // Mesma URL de origem não pode gerar duas descobertas do mesmo tipo.
    let mut seen = Vec::new();
    primary.retain(|f| {
        let k = (
            f.id.clone(),
            f.game_app_id.clone(),
            f.scope.clone(),
            f.sources.first().map(|s| s.url.clone()).unwrap_or_default(),
        );
        if seen.contains(&k) {
            false
        } else {
            seen.push(k);
            true
        }
    });
    primary
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mention(t: Tweak, url: &str, pos: bool, neg: bool, gpus: Vec<GpuReq>) -> Mention {
        Mention {
            scope: DiscoveryScope::verified("1", "fixture: explicit AppID in issue title"),
            performance_reports: vec![],
            tweak: t,
            source: FindingSource {
                provider: ProviderId::Github,
                label: "c".into(),
                url: url.into(),
                reference_id: None,
                snippet: None,
                published_at: Some(url.into()),
            },
            positive: pos,
            negative: neg,
            gpus,
            proton_versions: vec![],
        }
    }

    fn env(n: &str, v: &str) -> Tweak {
        Tweak::Env {
            name: n.into(),
            value: v.into(),
        }
    }

    #[test]
    fn groups_identical_tweaks_and_counts_evidence() {
        let ms = vec![
            mention(
                env("PROTON_USE_WINED3D", "1"),
                "u1",
                true,
                false,
                vec![GpuReq::Amd],
            ),
            mention(
                env("PROTON_USE_WINED3D", "1"),
                "u2",
                true,
                false,
                vec![GpuReq::Amd],
            ),
            mention(env("PROTON_USE_WINED3D", "1"), "u2", true, false, vec![]),
            mention(env("PROTON_USE_WINED3D", "0"), "u3", true, false, vec![]),
            mention(Tweak::Verb("dotnet40".into()), "u4", false, true, vec![]),
        ];
        let out = group_mentions("1", ms, "now");
        assert_eq!(out.len(), 3, "valores diferentes não se fundem");
        let wined3d = out.iter().find(|f| f.title.ends_with("=1")).unwrap();
        assert_eq!(wined3d.sources.len(), 2, "mesmo comentário conta uma vez");
        assert_eq!(wined3d.confidence, Confidence::CommunityReported);
        assert_eq!(wined3d.requirements.gpu_vendor, Some(GpuReq::Amd));
        let dotnet = out.iter().find(|f| f.title.contains("dotnet40")).unwrap();
        assert_eq!(dotnet.confidence, Confidence::Unverified);
        assert!(dotnet.experimental);
        assert_eq!(dotnet.evidence.negative, 1);
    }

    #[test]
    fn single_or_disputed_reports_never_become_community_confirmed() {
        let ms = vec![
            mention(env("DXVK_ASYNC", "1"), "a", true, false, vec![]),
            mention(env("DXVK_ASYNC", "1"), "b", false, true, vec![]),
            mention(env("DXVK_ASYNC", "1"), "c", false, true, vec![]),
        ];
        let out = group_mentions("1", ms, "now");
        assert_eq!(out[0].confidence, Confidence::Unverified);
    }

    #[test]
    fn merges_community_mentions_into_matching_protonfix_only() {
        let mut pf = group_mentions(
            "1",
            vec![mention(Tweak::Verb("x".into()), "z", true, false, vec![])],
            "now",
        )
        .remove(0);
        pf.kind = FindingKind::Protonfix;
        pf.id = "umu".into();
        pf.keys = vec!["verb:dotnet40".into(), "env:PROTON_USE_XALIA=0".into()];
        let others = group_mentions(
            "1",
            vec![
                mention(Tweak::Verb("dotnet40".into()), "c1", true, false, vec![]),
                mention(
                    Tweak::Launch("PROTON_USE_XALIA=0 %command%".into()),
                    "c2",
                    true,
                    false,
                    vec![],
                ),
                mention(Tweak::Verb("dotnet48".into()), "c3", true, false, vec![]),
            ],
            "now",
        );
        let merged = merge(vec![pf], others);
        assert_eq!(merged.len(), 2, "{merged:#?}");
        let umu = merged.iter().find(|f| f.id == "umu").unwrap();
        assert_eq!(umu.sources.len(), 3);
        assert!(merged.iter().any(|f| f.title.contains("dotnet48")));
    }

    #[test]
    fn launch_option_components() {
        assert_eq!(
            component_keys(&Tweak::Launch(
                "gamemoderun PROTON_NO_FSYNC=1 %command% -dx11".into()
            )),
            vec!["wrapper:gamemoderun", "env:PROTON_NO_FSYNC=1", "arg:-dx11"]
        );
    }
    #[test]
    fn equal_actions_never_merge_across_appids_or_unknown_attribution() {
        let a = mention(env("PROTON_NO_D3D11", "1"), "a", true, false, vec![]);
        let mut b = mention(env("PROTON_NO_D3D11", "1"), "b", true, false, vec![]);
        b.scope = DiscoveryScope::verified("2", "fixture: game B title");
        let mut unknown = mention(env("PROTON_NO_D3D11", "1"), "u", true, false, vec![]);
        unknown.scope = DiscoveryScope::default();
        let mut groups = group_mentions("1", vec![a, b, unknown], "fixture");
        assert_eq!(groups.len(), 3);
        assert_ne!(groups[0].id, groups[1].id);
        assert!(groups
            .iter()
            .any(|f| f.game_app_id == "1" && f.sources.len() == 1));
        assert!(groups
            .iter()
            .any(|f| f.game_app_id == "2" && f.sources.len() == 1));
        assert!(groups
            .iter()
            .any(|f| f.game_app_id.is_empty() && f.sources.len() == 1));
        let mut primary = groups.remove(groups.iter().position(|f| f.game_app_id == "1").unwrap());
        primary.kind = FindingKind::Protonfix;
        let merged = merge(vec![primary], groups);
        assert_eq!(merged.len(), 3);
        assert!(merged
            .iter()
            .all(|f| f.sources.len() == 1 && f.evidence.positive == 1));
    }

    #[test]
    fn neutral_mentions_do_not_become_positive_reports() {
        let findings = group_mentions(
            "1",
            vec![mention(
                env("PROTON_NO_D3D11", "1"),
                "source",
                false,
                false,
                vec![],
            )],
            "fixture",
        );
        assert_eq!(findings[0].evidence.positive, 0);
        assert_eq!(
            findings[0].performance_evidence,
            PerformanceEvidence::default()
        );
    }
    #[test]
    fn all_fetched_sources_are_retained_including_a_seventh_negative_claim() {
        let mentions: Vec<_> = (0..7)
            .map(|i| {
                let negative = i == 6;
                let url =
                    format!("https://github.com/ValveSoftware/Proton/issues/1#issuecomment-{i}");
                let mut mention = mention(
                    env("PROTON_NO_D3D11", "1"),
                    &url,
                    !negative,
                    negative,
                    vec![],
                );
                mention.source.published_at = Some(format!("2026-10-{:02}T00:00:00Z", 27 - i));
                mention.performance_reports = vec![PerformanceReport {
                    app_id: "1".into(),
                    action_key: "env:PROTON_NO_D3D11=1".into(),
                    source_url: url,
                    author: Some(format!("synthetic-author-{i}")),
                    reported_at: mention.source.published_at.clone(),
                    statement: "Synthetic performance claim, not a real benchmark".into(),
                    outcome: if negative {
                        PerformanceOutcome::Negative
                    } else {
                        PerformanceOutcome::Positive
                    },
                    proton_version: None,
                    gpu_vendor: None,
                    confidence: Confidence::CommunityReported,
                    limitations: vec![],
                }];
                mention
            })
            .collect();
        let findings = group_mentions("1", mentions, "synthetic-fixture");
        let finding = &findings[0];
        assert_eq!(finding.sources.len(), 7);
        assert_eq!(finding.performance_evidence.positive, 6);
        assert_eq!(finding.performance_evidence.negative, 1);
        let negative = finding
            .performance_evidence
            .reports
            .iter()
            .find(|r| r.outcome == PerformanceOutcome::Negative)
            .unwrap();
        assert_eq!(
            finding.sources[6].url, negative.source_url,
            "oldest report is deliberately beyond the former six-source display bound"
        );
        assert!(
            finding
                .performance_evidence
                .reports
                .iter()
                .all(|r| finding.sources.iter().any(|s| s.url == r.source_url)),
            "every public performance claim retains its source for contextual revalidation"
        );
    }

    #[test]
    fn final_deduplication_preserves_equal_ids_and_urls_with_different_game_scopes() {
        let a = group_mentions(
            "1",
            vec![mention(
                env("PROTON_NO_D3D11", "1"),
                "same-source",
                true,
                false,
                vec![],
            )],
            "synthetic-fixture",
        )
        .remove(0);
        let mut b = a.clone();
        b.game_app_id = "2".into();
        b.scope = DiscoveryScope::verified("2", "Synthetic source attribution to game B");
        let mut unknown = a.clone();
        unknown.scope = DiscoveryScope::default();
        let merged = merge(vec![a.clone()], vec![a, b, unknown]);
        assert_eq!(
            merged.len(),
            3,
            "only the true same-game same-scope duplicate is removed"
        );
        assert!(merged.iter().any(|f| f.scope.matches_game("1")));
        assert!(merged.iter().any(|f| f.scope.matches_game("2")));
        assert!(merged
            .iter()
            .any(|f| f.scope.kind == DiscoveryScopeKind::UnknownScope));
    }
}
