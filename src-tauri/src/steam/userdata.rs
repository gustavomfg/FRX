//! Tempo de jogo do usuário mais recente (somente leitura de `localconfig.vdf`).

use std::collections::HashMap;
use std::path::Path;

use super::libraries::read_text;
use super::vdf;

const STEAMID64_BASE: u64 = 76_561_197_960_265_728;

/// AppID → minutos jogados, do último usuário que entrou na Steam.
pub fn playtime_minutes(root: &Path) -> HashMap<String, u64> {
    let mut out = HashMap::new();
    let Some(account) = most_recent_account(root) else {
        return out;
    };
    let file = root
        .join("userdata")
        .join(account.to_string())
        .join("config/localconfig.vdf");
    let Ok(text) = read_text(&file) else {
        return out;
    };
    let Ok(doc) = vdf::parse(&text) else {
        return out;
    };
    let Some(apps) = doc.path(&["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"])
    else {
        return out;
    };
    for (app, info) in apps.entries() {
        if let Some(min) = info.u64_at("Playtime").filter(|m| *m > 0) {
            out.insert(app.clone(), min);
        }
    }
    out
}

/// Opções de inicialização atuais do jogo na Steam (somente leitura).
pub fn launch_options(root: &Path, app_id: &str) -> Option<String> {
    let account = most_recent_account(root)?;
    let file = root
        .join("userdata")
        .join(account.to_string())
        .join("config/localconfig.vdf");
    let text = read_text(&file).ok()?;
    let doc = vdf::parse(&text).ok()?;
    let apps = doc.path(&["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"])?;
    apps.get(app_id)?
        .str_at("LaunchOptions")
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

pub fn most_recent_account(root: &Path) -> Option<u64> {
    let text = read_text(&root.join("config/loginusers.vdf")).ok()?;
    let doc = vdf::parse(&text).ok()?;
    let users = doc.get("users")?;
    let pick = users
        .entries()
        .iter()
        .find(|(_, u)| u.str_at("MostRecent") == Some("1"))
        .or_else(|| users.entries().first())?;
    pick.0.parse::<u64>().ok()?.checked_sub(STEAMID64_BASE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_playtime_of_most_recent_user() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("config")).unwrap();
        std::fs::write(
            root.join("config/loginusers.vdf"),
            r#""users" { "76561197960265729" { "MostRecent" "0" } "76561197960265738" { "MostRecent" "1" } }"#,
        )
        .unwrap();
        let cfg = root.join("userdata/10/config");
        std::fs::create_dir_all(&cfg).unwrap();
        std::fs::write(
            cfg.join("localconfig.vdf"),
            r#""UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" { "200260" { "Playtime" "125" "LaunchOptions" "PROTON_LOG=1 %command%" } "7" { "Playtime" "0" } } } } } }"#,
        )
        .unwrap();
        let p = playtime_minutes(root);
        assert_eq!(p.get("200260"), Some(&125));
        assert!(!p.contains_key("7"));
        assert_eq!(
            launch_options(root, "200260").as_deref(),
            Some("PROTON_LOG=1 %command%")
        );
        assert_eq!(launch_options(root, "7"), None);
    }
}
