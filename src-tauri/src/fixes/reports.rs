//! FRX AI — leitura de relatos. Para uma opção citada num comentário, decide se
//! o autor relata o resultado de usá-la, qual foi e em qual frase. O problema é
//! decomposto: limpeza (citações e modelos de issue saem), ligação entre a opção
//! e a frase que fala dela, ato de fala (pergunta, sugestão ou relato), resultado
//! com escopo de negação e contraste, e bloqueio de causa atribuída a outra coisa
//! (versão do Proton). Na dúvida, abstém-se: `None` não é erro, é "não sei".

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// Resolveu ou melhorou.
    Positive,
    NotResolved,
    /// Causou um problema novo ou piorou.
    Worsened,
    /// Ajudou em parte e falhou em parte.
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionReport {
    pub option: String,
    pub outcome: Outcome,
    /// O autor desligou ou removeu a opção (`=0`, "disabled", "without").
    pub disabled: bool,
    /// Outras opções usadas junto, na mesma linha: o resultado é da combinação.
    pub combined_with: Vec<String>,
    /// Frase do comentário que sustenta o resultado, sem alterações.
    pub quote: String,
}

struct Unit {
    text: String,
    code: bool,
    list_item: bool,
    table_row: bool,
    header: Option<String>,
}

/// Texto em minúsculas, contrações abertas e só palavras, entre espaços.
fn norm(s: &str) -> String {
    let s = s
        .to_lowercase()
        .replace(['’', '‘'], "'")
        .replace("n't", " not")
        .replace("-free", " free");
    let mut out = String::from(" ");
    for c in s.chars() {
        if c.is_alphanumeric() || c == '_' {
            out.push(c);
        } else if !out.ends_with(' ') {
            out.push(' ');
        }
    }
    if !out.ends_with(' ') {
        out.push(' ');
    }
    out
}

fn has(n: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|p| n.contains(&format!(" {p} ")))
}

const SUGGEST: &[&str] = &[
    "try",
    "you can",
    "you could",
    "you should",
    "you may",
    "you might",
    "you need",
    "you have to",
    "should",
    "maybe",
    "perhaps",
    "recommend",
    "please",
    "make sure",
    "consider",
    "if you",
    "in case",
    "can someone",
    "does anybody",
    "anyone",
];
const FIRST_PERSON: &[&str] = &["i", "i m", "i ve", "i d", "me", "my", "we", "our", "us"];
const DEICTIC: &[&str] = &[
    "this",
    "it",
    "that",
    "these",
    "with this",
    "with that",
    "now",
    "which",
];
const UNCERTAIN: &[&str] = &["unsure", "not sure", "do not know if", "no idea if"];
const INFO: &[&str] = &[
    "exists",
    "release",
    "commit",
    "upstream",
    "merged",
    "changelog",
];
const OTHER_CAUSE: &[&str] = &[
    "regression",
    "on the version",
    "version above",
    "downgrading",
    "downgraded",
    "upgrading",
    "upgraded",
    "switching to",
    "switched to",
];
const NOT_RESOLVED: &[&str] = &[
    "did not work",
    "does not work",
    "not working",
    "do not work",
    "did not help",
    "does not help",
    "did not fix",
    "does not fix",
    "not fix",
    "not fixed",
    "no impact",
    "no noticeable impact",
    "no change",
    "no difference",
    "no effect",
    "no luck",
    "no dice",
    "none of which",
    "still crash",
    "still crashes",
    "still crashing",
    "still freeze",
    "still freezes",
    "still froze",
    "still face",
    "still happens",
    "still get",
    "still have",
    "still stutter",
    "still hangs",
    "still no",
    "same result",
    "same issue",
    "same behavior",
    "same behaviour",
    "same crash",
    "same error",
    "same problem",
    "persisted",
    "persists",
    "did not change",
    "not resolve",
    "not solved",
];
const WORSE_VERB: &[&str] = &[
    "caused",
    "causes",
    "causing",
    "results in",
    "resulted in",
    "result in",
    "broke",
    "breaks",
    "black screen",
    "crashes consistently",
    "started crashing",
    "now crashes",
    "the reason for",
    "the actual reason",
    "makes it crash",
];
const PROBLEM: &[&str] = &[
    "crash",
    "crashes",
    "crashing",
    "freeze",
    "freezes",
    "freezing",
    "hang",
    "hangs",
    "black",
    "artifact",
    "artifacts",
    "distorted",
    "distortion",
    "flicker",
    "flickering",
    "stutter",
    "stuttering",
    "hitching",
    "hitch",
    "glitch",
    "glitches",
    "corruption",
    "corrupted",
    "error",
    "slowdown",
    "lag",
    "screen",
];
const POSITIVE: &[&str] = &[
    "fixed",
    "fixes",
    "fix it",
    "resolved",
    "resolves",
    "solved",
    "cured",
    "works",
    "worked",
    "working",
    "runs perfectly",
    "runs fine",
    "runs well",
    "runs great",
    "ran fine",
    "no more",
    "stable",
    "got rid",
    "prevents",
    "prevented",
    "playable",
    "crash free",
    "improvement",
    "improved",
    "improves",
    "helps",
    "helped",
    "better",
    "good performance",
    "great performance",
    "start running",
    "to work",
];
const CONTRAST: &[&str] = &["but", "however", "although", "though", "except", "still"];
const PARTIAL: &[&str] = &["partially", "some but not all", "not all"];
const NEGATION: &[&str] = &["not", "no", "never", "without"];

