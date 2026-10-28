//! Monta a receita estruturada a partir de ações já validadas. O formato é o
//! mesmo das receitas do catálogo — o planejador, o backup e o rollback não
//! distinguem uma da outra, exceto pelo estado de confiança `generated`.

use super::compatibility::{Knowledge, PROTON_README};
use super::provenance::Provenance;
use super::{EvidenceLevel, GENERATOR_VERSION, RULES_VERSION};
use crate::fix_engine::files::sha256_hex;
use crate::fix_engine::models::*;
use crate::fixes::models::{FindingKind, FixFinding};

/// Identificador estável: AppID + ações normalizadas (ordenadas). Não
/// depende do texto exibido nem da ordem em que a fonte citou as variáveis.
pub fn recipe_id(app_id: &str, actions: &[(String, String)]) -> String {
    let mut norm: Vec<String> = actions.iter().map(|(k, v)| format!("{k}={v}")).collect();
    norm.sort();
    norm.dedup();
    let h = sha256_hex(format!("{app_id}\n{}", norm.join("\n")).as_bytes());
    format!("gen.{app_id}.{}", &h[..16])
}

pub struct Input<'a> {
    pub finding: &'a FixFinding,
    pub app_id: &'a str,
    pub actions: Vec<(String, String, &'static Knowledge)>,
    pub evidence: EvidenceLevel,
    pub evidence_note: String,
    pub performance_evidence: super::GamePerformanceEvidence,
    pub provenance: Vec<Provenance>,
    pub interpreter: &'static str,
    /// Opções da fonte que a ferramenta instalada não lê (sem efeito aqui).
    pub omitted: Vec<String>,
}

fn cut(s: String, max: usize) -> String {
    if s.chars().count() <= max {
        s
    } else {
        s.chars().take(max - 1).collect::<String>() + "…"
    }
}

pub fn build(i: Input<'_>) -> Recipe {
    let pairs: Vec<(String, String)> = i
        .actions
        .iter()
        .map(|(k, v, _)| (k.clone(), v.clone()))
        .collect();
    let id = recipe_id(i.app_id, &pairs);
    let ks: Vec<&Knowledge> = i.actions.iter().map(|(_, _, k)| *k).collect();
    let line = pairs
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(" ");
    let title = cut(
        ks.iter().map(|k| k.title).collect::<Vec<_>>().join(" + "),
        120,
    );
    let first = i.provenance.first();
    let description = cut(
        format!(
            "Gerada automaticamente pelo FRX a partir de {} ({}). As ações foram reconhecidas e validadas pelo núcleo, mas a receita não foi revisada por pessoas.{}",
            if i.provenance.len() == 1 { "uma fonte pública".to_owned() } else { format!("{} fontes públicas", i.provenance.len()) },
            first.map(|p| p.label.as_str()).unwrap_or("fonte"),
            if i.omitted.is_empty() { String::new() } else { format!(" Omitidas por não terem efeito na ferramenta instalada: {}.", i.omitted.join(", ")) }
        ),
        1200,
    );
    let mut sources: Vec<RecipeSource> = Vec::new();
    for p in &i.provenance {
        if !sources.iter().any(|s| s.url == p.url) {
            sources.push(RecipeSource {
                label: p.label.clone(),
                url: p.url.clone(),
            });
        }
    }
    sources.truncate(7);
    sources.push(RecipeSource {
        label: "Proton — opções de execução (README oficial)".into(),
        url: PROTON_README.into(),
    });
    let discovered_at = i
        .provenance
        .iter()
        .filter_map(|p| p.published_at.clone())
        .max();
    Recipe {
        hardware: i.finding.requirements.hardware.clone(),
        schema: RECIPE_SCHEMA,
        id,
        version: GENERATOR_VERSION.into(),
        trust: Trust::Generated,
        title,
        summary: cut(
            format!("Define {line} nas opções de inicialização deste jogo."),
            400,
        ),
        description,
        category: ks[0].category,
        purpose: if ks.iter().all(|k| k.purpose == ks[0].purpose) {
            ks[0].purpose
        } else {
            Purpose::Compatibility
        },
        risk: ks.iter().map(|k| k.risk).max().unwrap_or(Risk::Low),
        scope: Scope {
            app_ids: vec![i.app_id.to_owned()],
            requires_proton: true,
            gpu_vendor: ks.iter().find_map(|k| k.gpu),
        },
        actions: pairs
            .iter()
            .map(|(k, v)| RecipeAction::SetLaunchEnv {
                key: k.clone(),
                value: v.clone(),
            })
            .collect(),
        already_included_by_protonfixes: i.finding.kind == FindingKind::Protonfix,
        conflicts_with: vec![],
        related_keys: pairs.iter().map(|(k, v)| format!("env:{k}={v}")).collect(),
        effects: i
            .actions
            .iter()
            .map(|(_, v, k)| super::compatibility::describe(k, v))
            .collect(),
        side_effects: ks
            .iter()
            .filter_map(|k| k.side_effect.map(str::to_owned))
            .collect(),
        sources,
        review: None,
        pack: None,
        generated: Some(GeneratedInfo {
            game_app_id: Some(i.app_id.to_owned()),
            performance_evidence: i.performance_evidence,
            generator_version: GENERATOR_VERSION.into(),
            rules_version: RULES_VERSION.into(),
            interpreter: i.interpreter.into(),
            finding_ids: vec![i.finding.id.clone()],
            evidence: i.evidence,
            evidence_note: i.evidence_note,
            provenance: i.provenance,
            discovered_at,
        }),
    }
}

