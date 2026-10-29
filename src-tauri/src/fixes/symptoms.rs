//! Problema que um achado descreve, lido do próprio texto da fonte (título,
//! descrição e trechos). Usa os identificadores de sintoma da resolução. Achado
//! sem texto que descreva um problema não entra em nenhum problema do jogo.

use super::models::{Category, FixFinding};

/// Palavras-chave em minúsculas por sintoma. Só texto da fonte conta: opções
/// como "launch options" não indicam crash.
const RULES: &[(&str, &[&str])] = &[
    (
        "startup",
        &[
            "crash",
            "won't start",
            "doesn't start",
            "does not start",
            "hangs",
            "freez",
            "black screen",
            "tela preta",
            "device_lost",
            "device lost",
            "não inicia",
        ],
    ),
    (
        "stuttering",
        &[
            "stutter",
            "hitch",
            "micro-travamento",
            "microtravamento",
            "gagu",
        ],
    ),
    (
        "low-performance",
        &[
            "fps",
            "framerate",
            "frame rate",
            "performance",
            "desempenho",
            "slow",
        ],
    ),
    ("ray-tracing", &["ray trac", "raytrac", "dxr", "ray recon"]),
    (
        "frame-generation",
        &[
            "frame gen",
            "framegen",
            "frame-gen",
            "dlss-g",
            "dlssg",
            "dlss fg",
        ],
    ),
    ("dlss", &["dlss"]),
    ("fsr", &["fsr", "fidelityfx"]),
    ("physx", &["physx"]),
    (
        "graphics",
        &[
            "flicker", "flashing", "artifact", "artefato", "texture", "textura", "glitch",
            "shimmer", "stripe",
        ],
    ),
    ("audio", &["audio", "áudio", "sound"]),
    ("video", &["cutscene", "video", "vídeo", "movie"]),
    (
        "controls",
        &[
            "controller",
            "gamepad",
            "input",
            "mouse",
            "keyboard",
            "controle",
        ],
    ),
];

/// Nome do problema para o usuário.
pub fn label(id: &str) -> &'static str {
    match id {
        "startup" => "não inicia ou trava (crash)",
        "stuttering" => "stuttering",
        "low-performance" => "desempenho baixo",
        "ray-tracing" => "ray tracing",
        "dlss" => "DLSS",
        "frame-generation" => "frame generation",
        "fsr" => "FSR",
        "physx" => "física (PhysX)",
        "graphics" => "artefatos gráficos",
        "audio" => "áudio",
        "video" => "vídeos e cutscenes",
        "controls" => "controles",
        _ => "problema não identificado",
    }
}

/// Sintomas citados em um texto.
pub fn of_text(text: &str) -> Vec<&'static str> {
    let text = text.to_lowercase();
    RULES
        .iter()
        .filter(|(_, keys)| keys.iter().any(|k| text.contains(k)))
        .map(|(id, _)| *id)
        .collect()
}

/// Sintomas de um achado: texto da fonte e a categoria que o publicador deu.
pub fn of_finding(f: &FixFinding) -> Vec<&'static str> {
    let mut text = format!("{} {}", f.title, f.description);
    for s in &f.sources {
        if let Some(snippet) = &s.snippet {
            text.push(' ');
            text.push_str(snippet);
        }
    }
    let mut out = of_text(&text);
    let by_category = match f.category {
        Category::Performance => Some("low-performance"),
        Category::Graphics => Some("graphics"),
        Category::Audio => Some("audio"),
        Category::Video => Some("video"),
        Category::Input => Some("controls"),
        Category::Physics => Some("physx"),
        _ => None,
    };
    if let Some(id) = by_category.filter(|id| !out.contains(id)) {
        out.push(id);
    }
    out
}

/// Tecnologia que um problema exige na instalação. Sem ela, o problema não existe para o jogo.
pub fn required_technology(id: &str) -> Option<&'static str> {
    match id {
        "physx" => Some("physx"),
        "dlss" => Some("dlss"),
        "frame-generation" => Some("frame-generation"),
        "ray-tracing" => Some("ray-tracing"),
        "fsr" => Some("fsr"),
        _ => None,
    }
}

/// O problema se aplica ao jogo, dado o que a instalação traz.
pub fn applies(id: &str, technologies: &[&str]) -> bool {
    required_technology(id).is_none_or(|t| technologies.contains(&t))
}

/// Problemas do jogo: união dos sintomas dos achados dele, só os que se aplicam
/// às tecnologias que a instalação traz.
pub fn of_game(findings: &[FixFinding], app_id: &str, technologies: &[&str]) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for f in findings.iter().filter(|f| f.game_app_id == app_id) {
        for id in of_finding(f) {
            if applies(id, technologies) && !out.contains(&id) {
                out.push(id);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symptoms_come_from_the_source_text_not_from_option_names() {
        assert!(
            of_text("[BUG] Crimson Desert crashes on launch after update").contains(&"startup")
        );
        assert!(of_text("flashing textures on screen").contains(&"graphics"));
        assert!(of_text("cutscene freezes in boss fights").contains(&"video"));
        assert!(of_text("Set PROTON_DLSS_UPGRADE with frame gen").contains(&"frame-generation"));
        // "launch options" descreve onde a opção é colocada, não um crash.
        assert!(
            of_text("Add this to your launch options: VKD3D_CONFIG=descriptor_heap").is_empty()
        );
    }
}
