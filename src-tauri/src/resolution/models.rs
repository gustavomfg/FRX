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
