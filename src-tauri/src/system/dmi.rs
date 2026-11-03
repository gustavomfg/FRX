//! Identidade do laptop pelo DMI exposto pelo kernel em
//! `/sys/devices/virtual/dmi/id`. Esses arquivos (`sys_vendor`,
//! `product_name`, `product_version`, `board_name`, `board_vendor`) são legíveis sem privilégios;
//! os campos sensíveis (`product_serial`, `product_uuid`) nunca são lidos.
//!
//! Muitos firmwares preenchem o DMI com textos genéricos ("To Be Filled By
//! O.E.M.", "System Product Name", "Default string"). Esses valores são
//! tratados como ausentes: identificar errado é pior do que não identificar.

use std::path::Path;

use serde::Serialize;

pub const DMI_DIR: &str = "/sys/devices/virtual/dmi/id";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaptopIdentity {
    /// `sys_vendor`, ou `board_vendor` quando o primeiro é genérico.
    pub vendor: Option<String>,
    /// `product_name`, ou `board_name` quando o primeiro é genérico.
    pub model: Option<String>,
    /// `board_name` cru (já filtrado), útil quando o modelo veio do produto.
    pub board_name: Option<String>,
    /// `product_version`. Na Lenovo é o nome comercial ("ThinkPad X1 Carbon
    /// Gen 9"), enquanto `product_name` é o código do tipo de máquina
    /// ("20XWCTO1WW"). Em outros fabricantes costuma ser só uma revisão.
    pub product_version: Option<String>,
    /// Nome para exibir e pesquisar: `product_version` na Lenovo, senão `model`.
    pub display_name: Option<String>,
}

/// Lê o DMI real do sistema.
pub fn identify_laptop() -> LaptopIdentity {
    identify_from(Path::new(DMI_DIR))
}

/// Lê a identidade a partir de um diretório com o layout do DMI. Separado de
/// [`identify_laptop`] para os testes usarem um DMI fictício.
pub fn identify_from(dir: &Path) -> LaptopIdentity {
    let read = |name: &str| clean(&std::fs::read_to_string(dir.join(name)).ok()?);
    let board_name = read("board_name");
    let vendor = read("sys_vendor").or_else(|| read("board_vendor"));
    let model = read("product_name").or_else(|| board_name.clone());
    let product_version = read("product_version");
    let lenovo = vendor
        .as_deref()
        .is_some_and(|v| v.eq_ignore_ascii_case("lenovo"));
    let display_name = product_version
        .clone()
        .filter(|v| lenovo && v.chars().any(|c| c.is_ascii_alphabetic()))
        .or_else(|| model.clone());
    LaptopIdentity {
        vendor,
        model,
        board_name,
        product_version,
        display_name,
    }
}

fn clean(raw: &str) -> Option<String> {
    let v = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    (!v.is_empty() && !is_placeholder(&v)).then_some(v)
}

fn is_placeholder(v: &str) -> bool {
    const GENERIC: &[&str] = &[
        "to be filled by o.e.m.",
        "to be filled by oem",
        "system product name",
        "system manufacturer",
        "default string",
        "not applicable",
        "not specified",
        "none",
        "oem",
        "o.e.m.",
        "type1productconfigid",
        "123456789",
        "x.x",
    ];
    let lower = v.to_ascii_lowercase();
    GENERIC.contains(&lower.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dmi(files: &[(&str, &str)]) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        for (name, value) in files {
            std::fs::write(tmp.path().join(name), value).unwrap();
        }
        tmp
    }

    #[test]
    fn reads_vendor_model_and_board_from_fictional_laptop() {
        let tmp = dmi(&[
            ("sys_vendor", "Acme Computers\n"),
            ("product_name", "Acme Book 14 (AB14-2026)\n"),
            ("board_name", "AB14-MB\n"),
            ("board_vendor", "Acme Boards\n"),
        ]);
        assert_eq!(
            identify_from(tmp.path()),
            LaptopIdentity {
                vendor: Some("Acme Computers".into()),
                model: Some("Acme Book 14 (AB14-2026)".into()),
                board_name: Some("AB14-MB".into()),
                product_version: None,
                display_name: Some("Acme Book 14 (AB14-2026)".into()),
            }
        );
    }

    #[test]
    fn placeholder_product_name_falls_back_to_board_name() {
        let tmp = dmi(&[
            ("sys_vendor", "To Be Filled By O.E.M.\n"),
            ("product_name", "System Product Name\n"),
            ("board_name", "AB14-MB\n"),
            ("board_vendor", "Acme Boards\n"),
        ]);
        let id = identify_from(tmp.path());
        assert_eq!(id.vendor.as_deref(), Some("Acme Boards"));
        assert_eq!(id.model.as_deref(), Some("AB14-MB"));
        assert_eq!(id.board_name.as_deref(), Some("AB14-MB"));
    }

    #[test]
    fn missing_product_name_falls_back_to_board_name() {
        let tmp = dmi(&[("board_name", "  AB14-MB  \n")]);
        let id = identify_from(tmp.path());
        assert_eq!(id.model.as_deref(), Some("AB14-MB"));
        assert_eq!(id.vendor, None);
    }

    #[test]
    fn empty_or_absent_dmi_identifies_nothing() {
        let tmp = dmi(&[("product_name", "Default string\n"), ("board_name", "\n")]);
        assert_eq!(
            identify_from(tmp.path()),
            LaptopIdentity {
                vendor: None,
                model: None,
                board_name: None,
                product_version: None,
                display_name: None,
            }
        );
        let missing = tmp.path().join("nao-existe");
        assert_eq!(identify_from(&missing).model, None);
    }

    #[test]
    fn lenovo_uses_product_version_as_display_name() {
        let tmp = dmi(&[
            ("sys_vendor", "LENOVO\n"),
            ("product_name", "20XWCTO1WW\n"),
            ("product_version", "ThinkPad X1 Carbon Gen 9\n"),
            ("board_name", "20XWCTO1WW\n"),
        ]);
        let id = identify_from(tmp.path());
        assert_eq!(id.model.as_deref(), Some("20XWCTO1WW"));
        assert_eq!(id.product_version.as_deref(), Some("ThinkPad X1 Carbon Gen 9"));
        assert_eq!(id.display_name.as_deref(), Some("ThinkPad X1 Carbon Gen 9"));
    }

    #[test]
    fn other_vendors_keep_product_name_even_with_a_revision() {
        let tmp = dmi(&[
            ("sys_vendor", "Acme Computers\n"),
            ("product_name", "Acme Book 14\n"),
            ("product_version", "1.0\n"),
        ]);
        let id = identify_from(tmp.path());
        assert_eq!(id.product_version.as_deref(), Some("1.0"));
        assert_eq!(id.display_name.as_deref(), Some("Acme Book 14"));
    }
}
