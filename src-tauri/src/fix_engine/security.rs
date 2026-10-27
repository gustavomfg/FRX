//! Regras de segurança das receitas: allowlist de variáveis, argumentos
//! permitidos e caminhos confinados à pasta do jogo.
//!
//! Tudo o que uma receita declara passa por aqui duas vezes: ao carregar o
//! catálogo e ao montar o plano. O que não estiver explicitamente permitido é
//! recusado.

use std::path::{Component, Path, PathBuf};

use super::models::{Recipe, RecipeAction, Trust, RECIPE_SCHEMA};

/// Formato aceito para o valor de cada variável.
#[derive(Clone, Copy)]
enum Rule {
    /// Um dos valores listados.
    OneOf(&'static [&'static str]),
    /// Inteiro de até N dígitos.
    Digits(usize),
    /// Lista simples (`fps,devinfo`): letras minúsculas, dígitos, `,` `=` `.`.
    HudList,
}

/// Variáveis que uma receita pode definir. Nada de `LD_*`, `WINEDLLOVERRIDES`,
/// `PATH`, camadas Vulkan, caminhos ou executáveis: não basta começar com
/// `PROTON_` — cada nome é aprovado individualmente.
const ENV_ALLOWLIST: &[(&str, Rule)] = &[
    ("PROTON_LOG", Rule::OneOf(&["1"])),
    ("PROTON_NO_ESYNC", Rule::OneOf(&["1"])),
    ("PROTON_NO_FSYNC", Rule::OneOf(&["1"])),
    ("PROTON_NO_NTSYNC", Rule::OneOf(&["1"])),
    ("PROTON_USE_WINED3D", Rule::OneOf(&["1"])),
    ("PROTON_NO_D3D11", Rule::OneOf(&["1"])),
    ("PROTON_NO_D3D10", Rule::OneOf(&["1"])),
    ("PROTON_ENABLE_NVAPI", Rule::OneOf(&["0", "1"])),
    ("PROTON_HIDE_NVIDIA_GPU", Rule::OneOf(&["0", "1"])),
    ("PROTON_FORCE_LARGE_ADDRESS_AWARE", Rule::OneOf(&["0", "1"])),
    ("PROTON_HEAP_DELAY_FREE", Rule::OneOf(&["1"])),
    ("PROTON_OLD_GL_STRING", Rule::OneOf(&["1"])),
    ("PROTON_USE_XALIA", Rule::OneOf(&["0", "1"])),
    ("PROTON_DISABLE_NVAPI", Rule::OneOf(&["1"])),
    ("DXVK_FRAME_RATE", Rule::Digits(3)),
    ("VKD3D_FRAME_RATE", Rule::Digits(3)),
    ("DXVK_HUD", Rule::HudList),
];

/// Nomes recusados com mensagem própria (além de não estarem na allowlist).
pub fn sensitive(key: &str) -> bool {
    let k = key.to_ascii_uppercase();
    k.starts_with("LD_")
        || k.contains("PATH")
        || k.contains("DLL")
        || k.contains("LAYER")
        || k.contains("CONFIGFILE")
        || k.contains("PRELOAD")
        || k.starts_with("STEAM_")
        || k.starts_with("WINE")
        || k == "HOME"
        || k == "SHELL"
}

pub fn check_env(key: &str, value: &str) -> Result<(), String> {
    let Some((_, rule)) = ENV_ALLOWLIST.iter().find(|(k, _)| *k == key) else {
        return Err(if sensitive(key) {
            format!("A variável {key} pode alterar o carregamento de código ou caminhos sensíveis e nunca é aplicada pelo FRX.")
        } else {
            format!("A variável {key} não está na lista de variáveis permitidas.")
        });
    };
    let ok = match rule {
        Rule::OneOf(vals) => vals.contains(&value),
        Rule::Digits(n) => {
            !value.is_empty() && value.len() <= *n && value.bytes().all(|b| b.is_ascii_digit())
        }
        Rule::HudList => {
            !value.is_empty()
                && value.len() <= 64
                && value.bytes().all(|b| {
                    b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b',' | b'=' | b'.')
                })
        }
    };
    if ok {
        Ok(())
    } else {
        Err(format!("Valor recusado para {key}."))
    }
}

