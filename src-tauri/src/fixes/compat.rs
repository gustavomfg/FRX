//! Relaciona cada descoberta com o hardware e a ferramenta do usuário.
//!
//! Nunca descartamos uma descoberta por falta de dados: quando não dá para
//! decidir, o estado é `Unknown` e a nota explica o motivo.

use super::models::{ActionKind, Compatibility, FindingKind, FixFinding, GpuReq, HardwareScope};
use super::pyscan::gpu_for_env;
use crate::fix_engine::dynamic::compatibility::{knowledge, vendor_bound};
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

/// Escopo de um conjunto de variáveis. Só vira `Any` se todas forem conhecidas e
/// nenhuma exigir fabricante. Fabricantes diferentes não definem um escopo único.
pub fn scope_of_env<'a>(pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> HardwareScope {
    let mut vendor: Option<GpuReq> = None;
    let mut any_pair = false;
    let mut all_known = true;
    for (key, value) in pairs {
        any_pair = true;
        let known = knowledge(key, value);
        all_known &= known.is_some();
        let required = known
            .and_then(|k| k.gpu)
            .or(vendor_bound(key))
            .or(gpu_for_env(key));
        match (vendor, required) {
            (Some(a), Some(b)) if a != b => return HardwareScope::Unknown,
            (None, Some(b)) => vendor = Some(b),
            _ => {}
        }
    }
    match vendor {
        Some(v) => v.into(),
        None if any_pair && all_known => HardwareScope::Any,
        None => HardwareScope::Unknown,
    }
}

/// Escopo de ações escritas como `NOME=valor`. Texto em outro formato é desconhecido.
pub fn scope_of_changes(changes: &[String]) -> HardwareScope {
    match changes
        .iter()
        .map(|c| c.split_once('='))
        .collect::<Option<Vec<_>>>()
    {
        Some(pairs) => scope_of_env(pairs),
        None => HardwareScope::Unknown,
    }
}

/// O requisito da fonte vale primeiro; sem ele, valem as variáveis das ações.
pub fn finding_scope(f: &FixFinding) -> HardwareScope {
    if let Some(vendor) = f
        .requirements
        .gpu_vendor
        .or(f.requirements.hardware.gpu_vendor)
    {
        return vendor.into();
    }
    let mut pairs = Vec::new();
    for action in &f.actions {
        match action.kind {
            ActionKind::Env => match action.value.split_once('=') {
                Some(pair) => pairs.push(pair),
                None => return HardwareScope::Unknown,
            },
            ActionKind::LaunchOption => pairs.extend(
                action
                    .value
                    .split_whitespace()
                    .take_while(|t| *t != "%command%")
                    .filter_map(|t| t.split_once('=')),
            ),
            _ => return HardwareScope::Unknown,
        }
    }
    scope_of_env(pairs)
}

/// Avalia a compatibilidade de hardware. Não sobrescreve `AlreadyIncluded`.
pub fn assess(f: &mut FixFinding, hw: &HardwareContext) {
    f.hardware_scope = finding_scope(f);
    if f.compatibility == Compatibility::AlreadyIncluded {
        return;
    }
    if !f.requirements.hardware.is_empty() {
        let context = crate::system::context::EnvironmentContext {
            detected_gpus: hw.gpus.clone(),
            cpu_arch: Some(std::env::consts::ARCH.into()),
            ..Default::default()
        };
        let matched = f.requirements.hardware.evaluate(Some(&context));
        f.compatibility = match matched.state {
            crate::system::context::MatchState::Compatible => Compatibility::Compatible,
            crate::system::context::MatchState::Unknown => Compatibility::Unknown,
            crate::system::context::MatchState::Incompatible => Compatibility::Incompatible,
        };
        f.compatibility_note = matched
            .reasons
            .into_iter()
            .chain(matched.limitations)
            .collect::<Vec<_>>()
            .join(" ");
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
    if primary == Some(req) && hw.gpus.len() == 1 {
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
            pci_address: None,
            gpu_family: None,
            vram_bytes: None,
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
            hardware_scope: Default::default(),
            artifacts: vec![],
            scope: DiscoveryScope::default(),
            performance_evidence: PerformanceEvidence::default(),
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
        assert_eq!(nv_fix.compatibility, Compatibility::Unknown);
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

#[cfg(test)]
mod hardware_scope_tests {
    use super::*;

    #[test]
    fn scope_comes_from_known_variables_and_their_vendor() {
        for (pairs, expected) in [
            (vec![("PROTON_ENABLE_NVAPI", "1")], HardwareScope::Nvidia),
            (vec![("PROTON_DLSS_UPGRADE", "1")], HardwareScope::Nvidia),
            (vec![("RADV_PERFTEST", "aco")], HardwareScope::Amd),
            (vec![("PROTON_NO_ESYNC", "1")], HardwareScope::Any),
            (
                vec![("PROTON_NO_ESYNC", "1"), ("PROTON_ENABLE_NVAPI", "1")],
                HardwareScope::Nvidia,
            ),
            // Fora da tabela: a fonte não diz para quem é.
            (vec![("PROTON_INVENTADA", "1")], HardwareScope::Unknown),
            (
                vec![("RADV_PERFTEST", "aco"), ("PROTON_ENABLE_NVAPI", "1")],
                HardwareScope::Unknown,
            ),
            (vec![], HardwareScope::Unknown),
        ] {
            assert_eq!(scope_of_env(pairs.clone()), expected, "{pairs:?}");
        }
        assert_eq!(
            scope_of_changes(&["settings.ini".into()]),
            HardwareScope::Unknown
        );
    }

    #[test]
    fn rank_puts_the_users_vendor_first_and_a_mismatch_last() {
        use HardwareScope::*;
        let nvidia = Some(GpuReq::Nvidia);
        assert_eq!(Nvidia.rank(nvidia), 0);
        assert_eq!(Any.rank(nvidia), 1);
        assert_eq!(Unknown.rank(nvidia), 2);
        assert_eq!(Amd.rank(nvidia), 3);
        assert_eq!(Amd.rank(None), 2);
    }
}