/// Preserve legacy source restrictions when translating a complete structured
/// procedure. Two declarations are conjunctive, never alternative permissions.
pub fn structured_hardware(
    f: &FixFinding,
) -> Result<crate::system::context::HardwareRequirements, String> {
    let mut hardware = f.requirements.hardware.clone();
    if let Some(legacy) = f.requirements.gpu_vendor {
        if hardware.gpu_vendor.is_some_and(|vendor| vendor != legacy) {
            return Err(
                "A fonte declara fabricantes de GPU contraditórios; requer revisão.".into(),
            );
        }
        hardware.gpu_vendor = Some(legacy);
    }
    if !f.requirements.proton_versions.is_empty() {
        if hardware.proton_versions.is_empty() {
            hardware.proton_versions = f.requirements.proton_versions.clone();
        } else {
            hardware.proton_versions.retain(|version| {
                f.requirements
                    .proton_versions
                    .iter()
                    .any(|legacy| legacy.eq_ignore_ascii_case(version))
            });
            if hardware.proton_versions.is_empty() {
                return Err(
                    "A fonte declara versões do Proton contraditórias; requer revisão.".into(),
                );
            }
        }
    }
    Ok(hardware)
}

pub fn artifact_recipe(
    f: &FixFinding,
    app: &str,
    provenance: Vec<Provenance>,
    hardware: crate::system::context::HardwareRequirements,
) -> Recipe {
    let hash =
        sha256_hex(&serde_json::to_vec(&(app, &f.artifacts, &f.requirements)).unwrap_or_default());
    Recipe{
        hardware,schema:RECIPE_SCHEMA,id:format!("gen.{app}.{}",&hash[..16]),version:GENERATOR_VERSION.into(),trust:Trust::Generated,
        title:cut(f.title.clone(),120),summary:cut(format!("Baixa e instala {} artefato(s) de configuração para este jogo.",f.artifacts.len()),400),
        description:"Gerada de metadados e instruções explícitas do publicador. A origem e o SHA-256 não garantem conteúdo benigno. A confirmação autoriza somente os destinos da prévia.".into(),
        category:f.category,purpose:Purpose::Compatibility,risk:Risk::Moderate,
        scope:Scope{app_ids:vec![app.into()],requires_proton:false,gpu_vendor:f.requirements.gpu_vendor},
        actions:f.artifacts.iter().cloned().map(|artifact|RecipeAction::InstallArtifact{artifact:Box::new(artifact)}).collect(),already_included_by_protonfixes:false,
        conflicts_with:vec![],related_keys:vec![],effects:vec!["Instala somente os arquivos selecionados nos destinos relativos à instalação detectada.".into()],side_effects:vec!["Pode alterar o comportamento do jogo. Uma atualização Steam pode sobrescrever a correção; não há reaplicação automática.".into()],
        sources:f.sources.iter().take(8).map(|s|RecipeSource{label:s.label.clone(),url:s.url.clone()}).collect(),review:None,pack:None,
        generated:Some(GeneratedInfo{game_app_id:Some(app.into()),performance_evidence:Default::default(),generator_version:GENERATOR_VERSION.into(),rules_version:RULES_VERSION.into(),interpreter:"publisher-artifact-metadata".into(),finding_ids:vec![f.id.clone()],evidence:EvidenceLevel::Documented,evidence_note:"Publicador, versão, digest e destinos identificados; nenhuma melhoria de FPS foi medida por essa identificação.".into(),provenance,discovered_at:f.published_at.clone()}),
    }
}