/// Argumento do jogo: uma opção simples (`-vulkan`, `-dx11`, `--foo=bar`), sem
/// caminhos, aspas, espaços ou metacaracteres de shell.
pub fn check_arg(arg: &str) -> Result<(), String> {
    let b = arg.as_bytes();
    let body = arg.trim_start_matches('-');
    let ok = (1..=2).contains(&(arg.len() - body.len()))
        && b.len() <= 64
        && body
            .as_bytes()
            .first()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        && body
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.' | b'=' | b':' | b'-'));
    if ok {
        Ok(())
    } else {
        Err(format!("Argumento recusado: {arg}"))
    }
}

/// Extensões de arquivos de configuração que uma receita pode escrever.
const FILE_EXTENSIONS: &[&str] = &["ini", "cfg", "conf", "txt", "json", "xml"];
const MAX_FILE_BYTES: usize = 256 * 1024;

/// Caminho declarado pela receita: relativo, sem `..`, sem componentes
/// estranhos e com extensão de configuração.
pub fn check_rel_path(rel: &str) -> Result<(), String> {
    let p = Path::new(rel);
    if rel.is_empty() || rel.len() > 200 || rel.chars().any(|c| c.is_control() || c == '\\') {
        return Err("Caminho inválido.".into());
    }
    if !p.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err("A receita só pode usar caminhos relativos à pasta do jogo, sem “..”.".into());
    }
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    if !ext.is_some_and(|e| FILE_EXTENSIONS.contains(&e.as_str())) {
        return Err("Só arquivos de configuração de texto podem ser escritos (nada de executáveis ou bibliotecas).".into());
    }
    Ok(())
}

/// Resolve `rel` dentro de `root` sem seguir links simbólicos e sem escapar
/// da pasta. Devolve o caminho final (que pode ainda não existir).
pub fn resolve_in_root(root: &Path, rel: &str) -> Result<PathBuf, String> {
    check_rel_path(rel)?;
    let root =
        std::fs::canonicalize(root).map_err(|_| "Pasta do jogo não encontrada.".to_owned())?;
    if !root.is_dir() {
        return Err("Pasta do jogo inválida.".into());
    }
    let mut cur = root.clone();
    let parts: Vec<_> = Path::new(rel).components().collect();
    for (i, c) in parts.iter().enumerate() {
        cur.push(c.as_os_str());
        match std::fs::symlink_metadata(&cur) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err("O caminho passa por um link simbólico; o FRX não o segue.".into())
            }
            Ok(m) if i + 1 < parts.len() && !m.is_dir() => {
                return Err("Caminho inválido dentro da pasta do jogo.".into())
            }
            Ok(m) if i + 1 == parts.len() && !m.is_file() => {
                return Err("O destino existe e não é um arquivo comum.".into())
            }
            Ok(_) => {}
            Err(_) if i + 1 < parts.len() => {
                return Err("A pasta de destino não existe na instalação do jogo.".into())
            }
            Err(_) => {}
        }
    }
    if !cur.starts_with(&root) {
        return Err("O caminho sai da pasta do jogo.".into());
    }
    Ok(cur)
}

/// Pastas que nunca são aceitas como "pasta do jogo" (prefixos, home, sistema).
pub fn check_game_root(root: &Path) -> Result<(), String> {
    let s = root.to_string_lossy();
    if !root.is_absolute() || s.contains("/compatdata/") || s.contains("/pfx/") {
        return Err("A pasta do jogo não pode ser um prefixo do Proton.".into());
    }
    if !s.contains("/steamapps/common/") {
        return Err(
            "Só pastas dentro de steamapps/common são aceitas como instalação do jogo.".into(),
        );
    }
    Ok(())
}

fn id_chars(id: &str) -> bool {
    (3..=80).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'-'))
}

fn valid_id(id: &str) -> bool {
    id_chars(id) && id.starts_with("frx.")
}

