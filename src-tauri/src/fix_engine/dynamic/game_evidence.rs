//! Relatos de desempenho de UM jogo, separados dos votos de compatibilidade
//! e do conhecimento técnico global. Nenhum relato externo é benchmark local.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::analyzer::Ctx;
use crate::fixes::models::{Confidence, FixFinding, PerformanceOutcome};
use crate::system::context::MatchState;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GamePerformanceEvidence {
    pub app_id: Option<String>,
    pub positive: u32,
    pub negative: u32,
    #[serde(default)]
    pub authoritative_positive: bool,
    pub source_urls: Vec<String>,
    pub context_compatible: bool,
    pub limitations: Vec<String>,
}

impl GamePerformanceEvidence {
    pub fn conflicting(&self) -> bool {
        self.positive > 0 && self.negative > 0
    }

    pub fn contextual_candidate(&self, app_id: &str) -> bool {
        self.app_id.as_deref() == Some(app_id)
            && self.context_compatible
            && (self.authoritative_positive && self.positive > 0
                || self.positive >= 2
                    && self.source_urls.iter().collect::<BTreeSet<_>>().len() >= 2)
            && self.negative == 0
    }

    /// Mescla apenas relatos do mesmo jogo. Contradições nunca desaparecem
    /// ao escolher a descoberta de maior confiança.
    pub fn merge(&mut self, other: &Self) {
        if self.app_id.is_none() {
            *self = other.clone();
            return;
        }
        if self.app_id != other.app_id {
            return;
        }
        // Contagens conservadoras: duas descobertas não provam independência.
        self.positive = self.positive.max(other.positive);
        self.negative = self.negative.max(other.negative);
        self.authoritative_positive |= other.authoritative_positive;
        self.context_compatible &= other.context_compatible;
        for url in &other.source_urls {
            if !self.source_urls.contains(url) {
                self.source_urls.push(url.clone());
            }
        }
        for note in &other.limitations {
            if !self.limitations.contains(note) {
                self.limitations.push(note.clone());
            }
        }
    }
}

pub fn assess(
    f: &FixFinding,
    ctx: &Ctx<'_>,
    actions: &[(String, String)],
) -> GamePerformanceEvidence {
    if f.game_app_id != ctx.app_id || !f.scope.matches_game(ctx.app_id) {
        return GamePerformanceEvidence::default();
    }
    let mut out = GamePerformanceEvidence {
        app_id: Some(ctx.app_id.to_owned()),
        context_compatible: true,
        limitations: vec!["Relatos públicos específicos deste AppID; não são medições neste computador. Hardware, versão do jogo e condições não informados não são presumidos iguais.".into()],
        ..Default::default()
    };
    let mut minimum_positive = u32::MAX;
    let required_hardware = f.requirements.hardware.evaluate(ctx.environment);
    if required_hardware.state != MatchState::Compatible {
        out.context_compatible = false;
    }
    out.limitations.extend(required_hardware.limitations);
    if required_hardware.state == MatchState::Incompatible {
        out.limitations.extend(required_hardware.reasons);
    }
    if f.requirements
        .game_version
        .as_deref()
        .is_some_and(|v| ctx.game_build_id != Some(v))
    {
        out.context_compatible = false;
        out.limitations.push("A versão do jogo informada pela fonte não foi confirmada no build atual; não há promoção automática.".into());
    }
    let mut all_authoritative = !actions.is_empty();
    for (key, value) in actions {
        let expected = format!("env:{key}={value}");
        let mut positive = BTreeSet::new();
        let mut negative = BTreeSet::new();
        let mut authoritative = false;
        for r in &f.performance_evidence.reports {
            if r.app_id != ctx.app_id
                || r.action_key != expected
                || !f.sources.iter().any(|s| s.url == r.source_url)
                || !super::provenance::trusted_url(&r.source_url)
            {
                continue;
            }
            let compatible = (ctx.environment.is_some()
                || r.gpu_vendor.is_none_or(|gpu| ctx.gpu == Some(gpu)))
                && r.proton_version.as_ref().is_none_or(|p| {
                    ctx.tool_name
                        .is_some_and(|tool| tool.eq_ignore_ascii_case(p))
                });
            let mut requirements = r.hardware.clone();
            if requirements.gpu_vendor.is_none() && ctx.environment.is_some() {
                requirements.gpu_vendor = r.gpu_vendor;
            }
            let report_hardware = requirements.evaluate(ctx.environment);
            if !compatible || report_hardware.state != MatchState::Compatible {
                // Evidence from another GPU/driver stays at its original source,
                // but cannot improve or penalize this environment's assessment.
                out.limitations.push("Relato de outro hardware/Proton ou contexto não confirmado foi preservado na origem e não participa da avaliação deste ambiente.".into());
                out.limitations.extend(report_hardware.limitations);
                continue;
            }
            if !out.source_urls.contains(&r.source_url) {
                out.source_urls.push(r.source_url.clone());
            }
            match r.outcome {
                PerformanceOutcome::Positive => {
                    if r.confidence == Confidence::Verified {
                        authoritative = true;
                        positive.insert(r.author.as_deref().unwrap_or(&r.source_url));
                    } else if r.confidence == Confidence::CommunityReported {
                        if let Some(author) = r.author.as_deref().filter(|a| !a.is_empty()) {
                            positive.insert(author);
                        }
                    }
                }
                PerformanceOutcome::Negative => {
                    negative.insert(&r.source_url);
                }
            }
        }
        minimum_positive = minimum_positive.min(positive.len() as u32);
        out.negative = out.negative.max(negative.len() as u32);
        all_authoritative &= authoritative;
    }
    out.positive = if actions.is_empty() {
        0
    } else {
        minimum_positive
    };
    out.authoritative_positive = all_authoritative;
    if out.conflicting() {
        out.limitations.push(
            "Evidências de desempenho conflitantes para este jogo; nenhum ganho é presumido."
                .into(),
        );
    }
    out.limitations.sort();
    out.limitations.dedup();
    out
}

