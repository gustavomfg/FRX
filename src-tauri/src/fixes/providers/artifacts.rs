//! Artifact discovery reads publisher metadata only. It never downloads a
//! payload, executes a source instruction or lets a comment choose a target.
use reqwest::Url;
use serde::Deserialize;

use super::super::aggregation::finding_id;
use super::super::http::{Ctx, Outcome};
use super::super::models::*;
use super::GameQuery;
use crate::net::{Fetcher, HttpRequest};

const MAX_REFERENCES: usize = 4;
const MAX_ASSETS: usize = 8;
const MAX_METADATA: usize = 2 * 1024 * 1024;

/// A reviewed publisher is still not an assertion that its payload is benign.
/// The execution policy additionally restricts type, checksum and destination.
pub fn recognized_repository(repo: &str) -> bool {
    [
        "ValveSoftware/Proton",
        "GloriousEggroll/proton-ge-custom",
        "Open-Wine-Components/umu-protonfixes",
        "CachyOS/proton-cachyos",
        "doitsujin/dxvk",
        "HansKristian-Work/vkd3d-proton",
        "jp7677/dxvk-nvapi",
        "SveSop/nvidia-libs",
        "SveSop/nvcuda",
    ]
    .contains(&repo)
}

#[derive(Clone)]
pub struct Document {
    pub source: FindingSource,
    pub scope: DiscoveryScope,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
struct Reference {
    repository: String,
    tag: Option<String>,
    asset_name: Option<String>,
}

fn clean_component(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 120
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

fn links(text: &str) -> Vec<String> {
    text.split(|c: char| c.is_whitespace() || "<>\"'`()[]".contains(c))
        .filter(|s| s.starts_with("https://"))
        .map(|s| s.trim_end_matches([',', '.', ';']).to_owned())
        .take(32)
        .collect()
}

fn reference(raw: &str) -> Option<Reference> {
    let u = Url::parse(raw).ok()?;
    if u.scheme() != "https"
        || u.host_str() != Some("github.com")
        || !u.username().is_empty()
        || u.password().is_some()
        || u.port().is_some()
        || u.query().is_some()
    {
        return None;
    }
    let p: Vec<_> = u.path().trim_matches('/').split('/').collect();
    if p.len() < 2 || !clean_component(p[0]) || !clean_component(p[1]) {
        return None;
    }
    let repository = format!("{}/{}", p[0], p[1]);
    match p.as_slice() {
        [_, _] => Some(Reference {
            repository,
            tag: None,
            asset_name: None,
        }),
        [_, _, "releases", "latest"] => Some(Reference {
            repository,
            tag: None,
            asset_name: None,
        }),
        [_, _, "releases", "tag", tag] if clean_component(tag) => Some(Reference {
            repository,
            tag: Some((*tag).into()),
            asset_name: None,
        }),
        [_, _, "releases", "download", tag, name]
            if clean_component(tag) && clean_component(name) =>
        {
            Some(Reference {
                repository,
                tag: Some((*tag).into()),
                asset_name: Some((*name).into()),
            })
        }
        _ => None,
    }
}

pub fn valid_sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn digest(s: Option<&str>) -> Option<String> {
    s.and_then(|s| s.strip_prefix("sha256:").or(Some(s)))
        .filter(|s| valid_sha256(s))
        .map(str::to_ascii_lowercase)
}

fn text_file(path: &str) -> bool {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    ["ini", "cfg", "conf", "json", "xml", "txt"].contains(&ext.as_str())
}

fn relative(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 200
        && !path.contains(['\\', ':', '\0'])
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
        && !path.starts_with('/')
}

#[derive(Deserialize)]
struct Author {
    login: String,
}
#[derive(Deserialize)]
struct Asset {
    id: u64,
    name: String,
    browser_download_url: String,
    size: u64,
    #[serde(default)]
    digest: Option<String>,
    #[serde(default)]
    content_type: Option<String>,
    #[serde(default)]
    state: Option<String>,
}
#[derive(Deserialize)]
struct Release {
    id: u64,
    tag_name: String,
    html_url: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    author: Option<Author>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    schema: u32,
    app_id: String,
    asset: String,
    #[serde(default)]
    platform: Option<String>,
    #[serde(default)]
    architecture: Option<String>,
    files: Vec<ArtifactFile>,
    #[serde(default)]
    license: Option<String>,
}

fn explicit_ids(body: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let low = body.to_ascii_lowercase();
    for prefix in [
        "steam appid:",
        "steam appid: ",
        "steam app id:",
        "appid:",
        "store.steampowered.com/app/",
    ] {
        for (_, tail) in low
            .match_indices(prefix)
            .map(|(i, _)| (i, &low[i + prefix.len()..]))
        {
            let id: String = tail
                .trim_start_matches([' ', '`'])
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            if crate::art::valid_app_id(&id) && !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    ids
}

fn explicit_app_id(body: &str, app: &str) -> bool {
    explicit_ids(body) == [app]
}

fn clean_value(line: &str) -> String {
    line.trim().trim_matches('`').trim().to_owned()
}

fn supported_context_labels(body: &str) -> bool {
    let mut vendor_seen = false;
    let mut arch_seen = false;
    let mut build_seen = false;
    for line in body.lines().map(str::trim) {
        if let Some(v) = line.strip_prefix("GPU-Vendor:") {
            if vendor_seen
                || !["nvidia", "amd", "intel"]
                    .contains(&clean_value(v).to_ascii_lowercase().as_str())
            {
                return false;
            }
            vendor_seen = true;
        }
        if let Some(v) = line.strip_prefix("CPU-Arch:") {
            if arch_seen || !["x86", "x86_64", "aarch64"].contains(&clean_value(v).as_str()) {
                return false;
            }
            arch_seen = true;
        }
        if let Some(v) = line.strip_prefix("Game-Build:") {
            if build_seen || clean_value(v).is_empty() {
                return false;
            }
            build_seen = true;
        }
        for field in ["GPU-Device:", "GPU-Model:", "Driver:", "Proton:"] {
            if let Some(v) = line.strip_prefix(field) {
                let value = clean_value(v);
                if value.is_empty() || value.len() > 160 {
                    return false;
                }
                if field == "GPU-Device:"
                    && !value
                        .split(':')
                        .all(|part| part.len() == 4 && part.bytes().all(|b| b.is_ascii_hexdigit()))
                {
                    return false;
                }
                if field == "GPU-Device:" && value.split(':').count() > 2 {
                    return false;
                }
            }
        }
    }
    true
}

fn metadata_line(line: &str) -> bool {
    line.is_empty()
        || line.starts_with('#')
        || [
            "Steam AppID:",
            "Steam App ID:",
            "AppID:",
            "Platform:",
            "Architecture:",
            "License:",
            "GPU-Vendor:",
            "GPU-Device:",
            "GPU-Model:",
            "Driver:",
            "Proton:",
            "CPU-Arch:",
            "Game-Build:",
            "Description:",
        ]
        .iter()
        .any(|field| line.starts_with(field))
}

/// Exact publisher labels are constraints, never guessed from a filename.
fn hardware_requirements(body: &str) -> Requirements {
    let mut r = Requirements::default();
    for line in body.lines().map(str::trim) {
        if let Some(v) = line.strip_prefix("GPU-Vendor:") {
            r.hardware.gpu_vendor = match clean_value(v).to_ascii_lowercase().as_str() {
                "nvidia" => Some(GpuReq::Nvidia),
                "amd" => Some(GpuReq::Amd),
                "intel" => Some(GpuReq::Intel),
                _ => None,
            };
            r.gpu_vendor = r.hardware.gpu_vendor;
        }
        if let Some(v) = line.strip_prefix("GPU-Device:") {
            r.hardware.gpu_device_ids.push(clean_value(v));
        }
        if let Some(v) = line.strip_prefix("GPU-Model:") {
            r.hardware.gpu_models.push(clean_value(v));
        }
        if let Some(v) = line.strip_prefix("Driver:") {
            r.hardware.driver_versions.push(clean_value(v));
        }
        if let Some(v) = line.strip_prefix("Proton:") {
            let version = clean_value(v);
            r.proton_versions.push(version.clone());
            r.hardware.proton_versions.push(version);
        }
        if let Some(v) = line.strip_prefix("CPU-Arch:") {
            r.hardware.cpu_arch = Some(clean_value(v));
        }
        if let Some(v) = line.strip_prefix("Game-Build:") {
            r.game_version = Some(clean_value(v));
        }
    }
    r
}

/// Publishers may document exact mappings with a small strict Markdown
/// convention; unknown prose remains a review item, never guessed actions.
fn instructions(body: &str, app: &str, asset: &str, format: ArtifactFormat) -> Option<Manifest> {
    let low = body.to_ascii_lowercase();
    if explicit_ids(body).iter().any(|id| id != app)
        || !supported_context_labels(body)
        || [
            "run:",
            "execute:",
            "then run ",
            "winetricks ",
            "protontricks ",
            "sudo ",
            "regedit",
            "winedlloverrides",
        ]
        .iter()
        .any(|step| low.contains(step))
    {
        return None;
    }
    if let Some((before, after)) = body.split_once("```frx-file-fix\n") {
        let (json, rest) = after.split_once("```")?;
        if json.len() > 32 * 1024
            || before
                .lines()
                .chain(rest.lines())
                .map(str::trim)
                .any(|line| !metadata_line(line))
        {
            return None;
        }
        let m: Manifest = serde_json::from_str(json).ok()?;
        return (m.schema == 1
            && m.app_id == app
            && m.asset == asset
            && (1..=8).contains(&m.files.len()))
        .then_some(m);
    }
    if !explicit_app_id(body, app) {
        return None;
    }
    // This is a strict publisher protocol, not a best-effort prose parser.
    // An unrecognized line could be an essential extra installation step.
    let fields = [
        "Steam AppID:",
        "Steam App ID:",
        "AppID:",
        "Platform:",
        "Architecture:",
        "License:",
        "GPU-Vendor:",
        "GPU-Device:",
        "GPU-Model:",
        "Driver:",
        "Proton:",
        "CPU-Arch:",
        "Game-Build:",
        "Install:",
        "Replace:",
        "Extract:",
        "SHA256 (",
        "Description:",
    ];
    if body.lines().map(str::trim).any(|line| {
        !line.is_empty()
            && !line.starts_with('#')
            && !fields.iter().any(|field| line.starts_with(field))
    }) {
        return None;
    }
    let mut m = Manifest {
        schema: 1,
        app_id: app.into(),
        asset: asset.into(),
        ..Default::default()
    };
    for line in body.lines().map(str::trim) {
        if let Some(v) = line.strip_prefix("Platform:") {
            m.platform = Some(clean_value(v));
        }
        if let Some(v) = line.strip_prefix("Architecture:") {
            m.architecture = Some(clean_value(v));
        }
        if let Some(v) = line.strip_prefix("License:") {
            m.license = Some(clean_value(v));
        }
        if let Some(v) = line.strip_prefix("Install:") {
            let (target, name) = v.split_once("<-")?;
            if clean_value(name) != asset || format != ArtifactFormat::File {
                return None;
            }
            m.files.push(ArtifactFile {
                source_path: None,
                target_path: clean_value(target),
                sha256: None,
                replace: false,
                expected_original_sha256: None,
            });
        }
        if let Some(v) = line.strip_prefix("Replace:") {
            let (target, name) = v.split_once("<-")?;
            if clean_value(name) != asset || format != ArtifactFormat::File {
                return None;
            }
            m.files.push(ArtifactFile {
                source_path: None,
                target_path: clean_value(target),
                sha256: None,
                replace: true,
                expected_original_sha256: None,
            });
        }
        if let Some(v) = line.strip_prefix("Extract:") {
            let (from, target) = v.split_once("->")?;
            let (archive, member) = from.split_once("::")?;
            if clean_value(archive) != asset || format != ArtifactFormat::Zip {
                return None;
            }
            m.files.push(ArtifactFile {
                source_path: Some(clean_value(member)),
                target_path: clean_value(target),
                sha256: None,
                replace: false,
                expected_original_sha256: None,
            });
        }
    }
    for f in &mut m.files {
        let name = f.source_path.as_deref().unwrap_or(asset);
        for line in body.lines().map(str::trim) {
            let prefix = format!("SHA256 ({name})=");
            if let Some(v) = line.strip_prefix(&prefix) {
                f.sha256 = digest(Some(v.trim()));
            }
        }
    }
    (1..=8).contains(&m.files.len()).then_some(m)
}

pub fn candidates_from_release(
    q: &GameQuery,
    document: &Document,
    repository: &str,
    _metadata_url: &str,
    json: &str,
) -> Vec<ArtifactCandidate> {
    let Ok(r) = serde_json::from_str::<Release>(json) else {
        return vec![];
    };
    if !clean_component(&r.tag_name)
        || r.html_url
            != format!(
                "https://github.com/{repository}/releases/tag/{}",
                r.tag_name
            )
    {
        return vec![];
    }
    let body = r.body.as_deref().unwrap_or("");
    let signatures: Vec<_> = r
        .assets
        .iter()
        .map(|a| (a.name.clone(), a.browser_download_url.clone()))
        .collect();
    let mut candidates = Vec::new();
    for a in r.assets.into_iter().take(MAX_ASSETS) {
        let format = if a.name.ends_with(".zip") {
            ArtifactFormat::Zip
        } else if text_file(&a.name) {
            ArtifactFormat::File
        } else {
            ArtifactFormat::Unknown
        };
        let sha256 = digest(a.digest.as_deref());
        let m = instructions(body, &q.app_id, &a.name, format);
        let scope = if m.is_some() {
            DiscoveryScope::verified(&q.app_id, format!(
                "A documentação do publicador identifica o AppID {} e os destinos exatos na release {}", q.app_id, r.html_url
            ))
        } else {
            document.scope.clone()
        };
        let mut files = m.as_ref().map(|m| m.files.clone()).unwrap_or_default();
        if format == ArtifactFormat::File {
            for f in &mut files {
                if f.sha256.is_none() {
                    f.sha256 = sha256.clone();
                }
            }
        }
        let canonical_url = format!(
            "https://github.com/{repository}/releases/download/{}/{}",
            r.tag_name, a.name
        );
        let mut reasons = vec!["Metadados da release identificam o publicador; checksum verifica integridade relativa à referência, não que o conteúdo seja benigno.".into()];
        let mut untrusted = false;
        if r.id == 0 || a.id == 0 || !clean_component(&a.name) {
            reasons.push("Identidade ou nome do asset publicado inválidos.".into());
            untrusted = true;
        }
        if !scope.matches_game(&q.app_id) {
            reasons.push(
                "A recomendação e o publicador não possuem vínculo confirmado com este AppID."
                    .into(),
            );
            untrusted = true;
        }
        if !recognized_repository(repository) {
            reasons.push(
                "O projeto ainda não integra a política de publicadores revisados do FRX.".into(),
            );
            untrusted = true;
        }
        if a.browser_download_url != canonical_url {
            reasons
                .push("A URL do asset não corresponde à release e ao arquivo publicados.".into());
            untrusted = true;
        }
        if sha256.is_none() {
            reasons.push("A API da release não publica um digest SHA-256 verificável.".into());
        }
        if m.is_none() {
            reasons.push("Falta um mapeamento de instalação completo no formato suportado do publicador, ou há etapas não reconhecidas; nenhum destino ou procedimento foi adivinhado.".into());
        }
        if format == ArtifactFormat::Unknown || files.iter().any(|f| !text_file(&f.target_path)) {
            reasons.push("Binários, DLLs, scripts e formatos não suportados exigem revisão adicional e não são instalados automaticamente.".into());
        }
        if files.iter().any(|f| {
            !relative(&f.target_path) || f.source_path.as_ref().is_some_and(|s| !relative(s))
        }) {
            reasons.push(
                "Há caminho absoluto, traversal ou nome de membro inválido nas instruções.".into(),
            );
            untrusted = true;
        }
        if files
            .iter()
            .any(|f| f.sha256.as_ref().is_none_or(|s| !valid_sha256(s)))
        {
            reasons.push("Falta SHA-256 publicado de cada arquivo selecionado.".into());
        }
        if r.draft || r.prerelease {
            reasons.push("Release preliminar ou rascunho: revisão adicional necessária.".into());
        }
        if r.author.as_ref().is_none_or(|u| u.login.trim().is_empty()) {
            reasons.push("Mantenedor da release não identificado.".into());
        }
        if a.state.as_deref() != Some("uploaded") {
            reasons.push("O asset não está confirmado como upload concluído na API.".into());
        }
        let digest_conflict = format == ArtifactFormat::File
            && files.iter().any(|f| {
                f.sha256
                    .as_ref()
                    .zip(sha256.as_ref())
                    .is_some_and(|(a, b)| a != b)
            });
        if digest_conflict {
            reasons.push("O checksum da instrução contradiz o digest publicado do asset; revisão adicional necessária.".into());
        }
        let eligible = !untrusted
            && !digest_conflict
            && sha256.is_some()
            && m.is_some()
            && !files.is_empty()
            && format != ArtifactFormat::Unknown
            && files.iter().all(|f| {
                text_file(&f.target_path)
                    && relative(&f.target_path)
                    && f.sha256.as_ref().is_some_and(|s| valid_sha256(s))
            })
            && !r.draft
            && !r.prerelease
            && r.author
                .as_ref()
                .is_some_and(|u| !u.login.trim().is_empty())
            && a.state.as_deref() == Some("uploaded");
        let trust = if untrusted {
            ArtifactTrust::Untrusted
        } else if eligible {
            ArtifactTrust::OriginAndIntegrity
        } else {
            ArtifactTrust::Partial
        };
        let key = format!(
            "{repository}@{}:{}:{}:{}",
            r.tag_name,
            a.name,
            sha256.as_deref().unwrap_or("unknown"),
            serde_json::to_string(&files).unwrap_or_default()
        );
        let signature_url = signatures
            .iter()
            .find(|(name, url)| {
                (name == &format!("{}.sig", a.name) || name == &format!("{}.asc", a.name))
                    && url.starts_with(&format!(
                        "https://github.com/{repository}/releases/download/{}/",
                        r.tag_name
                    ))
            })
            .map(|(_, url)| url.clone());
        candidates.push(ArtifactCandidate {
            id: finding_id("artifact", &q.app_id, &key),
            app_id: scope.app_id.clone().unwrap_or_default(),
            scope,
            recommendation_source: document.source.clone(),
            repository: repository.into(),
            release_tag: r.tag_name.clone(),
            release_url: r.html_url.clone(),
            metadata_url: format!(
                "https://api.github.com/repos/{repository}/releases/{}",
                r.id
            ),
            release_id: r.id,
            publisher: r.author.as_ref().map(|u| u.login.clone()),
            asset_id: a.id,
            asset_name: a.name,
            asset_url: a.browser_download_url,
            size_bytes: a.size,
            content_type: a.content_type,
            sha256,
            signature_url,
            format,
            platform: m.as_ref().and_then(|m| m.platform.clone()),
            architecture: m.as_ref().and_then(|m| m.architecture.clone()),
            files,
            requirements: hardware_requirements(body),
            trust,
            reasons,
            license: m.and_then(|m| m.license),
            published_at: r.published_at.clone(),
        });
    }
    candidates
}

fn finding(q: &GameQuery, c: ArtifactCandidate) -> FixFinding {
    FixFinding {
        artifacts: vec![c.clone()], scope: c.scope.clone(), performance_evidence: Default::default(),
        id: c.id.clone(), game_app_id: c.app_id.clone(),
        title: format!("Correção por arquivo · {}", c.asset_name),
        description: format!("Arquivo publicado por {} na release {}. {}", c.repository, c.release_tag, c.reasons.join(" ")),
        category: Category::Compatibility, kind: FindingKind::Tweak, actions: vec![],
        sources: vec![c.recommendation_source.clone(), FindingSource {
            provider: ProviderId::Github, label: format!("{} · {}", c.repository, c.release_tag),
            url: c.release_url.clone(), reference_id: Some(c.release_tag.clone()), snippet: None,
            published_at: c.published_at.clone(),
        }],
        requirements: c.requirements.clone(),
        confidence: if c.trust == ArtifactTrust::OriginAndIntegrity { Confidence::Curated } else { Confidence::Unverified },
        compatibility: Compatibility::Unknown,
        compatibility_note: "A compatibilidade de hardware e a política de instalação são revalidadas pelo núcleo antes da prévia.".into(),
        experimental: c.trust != ArtifactTrust::OriginAndIntegrity, analysis: AnalysisState::Parsed,
        evidence: Default::default(), published_at: c.published_at,
        retrieved_at: q.retrieved_at.clone(), keys: vec![],
    }
}

pub async fn discover<F: Fetcher>(
    ctx: &Ctx<'_, F>,
    q: &GameQuery,
    documents: &[Document],
) -> (Vec<FixFinding>, Vec<Outcome>) {
    let mut refs = Vec::new();
    let mut findings = Vec::new();
    for d in documents {
        for link in links(&d.text) {
            if let Some(r) = reference(&link) {
                if !refs.iter().any(|(_, old)| *old == r) {
                    refs.push((d.clone(), r));
                }
            } else if findings.len() < MAX_REFERENCES {
                if let Ok(url) = Url::parse(&link) {
                    let name = url
                        .path_segments()
                        .and_then(|mut p| p.next_back())
                        .unwrap_or("");
                    if ([".zip", ".dll", ".ini", ".cfg", ".conf", ".tar.xz"]
                        .iter()
                        .any(|ext| name.ends_with(ext))
                        || url.path().contains("/user-attachments/"))
                        && url.username().is_empty()
                        && url.password().is_none()
                    {
                        let c=ArtifactCandidate {id:finding_id("artifact-unverified",&q.app_id,&link),app_id:d.scope.app_id.clone().unwrap_or_default(),scope:d.scope.clone(),recommendation_source:d.source.clone(),repository:String::new(),release_tag:String::new(),release_url:link.clone(),metadata_url:String::new(),release_id:0,publisher:None,asset_id:0,asset_name:name.chars().take(120).collect(),asset_url:link,size_bytes:0,content_type:None,sha256:None,signature_url:None,format:ArtifactFormat::Unknown,platform:None,architecture:None,files:vec![],requirements:Requirements::default(),trust:ArtifactTrust::Untrusted,reasons:vec!["Download citado na fonte, sem release/publicador e integridade suficientemente verificáveis. Requer revisão; nenhum payload foi baixado e nenhum destino foi presumido.".into()],license:None,published_at:None};
                        if !findings.iter().any(|f: &FixFinding| f.id == c.id) {
                            findings.push(finding(q, c));
                        }
                    }
                }
            }
            if refs.len() >= MAX_REFERENCES {
                break;
            }
        }
        if refs.len() >= MAX_REFERENCES {
            break;
        }
    }
    let mut outcomes = Vec::new();
    for (document, r) in refs {
        let url = match &r.tag {
            Some(tag) => format!(
                "https://api.github.com/repos/{}/releases/tags/{tag}",
                r.repository
            ),
            None => format!(
                "https://api.github.com/repos/{}/releases/latest",
                r.repository
            ),
        };
        let o = ctx
            .get(
                HttpRequest::new(&url, "application/vnd.github+json", MAX_METADATA),
                Some("core"),
                ctx.ttl_secs,
            )
            .await;
        if let Some(body) = &o.body {
            for c in candidates_from_release(q, &document, &r.repository, &url, body)
                .into_iter()
                .filter(|c| {
                    r.asset_name
                        .as_ref()
                        .is_none_or(|name| name == &c.asset_name)
                })
            {
                if !findings.iter().any(|f: &FixFinding| f.id == c.id) {
                    findings.push(finding(q, c));
                }
            }
        }
        outcomes.push(o);
    }
    (findings, outcomes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;
    use crate::fixes::http::EngineState;
    use crate::net::mock::MockFetcher;
    use serde_json::json;

    fn q() -> GameQuery {
        GameQuery {
            app_id: "200260".into(),
            name: "Synthetic game A".into(),
            retrieved_at: "fixture".into(),
            tool: None,
        }
    }
    fn document() -> Document {
        Document {
            source: FindingSource {
                provider: ProviderId::Github,
                label: "Synthetic parent issue".into(),
                url: "https://github.com/ValveSoftware/Proton/issues/999999#issuecomment-1".into(),
                reference_id: None,
                snippet: None,
                published_at: None,
            },
            scope: DiscoveryScope::verified("200260", "Synthetic explicit Steam AppID in parent"),
            text: "https://github.com/ValveSoftware/Proton/releases/tag/fixture-1".into(),
        }
    }
    fn release(body: &str, name: &str) -> serde_json::Value {
        json!({"id":42,"tag_name":"fixture-1","html_url":"https://github.com/ValveSoftware/Proton/releases/tag/fixture-1","body":body,"author":{"login":"ValveSoftware"},"published_at":"2026-10-27T00:00:00Z","draft":false,"prerelease":false,"assets":[{"id":7,"name":name,"size":123,"state":"uploaded","content_type":"text/plain","digest":format!("sha256:{}","a".repeat(64)),"browser_download_url":format!("https://github.com/ValveSoftware/Proton/releases/download/fixture-1/{name}")}]})
    }
    fn parse(r: &serde_json::Value, d: &Document) -> ArtifactCandidate {
        candidates_from_release(
            &q(),
            d,
            "ValveSoftware/Proton",
            "https://api.github.com/repos/ValveSoftware/Proton/releases/tags/fixture-1",
            &r.to_string(),
        )
        .remove(0)
    }
    fn instructions() -> &'static str {
        "Steam AppID: 200260\nInstall: Config/fix.ini <- fix.ini\nPlatform: windows\nArchitecture: any\nLicense: MIT"
    }

    #[test]
    fn canonical_release_digest_and_publisher_mapping_make_a_scoped_config_candidate() {
        let c = parse(&release(instructions(), "fix.ini"), &document());
        assert_eq!(c.trust, ArtifactTrust::OriginAndIntegrity);
        assert_eq!(c.app_id, "200260");
        assert_eq!(
            c.metadata_url,
            "https://api.github.com/repos/ValveSoftware/Proton/releases/42"
        );
        assert_eq!(c.sha256, Some("a".repeat(64)));
        assert_eq!(c.files[0].target_path, "Config/fix.ini");
        assert_eq!(c.files[0].sha256, c.sha256);
        assert_eq!(c.license.as_deref(), Some("MIT"));
    }

    #[test]
    fn comments_do_not_choose_destinations_or_checksums() {
        let mut doc = document();
        doc.text = "Steam AppID: 200260\nInstall: Config/fix.ini <- fix.ini".into();
        let c = parse(&release("", "fix.ini"), &doc);
        assert_eq!(c.trust, ArtifactTrust::Partial);
        assert!(c.files.is_empty());
        assert!(c.reasons.iter().any(|r| r.contains("mapeamento")));
    }

    #[test]
    fn publisher_identity_can_verify_a_reference_without_appid_in_community_source() {
        let mut doc = document();
        doc.scope = DiscoveryScope::default();
        let good = parse(&release(instructions(), "fix.ini"), &doc);
        assert!(good.scope.matches_game("200260"));
        let ambiguous = parse(
            &release("Put the file in the game directory", "fix.ini"),
            &doc,
        );
        assert_eq!(ambiguous.trust, ArtifactTrust::Untrusted);
        assert!(ambiguous.app_id.is_empty());
    }

    #[test]
    fn another_games_publisher_instructions_are_not_transferred() {
        let c = parse(
            &release(
                "Steam AppID: 570\nInstall: Config/fix.ini <- fix.ini",
                "fix.ini",
            ),
            &document(),
        );
        assert_eq!(c.trust, ArtifactTrust::Partial);
        assert!(c.files.is_empty());
    }

    #[test]
    fn dll_or_unknown_binary_and_missing_digest_require_review() {
        let dll = parse(
            &release(
                "Steam AppID: 200260\nInstall: Binaries/fix.dll <- fix.dll",
                "fix.dll",
            ),
            &document(),
        );
        assert_ne!(dll.trust, ArtifactTrust::OriginAndIntegrity);
        assert!(dll.reasons.iter().any(|r| r.contains("DLLs")));
        let mut nohash = release(instructions(), "fix.ini");
        nohash["assets"][0]["digest"] = serde_json::Value::Null;
        assert_eq!(parse(&nohash, &document()).trust, ArtifactTrust::Partial);
    }

    #[test]
    fn incomplete_external_steps_and_conflicting_checksums_require_review() {
        let body = format!("{}\nThen run an-installer.exe", instructions());
        assert_eq!(
            parse(&release(&body, "fix.ini"), &document()).trust,
            ArtifactTrust::Partial
        );
        let body = format!("{}\nSHA256 (fix.ini)={}", instructions(), "b".repeat(64));
        assert_eq!(
            parse(&release(&body, "fix.ini"), &document()).trust,
            ArtifactTrust::Partial
        );
    }

    #[test]
    fn publisher_manifest_cannot_override_a_foreign_explicit_game_id() {
        let body = format!(
            "Steam AppID: 570\n```frx-file-fix\n{}\n```",
            json!({"schema":1,"appId":"200260","asset":"fix.ini","files":[{"sourcePath":null,"targetPath":"Config/fix.ini","sha256":"a".repeat(64),"replace":false}]})
        );
        assert_eq!(
            parse(&release(&body, "fix.ini"), &document()).trust,
            ArtifactTrust::Partial
        );
    }

    #[test]
    fn unknown_publisher_and_asset_url_mismatch_are_untrusted() {
        let r = release(instructions(), "fix.ini");
        let other =
            candidates_from_release(&q(), &document(), "random/not-reviewed", "", &r.to_string());
        assert!(
            other.is_empty(),
            "canonical release URL must match repository"
        );
        let mut r = r;
        r["assets"][0]["browser_download_url"] = "https://evil.example/fix.ini".into();
        assert_eq!(parse(&r, &document()).trust, ArtifactTrust::Untrusted);
    }

    #[test]
    fn selected_zip_members_require_exact_paths_and_published_individual_hashes() {
        let body=format!("Steam AppID: 200260\nExtract: fix.zip::config/fix.ini -> Config/fix.ini\nSHA256 (config/fix.ini)={}\n", "b".repeat(64));
        let c = parse(&release(&body, "fix.zip"), &document());
        assert_eq!(c.trust, ArtifactTrust::OriginAndIntegrity);
        assert_eq!(c.files[0].source_path.as_deref(), Some("config/fix.ini"));
        assert_eq!(c.files[0].sha256, Some("b".repeat(64)));
        let c = parse(
            &release(
                "Steam AppID: 200260\nExtract: fix.zip::config/fix.ini -> Config/fix.ini",
                "fix.zip",
            ),
            &document(),
        );
        assert_eq!(c.trust, ArtifactTrust::Partial);
    }

    #[test]
    fn traversal_and_absolute_destinations_never_become_applicable() {
        for target in [
            "../other-game/fix.ini",
            "/etc/fix.ini",
            "Config/../fix.ini",
            "C:\\Windows\\fix.ini",
        ] {
            let body = format!("Steam AppID: 200260\nInstall: {target} <- fix.ini");
            assert_eq!(
                parse(&release(&body, "fix.ini"), &document()).trust,
                ArtifactTrust::Untrusted,
                "{target}"
            );
        }
    }

    #[test]
    fn structured_manifest_is_scoped_and_does_not_ignore_unknown_steps() {
        let body = format!(
            "```frx-file-fix\n{}\n```",
            json!({"schema":1,"appId":"200260","asset":"fix.ini","files":[{"sourcePath":null,"targetPath":"Config/fix.ini","sha256":"a".repeat(64),"replace":true}],"platform":"windows","architecture":"any"})
        );
        let c = parse(&release(&body, "fix.ini"), &document());
        assert_eq!(c.trust, ArtifactTrust::OriginAndIntegrity);
        assert!(c.files[0].replace);
        let bad = body.replace(
            "\"schema\":1",
            "\"shell\":\"arbitrary command\",\"schema\":1",
        );
        assert_eq!(
            parse(&release(&bad, "fix.ini"), &document()).trust,
            ArtifactTrust::Partial
        );
        let incomplete = format!("{body}\nAlso copy the native library into the Proton prefix.");
        assert_eq!(
            parse(&release(&incomplete, "fix.ini"), &document()).trust,
            ArtifactTrust::Partial
        );
    }

    #[test]
    fn multiple_artifact_procedures_are_not_installed_partially() {
        let body = format!(
            "{}\nInstall: Config/required.ini <- required.ini",
            instructions()
        );
        let c = parse(&release(&body, "fix.ini"), &document());
        assert_eq!(c.trust, ArtifactTrust::Partial);
        assert!(c.files.is_empty());
    }

    #[test]
    fn invalid_hardware_requirements_are_not_silently_dropped() {
        for label in [
            "GPU-Vendor: unknown-vendor",
            "GPU-Vendor: NVIDIA\nGPU-Vendor: AMD",
            "GPU-Device: ../2882",
            "GPU-Device: 10de:2882:0000",
            "GPU-Model:",
            "Driver:",
            "Proton:",
            "CPU-Arch: unrecognized-arch",
        ] {
            let body = format!("{}\n{label}", instructions());
            assert_eq!(
                parse(&release(&body, "fix.ini"), &document()).trust,
                ArtifactTrust::Partial,
                "{label}"
            );
        }
    }

    #[test]
    fn hardware_requirements_keep_models_driver_proton_and_architecture_explicit() {
        let body=format!("{}\nGPU-Vendor: NVIDIA\nGPU-Device: 10de:2882\nGPU-Model: GeForce RTX 4060\nDriver: 615.78.08\nProton: fixture-Proton\nCPU-Arch: x86_64\nGame-Build: fixture-build",instructions());
        let c = parse(&release(&body, "fix.ini"), &document());
        assert_eq!(c.requirements.hardware.gpu_vendor, Some(GpuReq::Nvidia));
        assert_eq!(c.requirements.hardware.gpu_device_ids, vec!["10de:2882"]);
        assert_eq!(c.requirements.hardware.gpu_models, vec!["GeForce RTX 4060"]);
        assert_eq!(c.requirements.hardware.driver_versions, vec!["615.78.08"]);
        assert_eq!(
            c.requirements.hardware.proton_versions,
            vec!["fixture-Proton"]
        );
        assert_eq!(c.requirements.hardware.cpu_arch.as_deref(), Some("x86_64"));
    }

    #[tokio::test]
    async fn discovers_release_metadata_but_never_fetches_payloads() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let state = EngineState::default();
        let fetcher = MockFetcher::default();
        let meta = "https://api.github.com/repos/ValveSoftware/Proton/releases/tags/fixture-1";
        fetcher.on(meta, 200, &release(instructions(), "fix.ini").to_string());
        let ctx = Ctx {
            fetcher: &fetcher,
            cache: &cache,
            state: &state,
            ttl_secs: 3600,
            force: false,
        };
        let (f, _) = discover(&ctx, &q(), &[document()]).await;
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].artifacts[0].trust, ArtifactTrust::OriginAndIntegrity);
        assert_eq!(fetcher.calls.lock().unwrap().len(), 1);
        assert_eq!(fetcher.count("https://github.com/"), 0);
    }

    #[tokio::test]
    async fn external_attachments_remain_visible_without_being_downloaded() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = Cache::new(tmp.path());
        let state = EngineState::default();
        let fetcher = MockFetcher::default();
        let ctx = Ctx {
            fetcher: &fetcher,
            cache: &cache,
            state: &state,
            ttl_secs: 3600,
            force: false,
        };
        let mut doc = document();
        doc.text = "Download https://files.example/fix.dll and copy it somewhere".into();
        let (f, _) = discover(&ctx, &q(), &[doc]).await;
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].artifacts[0].trust, ArtifactTrust::Untrusted);
        assert!(f[0].artifacts[0].files.is_empty());
        assert!(fetcher.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn url_parser_rejects_shell_paths_credentials_and_spoofed_hosts() {
        for bad in [
            "https://github.com.evil.example/a/b/releases/tag/x",
            "https://user@github.com/a/b/releases/tag/x",
            "http://github.com/a/b/releases/tag/x",
            "https://github.com/a/../releases/tag/x",
            "https://github.com/a/b/releases/tag/x?url=evil",
        ] {
            assert!(reference(bad).is_none(), "{bad}");
        }
    }
}