/// `gen.<appid>.<hash>` — identificador estável de receita gerada.
pub fn valid_generated_id(id: &str) -> bool {
    let mut parts = id.split('.');
    id_chars(id)
        && parts.next() == Some("gen")
        && parts.next().is_some_and(crate::art::valid_app_id)
        && parts
            .next()
            .is_some_and(|h| h.len() == 16 && h.bytes().all(|b| b.is_ascii_hexdigit()))
        && parts.next().is_none()
}

fn valid_version(v: &str) -> bool {
    let parts: Vec<&str> = v.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 5 && p.bytes().all(|b| b.is_ascii_digit()))
}

/// Validação estrutural da receita. Uma receita inválida nunca entra no catálogo.
pub fn validate_recipe(r: &Recipe) -> Result<(), String> {
    if r.schema != RECIPE_SCHEMA {
        return Err(format!(
            "{}: versão de formato {} não suportada",
            r.id, r.schema
        ));
    }
    if !valid_id(&r.id) {
        return Err(format!("identificador inválido: {}", r.id));
    }
    if !valid_version(&r.version) {
        return Err(format!("{}: versão inválida", r.id));
    }
    if r.actions.is_empty() || r.actions.len() > 8 {
        return Err(format!("{}: número de ações inválido", r.id));
    }
    if r.scope.app_ids.iter().any(|a| !crate::art::valid_app_id(a)) {
        return Err(format!("{}: AppID inválido no escopo", r.id));
    }
    if r.sources.is_empty() || r.sources.iter().any(|s| !s.url.starts_with("https://")) {
        return Err(format!("{}: fontes precisam ser HTTPS", r.id));
    }
    if matches!(r.trust, Trust::Generated | Trust::Pack)
        || r.generated.is_some()
        || r.pack.is_some()
    {
        return Err(format!(
            "{}: receitas geradas não podem entrar no catálogo",
            r.id
        ));
    }
    if r.trust == Trust::Reviewed && r.review.is_none() {
        return Err(format!(
            "{}: receita revisada sem registro de revisão",
            r.id
        ));
    }
    let mut keys = Vec::new();
    for a in &r.actions {
        match a {
            RecipeAction::SetLaunchEnv { key, value } => {
                check_env(key, value).map_err(|e| format!("{}: {e}", r.id))?;
                if keys.contains(&key) {
                    return Err(format!("{}: variável repetida", r.id));
                }
                keys.push(key);
            }
            RecipeAction::AddLaunchArgs { args } => {
                if args.is_empty() || args.len() > 8 {
                    return Err(format!("{}: lista de argumentos inválida", r.id));
                }
                for a in args {
                    check_arg(a).map_err(|e| format!("{}: {e}", r.id))?;
                }
            }
            RecipeAction::WriteGameFile { path, content } => {
                check_rel_path(path).map_err(|e| format!("{}: {e}", r.id))?;
                if content.len() > MAX_FILE_BYTES || content.contains('\0') {
                    return Err(format!("{}: conteúdo de arquivo inválido", r.id));
                }
            }
        }
    }
    Ok(())
}

/// `pack.<appid>.<hash>` — identificador estável de Smart Pack.
pub fn valid_pack_id(id: &str) -> bool {
    let mut parts = id.split('.');
    id_chars(id)
        && parts.next() == Some("pack")
        && parts.next().is_some_and(crate::art::valid_app_id)
        && parts
            .next()
            .is_some_and(|h| h.len() == 16 && h.bytes().all(|b| b.is_ascii_hexdigit()))
        && parts.next().is_none()
}

