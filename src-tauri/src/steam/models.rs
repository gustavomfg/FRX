//! Modelos serializados enviados ao front-end (espelhados em `src/types/steam.ts`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ScanStatus {
    Ok,
    NoSteam,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InstallKind {
    Native,
    Flatpak,
    Snap,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SteamInstall {
    pub path: String,
    pub kind: InstallKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LibraryStatus {
    Available,
    /// Caminho ausente — disco desmontado ou removido.
    Unavailable,
    /// Existe, mas não pôde ser lido (permissão).
    Inaccessible,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryInfo {
    pub path: String,
    pub status: LibraryStatus,
    pub is_primary: bool,
    pub label: Option<String>,
    /// AppIDs que a Steam associa à biblioteca (de `libraryfolders.vdf`).
    pub declared_app_ids: Vec<String>,
    pub game_count: usize,
    pub steam_install: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InstallationStatus {
    Installed,
    Incomplete,
    Unavailable,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ToolKind {
    Official,
    Experimental,
    Hotfix,
    GeProton,
    Custom,
    Runtime,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompatTool {
    pub internal_name: Option<String>,
    pub display_name: String,
    pub kind: ToolKind,
    pub version: Option<String>,
    pub install_path: Option<String>,
    /// A ferramenta traz o UMU-Protonfixes embutido (GE-Proton, proton-cachyos…).
    pub includes_protonfixes: bool,
    pub origin: ToolOrigin,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ToolOrigin {
    SteamApps,
    CompatibilityToolsD,
    /// Citada na configuração, mas não encontrada no disco.
    Referenced,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ToolSource {
    /// Escolhida pelo usuário nas propriedades do jogo.
    PerGame,
    /// Padrão global do Steam Play — vale só se o jogo usar Proton.
    GlobalDefault,
    /// Deduzida do prefixo (última execução).
    LastUsed,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameCompatibility {
    pub tool: Option<CompatTool>,
    pub tool_source: ToolSource,
    /// Ferramenta da última execução, lida do prefixo (pode diferir da configurada).
    pub last_used_tool: Option<CompatTool>,
    pub prefix_path: Option<String>,
    /// Conteúdo de `compatdata/<appid>/version`.
    pub prefix_version: Option<String>,
    /// `Some(true)` quando há prefixo ou ferramenta por jogo; `None` quando não dá para saber.
    pub uses_proton: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstalledGame {
    pub app_id: String,
    pub name: String,
    pub install_path: Option<String>,
    pub library_path: String,
    pub installation_status: InstallationStatus,
    pub state_flags: Vec<String>,
    pub size_on_disk: Option<u64>,
    pub last_updated: Option<String>,
    pub last_played: Option<String>,
    pub build_id: Option<String>,
    pub playtime_minutes: Option<u64>,
    pub compatibility: GameCompatibility,
    pub source: String,
    pub steam_install: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub status: ScanStatus,
    pub steam_installs: Vec<SteamInstall>,
    pub libraries: Vec<LibraryInfo>,
    pub games: Vec<InstalledGame>,
    pub tools: Vec<CompatTool>,
    pub global_default_tool: Option<CompatTool>,
    pub warnings: Vec<String>,
    pub scanned_at: String,
    pub duration_ms: u64,
}
