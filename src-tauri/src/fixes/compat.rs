//! Relaciona cada descoberta com o hardware e a ferramenta do usuário.
//!
//! Nunca descartamos uma descoberta por falta de dados: quando não dá para
//! decidir, o estado é `Unknown` e a nota explica o motivo.

use super::models::{Compatibility, FindingKind, FixFinding, GpuReq};
use crate::system::{GpuInfo, GpuVendor};

#[derive(Debug, Clone, Default)]
pub struct HardwareContext {
    pub gpus: Vec<GpuInfo>,
    pub primary: Option<GpuInfo>,
}

impl HardwareContext {
    pub fn primary_req(&self) -> Option<GpuReq> {
        self.primary.as_ref().and_then(|g| to_req(g.vendor))
    }
}

pub fn to_req(v: GpuVendor) -> Option<GpuReq> {
    match v {
        GpuVendor::Nvidia => Some(GpuReq::Nvidia),
        GpuVendor::Amd => Some(GpuReq::Amd),
        GpuVendor::Intel => Some(GpuReq::Intel),
        GpuVendor::Other => None,
    }
}

pub fn vendor_name(r: GpuReq) -> &'static str {
    match r {
        GpuReq::Nvidia => "NVIDIA",
        GpuReq::Amd => "AMD",
        GpuReq::Intel => "Intel",
    }
}

/// Avalia a compatibilidade de hardware. Não sobrescreve `AlreadyIncluded`.
pub fn assess(f: &mut FixFinding, hw: &HardwareContext) {
    if f.compatibility == Compatibility::AlreadyIncluded {
        return;
    }
    let Some(req) = f.requirements.gpu_vendor else {
        // Sem requisito explícito.
        if f.kind == FindingKind::Protonfix || f.kind == FindingKind::ReleaseNote {
            f.compatibility = Compatibility::Compatible;
            if f.compatibility_note.is_empty() {
                f.compatibility_note =
                    "A fonte não indica requisitos específicos de hardware.".into();
            }
        } else {
            f.compatibility = Compatibility::Unknown;
            if f.compatibility_note.is_empty() {
                f.compatibility_note =
                    "Os relatos não informam hardware suficiente para confirmar a compatibilidade."
                        .into();
            }
        }
        return;
    };
    if hw.gpus.is_empty() {
        f.compatibility = Compatibility::Unknown;
        f.compatibility_note = format!(
            "Citada para GPUs {}, mas não foi possível identificar a sua GPU.",
            vendor_name(req)
        );
        return;
    }
    let has = hw.gpus.iter().any(|g| to_req(g.vendor) == Some(req));
    let primary = hw.primary_req();
    let primary_name = hw
        .primary
        .as_ref()
        .and_then(|g| g.name.clone())
        .unwrap_or_else(|| {
            primary
                .map(vendor_name)
                .unwrap_or("desconhecida")
                .to_owned()
        });
    if primary == Some(req) {
        f.compatibility = Compatibility::Compatible;
        f.compatibility_note = format!(
            "Citada para GPUs {} — a sua é {}.",
            vendor_name(req),
            primary_name
        );
    } else if has {
        f.compatibility = Compatibility::Unknown;
        f.compatibility_note = format!(
            "Citada para GPUs {}. Você tem uma, mas a GPU principal parece ser {} — depende de qual delas roda o jogo.",
            vendor_name(req),
            primary_name
        );
    } else {
        f.compatibility = Compatibility::Incompatible;
        f.compatibility_note = format!(
            "Citada apenas para GPUs {}; seu sistema usa {}.",
            vendor_name(req),
            primary_name
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixes::models::*;

    fn gpu(v: GpuVendor, name: &str) -> GpuInfo {
        GpuInfo {
            vendor: v,
            vendor_id: String::new(),
            device_id: String::new(),
            name: Some(name.into()),
            driver: None,
            driver_version: None,
            boot_vga: false,
        }
    }

    pub fn finding(kind: FindingKind, req: Option<GpuReq>) -> FixFinding {
        FixFinding {
            id: "x".into(),
            game_app_id: "1".into(),
            title: "t".into(),
            description: String::new(),
            category: Category::Other,
            kind,
            actions: vec![],
            sources: vec![],
            requirements: Requirements {
                gpu_vendor: req,
                ..Default::default()
            },
            confidence: Confidence::Unverified,
            compatibility: Compatibility::Unknown,
            compatibility_note: String::new(),
            experimental: false,
            analysis: AnalysisState::Parsed,
            evidence: Evidence::default(),
            published_at: None,
            retrieved_at: String::new(),
            keys: vec![],
        }
    }

    #[test]
    fn nvidia_only_fix_on_amd_is_incompatible() {
        let hw = HardwareContext {
            gpus: vec![gpu(GpuVendor::Amd, "RX 7800 XT")],
            primary: Some(gpu(GpuVendor::Amd, "RX 7800 XT")),
        };
        let mut f = finding(FindingKind::Tweak, Some(GpuReq::Nvidia));
        assess(&mut f, &hw);
        assert_eq!(f.compatibility, Compatibility::Incompatible);
    }

    #[test]
    fn hybrid_systems_are_unknown_not_rejected() {
        let nv = gpu(GpuVendor::Nvidia, "RTX 4060");
        let hw = HardwareContext {
            gpus: vec![gpu(GpuVendor::Amd, "Vega"), nv.clone()],
            primary: Some(nv),
        };
        let mut amd_fix = finding(FindingKind::Tweak, Some(GpuReq::Amd));
        assess(&mut amd_fix, &hw);
        assert_eq!(amd_fix.compatibility, Compatibility::Unknown);
        let mut nv_fix = finding(FindingKind::Tweak, Some(GpuReq::Nvidia));
        assess(&mut nv_fix, &hw);
        assert_eq!(nv_fix.compatibility, Compatibility::Compatible);
    }

    #[test]
    fn missing_data_stays_unknown_and_included_is_preserved() {
        let mut t = finding(FindingKind::Tweak, None);
        assess(&mut t, &HardwareContext::default());
        assert_eq!(t.compatibility, Compatibility::Unknown);
        let mut p = finding(FindingKind::Protonfix, Some(GpuReq::Nvidia));
        assess(&mut p, &HardwareContext::default());
        assert_eq!(p.compatibility, Compatibility::Unknown);
        let mut inc = finding(FindingKind::Protonfix, Some(GpuReq::Amd));
        inc.compatibility = Compatibility::AlreadyIncluded;
        assess(&mut inc, &HardwareContext::default());
        assert_eq!(inc.compatibility, Compatibility::AlreadyIncluded);
    }
}
