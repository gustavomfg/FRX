use crate::system::context::EnvironmentContext;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub id: String,
    pub kind: String,
    pub summary: String,
    pub source_url: Option<String>,
    pub publisher: Option<String>,
    pub published_at: Option<String>,
    pub updated_at: Option<String>,
    pub fetched_at: String,
    pub version: Option<String>,
    pub app_id: Option<String>,
    pub technology: Option<String>,
    pub stale: bool,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Technology {
    pub id: String,
    pub name: String,
    pub hardware: String,
    pub game_support: String,
    pub environment: String,
    pub configured: String,
    pub runtime: String,
    pub evidence: Vec<Evidence>,
    pub limitations: Vec<String>,
}
impl Technology {
    pub fn unknown(id: &str, name: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            hardware: "unknown".into(),
            game_support: "unknown".into(),
            environment: "unknown".into(),
            configured: "unknown".into(),
            runtime: "unknown".into(),
            evidence: vec![],
            limitations: vec![],
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub id: String,
    pub technology: String,
    pub symptom: String,
    pub certainty: String,
    pub evidence: Vec<Evidence>,
    pub hypotheses: Vec<String>,
    pub ruled_out: Vec<String>,
    pub limitations: Vec<String>,
    pub next_action: String,
    pub plan_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionPlan {
    #[serde(default)]
    pub availability: crate::fix_engine::availability::Availability,
    pub id: String,
    pub app_id: String,
    pub title: String,
    pub objective: String,
    pub problem_id: String,
    pub recipe_id: Option<String>,
    pub state: String,
    pub actions: Vec<String>,
    pub dependencies: Vec<String>,
    pub risks: Vec<String>,
    pub sources: Vec<Evidence>,
    pub reversible: bool,
    pub verification_method: String,
    pub success_criterion: String,
    pub operation_id: Option<String>,
    pub verification: String,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionPack {
    pub id: String,
    pub app_id: String,
    pub purpose: String,
    pub title: String,
    pub plan_ids: Vec<String>,
    pub smart_pack_id: Option<String>,
    pub explanation: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    #[serde(default)]
    pub symptoms: Vec<String>,
    pub schema: u32,
    pub app_id: String,
    pub game_name: String,
    pub generated_at: String,
    pub context_fingerprint: String,
    pub environment: Option<EnvironmentContext>,
    pub technologies: Vec<Technology>,
    pub problems: Vec<Problem>,
    pub plans: Vec<ResolutionPlan>,
    pub packs: Vec<ResolutionPack>,
    pub knowledge: Vec<Evidence>,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LogObservation {
    pub app_id: String,
    pub imported_at: String,
    pub digest: String,
    pub evidence: Vec<Evidence>,
    pub limitations: Vec<String>,
}
