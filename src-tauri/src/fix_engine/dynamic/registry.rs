//! Registro local das análises e receitas geradas
//! (`fix-engine/generated/<appid>.json`, pasta 0700).
//!
//! Evita refazer a análise quando nada mudou e registra quando e por que uma
//! receita foi reavaliada. Uma entrada só é reaproveitada se **todas** as
//! entradas forem iguais: conteúdo da descoberta, ferramenta (Proton), GPU,
//! driver, versão do gerador, versão das regras e catálogo. Qualquer mudança
//! força uma nova análise — receitas antigas nunca são tratadas como
//! compatíveis depois de uma mudança relevante.
//!
//! O registro é cache de exibição e histórico: a execução sempre regenera a
//! receita a partir do cache da pesquisa (`analyzer::resolve`). Um arquivo
//! adulterado aqui não consegue mudar o que é gravado no jogo.
//!
//! Formato pensado para sincronizar com um catálogo remoto no futuro (mesma
//! receita, mesmo ID estável); nenhum servidor é usado nesta versão.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{Assessment, GENERATOR_VERSION, RULES_VERSION};
use crate::fix_engine::files::{atomic_write, private_dir, sha256_hex};
use crate::fixes::models::{FixFinding, GpuReq};
use crate::time::now_iso;

pub const REGISTRY_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Inputs {
    #[serde(default)]
    pub game_build_id: Option<String>,
    pub finding: String,
    pub tool: Option<String>,
    pub gpu: Option<GpuReq>,
    pub driver: Option<String>,
    pub generator: String,
    pub rules: String,
    pub catalog: String,
}

impl Inputs {
    /// O que mudou em relação à análise anterior (para explicar a reavaliação).
    fn diff(&self, old: &Inputs) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.game_build_id != old.game_build_id {
            v.push("o build do jogo");
        }
        if self.finding != old.finding {
            v.push("a descoberta");
        }
        if self.tool != old.tool {
            v.push("o Proton");
        }
        if self.gpu != old.gpu || self.driver != old.driver {
            v.push("o driver ou a GPU");
        }
        if self.generator != old.generator {
            v.push("a versão do gerador");
        }
        if self.rules != old.rules {
            v.push("as regras de segurança");
        }
        if self.catalog != old.catalog {
            v.push("o catálogo do FRX");
        }
        v
    }
}

/// Contexto do sistema que entra nas `Inputs` de cada descoberta.
pub struct System<'a> {
    pub game_build_id: Option<&'a str>,
    pub tool: Option<&'a str>,
    pub gpu: Option<GpuReq>,
    pub driver: Option<&'a str>,
    pub catalog: &'a str,
}

pub fn inputs(f: &FixFinding, sys: &System<'_>) -> Inputs {
    Inputs {
        game_build_id: sys.game_build_id.map(str::to_owned),
        finding: sha256_hex(&serde_json::to_vec(f).unwrap_or_default())[..32].to_owned(),
        tool: sys.tool.map(str::to_owned),
        gpu: sys.gpu,
        driver: sys.driver.map(str::to_owned),
        generator: GENERATOR_VERSION.into(),
        rules: RULES_VERSION.into(),
        catalog: sys.catalog.to_owned(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub finding_id: String,
    pub inputs: Inputs,
    pub assessment: Assessment,
    pub schema_version: u32,
    pub discovered_at: Option<String>,
    pub generated_at: String,
    pub updated_at: String,
    /// Por que a última análise foi refeita (vazio = primeira análise).
    pub reevaluated: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryFile {
    schema: u32,
    app_id: String,
    entries: Vec<Entry>,
}

pub struct Registry {
    dir: PathBuf,
}

impl Registry {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn path(&self, app_id: &str) -> Option<PathBuf> {
        crate::art::valid_app_id(app_id).then(|| self.dir.join(format!("{app_id}.json")))
    }

    pub fn load(&self, app_id: &str) -> Vec<Entry> {
        self.path(app_id)
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice::<RegistryFile>(&b).ok())
            .filter(|f| f.schema == REGISTRY_SCHEMA && f.app_id == app_id)
            .map(|f| {
                f.entries
                    .into_iter()
                    .filter(|e| {
                        e.assessment.app_id == app_id
                            && e.assessment.recipe.as_ref().is_none_or(|r| {
                                r.scope.app_ids == [app_id]
                                    && r.generated
                                        .as_ref()
                                        .is_some_and(|g| g.game_app_id.as_deref() == Some(app_id))
                            })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Reaproveita análises cujas entradas não mudaram; refaz as outras com
    /// `assess` e grava o registro. Devolve as entradas na ordem das descobertas.
    pub fn refresh(
        &self,
        app_id: &str,
        findings: &[FixFinding],
        sys: &System<'_>,
        assess: impl Fn(&FixFinding) -> Assessment,
    ) -> Result<(Vec<Entry>, usize), String> {
        let old = self.load(app_id);
        let now = now_iso();
        let mut regenerated = 0;
        let entries: Vec<Entry> = findings
            .iter()
            .map(|f| {
                let inp = inputs(f, sys);
                let discovered_at = f
                    .published_at
                    .clone()
                    .or_else(|| Some(f.retrieved_at.clone()));
                match old.iter().find(|e| e.finding_id == f.id) {
                    Some(e) if e.inputs == inp => e.clone(),
                    prev => {
                        regenerated += 1;
                        let assessment = assess(f);
                        let same_recipe =
                            prev.is_some_and(|p| p.assessment.recipe_id == assessment.recipe_id);
                        Entry {
                            finding_id: f.id.clone(),
                            reevaluated: prev.map(|p| {
                                let what = inp.diff(&p.inputs);
                                format!(
                                    "Reavaliada porque mudou {}.",
                                    if what.is_empty() {
                                        "o formato do registro".into()
                                    } else {
                                        what.join(", ")
                                    }
                                )
                            }),
                            generated_at: prev
                                .filter(|_| same_recipe)
                                .map(|p| p.generated_at.clone())
                                .unwrap_or_else(|| now.clone()),
                            inputs: inp,
                            assessment,
                            schema_version: REGISTRY_SCHEMA,
                            discovered_at,
                            updated_at: now.clone(),
                        }
                    }
                }
            })
            .collect();
        if regenerated > 0 || entries.len() != old.len() {
            let path = self.path(app_id).ok_or("AppID inválido")?;
            private_dir(&self.dir)?;
            let file = RegistryFile {
                schema: REGISTRY_SCHEMA,
                app_id: app_id.to_owned(),
                entries: entries.clone(),
            };
            let json = serde_json::to_vec_pretty(&file).map_err(|e| e.to_string())?;
            atomic_write(&path, &json, 0o600)?;
        }
        Ok((entries, regenerated))
    }
}