/// Resultado de uma frase, considerando negação e contraste entre orações.
fn outcome_of(sentence: &str, table_row: bool) -> Option<Outcome> {
    let n = norm(sentence);
    let worse = has(&n, WORSE_VERB)
        && has(&n, PROBLEM)
        && !has(&n, &["start running", "to work", "to start"]);
    if worse {
        return Some(Outcome::Worsened);
    }
    let not_resolved = has(&n, NOT_RESOLVED) || (table_row && has(&n, &["identical"]));
    // Primeira oração com resultado positivo; contraste depois dela torna misto.
    let mut positive_at = None;
    for (i, raw) in sentence.split([',', ';']).enumerate() {
        let c = norm(raw);
        if has(&c, POSITIVE) && !has(&c, NEGATION) && !has(&c, NOT_RESOLVED) {
            positive_at = Some(i);
            break;
        }
    }
    match positive_at {
        Some(_) if has(&n, PARTIAL) => Some(Outcome::Mixed),
        Some(i) => {
            let after: String = sentence
                .split([',', ';'])
                .skip(i)
                .collect::<Vec<_>>()
                .join(",");
            let rest = norm(&after);
            let contrast_pos = CONTRAST
                .iter()
                .filter_map(|c| rest.find(&format!(" {c} ")))
                .min();
            match contrast_pos {
                Some(p)
                    if has(&rest[p..], NEGATION)
                        || has(&rest[p..], PROBLEM)
                        || has(&rest[p..], &["stopped", "never"]) =>
                {
                    Some(Outcome::Mixed)
                }
                _ if not_resolved => None,
                _ => Some(Outcome::Positive),
            }
        }
        None if not_resolved => Some(Outcome::NotResolved),
        None => None,
    }
}

/// O texto fala de uma experiência do autor, e não de sugestão, pergunta ou nota.
fn is_report(sentence: &str, mentions_option: bool) -> bool {
    let n = norm(sentence);
    if sentence.contains('?') || has(&n, SUGGEST) || has(&n, UNCERTAIN) {
        return false;
    }
    let first = has(&n, FIRST_PERSON);
    let deictic = DEICTIC.iter().any(|d| n.starts_with(&format!(" {d} ")));
    // Sem primeira pessoa, a opção precisa ser o sujeito, fora de nota ou conversa.
    first || deictic || (mentions_option && !has(&n, &["you"]) && !has(&n, INFO))
}

fn split_sentences(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = line.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        cur.push(c);
        if matches!(c, '.' | '!' | '?') && chars.get(i + 1).is_some_and(|n| n.is_whitespace()) {
            out.push(cur.trim().to_owned());
            cur.clear();
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_owned());
    }
    out
}

