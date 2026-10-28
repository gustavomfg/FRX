//! O que o FRX sabe sobre cada variável que pode gerar receita: efeito,
//! risco, requisitos de hardware e onde está documentada.
//!
//! Estar em `security::ENV_ALLOWLIST` só diz que o valor é seguro de gravar.
//! Para virar receita gerada, o par `NOME=valor` também precisa estar aqui —
//! com efeito conhecido e reversível. Variáveis de diagnóstico (`PROTON_LOG`,
//! `DXVK_HUD`) ficam de fora: não são correções.

use crate::fix_engine::models::{Outlook, Purpose, RecipeAction, Risk};
use crate::fixes::models::{Category, GpuReq};

pub const PROTON_README: &str = "https://github.com/ValveSoftware/Proton#runtime-config-options";

#[derive(Clone, Copy)]
pub enum Value {
    Exact(&'static str),
    /// Limite de FPS entre 10 e 360.
    FrameRate,
    /// Lista separada por vírgulas, cada item com efeito descrito em `VKD3D_FLAGS`.
    Vkd3dFlags,
}

/// O que precisa existir no ambiente para a opção ter efeito. Conferido em
/// `fix_engine::environment` na análise, na prévia e na aplicação.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Needs {
    Nothing,
    /// Sessão Wayland: fora dela o Proton ignora a opção.
    Wayland,
    /// O jogo traz este arquivo; `manifest` é a entrada correspondente no
    /// manifesto de atualização que o próprio Proton usa.
    Component {
        file: &'static str,
        manifest: &'static str,
    },
    /// O jogo traz um destes arquivos (sem versão a comparar).
    GameFile(&'static [&'static str]),
}

/// Opções do VKD3D-Proton com efeito conhecido. Antes de aplicar, cada uma é
/// procurada na tabela de opções do VKD3D-Proton da ferramenta instalada.
pub const VKD3D_FLAGS: &[(&str, &str)] = &[
    (
        "descriptor_heap",
        "usa o modelo de descriptor heap do Vulkan para os descritores do Direct3D 12",
    ),
    (
        "enable_experimental_features",
        "liga recursos experimentais do VKD3D-Proton",
    ),
    (
        "no_upload_hvv",
        "não usa memória de vídeo visível pela CPU (ReBAR) para uploads",
    ),
    (
        "nodxr",
        "desativa o ray tracing do Direct3D 12 (DXR); efeitos de ray tracing deixam de existir",
    ),
    (
        "single_queue",
        "usa uma única fila da GPU, sem filas assíncronas de cópia e computação",
    ),
];

pub struct Knowledge {
    pub key: &'static str,
    pub value: Value,
    pub title: &'static str,
    pub effect: &'static str,
    pub side_effect: Option<&'static str>,
    pub category: Category,
    pub purpose: Purpose,
    pub risk: Risk,
    pub gpu: Option<GpuReq>,
    pub needs: Needs,
}

const fn k(
    key: &'static str,
    value: Value,
    title: &'static str,
    effect: &'static str,
    side_effect: Option<&'static str>,
    category: Category,
    risk: Risk,
) -> Knowledge {
    Knowledge {
        key,
        value,
        title,
        effect,
        side_effect,
        category,
        purpose: Purpose::Compatibility,
        risk,
        gpu: None,
        needs: Needs::Nothing,
    }
}

use Category::*;
use Value::*;

