//! Pack Builder: compõe candidatos de forma determinística e limitada.
//!
//! Estratégia (nunca todas as combinações possíveis):
//!
//! 1. **Baseline** — nenhuma correção do FRX.
//! 2. **Receitas individuais** — até `max_units`, na ordem: evidência mais
//!    forte, menor risco, menos variáveis, ID.
//! 3. **Pacotes por perfil** — união gulosa de receitas *elegíveis* e
//!    compatíveis entre si, até `max_pack_size`:
//!    - compatibilidade: contornos com evidência documentada ou confirmada;
//!    - desempenho: receitas que, medidas sozinhas, melhoraram o FPS médio
//!      sem piorar o 1% low;
//!    - equilibrado: receitas que não pioraram FPS médio nem 1% low e
//!      melhoraram um deles.
//! 4. **Pares** de receitas elegíveis sem conflito conhecido.
//!
//! Elegível para combinação = evidência documentada/confirmada **ou**
//! medição local conclusiva sem regressão. Receitas de um relato só, ainda
//! não medidas, só aparecem sozinhas: comentários diferentes não são somados
//! às cegas. Limitadores de FPS nunca entram em combinações (mudam as
//! condições do benchmark). Total limitado por `max_packs`.

use super::benchmark::{baseline_id, pack_id_with_files, unit_file_fixes, units_file_fixes};
use super::models::*;
use crate::fix_engine::dynamic::compatibility::performance_warnings;
use crate::fix_engine::dynamic::EvidenceLevel;
use crate::fix_engine::files::sha256_hex;
use crate::fix_engine::models::{RecipeAction, RecipeSource, Risk};
use crate::fixes::compat::vendor_name;

/// A unidade pode entrar num pacote deste jogo? Senão, o motivo: receita de
/// outro AppID ou ação de arquivo incompleta nunca vira pacote parcial.
pub fn admissible(app_id: &str, u: &Unit) -> Result<(), String> {
    if u.app_id.as_deref() != Some(app_id) || !crate::art::valid_app_id(app_id) {
        return Err("Receita de outro AppID ou sem atribuição confirmada: não pode integrar os pacotes deste jogo.".into());
    }
    unit_file_fixes(app_id, u).map(|_| ())
}

pub fn baseline(app_id: &str) -> SmartPack {
    SmartPack {
        availability: Default::default(),
        artifacts: vec![],
        file_fixes: vec![],
        performance_evidence: Default::default(),
        performance_warnings: vec![],
        id: baseline_id(app_id),
        app_id: app_id.to_owned(),
        version: COMPOSER_VERSION.into(),
        name: "Baseline — sem correções do FRX".into(),
        kind: PackKind::Baseline,
        goal: Goal::Reference,
        goal_text: "Configuração de referência: opções e arquivos do jogo sem nenhuma correção ativa do FRX. É contra ela que os outros pacotes são medidos.".into(),
        members: vec![],
        env: vec![],
        apply_recipe_id: None,
        fingerprint: sha256_hex(format!("baseline-v{PACK_SCHEMA}:{app_id}").as_bytes()),
        requirements: vec![],
        compatibility: vec![],
        conflicts_avoided: vec![],
        sources: vec![],
        evidence: EvidenceLevel::None,
        evidence_note: "Referência: não é uma correção.".into(),
        risk: Risk::Low,
        effects: vec![],
        side_effects: vec![],
        reversible: true,
        state: PackState::Candidate,
        state_detail: String::new(),
        plan_status: None,
        operation_id: None,
    }
}

