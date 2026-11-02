//! Uma sugestão por configuração. Relatos contam por fonte distinta, e o grau
//! de confiança sai só do que as fontes afirmam, nunca de popularidade.

use std::collections::BTreeMap;

use super::gates;
use super::models::*;
use crate::fix_engine::availability::AvailabilityState;
use crate::fix_engine::dynamic::EvidenceLevel;
use crate::fixes::models::HardwareScope;
use crate::fixes::reports::Outcome;

pub fn tally(claims: &[Claim]) -> (u32, u32) {
    let mut pos = 0;
    let mut neg = 0;
    for c in claims {
        match c.kind {
            ClaimKind::ReportedResult(Outcome::Positive) => pos += 1,
            ClaimKind::ReportedResult(Outcome::NotResolved | Outcome::Worsened) => neg += 1,
            _ => {}
        }
    }
    (pos, neg)
}

pub fn confidence(claims: &[Claim]) -> EvidenceLevel {
    let (pos, neg) = tally(claims);
    let documented = claims.iter().any(|c| c.kind == ClaimKind::Documented);
    if neg > pos || (neg > 0 && neg == pos && !documented) {
        return EvidenceLevel::Contested;
    }
    if documented {
        return EvidenceLevel::Documented;
    }
    match pos {
        0 => EvidenceLevel::None,
        1 if neg == 0 => EvidenceLevel::Single,
        _ if pos >= 2 && pos > neg => EvidenceLevel::Corroborated,
        _ => EvidenceLevel::Contested,
    }
}

/// Uma afirmação por URL; quando repetida, vale a mais informativa.
fn merge_claims(group: &[Candidate]) -> Vec<Claim> {
    fn rank(c: &Claim) -> u8 {
        match c.kind {
            ClaimKind::Documented => 2,
            ClaimKind::ReportedResult(_) => 1,
            ClaimKind::MentionOnly => 0,
        }
    }
    let mut by_url: BTreeMap<String, Claim> = BTreeMap::new();
    for c in group.iter().flat_map(|g| g.claims.iter()) {
        match by_url.get(&c.source.url) {
            Some(old) if (rank(old), &old.quote) >= (rank(c), &c.quote) => {}
            _ => {
                by_url.insert(c.source.url.clone(), c.clone());
            }
        }
    }
    by_url.into_values().collect()
}

fn mapping_rank(m: &Mapping) -> u8 {
    match m {
        Mapping::AlreadyIncluded { .. } => 0,
        Mapping::Mapped { availability, .. } => match availability.state {
            AvailabilityState::Applied => 0,
            AvailabilityState::Available => 1,
            AvailabilityState::Temporary => 2,
            AvailabilityState::Investigation => 3,
            AvailabilityState::Incompatible => 5,
        },
        Mapping::Review { code, .. } if code == "command-rejected" => 6,
        Mapping::Review { .. } => 4,
        Mapping::Incompatible { .. } => 5,
        Mapping::NoAction => 7,
    }
}

fn compat_of(group: &Candidate, ctx: &DiagnosisContext) -> (CompatStatus, String) {
    if let Mapping::Mapped { availability, .. } = &group.mapping {
        if matches!(
            availability.state,
            AvailabilityState::Available
                | AvailabilityState::Temporary
                | AvailabilityState::Applied
        ) {
            return (
                CompatStatus::Compatible,
                "O planejador conferiu GPU, Proton e arquivos deste jogo.".into(),
            );
        }
    }
    let detail = if group.compatibility_note.is_empty() {
        if ctx.gpu_vendor.is_none() {
            "A GPU não foi identificada; a compatibilidade não pode ser confirmada.".to_owned()
        } else {
            "As fontes não informam hardware suficiente para confirmar.".to_owned()
        }
    } else {
        group.compatibility_note.clone()
    };
    (CompatStatus::Unknown, detail)
}

fn evidence_text(level: EvidenceLevel, pos: u32, neg: u32) -> String {
    match level {
        EvidenceLevel::Documented => "Documentada por mantenedores ou repositório curado.".into(),
        EvidenceLevel::Corroborated => {
            format!("{pos} fontes relatam melhora; {neg} relatam o contrário.")
        }
        EvidenceLevel::Single => "Um relato só, sem confirmação de outra fonte.".into(),
        EvidenceLevel::Contested => format!("Relatos divididos: {pos} a favor, {neg} contra."),
        EvidenceLevel::None => {
            "As fontes citam a configuração, mas nenhuma relata o resultado.".into()
        }
    }
}

pub struct Grouped {
    pub suggestions: Vec<Suggestion>,
    pub in_review: Vec<Suggestion>,
    pub excluded: Vec<Excluded>,
}