pub const KNOWLEDGE: &[Knowledge] = &[
    k(
        "PROTON_NO_ESYNC",
        Exact("1"),
        "Desativar esync",
        "O Proton deixa de usar esync (sincronização por eventfd).",
        Some("Pode reduzir o desempenho em jogos com muitas threads."),
        Compatibility,
        Risk::Low,
    ),
    k(
        "PROTON_NO_FSYNC",
        Exact("1"),
        "Desativar fsync",
        "O Proton deixa de usar fsync (sincronização por futex).",
        Some("Pode reduzir o desempenho em jogos com muitas threads."),
        Compatibility,
        Risk::Low,
    ),
    k(
        "PROTON_NO_NTSYNC",
        Exact("1"),
        "Desativar ntsync",
        "O Proton deixa de usar o ntsync do kernel.",
        Some("Pode reduzir o desempenho em jogos com muitas threads."),
        Compatibility,
        Risk::Low,
    ),
    k(
        "PROTON_USE_WINED3D",
        Exact("1"),
        "Renderizar com WineD3D (OpenGL)",
        "Direct3D 9/10/11 passa a ser traduzido para OpenGL pelo WineD3D em vez do DXVK.",
        Some("O custo da tradução pode reduzir o desempenho; o resultado depende do jogo e do ambiente."),
        Graphics,
        Risk::Moderate,
    ),
    k(
        "PROTON_NO_D3D11",
        Exact("1"),
        "Desativar Direct3D 11",
        "O d3d11.dll fica indisponível, para o jogo cair no modo Direct3D 9.",
        Some("Se o jogo não tiver modo Direct3D 9, ele deixa de abrir até você desfazer."),
        Graphics,
        Risk::Moderate,
    ),
    k(
        "PROTON_NO_D3D10",
        Exact("1"),
        "Desativar Direct3D 10",
        "O d3d10.dll e o dxgi.dll ficam indisponíveis, para o jogo cair no modo Direct3D 9.",
        Some("Se o jogo não tiver modo Direct3D 9, ele deixa de abrir até você desfazer."),
        Graphics,
        Risk::Moderate,
    ),
    k(
        "PROTON_FORCE_LARGE_ADDRESS_AWARE",
        Exact("1"),
        "Forçar Large Address Aware",
        "Executáveis de 32 bits podem usar mais de 2 GB de memória.",
        None,
        Compatibility,
        Risk::Low,
    ),
    k(
        "PROTON_HEAP_DELAY_FREE",
        Exact("1"),
        "Atrasar liberação de memória",
        "Contorna erros de uso de memória já liberada em alguns jogos.",
        Some("Pode aumentar um pouco o uso de memória."),
        Compatibility,
        Risk::Low,
    ),
    k(
        "PROTON_OLD_GL_STRING",
        Exact("1"),
        "Lista curta de extensões OpenGL",
        "Limita a lista de extensões OpenGL para jogos antigos que travam com listas longas.",
        None,
        Compatibility,
        Risk::Low,
    ),
    k(
        "PROTON_USE_XALIA",
        Exact("0"),
        "Desativar Xalia",
        "Desliga a camada de interface para controle (Xalia).",
        None,
        Input,
        Risk::Low,
    ),
    Knowledge {
        gpu: Some(GpuReq::Nvidia),
        ..k(
            "PROTON_DISABLE_NVAPI",
            Exact("1"),
            "Desativar NVAPI",
            "Desliga a biblioteca NVAPI da NVIDIA dentro do Proton.",
            Some("Recursos como DLSS deixam de funcionar."),
            Graphics,
            Risk::Low,
        )
    },
    Knowledge {
        gpu: Some(GpuReq::Nvidia),
        ..k(
            "PROTON_ENABLE_NVAPI",
            Exact("1"),
            "Ativar NVAPI",
            "Expõe a NVAPI da NVIDIA ao jogo (DLSS, Reflex).",
            None,
            Graphics,
            Risk::Low,
        )
    },
    Knowledge {
        gpu: Some(GpuReq::Nvidia),
        ..k(
            "PROTON_HIDE_NVIDIA_GPU",
            Exact("1"),
            "Ocultar GPU NVIDIA",
            "A GPU NVIDIA é informada ao jogo como AMD.",
            Some("Recursos exclusivos da NVIDIA ficam indisponíveis."),
            Graphics,
            Risk::Low,
        )
    },
    Knowledge {
        purpose: Purpose::Performance,
        ..k(
            "DXVK_FRAME_RATE",
            FrameRate,
            "Limitar FPS (DXVK)",
            "O DXVK limita a taxa de quadros ao valor indicado.",
            None,
            Performance,
            Risk::Low,
        )
    },
    Knowledge {
        purpose: Purpose::Performance,
        ..k(
            "VKD3D_FRAME_RATE",
            FrameRate,
            "Limitar FPS (VKD3D-Proton)",
            "O VKD3D-Proton limita a taxa de quadros ao valor indicado.",
            None,
            Performance,
            Risk::Low,
        )
    },
    // Atualizações feitas pelo próprio Proton: ele baixa a versão do manifesto
    // dele, confere a integridade e a usa no lugar da que acompanha o jogo,
    // sem alterar os arquivos do jogo. O FRX só liga a opção.
    Knowledge {
        gpu: Some(GpuReq::Nvidia),
        purpose: Purpose::Quality,
        needs: Needs::Component { file: "nvngx_dlss.dll", manifest: "dlss" },
        ..k(
            "PROTON_DLSS_UPGRADE",
            Exact("1"),
            "Atualizar o DLSS pelo Proton",
            "Na próxima execução, o próprio Proton baixa a versão mais recente do DLSS (super resolução, geração de quadros e ray reconstruction), confere a integridade e a usa no lugar da que acompanha o jogo, sem alterar os arquivos do jogo.",
            Some("Uma versão mais nova pode mudar a nitidez, artefatos e desempenho; compare no Performance Lab. A primeira execução precisa de internet."),
            Graphics,
            Risk::Moderate,
        )
    },
    Knowledge {
        purpose: Purpose::Quality,
        needs: Needs::Component { file: "libxess.dll", manifest: "xess" },
        ..k(
            "PROTON_XESS_UPGRADE",
            Exact("1"),
            "Atualizar o XeSS pelo Proton",
            "Na próxima execução, o próprio Proton baixa a versão mais recente do XeSS, confere a integridade e a usa no lugar da que acompanha o jogo, sem alterar os arquivos do jogo.",
            Some("Uma versão mais nova pode mudar a nitidez, artefatos e desempenho; compare no Performance Lab. A primeira execução precisa de internet."),
            Graphics,
            Risk::Moderate,
        )
    },
    Knowledge {
        needs: Needs::Wayland,
        ..k(
            "PROTON_ENABLE_WAYLAND",
            Exact("1"),
            "Janela Wayland nativa",
            "O jogo abre como janela Wayland pelo driver Wayland do Wine, sem passar pelo XWayland.",
            Some("Sobreposições, captura de tela e alguns controles podem se comportar diferente; desfaça se algo piorar."),
            Compatibility,
            Risk::Moderate,
        )
    },
    Knowledge {
        needs: Needs::Wayland,
        ..k(
            "PROTON_USE_WAYLAND",
            Exact("1"),
            "Janela Wayland nativa",
            "O jogo abre como janela Wayland pelo driver Wayland do Wine, sem passar pelo XWayland.",
            Some("Sobreposições, captura de tela e alguns controles podem se comportar diferente; desfaça se algo piorar."),
            Compatibility,
            Risk::Moderate,
        )
    },
    // Bibliotecas NVIDIA alternativas do Proton-CachyOS: só têm alvo em jogos
    // que trazem o PhysX legado (CUDA dentro do Wine).
    Knowledge {
        gpu: Some(GpuReq::Nvidia),
        needs: Needs::GameFile(&["physxloader.dll", "physxloader64.dll", "physxcore.dll", "physxdevice.dll"]),
        ..k(
            "PROTON_NVIDIA_LIBS",
            Exact("1"),
            "CUDA real do Proton (nvidia-libs)",
            "O Proton instala no prefixo o nvcuda.dll do nvidia-libs no lugar do stub, para o PhysX legado encontrar CUDA.",
            Some("As bibliotecas alternativas valem para todo o jogo; compare no Performance Lab."),
            Physics,
            Risk::Moderate,
        )
    },
    k(
        "VKD3D_CONFIG",
        Vkd3dFlags,
        "Opções do VKD3D-Proton (Direct3D 12)",
        "O VKD3D-Proton, que traduz Direct3D 12 para Vulkan, muda de comportamento conforme as opções indicadas.",
        Some("Contornos citados para problemas específicos; podem causar artefatos ou perda de desempenho em outros cenários. Desfaça se não houver o problema."),
        Graphics,
        Risk::Moderate,
    ),
];

