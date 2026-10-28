//! Catálogo local de receitas, embutido no binário.
//!
//! O manifesto (`recipes/catalog.json`) lista cada arquivo com o SHA-256
//! esperado e as receitas revogadas. Um arquivo cujo hash não confere, que
//! não esteja no manifesto ou que falhe na validação fica fora do catálogo.
//! O campo `signature` está reservado para atualizações assinadas no futuro;
//! hoje nada é baixado — o catálogo só muda com uma nova versão do FRX.

use serde::Deserialize;

use super::files::sha256_hex;
use super::models::{Recipe, Trust};
use super::security::validate_recipe;

const MANIFEST: &str = include_str!("../../recipes/catalog.json");
const FILES: &[(&str, &str)] = &[
    (
        "frx.diagnostics.proton-log.json",
        include_str!("../../recipes/frx.diagnostics.proton-log.json"),
    ),
    (
        "frx.compat.no-esync-fsync.json",
        include_str!("../../recipes/frx.compat.no-esync-fsync.json"),
    ),
    (
        "frx.compat.wined3d.json",
        include_str!("../../recipes/frx.compat.wined3d.json"),
    ),
    (
        "frx.compat.no-d3d11.json",
        include_str!("../../recipes/frx.compat.no-d3d11.json"),
    ),
    (
        "frx.nvidia.gpu-physx.json",
        include_str!("../../recipes/frx.nvidia.gpu-physx.json"),
    ),
];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    schema: u32,
    catalog_version: String,
    recipes: Vec<ManifestEntry>,
    revoked: Vec<String>,
    #[allow(dead_code)]
    signature: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestEntry {
    file: String,
    sha256: String,
}

#[derive(Debug, Clone)]
pub struct Catalog {
    pub version: String,
    pub sha256: String,
    pub recipes: Vec<Recipe>,
    pub issues: Vec<String>,
}

impl Catalog {
    pub fn bundled() -> Self {
        Self::load(MANIFEST, FILES)
    }

    pub fn load(manifest: &str, files: &[(&str, &str)]) -> Self {
        let sha256 = sha256_hex(manifest.as_bytes());
        let m = match serde_json::from_str::<Manifest>(manifest) {
            Ok(m) if m.schema == 1 => m,
            Ok(_) => return Self::empty(sha256, "Manifesto do catálogo em formato desconhecido."),
            Err(e) => return Self::empty(sha256, &format!("Manifesto do catálogo inválido: {e}")),
        };
        let mut recipes: Vec<Recipe> = Vec::new();
        let mut issues = Vec::new();
        for (name, body) in files {
            let Some(entry) = m.recipes.iter().find(|e| e.file == *name) else {
                issues.push(format!("{name}: fora do manifesto, ignorada."));
                continue;
            };
            if sha256_hex(body.as_bytes()) != entry.sha256 {
                issues.push(format!("{name}: hash SHA-256 não confere, ignorada."));
                continue;
            }
            let mut r: Recipe = match serde_json::from_str(body) {
                Ok(r) => r,
                Err(e) => {
                    issues.push(format!("{name}: formato inválido ({e})."));
                    continue;
                }
            };
            if let Err(e) = validate_recipe(&r) {
                issues.push(format!("{name}: {e}"));
                continue;
            }
            if recipes.iter().any(|x| x.id == r.id) {
                issues.push(format!("{name}: identificador duplicado."));
                continue;
            }
            if m.revoked.contains(&r.id) {
                r.trust = Trust::Revoked;
            }
            recipes.push(r);
        }
        for e in &m.recipes {
            if !files.iter().any(|(n, _)| *n == e.file) {
                issues.push(format!("{}: listada no manifesto, mas ausente.", e.file));
            }
        }
        Self {
            version: m.catalog_version,
            sha256,
            recipes,
            issues,
        }
    }

    fn empty(sha256: String, issue: &str) -> Self {
        Self {
            version: "?".into(),
            sha256,
            recipes: vec![],
            issues: vec![issue.into()],
        }
    }

    pub fn get(&self, id: &str) -> Option<&Recipe> {
        self.recipes.iter().find(|r| r.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_is_intact() {
        let c = Catalog::bundled();
        assert!(c.issues.is_empty(), "{:?}", c.issues);
        assert_eq!(c.recipes.len(), FILES.len());
        assert!(
            c.recipes
                .iter()
                .filter(|r| r.trust == Trust::Reviewed)
                .count()
                >= 3
        );
        assert_eq!(
            c.get("frx.compat.no-d3d11").unwrap().trust,
            Trust::Candidate
        );
    }

    #[test]
    fn tampered_or_unlisted_files_are_rejected() {
        let body = FILES[0].1.replace("PROTON_LOG=1", "PROTON_LOG=2");
        let c = Catalog::load(MANIFEST, &[(FILES[0].0, &body), ("extra.json", FILES[1].1)]);
        assert!(c.recipes.is_empty());
        assert!(c.issues.iter().any(|i| i.contains("hash")));
        assert!(c.issues.iter().any(|i| i.contains("fora do manifesto")));
    }

    #[test]
    fn revoked_and_invalid_recipes() {
        let manifest_for = |body: &str, revoked: &str| {
            format!(
                r#"{{"schema":1,"catalogVersion":"t","recipes":[{{"file":"r.json","sha256":"{}"}}],"revoked":[{revoked}],"signature":null}}"#,
                sha256_hex(body.as_bytes())
            )
        };
        let good = FILES[0].1;
        let c = Catalog::load(
            &manifest_for(good, "\"frx.diagnostics.proton-log\""),
            &[("r.json", good)],
        );
        assert_eq!(c.recipes[0].trust, Trust::Revoked);

        // Ação desconhecida (ex.: execute_shell) não existe no formato.
        let shell = good.replace("\"set_launch_env\"", "\"execute_shell\"");
        let c = Catalog::load(&manifest_for(&shell, ""), &[("r.json", &shell)]);
        assert!(
            c.recipes.is_empty() && c.issues[0].contains("formato inválido"),
            "{:?}",
            c.issues
        );

        // Variável fora da allowlist.
        let ld = good.replace("\"PROTON_LOG\"", "\"LD_PRELOAD\"");
        let c = Catalog::load(&manifest_for(&ld, ""), &[("r.json", &ld)]);
        assert!(c.recipes.is_empty());

        // Campo extra desconhecido.
        let extra = good.replacen('{', "{\"command\":\"rm -rf /\",", 1);
        let c = Catalog::load(&manifest_for(&extra, ""), &[("r.json", &extra)]);
        assert!(c.recipes.is_empty());
    }
}
