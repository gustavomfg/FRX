//! Conflitos e relações entre receitas — o grafo de compatibilidade.
//!
//! Cada par de receitas recebe uma aresta: `excludes` (com o tipo e o
//! motivo) ou `no-known-conflict`. A ausência de conflito conhecido **não**
//! prova que duas ações funcionam bem juntas; por isso a composição ainda
//! exige evidência suficiente de cada uma (`composition`).

use super::models::*;
use crate::fixes::compat::vendor_name;
use crate::steam::launch_options::LaunchLine;

/// Versão destas regras: mudou → os pacotes são recompostos.
pub const RULES: &str = "2026.10.26-1";

/// Pares mutuamente exclusivos, com o motivo.
const EXCLUSIVE: &[(&str, &str, &str)] = &[
    ("PROTON_USE_WINED3D", "PROTON_NO_D3D11", "WineD3D e “sem Direct3D 11” são contornos alternativos de renderização; o FRX não os combina."),
    ("PROTON_USE_WINED3D", "PROTON_NO_D3D10", "WineD3D e “sem Direct3D 10” são contornos alternativos de renderização; o FRX não os combina."),
    ("PROTON_ENABLE_NVAPI", "PROTON_DISABLE_NVAPI", "Uma ativa e a outra desativa a NVAPI."),
    ("PROTON_ENABLE_NVAPI", "PROTON_HIDE_NVIDIA_GPU", "Ocultar a GPU NVIDIA anula o efeito de ativar a NVAPI."),
    ("DXVK_FRAME_RATE", "VKD3D_FRAME_RATE", "Dois limitadores de FPS na mesma configuração."),
];

pub fn perf_hint(key: &str) -> PerfHint {
    match key {
        "PROTON_USE_WINED3D" => PerfHint::Penalty,
        "DXVK_FRAME_RATE" | "VKD3D_FRAME_RATE" => PerfHint::FrameCap,
        _ => PerfHint::Unknown,
    }
}

fn exclusive(a: &str, b: &str) -> Option<&'static str> {
    EXCLUSIVE
        .iter()
        .find(|(x, y, _)| (*x == a && *y == b) || (*x == b && *y == a))
        .map(|(_, _, r)| *r)
}

fn subset(a: &[EnvPair], b: &[EnvPair]) -> bool {
    a.iter().all(|p| b.contains(p))
}

fn excl(a: &Unit, b: &Unit, kind: ConflictKind, reason: String) -> Edge {
    Edge {
        a: a.recipe_id.clone(),
        b: b.recipe_id.clone(),
        relation: Relation::Excludes,
        kind: Some(kind),
        reason,
    }
}

/// A combination must satisfy every member's hardware/software constraints.
pub fn combined_hardware(
    units: &[Unit],
) -> Result<crate::system::context::HardwareRequirements, String> {
    use crate::system::context::HardwareRequirements;
    let mut out = HardwareRequirements::default();
    fn intersect(target: &mut Vec<String>, extra: &[String], field: &str) -> Result<(), String> {
        if extra.is_empty() {
            return Ok(());
        }
        if target.is_empty() {
            *target = extra.to_vec();
            return Ok(());
        }
        target.retain(|a| {
            extra.iter().any(|b| {
                if field == "modelo GPU" {
                    crate::performance::comparison::same_condition("gpu", a, b)
                } else {
                    a.trim_start_matches("0x")
                        .eq_ignore_ascii_case(b.trim_start_matches("0x"))
                }
            })
        });
        if target.is_empty() {
            return Err(format!("Requisitos de {field} não possuem um contexto comum; as receitas não podem ser combinadas."));
        }
        Ok(())
    }
    for unit in units {
        let h = &unit.hardware;
        if let Some(v) = h.gpu_vendor {
            if out.gpu_vendor.is_some_and(|old| old != v) {
                return Err("As receitas exigem fabricantes de GPU diferentes.".into());
            }
            out.gpu_vendor = Some(v);
        }
        if let Some(arch) = &h.cpu_arch {
            if out
                .cpu_arch
                .as_ref()
                .is_some_and(|old| !old.eq_ignore_ascii_case(arch))
            {
                return Err("As receitas exigem arquiteturas diferentes.".into());
            }
            out.cpu_arch = Some(arch.clone());
        }
        intersect(&mut out.gpu_device_ids, &h.gpu_device_ids, "ID PCI")?;
        intersect(&mut out.gpu_models, &h.gpu_models, "modelo GPU")?;
        intersect(&mut out.driver_versions, &h.driver_versions, "driver")?;
        intersect(&mut out.proton_versions, &h.proton_versions, "Proton")?;
    }
    Ok(out)
}

