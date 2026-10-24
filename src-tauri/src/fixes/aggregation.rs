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
        groups.entry(m.tweak.key()).or_default().push(m);
    }
    let mut out = Vec::new();
    for (key, mut ms) in groups {
        ms.sort_by(|a, b| b.source.published_at.cmp(&a.source.published_at));
        // Uma pessoa repetindo o ajuste no mesmo comentário conta uma vez.
        ms.dedup_by(|a, b| a.source.url == b.source.url);
        let tweak = ms[0].tweak.clone();
        let positive = ms.iter().filter(|m| m.positive && !m.negative).count() as u32;
        let negative = ms.iter().filter(|m| m.negative).count() as u32;
        let neutral = ms.len() as u32 - positive - negative;

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
            "Citado em {distinct} comentário{} público{} sobre este jogo",
            if distinct == 1 { "" } else { "s" },
            if distinct == 1 { "" } else { "s" }
        );
        if negative > 0 {
            description.push_str(&format!(", {negative} dizendo que não resolveu"));
        }
        description.push_str(". O FRX não testou este ajuste; ele é exibido como referência, não como recomendação garantida.");
        let published = ms.first().and_then(|m| m.source.published_at.clone());
        out.push(FixFinding {
            id: finding_id("github", app_id, &key),
            game_app_id: app_id.to_owned(),
            title,
            description,
            category,
            kind: FindingKind::Tweak,
            actions,
            sources: ms.iter().take(6).map(|m| m.source.clone()).collect(),
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
            evidence: Evidence {
                positive: positive + neutral,
                negative,
            },
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
    let mut seen: Vec<(String, String)> = Vec::new();
    primary.retain(|f| {
        let k = (
            f.id.clone(),
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
}