pub fn knowledge(key: &str, value: &str) -> Option<&'static Knowledge> {
    KNOWLEDGE.iter().find(|k| {
        k.key == key
            && match k.value {
                Exact(v) => v == value,
                FrameRate => value
                    .parse::<u16>()
                    .is_ok_and(|n| (10..=360).contains(&n) && !value.starts_with('0')),
                Vkd3dFlags => vkd3d_flags(value).is_some(),
            }
    })
}

/// Opções válidas, sem repetição, todas com efeito descrito.
pub fn vkd3d_flags(value: &str) -> Option<Vec<&'static str>> {
    let mut out: Vec<&'static str> = Vec::new();
    for f in value.split(',') {
        let (name, _) = VKD3D_FLAGS.iter().find(|(n, _)| *n == f)?;
        if out.contains(name) {
            return None;
        }
        out.push(name);
    }
    (!out.is_empty()).then_some(out)
}

/// Efeito desta atribuição, com o detalhe de cada opção quando é uma lista.
pub fn describe(k: &Knowledge, value: &str) -> String {
    match (k.value, vkd3d_flags(value)) {
        (Vkd3dFlags, Some(flags)) => {
            let parts: Vec<String> = flags
                .iter()
                .filter_map(|f| VKD3D_FLAGS.iter().find(|(n, _)| n == f))
                .map(|(n, d)| format!("{n}: {d}"))
                .collect();
            format!("{} {}.", k.effect, parts.join("; "))
        }
        _ => k.effect.to_owned(),
    }
}

