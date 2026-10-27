//! Verifiable hardware/software scope. Detected devices are inventory, not proof
//! of which GPU renders the game. Optional facts stay unknown until observed.

use serde::{Deserialize, Serialize};

use super::{GpuInfo, GpuVendor, SystemProfile};
use crate::fixes::models::GpuReq;
use crate::steam::models::CompatTool;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum GpuSelection {
    #[default]
    Unknown,
    Measured,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentContext {
    #[serde(default)]
    pub detected_gpus: Vec<GpuInfo>,
    pub selected_gpu: Option<GpuInfo>,
    #[serde(default)]
    pub selection: GpuSelection,
    #[serde(default)]
    pub installed_vulkan_drivers: Vec<String>,
    /// An installed ICD is not proof of the driver active in the game.
    pub active_vulkan_driver: Option<String>,
    pub cpu_arch: Option<String>,
    pub session_type: Option<String>,
    pub proton_version: Option<String>,
    pub proton_origin: Option<String>,
}

impl EnvironmentContext {
    pub fn observed_gpu(&self) -> Option<&GpuInfo> {
        self.selected_gpu
            .as_ref()
            .filter(|_| self.selection == GpuSelection::Measured)
    }
    pub fn from_profile(profile: &SystemProfile, tool: Option<&CompatTool>) -> Self {
        Self {
            detected_gpus: profile.gpus.clone(),
            installed_vulkan_drivers: profile.vulkan_drivers.clone(),
            cpu_arch: Some(profile.arch.into()),
            session_type: profile.session_type.clone(),
            proton_version: tool
                .map(|t| t.version.clone().unwrap_or_else(|| t.display_name.clone())),
            proton_origin: tool
                .and_then(|t| serde_json::to_value(t.origin).ok())
                .and_then(|v| v.as_str().map(str::to_owned)),
            ..Default::default()
        }
    }

    /// The logger gives a device name. We enrich it only when exactly one
    /// detected physical device matches; duplicate boards remain ambiguous.
    pub fn with_observed_gpu(mut self, name: &str) -> Self {
        let matches: Vec<_> = self
            .detected_gpus
            .iter()
            .filter(|g| {
                g.name
                    .as_deref()
                    .is_some_and(|n| crate::performance::comparison::same_condition("gpu", n, name))
            })
            .collect();
        if let [gpu] = matches.as_slice() {
            self.selected_gpu = Some((*gpu).clone());
            self.selection = GpuSelection::Measured;
        } else {
            self.selected_gpu = None;
            self.selection = GpuSelection::Unknown;
        }
        self
    }

    pub fn selection_limitations(&self) -> Vec<String> {
        if self.observed_gpu().is_none() && self.detected_gpus.len() > 1 {
            vec![
                "Há várias GPUs detectadas; o FRX ainda não confirmou qual delas executa o jogo."
                    .into(),
            ]
        } else if self.observed_gpu().is_none() {
            vec!["A GPU foi detectada no sistema, mas sua utilização pelo jogo ainda não foi medida.".into()]
        } else {
            vec![]
        }
    }

    /// Does not include other GPU inventory: a second unused board must not
    /// invalidate a measured selected device. Callers compare optional fields.
    pub fn cohort_differences(&self, other: &Self) -> Vec<String> {
        let mut differences = Vec::new();
        fn device(e: &EnvironmentContext) -> Option<&GpuInfo> {
            e.observed_gpu().or_else(|| {
                if e.detected_gpus.len() == 1 {
                    e.detected_gpus.first()
                } else {
                    None
                }
            })
        }
        if (self.observed_gpu().is_some()
            && other.observed_gpu().is_none()
            && other.detected_gpus.len() > 1)
            || (other.observed_gpu().is_some()
                && self.observed_gpu().is_none()
                && self.detected_gpus.len() > 1)
        {
            differences.push("GPU de execução não confirmada no sistema com várias GPUs".into());
        }
        if let (Some(a), Some(b)) = (device(self), device(other)) {
            for (field, x, y) in [
                (
                    "fabricante da GPU",
                    Some(a.vendor_id.as_str()),
                    Some(b.vendor_id.as_str()),
                ),
                (
                    "modelo PCI da GPU",
                    Some(a.device_id.as_str()),
                    Some(b.device_id.as_str()),
                ),
                (
                    "GPU física",
                    a.pci_address.as_deref(),
                    b.pci_address.as_deref(),
                ),
                (
                    "driver da GPU",
                    a.driver_version.as_deref(),
                    b.driver_version.as_deref(),
                ),
            ] {
                if let (Some(x), Some(y)) = (x, y) {
                    if !x.eq_ignore_ascii_case(y) {
                        differences.push(field.into());
                    }
                }
            }
        }
        for (label, a, b) in [
            (
                "driver Vulkan ativo",
                &self.active_vulkan_driver,
                &other.active_vulkan_driver,
            ),
            ("arquitetura da CPU", &self.cpu_arch, &other.cpu_arch),
            ("sessão gráfica", &self.session_type, &other.session_type),
            (
                "versão do Proton",
                &self.proton_version,
                &other.proton_version,
            ),
            (
                "origem do Proton",
                &self.proton_origin,
                &other.proton_origin,
            ),
        ] {
            if let (Some(a), Some(b)) = (a, b) {
                if !a.eq_ignore_ascii_case(b) {
                    differences.push(label.into());
                }
            }
        }
        differences
    }
}

/// Exact, source-declared requirements. Version ranges are not guessed.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HardwareRequirements {
    pub gpu_vendor: Option<GpuReq>,
    #[serde(default)]
    pub gpu_device_ids: Vec<String>,
    #[serde(default)]
    pub gpu_models: Vec<String>,
    #[serde(default)]
    pub driver_versions: Vec<String>,
    #[serde(default)]
    pub proton_versions: Vec<String>,
    pub cpu_arch: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MatchState {
    Compatible,
    Unknown,
    Incompatible,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HardwareMatch {
    pub state: MatchState,
    pub reasons: Vec<String>,
    pub limitations: Vec<String>,
}

fn vendor(v: GpuVendor) -> Option<GpuReq> {
    match v {
        GpuVendor::Nvidia => Some(GpuReq::Nvidia),
        GpuVendor::Amd => Some(GpuReq::Amd),
        GpuVendor::Intel => Some(GpuReq::Intel),
        GpuVendor::Other => None,
    }
}

impl HardwareRequirements {
    pub fn is_empty(&self) -> bool {
        self.gpu_vendor.is_none()
            && self.gpu_device_ids.is_empty()
            && self.gpu_models.is_empty()
            && self.driver_versions.is_empty()
            && self.proton_versions.is_empty()
            && self.cpu_arch.is_none()
    }

    pub fn evaluate(&self, ctx: Option<&EnvironmentContext>) -> HardwareMatch {
        let mut result = HardwareMatch {
            state: MatchState::Compatible,
            reasons: vec![],
            limitations: vec![],
        };
        if self.is_empty() {
            return result;
        }
        let Some(ctx) = ctx else {
            result.state = MatchState::Unknown;
            result.limitations.push(
                "Não há contexto de hardware confirmado para avaliar os requisitos da fonte."
                    .into(),
            );
            return result;
        };
        // A single inventory device is useful to check applicability, but still
        // does not claim it was observed rendering the game.
        let gpu = ctx.observed_gpu().or_else(|| {
            if ctx.detected_gpus.len() == 1 {
                ctx.detected_gpus.first()
            } else {
                None
            }
        });
        let mut check = |label: &str, required: bool, known: Option<bool>| {
            if !required {
                return;
            }
            match known {
                Some(true) => result
                    .reasons
                    .push(format!("{label}: corresponde ao requisito documentado.")),
                Some(false) => {
                    result.state = MatchState::Incompatible;
                    result
                        .reasons
                        .push(format!("{label}: diferente do requisito documentado."));
                }
                None => {
                    if result.state != MatchState::Incompatible {
                        result.state = MatchState::Unknown;
                    }
                    result.limitations.push(format!(
                        "{label}: não confirmado; não foi presumido compatível."
                    ));
                }
            }
        };
        check(
            "Fabricante da GPU",
            self.gpu_vendor.is_some(),
            gpu.and_then(|g| vendor(g.vendor))
                .map(|v| Some(v) == self.gpu_vendor),
        );
        check(
            "Modelo PCI da GPU",
            !self.gpu_device_ids.is_empty(),
            gpu.filter(|g| !g.device_id.is_empty()).and_then(|g| {
                let determinate = self.gpu_vendor.is_some()
                    || self.gpu_device_ids.iter().all(|id| id.contains(':'));
                determinate.then(|| {
                    self.gpu_device_ids.iter().any(|id| {
                        if let Some((vendor, device)) = id.split_once(':') {
                            vendor
                                .trim_start_matches("0x")
                                .eq_ignore_ascii_case(&g.vendor_id)
                                && device
                                    .trim_start_matches("0x")
                                    .eq_ignore_ascii_case(&g.device_id)
                        } else {
                            id.trim_start_matches("0x")
                                .eq_ignore_ascii_case(&g.device_id)
                        }
                    })
                })
            }),
        );
        check(
            "Modelo da GPU",
            !self.gpu_models.is_empty(),
            gpu.and_then(|g| g.name.as_deref()).map(|name| {
                self.gpu_models
                    .iter()
                    .any(|n| crate::performance::comparison::same_condition("gpu", n, name))
            }),
        );
        check(
            "Driver",
            !self.driver_versions.is_empty(),
            gpu.and_then(|g| g.driver_version.as_deref()).map(|v| {
                self.driver_versions
                    .iter()
                    .any(|r| r.eq_ignore_ascii_case(v))
            }),
        );
        check(
            "Proton",
            !self.proton_versions.is_empty(),
            ctx.proton_version.as_deref().map(|v| {
                self.proton_versions
                    .iter()
                    .any(|r| r.eq_ignore_ascii_case(v))
            }),
        );
        check(
            "Arquitetura da CPU",
            self.cpu_arch.is_some(),
            ctx.cpu_arch.as_ref().map(|a| {
                self.cpu_arch
                    .as_ref()
                    .is_some_and(|r| r.eq_ignore_ascii_case(a))
            }),
        );
        if gpu.is_some() && ctx.observed_gpu().is_none() {
            result.limitations.extend(ctx.selection_limitations());
        }
        result
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn gpu(vendor: GpuVendor, model: &str, device: &str, driver: &str) -> GpuInfo {
        GpuInfo {
            vendor,
            vendor_id: match vendor {
                GpuVendor::Nvidia => "10de",
                GpuVendor::Amd => "1002",
                GpuVendor::Intel => "8086",
                GpuVendor::Other => "ffff",
            }
            .into(),
            device_id: device.into(),
            name: Some(model.into()),
            driver: Some(
                match vendor {
                    GpuVendor::Nvidia => "nvidia",
                    GpuVendor::Amd => "amdgpu",
                    _ => "xe",
                }
                .into(),
            ),
            driver_version: Some(driver.into()),
            boot_vga: false,
            pci_address: Some("0000:01:00.0".into()),
            gpu_family: None,
            vram_bytes: Some(8 * 1024 * 1024 * 1024),
        }
    }

    pub fn env(gpu: GpuInfo) -> EnvironmentContext {
        EnvironmentContext {
            detected_gpus: vec![gpu.clone()],
            selected_gpu: Some(gpu),
            selection: GpuSelection::Measured,
            cpu_arch: Some("x86_64".into()),
            proton_version: Some("Proton 10".into()),
            ..Default::default()
        }
    }

    #[test]
    fn exact_gpu_model_driver_and_proton_requirements_do_not_generalize() {
        let nv = env(gpu(
            GpuVendor::Nvidia,
            "GeForce RTX 4060",
            "2882",
            "580.126.09",
        ));
        let req = HardwareRequirements {
            gpu_vendor: Some(GpuReq::Nvidia),
            gpu_models: vec!["NVIDIA GeForce RTX 4060".into()],
            driver_versions: vec!["580.126.09".into()],
            proton_versions: vec!["Proton 10".into()],
            ..Default::default()
        };
        assert_eq!(req.evaluate(Some(&nv)).state, MatchState::Compatible);
        let mut other = nv.clone();
        other.selected_gpu.as_mut().unwrap().name = Some("GeForce RTX 4070".into());
        assert_eq!(req.evaluate(Some(&other)).state, MatchState::Incompatible);
        let mut other = nv.clone();
        other.selected_gpu.as_mut().unwrap().driver_version = Some("590.0".into());
        assert_eq!(req.evaluate(Some(&other)).state, MatchState::Incompatible);
        let mut other = nv.clone();
        other.proton_version = Some("Proton 11".into());
        assert_eq!(req.evaluate(Some(&other)).state, MatchState::Incompatible);
        let amd = env(gpu(GpuVendor::Amd, "Radeon RX 7800 XT", "747e", "25.3"));
        assert_eq!(req.evaluate(Some(&amd)).state, MatchState::Incompatible);
        assert_eq!(req.evaluate(None).state, MatchState::Unknown);
        assert_eq!(
            HardwareRequirements::default().evaluate(None).state,
            MatchState::Compatible,
            "optional metadata cannot disable every generic fix"
        );
    }

    #[test]
    fn inventory_and_selected_renderer_are_distinct_in_hybrid_and_duplicate_systems() {
        let nv = gpu(GpuVendor::Nvidia, "GeForce RTX 4060", "2882", "580.126.09");
        let amd = gpu(GpuVendor::Amd, "Radeon RX 7800 XT", "747e", "25.3");
        let inventory = EnvironmentContext {
            detected_gpus: vec![nv.clone(), amd],
            ..Default::default()
        };
        let req = HardwareRequirements {
            gpu_vendor: Some(GpuReq::Nvidia),
            ..Default::default()
        };
        assert_eq!(req.evaluate(Some(&inventory)).state, MatchState::Unknown);
        assert!(inventory.selected_gpu.is_none());
        let observed = inventory.with_observed_gpu("NVIDIA GeForce RTX 4060/PCIe/SSE2");
        assert_eq!(observed.selection, GpuSelection::Measured);
        assert_eq!(observed.selected_gpu.as_ref().unwrap().device_id, "2882");
        assert_eq!(req.evaluate(Some(&observed)).state, MatchState::Compatible);
        let mut unconfirmed = observed.clone();
        unconfirmed.selection = GpuSelection::Unknown;
        assert!(unconfirmed.observed_gpu().is_none());
        assert_eq!(
            req.evaluate(Some(&unconfirmed)).state,
            MatchState::Unknown,
            "a non-null device without measured provenance cannot select a hybrid GPU"
        );
        let mut duplicate = nv.clone();
        duplicate.pci_address = Some("0000:02:00.0".into());
        let both = EnvironmentContext {
            detected_gpus: vec![nv, duplicate],
            ..Default::default()
        }
        .with_observed_gpu("GeForce RTX 4060");
        assert!(
            both.selected_gpu.is_none(),
            "equal marketing names cannot identify one physical board"
        );
        assert_eq!(both.selection, GpuSelection::Unknown);
    }

    #[test]
    fn system_inventory_does_not_claim_primary_gpu_or_installed_icd_is_active() {
        let nv = gpu(GpuVendor::Nvidia, "GeForce RTX 4060", "2882", "580.0");
        let profile = SystemProfile {
            distro: Some("FixtureOS".into()),
            distro_id: None,
            kernel: Some("fixture-kernel".into()),
            arch: "x86_64",
            session_type: Some("wayland".into()),
            desktop: Some("FixtureWM".into()),
            cpu: Some("FixtureCPU".into()),
            cpu_threads: Some(8),
            ram_bytes: None,
            gpus: vec![nv.clone()],
            primary_gpu: Some(nv),
            vulkan_drivers: vec!["nvidia_icd".into()],
        };
        let context = EnvironmentContext::from_profile(&profile, None);
        assert!(context.observed_gpu().is_none());
        assert_eq!(context.selection, GpuSelection::Unknown);
        assert!(context.active_vulkan_driver.is_none());
        assert_eq!(context.installed_vulkan_drivers, ["nvidia_icd"]);
        assert_eq!(context.cpu_arch.as_deref(), Some("x86_64"));
        let unconfirmed = HardwareRequirements {
            gpu_device_ids: vec!["2882".into()],
            ..Default::default()
        };
        assert_eq!(
            unconfirmed.evaluate(Some(&context)).state,
            MatchState::Unknown,
            "bare device IDs need vendor identity"
        );
        let exact = HardwareRequirements {
            gpu_device_ids: vec!["10de:2882".into()],
            ..Default::default()
        };
        assert_eq!(exact.evaluate(Some(&context)).state, MatchState::Compatible);
    }

    #[test]
    fn known_mismatches_are_scoped_and_optional_absence_is_explained() {
        let nv = env(gpu(
            GpuVendor::Nvidia,
            "GeForce RTX 4060",
            "2882",
            "580.126.09",
        ));
        let mut other = nv.clone();
        other.selected_gpu.as_mut().unwrap().device_id = "2883".into();
        assert!(nv
            .cohort_differences(&other)
            .contains(&"modelo PCI da GPU".into()));
        let mut other = nv.clone();
        other.selected_gpu.as_mut().unwrap().driver_version = Some("590.0".into());
        assert!(nv
            .cohort_differences(&other)
            .contains(&"driver da GPU".into()));
        let mut optional = nv.clone();
        optional.active_vulkan_driver = None;
        optional.session_type = None;
        assert!(nv.cohort_differences(&optional).is_empty());
        let mut hybrid = nv.clone();
        hybrid.selected_gpu = None;
        hybrid
            .detected_gpus
            .push(gpu(GpuVendor::Intel, "Arc A770", "56a0", "25.3"));
        assert!(nv
            .cohort_differences(&hybrid)
            .iter()
            .any(|v| v.contains("várias GPUs")));
    }

    #[test]
    fn same_game_and_action_keep_nvidia_improvement_amd_regression_intel_unknown() {
        use crate::performance::models::{Conditions, SessionMeta};
        use crate::smart_packs::models::{EnvPair, Outcome};
        use crate::smart_packs::{benchmark, evidence};
        let app = "200260";
        let action = vec![EnvPair {
            key: "PROTON_NO_D3D11".into(),
            value: "1".into(),
        }];
        let nv = env(gpu(
            GpuVendor::Nvidia,
            "GeForce RTX 4060",
            "2882",
            "580.126.09",
        ));
        let amd = env(gpu(GpuVendor::Amd, "Radeon RX 7800 XT", "747e", "25.3"));
        let intel = env(gpu(GpuVendor::Intel, "Arc A770", "56a0", "25.3"));
        let mut sessions = Vec::<SessionMeta>::new();
        for (label, ctx, after) in [("nv", nv.clone(), 125.0), ("amd", amd.clone(), 60.0)] {
            for n in 0..2 {
                for (is_pack, fps) in [(false, 100.0), (true, after)] {
                    let mut m = crate::performance::comparison::tests::session(
                        &format!("synthetic-{label}-{n}-{is_pack}"),
                        &vec![1000.0 / fps; 6000],
                    );
                    m.notes="Synthetic automated-test fixture only, never exported as real measurements".into();
                    m.conditions = Conditions {
                        environment: Some(ctx.clone()),
                        gpu: ctx.selected_gpu.as_ref().and_then(|g| g.name.clone()),
                        resolution: Some("1920x1080".into()),
                        preset: Some("high".into()),
                        ..Default::default()
                    };
                    m.smart_pack = Some(benchmark::declared(
                        app,
                        if is_pack { action.clone() } else { vec![] },
                    ));
                    sessions.push(m);
                }
            }
        }
        let metas = sessions.iter().map(|s| (s.id.as_str(), s)).collect();
        for (ctx, expected) in [
            (nv, Some(Outcome::Improvement)),
            (amd, Some(Outcome::Regression)),
            (intel, None),
        ] {
            let current = Conditions {
                environment: Some(ctx),
                ..Default::default()
            };
            let groups = evidence::group_in_context(
                app,
                &sessions,
                |s| s.smart_pack.clone(),
                &benchmark::baseline_id(app),
                Some(&current),
            );
            let base = groups.measurements.get(&benchmark::baseline_id(app));
            let pack = groups.measurements.get(&benchmark::pack_id(app, &action));
            match expected {
                Some(outcome) => {
                    let result = evidence::compare(base.unwrap(), pack.unwrap(), &metas);
                    assert_eq!(result.get("fpsAvg").unwrap().outcome, outcome);
                    assert_eq!(pack.unwrap().used(), 2);
                }
                None => assert!(groups.measurements.values().all(|m| m.used() == 0)),
            }
        }
    }
}