/// Validação estrutural de um Smart Pack composto: mesmas regras de ações
/// das receitas geradas (só variáveis conhecidas, sem repetição), um jogo,
/// pelo menos duas receitas de origem revisada ou gerada.
pub fn validate_pack(r: &Recipe) -> Result<(), String> {
    let p = r
        .pack
        .as_ref()
        .ok_or("Smart Pack sem composição registrada")?;
    if r.schema != RECIPE_SCHEMA
        || r.trust != Trust::Pack
        || r.review.is_some()
        || r.generated.is_some()
    {
        return Err("estado de confiança inválido para Smart Pack".into());
    }
    if !valid_pack_id(&r.id) || !valid_version(&r.version) {
        return Err("identificador ou versão inválidos".into());
    }
    let [app] = r.scope.app_ids.as_slice() else {
        return Err("Smart Pack precisa valer para exatamente um jogo".into());
    };
    if !r.id.starts_with(&format!("pack.{app}.")) || !r.scope.requires_proton {
        return Err("escopo inconsistente".into());
    }
    if !(2..=6).contains(&p.members.len())
        || p.members.iter().any(|m| {
            !matches!(m.trust, Trust::Reviewed | Trust::Generated)
                || !(valid_id(&m.recipe_id) || valid_generated_id(&m.recipe_id))
                || m.fingerprint.len() != 64
                || !m.fingerprint.bytes().all(|b| b.is_ascii_hexdigit())
                || m.trust == Trust::Generated && !m.recipe_id.starts_with(&format!("gen.{app}."))
        })
    {
        return Err("composição inválida".into());
    }
    if r.actions.is_empty() || r.actions.len() > 8 {
        return Err("número de ações inválido".into());
    }
    let mut keys = Vec::new();
    for a in &r.actions {
        let RecipeAction::SetLaunchEnv { key, value } = a else {
            return Err("Smart Packs só definem variáveis de ambiente".into());
        };
        check_env(key, value)?;
        if super::dynamic::compatibility::knowledge(key, value).is_none() {
            return Err(format!("{key}={value} não tem efeito conhecido pelo FRX"));
        }
        if keys.contains(&key) {
            return Err(format!("variável repetida: {key}"));
        }
        keys.push(key);
    }
    if r.sources.is_empty()
        || r.sources
            .iter()
            .any(|s| !super::dynamic::provenance::trusted_url(&s.url))
    {
        return Err("fontes precisam ser HTTPS do GitHub".into());
    }
    Ok(())
}

