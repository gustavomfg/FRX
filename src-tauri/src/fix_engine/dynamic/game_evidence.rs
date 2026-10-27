//! Relatos de desempenho de UM jogo, separados dos votos de compatibilidade
//! e do conhecimento técnico global. Nenhum relato externo é benchmark local.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::analyzer::Ctx;
use crate::fixes::models::{Confidence, FixFinding, PerformanceOutcome};

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
            let compatible = r.gpu_vendor.is_none_or(|gpu| ctx.gpu == Some(gpu))
                && r.proton_version.as_ref().is_none_or(|p| {
                    ctx.tool_name
                        .is_some_and(|tool| tool.eq_ignore_ascii_case(p))
                });
            if !compatible {
                out.context_compatible = false;
                out.limitations.push("Há relato com GPU ou Proton diferente ou não confirmado no contexto atual; não promove um candidato automaticamente.".into());
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
