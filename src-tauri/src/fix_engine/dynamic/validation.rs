//! Duas perguntas independentes:
//!
//! - **segurança da operação** (`env`): o Fix Engine consegue gravar e
//!   desfazer este `NOME=valor` dentro de limites controlados, sabendo o que
//!   ele faz?
//! - **confiabilidade da solução** (`evidence`): os relatos sustentam que a
//!   ação resolve o problema?
//!
//! Só a primeira decide se uma receita pode ser gerada.

use super::compatibility::{knowledge, known_key, Knowledge};
use super::EvidenceLevel;
use crate::fix_engine::security::{check_env, sensitive};
use crate::fixes::models::{Confidence, FindingKind, FixFinding};

pub enum EnvVerdict {
    Ok(&'static Knowledge),
    /// Nunca automatizada (carrega código, muda caminhos ou executáveis).
    Blocked(String),
    /// Segura talvez, mas o FRX não sabe o efeito: precisa de validação.
    Unknown(String),
}

pub fn env(key: &str, value: &str) -> EnvVerdict {
    if sensitive(key) {
        return EnvVerdict::Blocked(format!(
            "{key} pode alterar o carregamento de código, bibliotecas ou caminhos. O FRX nunca aplica essa variável automaticamente."
        ));
    }
    if let Err(e) = check_env(key, value) {
        return EnvVerdict::Unknown(if known_key(key) {
            format!("{key}={value}: valor fora do formato permitido. {e}")
        } else {
            format!("O FRX não conhece o efeito de {key}. Variáveis fora da lista revisada não viram receita automaticamente.")
        });
    }
    match knowledge(key, value) {
        Some(k) => EnvVerdict::Ok(k),
        None => EnvVerdict::Unknown(format!(
            "{key}={value} é permitida só para diagnóstico ou com efeito ainda não descrito pelo FRX; não vira correção automática."
        )),
    }
}

fn plural(n: u32, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Nível de evidência e uma frase que o explica.
pub fn evidence(f: &FixFinding) -> (EvidenceLevel, String) {
    let (pos, neg) = (f.evidence.positive, f.evidence.negative);
    if neg > 0 && neg >= pos {
        return (
            EvidenceLevel::Contested,
            format!(
                "{} que não resolveu ou que não é suportado, contra {}.",
                plural(neg, "relato diz", "relatos dizem"),
                plural(pos, "favorável", "favoráveis")
            ),
        );
    }
    if matches!(f.confidence, Confidence::Verified | Confidence::Curated) {
        let what = if f.kind == FindingKind::Protonfix {
            "Correção mantida no repositório UMU-Protonfixes"
        } else {
            "Declaração de mantenedores"
        };
        return (
            EvidenceLevel::Documented,
            format!("{what}; não foi testada no seu sistema."),
        );
    }
    if pos >= 2 {
        let mut s = format!(
            "{} em fontes públicas",
            plural(pos, "menção favorável", "menções favoráveis")
        );
        if neg > 0 {
            s.push_str(&format!(", {}", plural(neg, "contrária", "contrárias")));
        }
        s.push('.');
        return (EvidenceLevel::Corroborated, s);
    }
    if pos == 1 {
        return (
            EvidenceLevel::Single,
            "Um único relato, sem confirmação de outras pessoas.".into(),
        );
    }
    (EvidenceLevel::None, "Nenhum relato sobre o efeito.".into())
}