/// Relação entre duas receitas.
pub fn edge(a: &Unit, b: &Unit) -> Edge {
    if a.app_id.is_none() || a.app_id != b.app_id {
        return excl(
            a,
            b,
            ConflictKind::GameScope,
            "Receitas sem o mesmo AppID confirmado nunca podem ser combinadas.".into(),
        );
    }
    if let Err(reason) = combined_hardware(&[a.clone(), b.clone()]) {
        return excl(a, b, ConflictKind::GpuMismatch, reason);
    }
    for x in &a.env {
        if let Some(y) = b.env.iter().find(|y| y.key == x.key && y.value != x.value) {
            return excl(
                a,
                b,
                ConflictKind::SameKeyDifferentValue,
                format!(
                    "As duas definem {} com valores diferentes ({} × {}).",
                    x.key, x.value, y.value
                ),
            );
        }
    }
    let a_files = super::benchmark::action_file_fixes(
        a.app_id.as_deref().unwrap_or_default(),
        &a.file_actions,
    );
    let b_files = super::benchmark::action_file_fixes(
        b.app_id.as_deref().unwrap_or_default(),
        &b.file_actions,
    );
    let (Ok(a_files), Ok(b_files)) = (a_files, b_files) else {
        return excl(a,b,ConflictKind::SameFile,"Uma ação de arquivo não passou na validação integral; o pacote não pode ser aplicado parcialmente.".into());
    };
    if let Some(path) = a_files.iter().find(|x| {
        b_files
            .iter()
            .any(|y| x.target_path.eq_ignore_ascii_case(&y.target_path))
    }) {
        return excl(a,b,ConflictKind::SameFile,format!("As duas correções disputam o destino {}; cada arquivo tem uma única origem por operação.",path.target_path));
    }
    if (subset(&a.env, &b.env) && a_files.is_empty())
        || (subset(&b.env, &a.env) && b_files.is_empty())
    {
        return excl(
            a,
            b,
            ConflictKind::Redundant,
            "Uma já contém todas as variáveis da outra; combinar não acrescenta nada.".into(),
        );
    }
    for x in &a.env {
        for y in &b.env {
            if let Some(r) = exclusive(&x.key, &y.key) {
                return excl(a, b, ConflictKind::MutuallyExclusive, r.into());
            }
        }
    }
    if let (Some(x), Some(y)) = (a.gpu, b.gpu) {
        if x != y {
            return excl(
                a,
                b,
                ConflictKind::GpuMismatch,
                format!(
                    "Uma é para GPUs {} e a outra para {}.",
                    vendor_name(x),
                    vendor_name(y)
                ),
            );
        }
    }
    if a.perf == PerfHint::FrameCap && b.perf == PerfHint::FrameCap {
        return excl(
            a,
            b,
            ConflictKind::FrameLimiter,
            "Dois limitadores de FPS.".into(),
        );
    }
    if !a.proton_versions.is_empty()
        && !b.proton_versions.is_empty()
        && !a
            .proton_versions
            .iter()
            .any(|v| b.proton_versions.contains(v))
    {
        return excl(
            a,
            b,
            ConflictKind::ProtonVersions,
            format!(
                "Relatadas com versões diferentes do Proton ({} × {}); não há evidência de que funcionem juntas.",
                a.proton_versions.join(", "),
                b.proton_versions.join(", ")
            ),
        );
    }
    Edge {
        a: a.recipe_id.clone(),
        b: b.recipe_id.clone(),
        relation: Relation::NoKnownConflict,
        kind: None,
        reason: "Nenhum conflito conhecido — isso não prova que funcionem bem juntas.".into(),
    }
}

pub fn graph(units: &[Unit]) -> Vec<Edge> {
    let mut out = Vec::new();
    for i in 0..units.len() {
        for j in i + 1..units.len() {
            out.push(edge(&units[i], &units[j]));
        }
    }
    out
}

/// Uma composição só vale se **todos** os pares forem compatíveis.
pub fn validate_combination(units: &[Unit], max_size: usize) -> Result<(), Vec<Edge>> {
    if units.len() < 2 || units.len() > max_size {
        return Err(vec![]);
    }
    let mut bad: Vec<Edge> = graph(units)
        .into_iter()
        .filter(|e| e.relation == Relation::Excludes)
        .collect();
    if let Err(reason) = combined_hardware(units) {
        bad.push(excl(
            &units[0],
            &units[units.len() - 1],
            ConflictKind::GpuMismatch,
            reason,
        ));
    }
    if bad.is_empty() {
        Ok(())
    } else {
        Err(bad)
    }
}

/// Interação com a linha de inicialização atual (MangoHud, gamescope e
/// variáveis que o usuário já definiu). Só leitura — vira nota de compatibilidade.
pub fn launch_notes(env: &[EnvPair], line: Option<&str>) -> Vec<String> {
    let Some(text) = line.filter(|l| !l.trim().is_empty()) else {
        return vec![];
    };
    let Ok(parsed) = LaunchLine::parse(text) else {
        return vec!["As opções de inicialização atuais usam sintaxe que o FRX não reescreve; a prévia vai pedir o passo manual ou recusar.".into()];
    };
    let mut notes = Vec::new();
    for (_, k, v) in parsed.leading_env() {
        if let Some(p) = env.iter().find(|p| p.key == k) {
            if p.value != v {
                notes.push(format!("Suas opções já definem {k}={v}; o pacote usa {}={}, e a prévia mostra a troca.", p.key, p.value));
            }
        }
    }
    let words: Vec<&str> = parsed.words.iter().map(|w| w.value.as_str()).collect();
    let caps = env.iter().any(|p| perf_hint(&p.key) == PerfHint::FrameCap);
    let gamescope = words.iter().any(|w| w.ends_with("gamescope"));
    if caps
        && gamescope
        && words
            .iter()
            .any(|w| *w == "-r" || w.starts_with("--framerate-limit"))
    {
        notes.push(
            "O gamescope já limita o FPS (-r): dois limitadores mudam as condições do benchmark."
                .into(),
        );
    }
    if caps && words.iter().any(|w| w.ends_with("mangohud")) {
        notes.push("O MangoHud está ativo; se a configuração dele tiver fps_limit, haverá dois limitadores.".into());
    }
    if gamescope {
        notes.push("O jogo roda dentro do gamescope: resolução e limite de FPS dele também fazem parte das condições do benchmark.".into());
    }
    notes
}