#[cfg(test)]
mod hardware_tests {
    use super::*;
    use crate::fix_engine::catalog::Catalog;
    use crate::fixes::models::*;
    use crate::system::context::{
        tests::{env, gpu},
        HardwareRequirements,
    };
    use crate::system::GpuVendor;

    #[test]
    fn positive_and_negative_reports_for_different_hardware_are_not_global_conflicts() {
        // Synthetic source claims, never benchmark sessions or application data.
        let mut finding = crate::fix_engine::dynamic::tests::finding(
            "synthetic-hardware",
            ActionKind::Env,
            "PROTON_NO_D3D11=1",
            2,
            0,
        );
        finding.sources.clear();
        for (vendor, model, device, outcome) in [
            (
                GpuReq::Nvidia,
                "GeForce RTX 4060",
                "10de:2882",
                PerformanceOutcome::Positive,
            ),
            (
                GpuReq::Nvidia,
                "GeForce RTX 4070",
                "10de:2883",
                PerformanceOutcome::Negative,
            ),
            (
                GpuReq::Amd,
                "Radeon RX 7800 XT",
                "1002:747e",
                PerformanceOutcome::Negative,
            ),
        ] {
            for n in 0..2 {
                let url=format!("https://github.com/ValveSoftware/Proton/issues/99999#issuecomment-{device}-{n}");
                finding.sources.push(FindingSource {
                    provider: ProviderId::Github,
                    label: "Synthetic hardware evidence".into(),
                    url: url.clone(),
                    reference_id: None,
                    snippet: None,
                    published_at: None,
                });
                finding
                    .performance_evidence
                    .reports
                    .push(PerformanceReport {
                        hardware: HardwareRequirements {
                            gpu_vendor: Some(vendor),
                            gpu_models: vec![model.into()],
                            gpu_device_ids: vec![device.into()],
                            driver_versions: if model == "GeForce RTX 4060" {
                                vec!["580.0".into()]
                            } else {
                                vec![]
                            },
                            ..Default::default()
                        },
                        app_id: "200260".into(),
                        action_key: "env:PROTON_NO_D3D11=1".into(),
                        source_url: url,
                        author: Some(format!("synthetic-{device}-{n}")),
                        reported_at: None,
                        statement: "Synthetic scoped performance claim".into(),
                        outcome,
                        proton_version: None,
                        gpu_vendor: Some(vendor),
                        confidence: Confidence::CommunityReported,
                        limitations: vec!["Automated-test fixture".into()],
                    });
            }
        }
        let catalog = Catalog::bundled();
        for (vendor, context, pos, neg) in [
            (
                GpuReq::Nvidia,
                env(gpu(GpuVendor::Nvidia, "GeForce RTX 4060", "2882", "580.0")),
                2,
                0,
            ),
            (
                GpuReq::Nvidia,
                env(gpu(GpuVendor::Nvidia, "GeForce RTX 4070", "2883", "580.0")),
                0,
                2,
            ),
            (
                GpuReq::Nvidia,
                env(gpu(GpuVendor::Nvidia, "GeForce RTX 4060", "2882", "590.0")),
                0,
                0,
            ),
            (
                GpuReq::Amd,
                env(gpu(GpuVendor::Amd, "Radeon RX 7800 XT", "747e", "25.3")),
                0,
                2,
            ),
            (
                GpuReq::Intel,
                env(gpu(GpuVendor::Intel, "Arc A770", "56a0", "25.3")),
                0,
                0,
            ),
        ] {
            let ctx = Ctx {
                environment: Some(&context),
                game_build_id: None,
                app_id: "200260",
                gpu: Some(vendor),
                tool_name: Some("Proton 10"),
                catalog: &catalog,
                game: None,
            };
            let result = assess(&finding, &ctx, &[("PROTON_NO_D3D11".into(), "1".into())]);
            assert_eq!((result.positive, result.negative), (pos, neg));
            assert!(
                !result.conflicting(),
                "reports from an incompatible GPU do not make this cohort contradictory"
            );
            assert_eq!(result.contextual_candidate("200260"), pos >= 2);
            assert_eq!(
                finding.performance_evidence.reports.len(),
                6,
                "all original claims remain available"
            );
        }
    }
}