/// Validação independente de uma receita gerada, campo a campo. Mais
/// estreita que a do catálogo: só variáveis de ambiente com efeito conhecido
/// (`dynamic::compatibility`), um jogo só, fontes do GitHub e confirmação.
pub fn validate_generated(r: &Recipe) -> Result<(), String> {
    let g = r
        .generated
        .as_ref()
        .ok_or("receita gerada sem registro de origem")?;
    if r.schema != RECIPE_SCHEMA || r.trust != Trust::Generated || r.review.is_some() {
        return Err("estado de confiança inválido para receita gerada".into());
    }
    if !valid_generated_id(&r.id) || !valid_version(&r.version) {
        return Err("identificador ou versão inválidos".into());
    }
    let [app] = r.scope.app_ids.as_slice() else {
        return Err("receita gerada precisa valer para exatamente um jogo".into());
    };
    if !r.id.starts_with(&format!("gen.{app}.")) || !r.scope.requires_proton {
        return Err("escopo inconsistente".into());
    }
    if g.game_app_id.as_deref() != Some(app.as_str())
        || g.performance_evidence
            .app_id
            .as_ref()
            .is_some_and(|a| a != app)
    {
        return Err("origem gerada sem AppID confirmado ou com evidência de outro jogo; reanalise a descoberta".into());
    }
    if r.actions.is_empty() || r.actions.len() > 4 {
        return Err("número de ações inválido".into());
    }
    if !r.conflicts_with.is_empty() {
        return Err("campos não permitidos".into());
    }
    let mut keys = Vec::new();
    for a in &r.actions {
        let RecipeAction::SetLaunchEnv { key, value } = a else {
            return Err("receitas geradas só definem variáveis de ambiente".into());
        };
        check_env(key, value)?;
        if super::dynamic::compatibility::knowledge(key, value).is_none() {
            return Err(format!("{key}={value} não tem efeito conhecido pelo FRX"));
        }
        if keys.contains(&key) {
            return Err(format!("variável repetida: {key}"));
        }
        keys.push(key);
    }
    let text_ok = |s: &str, max: usize| s.len() <= max && !s.chars().any(char::is_control);
    if !text_ok(&r.title, 120)
        || !text_ok(&r.summary, 400)
        || !text_ok(&r.description, 1200)
        || r.effects
            .iter()
            .chain(&r.side_effects)
            .any(|e| !text_ok(e, 300))
    {
        return Err("texto inválido".into());
    }
    if r.sources.is_empty()
        || r.sources.len() > 8
        || r.sources
            .iter()
            .any(|s| !super::dynamic::provenance::trusted_url(&s.url))
    {
        return Err("fontes precisam ser HTTPS do GitHub".into());
    }
    if g.finding_ids.is_empty() || g.provenance.is_empty() {
        return Err("receita gerada sem descoberta de origem".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_allowlist_is_explicit() {
        assert!(check_env("PROTON_LOG", "1").is_ok());
        assert!(check_env("PROTON_LOG", "2").is_err());
        assert!(check_env("DXVK_FRAME_RATE", "60").is_ok());
        assert!(check_env("DXVK_FRAME_RATE", "6000").is_err());
        assert!(check_env("DXVK_HUD", "fps,devinfo").is_ok());
        assert!(check_env("DXVK_HUD", "fps;rm -rf").is_err());
        // Prefixo PROTON_ não basta.
        assert!(check_env("PROTON_SOMETHING_NEW", "1").is_err());
        for k in [
            "LD_PRELOAD",
            "LD_LIBRARY_PATH",
            "WINEDLLOVERRIDES",
            "PATH",
            "VK_INSTANCE_LAYERS",
            "MANGOHUD_CONFIGFILE",
            "STEAM_COMPAT_DATA_PATH",
            "WINEPREFIX",
        ] {
            let e = check_env(k, "1").unwrap_err();
            assert!(e.contains("nunca é aplicada"), "{k}: {e}");
        }
        for v in ["1;id", "$(id)", "`id`", "1 2", "1\n", "'1'"] {
            assert!(check_env("PROTON_LOG", v).is_err());
        }
    }

    #[test]
    fn args_are_simple_options() {
        for ok in ["-vulkan", "-dx11", "--skip-intro", "-fps=60"] {
            assert!(check_arg(ok).is_ok(), "{ok}");
        }
        for bad in [
            "vulkan",
            "---x",
            "-",
            "-a b",
            "-x;id",
            "-x/../../y",
            "-$(id)",
            "-\"x\"",
            "--",
        ] {
            assert!(check_arg(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn paths_stay_inside_the_game() {
        for bad in [
            "/etc/passwd",
            "../x.ini",
            "a/../../x.ini",
            "",
            "x.dll",
            "bin/game.exe",
            "a\\b.ini",
            "./x.ini",
        ] {
            assert!(check_rel_path(bad).is_err(), "{bad}");
        }
        assert!(check_rel_path("Config/Engine.ini").is_ok());
    }

    #[test]
    fn resolve_blocks_symlink_escapes() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("steamapps/common/Game");
        std::fs::create_dir_all(game.join("cfg")).unwrap();
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, game.join("link")).unwrap();
        std::os::unix::fs::symlink(outside.join("x.ini"), game.join("cfg/evil.ini")).unwrap();
        assert!(resolve_in_root(&game, "cfg/new.ini")
            .unwrap()
            .starts_with(std::fs::canonicalize(&game).unwrap()));
        assert!(resolve_in_root(&game, "link/x.ini")
            .unwrap_err()
            .contains("link simbólico"));
        assert!(resolve_in_root(&game, "cfg/evil.ini")
            .unwrap_err()
            .contains("link simbólico"));
        assert!(resolve_in_root(&game, "missing/x.ini").is_err());
        assert!(check_game_root(&game).is_ok());
        assert!(check_game_root(&tmp.path().join("steamapps/compatdata/1/pfx")).is_err());
        assert!(check_game_root(Path::new("/home/u")).is_err());
    }
}