/// Variáveis lidas só pelo driver Mesa RADV (AMD): sem efeito em outra GPU.
pub fn vendor_bound(key: &str) -> Option<GpuReq> {
    key.starts_with("RADV_").then_some(GpuReq::Amd)
}

/// Efeito provável de `NOME=valor`. Tabela curada: o padrão é `Uncertain`
/// (depende do jogo e precisa de medição), não uma promessa de ganho.
pub fn outlook(key: &str, value: &str) -> Outlook {
    use Outlook::*;
    match (key, value) {
        ("PROTON_NVIDIA_LIBS", "1") | ("PROTON_ENABLE_NVAPI", "1") => Helps,
        ("PROTON_NO_ESYNC" | "PROTON_NO_FSYNC" | "PROTON_NO_NTSYNC", "1") => MayHurt,
        ("PROTON_USE_WINED3D", "1") | ("PROTON_NO_D3D11" | "PROTON_NO_D3D10", "1") => MayHurt,
        ("PROTON_DISABLE_NVAPI" | "PROTON_HIDE_NVIDIA_GPU", "1") => MayHurt,
        ("DXVK_FRAME_RATE" | "VKD3D_FRAME_RATE", _) => MayHurt,
        _ => Uncertain,
    }
}

/// Problemas que `NOME=valor` costuma resolver (ids de sintoma da resolução).
/// Vazio = não ajuda com nenhum problema descrito, então não é sugestão.
pub fn aims(key: &str, value: &str) -> Vec<&'static str> {
    match (key, value) {
        ("PROTON_NVIDIA_LIBS", "1") => vec!["physx"],
        ("PROTON_DLSS_UPGRADE", "1") => vec!["dlss"],
        ("PROTON_XESS_UPGRADE", "1") => vec!["graphics"],
        ("PROTON_ENABLE_NVAPI" | "PROTON_DISABLE_NVAPI", "1") => vec!["dlss", "frame-generation"],
        ("PROTON_ENABLE_WAYLAND" | "PROTON_USE_WAYLAND", "1") => vec!["graphics"],
        ("PROTON_USE_XALIA", "0") => vec!["controls"],
        ("PROTON_USE_WINED3D", "1") => vec!["startup", "graphics"],
        ("PROTON_NO_D3D11" | "PROTON_NO_D3D10", "1") => vec!["startup"],
        ("PROTON_NO_ESYNC" | "PROTON_NO_FSYNC" | "PROTON_NO_NTSYNC", "1") => {
            vec!["startup", "stuttering"]
        }
        (
            "PROTON_FORCE_LARGE_ADDRESS_AWARE"
            | "PROTON_HEAP_DELAY_FREE"
            | "PROTON_OLD_GL_STRING"
            | "PROTON_HIDE_NVIDIA_GPU",
            "1",
        ) => vec!["startup"],
        ("DXVK_FRAME_RATE" | "VKD3D_FRAME_RATE", _) => vec!["stuttering"],
        ("VKD3D_CONFIG", flags) => {
            let mut out = Vec::new();
            for flag in flags.split(',') {
                let more: &[&'static str] = match flag {
                    "nodxr" => &["ray-tracing"],
                    "descriptor_heap" => &["startup", "graphics"],
                    "enable_experimental_features" => &["startup"],
                    "no_upload_hvv" | "single_queue" => &["stuttering"],
                    _ => &[],
                };
                for id in more {
                    if !out.contains(id) {
                        out.push(*id);
                    }
                }
            }
            out
        }
        _ => vec![],
    }
}