pub(crate) fn make(
    app_id: &str,
    members: &[&Unit],
    kind: PackKind,
    goal: Goal,
    name: String,
    goal_text: String,
    graph: &[Edge],
) -> SmartPack {
    let mut env: Vec<EnvPair> = members.iter().flat_map(|u| u.env.clone()).collect();
    env.sort();
    env.dedup();
    let file_actions: Vec<_> = members
        .iter()
        .flat_map(|u| u.file_actions.clone())
        .collect();
    let file_fixes = units_file_fixes(
        app_id,
        &members.iter().map(|u| (*u).clone()).collect::<Vec<_>>(),
    )
    .expect("composition only accepts validated complete file actions");
    let artifacts = file_actions
        .iter()
        .filter_map(|a| match a {
            RecipeAction::InstallArtifact { artifact } => Some((**artifact).clone()),
            _ => None,
        })
        .collect();
    let id = pack_id_with_files(app_id, &env, &file_fixes);
    let mut sources: Vec<RecipeSource> = Vec::new();
    for s in members.iter().flat_map(|u| u.sources.iter()) {
        if !sources.iter().any(|x| x.url == s.url) {
            sources.push(s.clone());
        }
    }
    let uniq = |v: Vec<String>| {
        let mut out: Vec<String> = Vec::new();
        for x in v {
            if !out.contains(&x) {
                out.push(x);
            }
        }
        out
    };
    let ids: Vec<&str> = members.iter().map(|u| u.recipe_id.as_str()).collect();
    let mut requirements = vec!["Jogo rodando com Proton.".to_owned()];
    if let Some(g) = members.iter().find_map(|u| u.gpu) {
        requirements.push(format!("GPU {}.", vendor_name(g)));
    }
    let mut compatibility = Vec::new();
    for u in members {
        if !u.proton_versions.is_empty() {
            compatibility.push(format!(
                "“{}” foi relatada com {}{}.",
                u.title,
                u.proton_versions.join(", "),
                u.discovered_at
                    .as_deref()
                    .map(|d| format!(" em {}", &d[..d.len().min(10)]))
                    .unwrap_or_default()
            ));
        }
    }
    let evidence = members
        .iter()
        .map(|u| u.evidence)
        .max()
        .unwrap_or(EvidenceLevel::None);
    SmartPack {
        availability: Default::default(),
        artifacts,
        performance_evidence: if members.len() == 1 {
            members[0].performance_evidence.clone()
        } else {
            Default::default()
        },
        performance_warnings: performance_warnings(
            env.iter().map(|p| (p.key.as_str(), p.value.as_str())),
        ),
        apply_recipe_id: Some(if members.len() == 1 {
            members[0].recipe_id.clone()
        } else {
            id.clone()
        }),
        fingerprint: {
            let mut identities: Vec<String> = members
                .iter()
                .map(|u| format!("{}@{}#{}", u.recipe_id, u.version, u.fingerprint))
                .collect();
            identities.sort();
            sha256_hex(
                &serde_json::to_vec(&(PACK_SCHEMA, app_id, &env, &file_fixes, identities))
                    .unwrap_or_default(),
            )
        },
        id,
        app_id: app_id.to_owned(),
        version: COMPOSER_VERSION.into(),
        name,
        kind,
        goal,
        goal_text,
        members: members
            .iter()
            .map(|u| PackMemberView {
                file_actions: u.file_actions.clone(),
                recipe_id: u.recipe_id.clone(),
                version: u.version.clone(),
                fingerprint: u.fingerprint.clone(),
                trust: u.trust,
                title: u.title.clone(),
                env: u.env.clone(),
                evidence: u.evidence,
                discovered_at: u.discovered_at.clone(),
                proton_versions: u.proton_versions.clone(),
            })
            .collect(),
        env,
        file_fixes,
        requirements,
        compatibility,
        conflicts_avoided: graph
            .iter()
            .filter(|e| {
                e.relation == Relation::Excludes
                    && (ids.contains(&e.a.as_str()) || ids.contains(&e.b.as_str()))
            })
            .cloned()
            .collect(),
        sources,
        evidence,
        evidence_note: members
            .iter()
            .map(|u| format!("{}: {}", u.title, u.evidence_note))
            .collect::<Vec<_>>()
            .join(" "),
        risk: members.iter().map(|u| u.risk).max().unwrap_or(Risk::Low),
        effects: uniq(members.iter().flat_map(|u| u.effects.clone()).collect()),
        side_effects: uniq(
            members
                .iter()
                .flat_map(|u| u.side_effects.clone())
                .collect(),
        ),
        reversible: true,
        state: PackState::Candidate,
        state_detail: String::new(),
        plan_status: None,
        operation_id: None,
    }
}