/// Comentário em unidades: frases de prosa, itens de lista, linhas de tabela e
/// linhas de código. Citações (`>`) e modelos de issue (`<!-- -->`) saem: não são
/// relato do autor.
fn units(body: &str) -> Vec<Unit> {
    let mut text = body.replace('\r', "");
    while let (Some(a), Some(b)) = (text.find("<!--"), text.find("-->")) {
        if b < a {
            break;
        }
        text.replace_range(a..b + 3, "");
    }
    let mut out = Vec::new();
    let mut fence = false;
    let mut header: Option<String> = None;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("```") {
            fence = !fence;
            continue;
        }
        if t.is_empty() || t.starts_with('>') {
            continue;
        }
        if fence {
            out.push(Unit {
                text: t.to_owned(),
                code: true,
                list_item: false,
                table_row: false,
                header: header.clone(),
            });
            continue;
        }
        if t.starts_with('#') || (t.ends_with(':') && !t.starts_with(['-', '*', '|'])) {
            header = Some(t.trim_start_matches('#').trim().to_owned());
        }
        let table_row = t.starts_with('|');
        let list_item = t.starts_with("- ")
            || t.starts_with("* ")
            || t.chars().next().is_some_and(|c| c.is_ascii_digit()) && t.contains(". ");
        let words = norm(t)
            .split_whitespace()
            .filter(|w| !w.chars().any(|c| c.is_ascii_uppercase()) && !w.contains('_'))
            .count();
        let launch = t.contains("%command%") && words < 8;
        if launch || (t.starts_with('`') && t.ends_with('`') && t.matches('`').count() == 2) {
            out.push(Unit {
                text: t.to_owned(),
                code: true,
                list_item,
                table_row,
                header: header.clone(),
            });
            continue;
        }
        for s in if table_row || list_item {
            vec![t.to_owned()]
        } else {
            split_sentences(t)
        } {
            out.push(Unit {
                text: s,
                code: false,
                list_item,
                table_row,
                header: header.clone(),
            });
        }
    }
    out
}

