//! Dynamic Recipe Generator — transforma descobertas em receitas executáveis
//! **sem cadastro manual**, quando a ação é reconhecida e validada.
//!
//! Fluxo: descoberta (`FixFinding`, do cache da pesquisa) → normalização e
//! proveniência → extração determinística de ações → validação de segurança
//! (a operação é controlada e reversível?) → avaliação de evidência (os
//! relatos sustentam a correção?) → receita `generated` → registro local.
//!
//! Segurança e confiabilidade são independentes: uma variável pode ser segura
//! de aplicar e ainda assim não resolver o problema. Só a primeira libera o
//! botão Aplicar; a segunda é mostrada como nível de evidência.
//!
//! Nada aqui escreve na configuração do jogo. A receita gerada segue para o
//! mesmo planejador, backup, diário e rollback das receitas do catálogo, e é
//! **regenerada e revalidada** a partir do cache da pesquisa antes de cada
//! prévia e de cada aplicação — o registro em disco serve para exibição e
//! histórico, nunca como fonte para executar.

pub mod analyzer;
pub mod compatibility;
pub mod extraction;
pub mod game_evidence;
pub mod generator;
pub mod provenance;
pub mod registry;
pub mod validation;

#[cfg(test)]
pub(crate) mod tests;

use serde::{Deserialize, Serialize};

use super::models::Recipe;
use crate::fixes::models::ActionKind;
use provenance::Provenance;

/// Versão do gerador. Mudou → todas as receitas geradas são reavaliadas.
pub const GENERATOR_VERSION: &str = "1.1.0";
/// Versão das regras de segurança e do conhecimento de variáveis
/// (`security::ENV_ALLOWLIST` + `compatibility::KNOWLEDGE`). Aumente ao mudar
/// qualquer uma das duas.
pub const RULES_VERSION: &str = "2026.10.27-2";

pub use game_evidence::GamePerformanceEvidence;

/// Categoria exibida ao usuário.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DynClass {
    /// A — aplicável com confirmação.
    Applicable,
    /// B — exige validação adicional; nunca é executada automaticamente.
    NeedsValidation,
    /// C — apenas informativa.
    Informational,
}

/// Situação específica (o motivo exato aparece em `reasons`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DynStatus {
    /// Receita gerada e validada.
    Generated,
    /// Uma receita revisada do catálogo cobre exatamente este ajuste.
    Reviewed,
    AlreadyIncluded,
    Incompatible,
    NeedsValidation,
    /// Ação perigosa: nunca automatizada (carrega código, caminhos sensíveis…).
    Blocked,
    /// Relatos contrários ou divididos.
    Contested,
    NoActions,
    InsufficientData,
    /// AppID diferente, conteúdo malformado ou sem fonte rastreável.
    Invalid,
}

impl DynStatus {
    pub fn class(self) -> DynClass {
        match self {
            DynStatus::Generated | DynStatus::Reviewed => DynClass::Applicable,
            DynStatus::NeedsValidation | DynStatus::Blocked => DynClass::NeedsValidation,
            _ => DynClass::Informational,
        }
    }
}

/// Confiabilidade da solução — o que os relatos sustentam, não se é seguro.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceLevel {
    /// Repositório curado (UMU-Protonfixes) ou declaração de mantenedor.
    Documented,
    /// Pelo menos dois relatos favoráveis, mais favoráveis que contrários.
    Corroborated,
    /// Um relato só, sem contestação: experimental.
    Single,
    /// Relatos contrários iguais ou maiores que os favoráveis.
    Contested,
    /// Nenhum relato sobre o efeito.
    None,
}

/// Uma ação reconhecida no texto, com o veredito do validador.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActionView {
    pub kind: ActionKind,
    pub raw: String,
    pub supported: bool,
    pub note: String,
}

/// Resultado da análise de uma descoberta.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Assessment {
    #[serde(default)]
    pub performance_evidence: GamePerformanceEvidence,
    #[serde(default)]
    pub performance_warnings: Vec<String>,
    pub finding_id: String,
    pub app_id: String,
    pub class: DynClass,
    pub status: DynStatus,
    /// Motivos específicos (bloqueio, alerta ou o que foi reconhecido).
    pub reasons: Vec<String>,
    pub evidence: EvidenceLevel,
    pub evidence_note: String,
    pub experimental: bool,
    pub actions: Vec<ActionView>,
    /// Receita a abrir no Fix Engine (gerada ou do catálogo).
    pub recipe_id: Option<String>,
    pub recipe: Option<Recipe>,
    pub provenance: Vec<Provenance>,
    pub interpreter: String,
}

/// Análise com as datas do registro local.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentView {
    #[serde(flatten)]
    pub assessment: Assessment,
    pub generated_at: String,
    pub updated_at: String,
    pub discovered_at: Option<String>,
    pub reevaluated: Option<String>,
}