/// Problemas de todas as variáveis de uma receita, sem repetição.
pub fn recipe_aims(actions: &[RecipeAction]) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for a in actions {
        if let RecipeAction::SetLaunchEnv { key, value } = a {
            for id in aims(key, value) {
                if !out.contains(&id) {
                    out.push(id);
                }
            }
        }
    }
    out
}

/// Pior efeito entre as variáveis da receita; `None` se não houver nenhuma.
pub fn recipe_outlook(actions: &[RecipeAction]) -> Option<Outlook> {
    actions
        .iter()
        .filter_map(|a| match a {
            RecipeAction::SetLaunchEnv { key, value } => Some(outlook(key, value)),
            _ => None,
        })
        .max()
}

/// A variável existe na base, mas com outro valor?
pub fn known_key(key: &str) -> bool {
    KNOWLEDGE.iter().any(|k| k.key == key)
}

/// Triagem preventiva por ação conhecida. Não inventa FPS ou regressão medida.
/// É independente de a receita ser segura de aplicar ou resolver compatibilidade.
pub fn performance_warnings<'a>(env: impl IntoIterator<Item = (&'a str, &'a str)>) -> Vec<String> {
    let mut out = Vec::new();
    for (key, value) in env {
        let reason = match (key, value) {
            ("PROTON_USE_WINED3D", "1") => Some("WineD3D troca DXVK/Vulkan por OpenGL. Há risco técnico de perda de desempenho; o impacto neste jogo depende de evidências específicas e medições locais."),
            ("PROTON_NO_ESYNC" | "PROTON_NO_FSYNC" | "PROTON_NO_NTSYNC", "1") => Some("Desativa uma forma de sincronização otimizada. Pode reduzir o desempenho em jogos com muitas threads; use para contornar um problema específico, não como otimização genérica."),
            ("PROTON_NO_D3D11" | "PROTON_NO_D3D10", "1") => Some("Desativa uma API gráfica e força o jogo a usar outro caminho de renderização. Pode perder desempenho ou recursos e até impedir a inicialização; não é um ganho de FPS presumido."),
            ("DXVK_FRAME_RATE" | "VKD3D_FRAME_RATE", v) if v.parse::<u32>().is_ok_and(|n| (10..=360).contains(&n)) => Some("Limita intencionalmente o FPS. Pode ajudar a controlar consumo ou estabilidade, mas não representa aumento de desempenho."),
            _ => None,
        };
        if let Some(reason) = reason {
            let reason = format!("{key}={value}: {reason}");
            if !out.contains(&reason) {
                out.push(reason);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fix_engine::security::check_env;

    #[test]
    fn every_known_pair_also_passes_the_allowlist() {
        for k in KNOWLEDGE {
            let v = match k.value {
                Exact(v) => v,
                FrameRate => "60",
                Vkd3dFlags => "descriptor_heap,nodxr",
            };
            assert!(check_env(k.key, v).is_ok(), "{}", k.key);
            assert!(knowledge(k.key, v).is_some());
        }
        assert!(knowledge("DXVK_FRAME_RATE", "5").is_none());
        assert!(knowledge("DXVK_FRAME_RATE", "060").is_none());
        assert!(
            knowledge("PROTON_LOG", "1").is_none(),
            "diagnóstico não é correção"
        );
        assert!(knowledge("PROTON_USE_XALIA", "1").is_none());
        assert!(knowledge("VKD3D_CONFIG", "nodxr,nodxr").is_none());
        assert!(knowledge("VKD3D_CONFIG", "nodxr,unknown_flag").is_none());
    }
}