pub fn group(candidates: Vec<Candidate>, input: &DiagnosisInput) -> Grouped {
    let ctx = &input.context;
    let mut map: BTreeMap<String, Vec<Candidate>> = BTreeMap::new();
    for c in candidates {
        map.entry(c.key.id.clone()).or_default().push(c);
    }
    let mut out = Grouped {
        suggestions: vec![],
        in_review: vec![],
        excluded: vec![],
    };
    for (_, mut list) in map {
        // A ordem das fontes não pode mudar o resultado.
        list.sort_by_cached_key(|c| {
            let recipe = match &c.mapping {
                Mapping::Mapped { recipe_id, .. } => recipe_id.clone(),
                _ => String::new(),
            };
            (mapping_rank(&c.mapping), recipe, c.title.clone())
        });
        let claims = merge_claims(&list);
        let best = &list[0];
        let (pos, neg) = tally(&claims);
        let level = confidence(&claims);
        let improvement = gates::is_improvement(&best.key, best.purpose);
        let hardware_scope = list
            .iter()
            .map(|c| c.hardware_scope)
            .find(|s| *s != HardwareScope::Unknown)
            .unwrap_or_default();
        let exclude = |kind, reason: String| Excluded {
            hardware_scope,
            key: best.key.clone(),
            title: best.title.clone(),
            kind,
            reason,
            claims: claims.clone(),
        };
        if let Some(reason) = gates::dlss_block(&best.key, ctx) {
            out.excluded
                .push(exclude(ExcludedKind::Incompatible, reason));
            continue;
        }
        if let Some(reason) = gates::wrong_for_symptom(input.symptom, improvement) {
            out.excluded
                .push(exclude(ExcludedKind::WrongPurpose, reason));
            continue;
        }
        if let Some(reason) = gates::vendor_conflict(&best.key, ctx) {
            out.excluded
                .push(exclude(ExcludedKind::Incompatible, reason));
            continue;
        }
        let already = gates::already_in_options(&best.key, ctx)
            || matches!(&best.mapping, Mapping::Mapped { availability, .. } if availability.state == AvailabilityState::Applied);
        if !already && level == EvidenceLevel::Contested && pos == 0 && neg > 0 {
            out.excluded.push(exclude(
                ExcludedKind::Negative,
                format!("{neg} fonte(s) relatam que não resolveu ou piorou."),
            ));
            continue;
        }
        if let (Mapping::Incompatible { reason }, false) = (&best.mapping, already) {
            out.excluded
                .push(exclude(ExcludedKind::Incompatible, reason.clone()));
            continue;
        }
        let strong = matches!(
            level,
            EvidenceLevel::Documented | EvidenceLevel::Corroborated
        );
        let evidence = evidence_text(level, pos, neg);
        let (state, reason, recipe_id, fingerprint) = if already {
            (
                SuggestionState::AlreadyApplied,
                format!("Já aplicada nas opções deste jogo. {evidence}"),
                None,
                None,
            )
        } else {
            match &best.mapping {
                Mapping::AlreadyIncluded { reason } => {
                    (SuggestionState::AlreadyIncluded, reason.clone(), None, None)
                }
                Mapping::Review { reason, .. } => {
                    (SuggestionState::Review, reason.clone(), None, None)
                }
                Mapping::Mapped {
                    recipe_id,
                    availability,
                } if strong => match availability.state {
                    AvailabilityState::Available => (
                        SuggestionState::Applicable,
                        evidence.clone(),
                        Some(recipe_id.clone()),
                        availability.plan_fingerprint.clone(),
                    ),
                    AvailabilityState::Temporary => (
                        SuggestionState::Temporary,
                        availability
                            .reasons
                            .first()
                            .map(|r| r.message.clone())
                            .unwrap_or_else(|| evidence.clone()),
                        None,
                        None,
                    ),
                    _ => (
                        SuggestionState::Review,
                        availability
                            .reasons
                            .first()
                            .map(|r| r.message.clone())
                            .unwrap_or_else(|| {
                                "O planejador não confirmou um plano automático.".into()
                            }),
                        None,
                        None,
                    ),
                },
                Mapping::Mapped { .. } => (
                    SuggestionState::Review,
                    format!("Evidência insuficiente para aplicar. {evidence}"),
                    None,
                    None,
                ),
                Mapping::Incompatible { .. } | Mapping::NoAction => continue,
            }
        };
        let (compatibility, compatibility_detail) = compat_of(best, ctx);
        let suggestion = Suggestion {
            hardware_scope,
            third_party_binary: list.iter().any(|c| c.third_party_binary),
            key: best.key.clone(),
            title: best.title.clone(),
            optional_improvement: improvement,
            state,
            confidence: level,
            reason,
            compatibility,
            compatibility_detail,
            positive: pos,
            negative: neg,
            claims,
            recipe_id,
            expected_fingerprint: fingerprint,
        };
        if state == SuggestionState::Review {
            out.in_review.push(suggestion);
        } else {
            out.suggestions.push(suggestion);
        }
    }
    let fit = |s: &Suggestion| s.hardware_scope.rank(ctx.gpu_vendor);
    out.suggestions.sort_by(|a, b| {
        (
            a.optional_improvement,
            a.state,
            fit(a),
            a.confidence,
            &a.key.id,
        )
            .cmp(&(
                b.optional_improvement,
                b.state,
                fit(b),
                b.confidence,
                &b.key.id,
            ))
    });
    out.in_review
        .sort_by(|a, b| (fit(a), a.confidence, &a.key.id).cmp(&(fit(b), b.confidence, &b.key.id)));
    out.excluded.sort_by(|a, b| a.key.id.cmp(&b.key.id));
    out
}
