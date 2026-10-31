//! Configuração normalizada: uma identidade estável para deduplicar sugestões.
//!
//! Com receita, a identidade vem das ações tipadas dela. Sem receita, vem dos
//! campos de ação da descoberta. O trecho citado da fonte nunca é consultado.

use super::models::{ConfigItem, ConfigKey};
use crate::fix_engine::dynamic::extraction::{dxvk_option, Deterministic, Extracted, Interpreter};
use crate::fix_engine::models::{Recipe, RecipeAction};
use crate::fixes::models::{ActionKind, FixAction, FixFinding};

fn item(kind: &str, name: &str, value: Option<&str>) -> ConfigItem {
    ConfigItem {
        kind: kind.into(),
        name: name.trim().to_owned(),
        value: value.map(|v| v.trim().to_owned()),
    }
}

pub fn key_of(mut items: Vec<ConfigItem>) -> ConfigKey {
    items.sort_by(|a, b| (&a.kind, &a.name, &a.value).cmp(&(&b.kind, &b.name, &b.value)));
    items.dedup();
    let id = items
        .iter()
        .map(|i| match &i.value {
            Some(v) => format!("{}:{}={}", i.kind, i.name, v),
            None => format!("{}:{}", i.kind, i.name),
        })
        .collect::<Vec<_>>()
        .join("+");
    ConfigKey { id, items }
}

pub fn from_recipe(recipe: &Recipe) -> ConfigKey {
    let items = recipe
        .actions
        .iter()
        .map(|a| match a {
            RecipeAction::SetLaunchEnv { key, value } => item("env", key, Some(value)),
            RecipeAction::SetDxvkOption { path, key, value } => {
                item("dxvk", &format!("{path}:{key}"), Some(value))
            }
            RecipeAction::AddLaunchArgs { args } => item("launch", &args.join(" "), None),
            RecipeAction::WriteGameFile { path, .. } => item("file", path, None),
            RecipeAction::PatchKnownConfig { path, .. } => item("file", path, None),
            RecipeAction::InstallArtifact { artifact } => item("artifact", &artifact.id, None),
        })
        .collect();
    key_of(items)
}

fn raw_item(a: &FixAction) -> ConfigItem {
    let kind = match a.kind {
        ActionKind::Winetricks => "component",
        ActionKind::DllOverride => "dll",
        ActionKind::Registry => "registry",
        ActionKind::ProtonVersion => "proton",
        ActionKind::ConfigFile => "config",
        ActionKind::LaunchOption => "launch",
        ActionKind::Env | ActionKind::Other => "other",
    };
    let value: String = a
        .value
        .chars()
        .filter(|c| !c.is_control())
        .take(120)
        .collect();
    let value = if a.kind == ActionKind::Winetricks {
        value.to_ascii_lowercase()
    } else {
        value
    };
    item(kind, &value, None)
}

pub fn from_finding(f: &FixFinding) -> ConfigKey {
    let parsed = Deterministic.interpret(f).ok();
    let mut items = Vec::new();
    let mut env = parsed.iter().flatten().filter_map(|e| match e {
        Extracted::Env { key, value, .. } => Some((key.clone(), value.clone())),
        Extracted::Unsupported { .. } => None,
    });
    for a in &f.actions {
        match a.kind {
            ActionKind::Env | ActionKind::LaunchOption if parsed.is_some() => {
                let pairs: Vec<_> = match a.kind {
                    ActionKind::Env => env.next().into_iter().collect(),
                    _ => a
                        .value
                        .split_whitespace()
                        .take_while(|t| *t != "%command%")
                        .filter(|t| t.contains('='))
                        .filter_map(|_| env.next())
                        .collect(),
                };
                if pairs.is_empty() {
                    items.push(raw_item(a));
                }
                items.extend(pairs.iter().map(|(k, v)| item("env", k, Some(v))));
            }
            ActionKind::ConfigFile => match dxvk_option(&a.value) {
                Ok((path, key, value)) => {
                    items.push(item("dxvk", &format!("{path}:{key}"), Some(&value)))
                }
                Err(_) => items.push(raw_item(a)),
            },
            _ => items.push(raw_item(a)),
        }
    }
    key_of(items)
}