fn mentions(text: &str, option: &str) -> bool {
    text.match_indices(option).any(|(i, _)| {
        let before = text[..i].chars().last();
        let after = text[i + option.len()..].chars().next();
        !before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
            && !after.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

/// Outras variáveis na mesma linha que a opção.
fn companions(line: &str, option: &str) -> Vec<String> {
    let mut out = Vec::new();
    for tok in line.split(|c: char| c.is_whitespace() || c == '`') {
        let name = tok.split('=').next().unwrap_or("");
        if tok.contains('=')
            && name.len() > 3
            && name != option
            && name
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            && !out.iter().any(|o| o == name)
        {
            out.push(name.to_owned());
        }
    }
    out
}

fn disabled(line: &str, option: &str) -> bool {
    let n = norm(line);
    line.contains(&format!("{option}=0"))
        || has(
            &n,
            &[
                "disabling",
                "disabled",
                "disable",
                "removed",
                "removing",
                "without",
                "unset",
                "turned off",
            ],
        )
}

/// Resultado relatado para a opção neste comentário, ou `None` se o texto não sustenta.
pub fn analyze(body: &str, option: &str) -> Option<OptionReport> {
    let us = units(body);
    let mut verdicts: Vec<(Outcome, String)> = Vec::new();
    for (i, u) in us.iter().enumerate() {
        if !mentions(&u.text, option) {
            continue;
        }
        // Frases candidatas: a própria (se prosa), a que introduz o código e a seguinte, se retomar.
        let mut cands: Vec<(&Unit, bool)> = Vec::new();
        if !u.code {
            cands.push((u, true));
        } else if let Some(p) = us[..i].iter().rev().take(2).find(|p| !p.code) {
            let n = norm(&p.text);
            if p.text.trim_end().ends_with(':')
                || has(&n, &["following", "below", "these", "this", "with"])
            {
                cands.push((p, false));
            }
        }
        if let Some(next) = us.get(i + 1).filter(|n| !n.code) {
            if DEICTIC
                .iter()
                .any(|d| norm(&next.text).starts_with(&format!(" {d} ")))
            {
                cands.push((next, false));
            }
        }
        for (c, own) in cands {
            let n = norm(&c.text);
            if has(&n, OTHER_CAUSE) || !is_report(&c.text, own) {
                continue;
            }
            let mut outcome = outcome_of(&c.text, c.table_row);
            // Item de lista sem resultado próprio herda o cabeçalho "o que já testei e não resolveu".
            if outcome.is_none() && (c.list_item || c.table_row) {
                if let Some(h) = &c.header {
                    if has(
                        &norm(h),
                        &[
                            "ruled out",
                            "did not help",
                            "nothing helped",
                            "no change",
                            "none of these",
                            "did not work",
                        ],
                    ) {
                        outcome = Some(Outcome::NotResolved);
                    }
                }
            }
            // Positivo seguido de "ainda trava" no mesmo trecho é misto.
            if outcome == Some(Outcome::Positive) {
                let later = us.iter().skip(i + 1).take(2).any(|l| {
                    !l.code && has(&norm(&l.text), &["still"]) && has(&norm(&l.text), PROBLEM)
                });
                if later {
                    outcome = Some(Outcome::Mixed);
                }
            }
            if let Some(o) = outcome {
                verdicts.push((o, c.text.clone()));
                break;
            }
        }
    }
    let (outcome, quote) = verdicts.first()?.clone();
    // Relatos diferentes para a mesma opção no mesmo comentário: não decide.
    if verdicts.iter().any(|(o, _)| *o != outcome) {
        return None;
    }
    let line = us
        .iter()
        .find(|u| mentions(&u.text, option))
        .map(|u| u.text.as_str())
        .unwrap_or("");
    Some(OptionReport {
        option: option.to_owned(),
        outcome,
        disabled: disabled(line, option),
        combined_with: companions(line, option),
        quote,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reported_result_suggestion_and_regression_are_told_apart() {
        let worse = "I also tried `VKD3D_CONFIG=descriptor_heap`, but that caused badly distorted cloth/geometry and a different crash.";
        assert_eq!(
            analyze(worse, "VKD3D_CONFIG").unwrap().outcome,
            Outcome::Worsened
        );
        let fixed = "This fixed the crashes for me:\n```\nVKD3D_CONFIG=no_upload_hvv PROTON_VKD3D_HEAP=1 %command%\n```";
        let r = analyze(fixed, "VKD3D_CONFIG").unwrap();
        assert_eq!(
            (r.outcome, r.combined_with.as_slice()),
            (Outcome::Positive, &["PROTON_VKD3D_HEAP".to_owned()][..])
        );
        assert!(analyze(
            "Try enabling `VKD3D_CONFIG=descriptor_heap`.",
            "VKD3D_CONFIG"
        )
        .is_none());
        assert!(
            analyze(
                "> This fixed the crashes for me: `VKD3D_CONFIG=x`",
                "VKD3D_CONFIG"
            )
            .is_none(),
            "citação de outra pessoa"
        );
        let mixed =
            "With `PROTON_USE_NTSYNC=1` the game works quite well, but it never loads the match.";
        assert_eq!(
            analyze(mixed, "PROTON_USE_NTSYNC").unwrap().outcome,
            Outcome::Mixed
        );
        let version = "Proton 11 seems to have some regression, I was using `PROTON_VKD3D_HEAP=1` but the game crashes consistently on the version above.";
        assert!(
            analyze(version, "PROTON_VKD3D_HEAP").is_none(),
            "a causa é a versão, não a opção"
        );
    }

    /// Avaliação em dados rotulados: `FRX_AI_GOLD=gold.json FRX_AI_OUT=pred.json`.
    #[test]
    #[ignore]
    fn evaluate_on_labeled_reports() {
        let gold: Vec<serde_json::Value> = serde_json::from_str(
            &std::fs::read_to_string(std::env::var("FRX_AI_GOLD").unwrap()).unwrap(),
        )
        .unwrap();
        let out: Vec<serde_json::Value> = gold
            .iter()
            .map(|g| {
                let r = analyze(g["body"].as_str().unwrap(), g["option"].as_str().unwrap());
                serde_json::json!({"id": g["id"], "report": r})
            })
            .collect();
        std::fs::write(
            std::env::var("FRX_AI_OUT").unwrap(),
            serde_json::to_string(&out).unwrap(),
        )
        .unwrap();
    }
}
