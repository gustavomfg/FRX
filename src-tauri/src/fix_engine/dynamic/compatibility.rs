//! O que o FRX sabe sobre cada variável que pode gerar receita: efeito,
//! risco, requisitos de hardware e onde está documentada.
//!
//! Estar em `security::ENV_ALLOWLIST` só diz que o valor é seguro de gravar.
//! Para virar receita gerada, o par `NOME=valor` também precisa estar aqui —
//! com efeito conhecido e reversível. Variáveis de diagnóstico (`PROTON_LOG`,
//! `DXVK_HUD`) ficam de fora: não são correções.

use crate::fix_engine::models::{Purpose, Risk};
use crate::fixes::models::{Category, GpuReq};

pub const PROTON_README: &str = "https://github.com/ValveSoftware/Proton#runtime-config-options";

#[derive(Clone, Copy)]
pub enum Value {
    Exact(&'static str),
    /// Limite de FPS entre 10 e 360.
    FrameRate,
}

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
];

pub fn knowledge(key: &str, value: &str) -> Option<&'static Knowledge> {
    KNOWLEDGE.iter().find(|k| {
        k.key == key
            && match k.value {
                Exact(v) => v == value,
                FrameRate => value
                    .parse::<u16>()
                    .is_ok_and(|n| (10..=360).contains(&n) && !value.starts_with('0')),
            }
    })
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
    }
}
