//! Da fonte à afirmação. Lê o que cada fonte relata sobre a opção, com
//! `fixes::reports`, e guarda o trecho literal. Nunca extrai ação do texto.

use super::models::{Claim, ClaimKind, ConfigKey};
use crate::fixes::models::{FindingKind, FindingSource, FixFinding};
use crate::fixes::reports::{analyze, Outcome};

fn options(key: &ConfigKey) -> Vec<&str> {
    key.items
        .iter()
        .filter(|i| i.kind == "env" || i.kind == "component")
        .map(|i| i.name.as_str())
        .collect()
}

fn claim_for(source: &FindingSource, key: &ConfigKey, verified: bool) -> Claim {
    let snippet = source.snippet.as_deref().unwrap_or("");
    let found = options(key)
        .into_iter()
        .find_map(|option| analyze(snippet, option));
    match found {
        Some(r) => Claim {
            kind: ClaimKind::ReportedResult(r.outcome),
            option: r.option,
            disabled: r.disabled,
            combined_with: r.combined_with,
            quote: Some(r.quote),
            source: source.clone(),
            game_verified: verified,
        },
        None => Claim {
            kind: ClaimKind::MentionOnly,
            option: options(key).first().copied().unwrap_or_default().to_owned(),
            disabled: false,
            combined_with: vec![],
            quote: (!snippet.is_empty()).then(|| snippet.to_owned()),
            source: source.clone(),
            game_verified: verified,
        },
    }
}

/// Afirmações por fonte, mais `true` se o provedor contou relatos que não dá
/// para atribuir a uma fonte específica.
pub fn extract(f: &FixFinding, key: &ConfigKey, app_id: &str) -> (Vec<Claim>, bool) {
    let verified = f.scope.matches_game(app_id);
    let documented = matches!(f.kind, FindingKind::Protonfix | FindingKind::ReleaseNote);
    let mut claims: Vec<Claim> = f
        .sources
        .iter()
        .map(|s| {
            let mut c = claim_for(s, key, verified);
            if documented {
                c.kind = ClaimKind::Documented;
            }
            c
        })
        .collect();
    if documented {
        return (claims, false);
    }
    // O provedor analisou o comentário inteiro; o trecho salvo pode não repetir
    // a opção. A contagem dele só é atribuída quando não há ambiguidade.
    let count = |want: fn(Outcome) -> bool, list: &[Claim]| {
        list.iter()
            .filter(|c| matches!(c.kind, ClaimKind::ReportedResult(o) if want(o)))
            .count() as u32
    };
    let is_pos = |o| o == Outcome::Positive;
    let is_neg = |o| matches!(o, Outcome::NotResolved | Outcome::Worsened);
    let missing_pos = f.evidence.positive.saturating_sub(count(is_pos, &claims));
    let missing_neg = f.evidence.negative.saturating_sub(count(is_neg, &claims));
    let mentions = claims
        .iter()
        .filter(|c| c.kind == ClaimKind::MentionOnly)
        .count() as u32;
    if missing_pos + missing_neg == 0 {
        return (claims, false);
    }
    if (missing_pos > 0) != (missing_neg > 0) && missing_pos + missing_neg == mentions {
        let outcome = if missing_pos > 0 {
            Outcome::Positive
        } else {
            Outcome::NotResolved
        };
        for c in &mut claims {
            if c.kind == ClaimKind::MentionOnly {
                c.kind = ClaimKind::ReportedResult(outcome);
            }
        }
        return (claims, false);
    }
    (claims, true)
}
