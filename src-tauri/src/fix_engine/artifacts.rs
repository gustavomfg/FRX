//! Bounded artifact preparation. It returns verified content for the existing
//! transaction engine: configuration text, or a DLL the user explicitly
//! accepted. It never extracts into a game, runs code, or writes Steam.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::{
    confined::{MAX_BINARY, MAX_CONFIG, PREFIX_REFUSED},
    files::sha256_hex,
};
use crate::fixes::models::{ArtifactCandidate, ArtifactFormat, ArtifactTrust};

pub const MAX_DOWNLOAD: usize = 8 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 128;
pub const MAX_EXPANDED: u64 = 2 * 1024 * 1024;

/// Conteúdo verificado de um destino. `Binary` só sai de um `DllArtifact`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Payload {
    Text(String),
    Binary(Vec<u8>),
}

/// Pacotes com DLL podem ser maiores; os de configuração mantêm o limite antigo.
pub fn download_limit(a: &ArtifactCandidate) -> usize {
    if a.dlls.is_empty() {
        MAX_DOWNLOAD
    } else {
        MAX_BINARY
    }
}

fn expanded_limit(a: &ArtifactCandidate) -> u64 {
    if a.dlls.is_empty() {
        MAX_EXPANDED
    } else {
        2 * MAX_BINARY as u64
    }
}

fn entry_limit(a: &ArtifactCandidate) -> u64 {
    if a.dlls.is_empty() {
        MAX_CONFIG as u64
    } else {
        MAX_BINARY as u64
    }
}

fn prefix_path(rel: &str) -> bool {
    Path::new(rel).components().any(|c| {
        let c = c.as_os_str();
        c.eq_ignore_ascii_case("compatdata") || c.eq_ignore_ascii_case("pfx")
    })
}

pub fn sha_valid(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit())
}

pub fn trusted_repository(repo: &str) -> bool {
    crate::fixes::providers::artifacts::recognized_repository(repo)
}

pub fn allowed_url(s: &str) -> bool {
    reqwest::Url::parse(s).is_ok_and(|u| {
        u.scheme() == "https"
            && u.username().is_empty()
            && u.password().is_none()
            && u.port().is_none()
            && u.fragment().is_none()
            && u.host_str().is_some_and(|h| {
                [
                    "api.github.com",
                    "github.com",
                    "release-assets.githubusercontent.com",
                    "objects.githubusercontent.com",
                ]
                .contains(&h)
            })
    })
}

pub fn validate(a: &ArtifactCandidate, app: &str) -> Result<(), String> {
    if a.app_id != app || !a.scope.matches_game(app) {
        return Err("O artefato não tem vínculo verificado com este AppID.".into());
    }
    if a.trust != ArtifactTrust::OriginAndIntegrity || !trusted_repository(&a.repository) {
        return Err(
            "Procedência insuficiente: este projeto/artefato requer revisão antes de instalação."
                .into(),
        );
    }
    let asset = reqwest::Url::parse(&a.asset_url).map_err(|_| "URL de download inválida")?;
    let expected_prefix = format!("/{}/releases/download/", a.repository);
    if !allowed_url(&a.asset_url)
        || asset.host_str() != Some("github.com")
        || !asset.path().starts_with(&expected_prefix)
        || asset.query().is_some()
        || a.release_tag.is_empty()
        || a.publisher.as_deref().is_none_or(str::is_empty)
        || a.release_id == 0
        || a.asset_id == 0
        || a.metadata_url
            != format!(
                "https://api.github.com/repos/{}/releases/{}",
                a.repository, a.release_id
            )
        || a.release_url
            != format!(
                "https://github.com/{}/releases/tag/{}",
                a.repository, a.release_tag
            )
    {
        return Err("A URL, versão ou identidade oficial da release não confere.".into());
    }
    if a.size_bytes == 0
        || a.size_bytes > download_limit(a) as u64
        || !a.sha256.as_deref().is_some_and(sha_valid)
    {
        return Err("Tamanho ou SHA-256 publicado ausente/inválido.".into());
    }
    if a.platform
        .as_deref()
        .is_some_and(|s| !["windows", "linux", "any"].contains(&s))
    {
        return Err("Plataforma não suportada.".into());
    }
    if a.architecture
        .as_deref()
        .is_some_and(|s| !["any", "x86", "x86_64"].contains(&s))
    {
        return Err("Arquitetura não suportada.".into());
    }
    let selected = a.files.len() + a.dlls.len();
    if selected == 0 || selected > 8 {
        return Err("Destino documentado ausente ou seleção excessiva de arquivos.".into());
    }
    let mut targets = BTreeSet::new();
    for d in &a.dlls {
        super::security::check_dll_rel_path(&d.destination_relative_path)?;
        if prefix_path(&d.destination_relative_path) {
            return Err(PREFIX_REFUSED.into());
        }
        if !d.requires_explicit_user_consent {
            return Err(
                "Uma DLL de terceiros sempre exige a confirmação explícita do usuário.".into(),
            );
        }
        if !sha_valid(&d.expected_sha256)
            || d.expected_original_sha256
                .as_deref()
                .is_some_and(|s| !sha_valid(s))
        {
            return Err("A DLL precisa de SHA-256 verificável.".into());
        }
        if !targets.insert(d.destination_relative_path.to_ascii_lowercase()) {
            return Err("Destinos duplicados no pacote.".into());
        }
        match a.format {
            ArtifactFormat::File
                if selected == 1
                    && d.source_path.is_none()
                    && a.sha256.as_deref() == Some(d.expected_sha256.as_str()) => {}
            ArtifactFormat::Zip | ArtifactFormat::Tar
                if d.source_path.as_deref().is_some_and(safe_member) => {}
            _ => return Err("Formato/seleção de arquivos não suportado; requer revisão.".into()),
        }
    }
    for f in &a.files {
        super::security::check_rel_path(&f.target_path)?;
        if Path::new(&f.target_path).extension().is_none_or(|e| {
            !["ini", "cfg", "conf", "json", "txt"]
                .iter()
                .any(|x| e.eq_ignore_ascii_case(x))
        }) {
            return Err("Este formato ainda requer revisão, incluindo XML e binários.".into());
        }
        if !targets.insert(f.target_path.to_ascii_lowercase()) {
            return Err("Destinos duplicados no pacote.".into());
        }
        if f.sha256.as_deref().is_none_or(|s| !sha_valid(s)) {
            return Err("Cada arquivo precisa de SHA-256 verificável.".into());
        }
        if f.expected_original_sha256
            .as_deref()
            .is_some_and(|s| !sha_valid(s))
        {
            return Err("Hash do original inválido.".into());
        }
        match a.format {
            ArtifactFormat::File
                if a.files.len() == 1 && f.source_path.is_none() && f.sha256 == a.sha256 => {}
            ArtifactFormat::Zip | ArtifactFormat::Tar
                if f.source_path.as_deref().is_some_and(safe_member) => {}
            _ => return Err("Formato/seleção de arquivos não suportado; requer revisão.".into()),
        }
    }
    Ok(())
}

fn safe_member(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 200
        && !name.contains(['\\', ':', '\0'])
        && !name.chars().any(char::is_control)
        && Path::new(name)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

/// Confere que os bytes são uma DLL PE. Não avalia o conteúdo: sem Authenticode,
/// a única garantia é o SHA-256 publicado, conferido antes por quem chama.
pub fn validate_dll(target: &str, bytes: &[u8]) -> Result<Vec<u8>, String> {
    super::security::check_dll_rel_path(target)?;
    if prefix_path(target) {
        return Err(PREFIX_REFUSED.into());
    }
    let pe = bytes.len() >= 0x40 && bytes.starts_with(b"MZ") && {
        let at = u32::from_le_bytes([bytes[0x3c], bytes[0x3d], bytes[0x3e], bytes[0x3f]]) as usize;
        bytes.get(at..at.saturating_add(4)) == Some(b"PE\0\0")
    };
    if bytes.len() > MAX_BINARY || !pe {
        return Err("O arquivo não é uma DLL do Windows dentro do limite de tamanho.".into());
    }
    Ok(bytes.to_vec())
}

/// Text is configuration data only; no executable archives, scripts, or loading
/// directives. Trusted origin and a checksum are still not a benignness proof.
pub fn validate_text(target: &str, bytes: &[u8]) -> Result<String, String> {
    super::security::check_rel_path(target)?;
    if bytes.len() > MAX_CONFIG || bytes.starts_with(b"MZ") || bytes.starts_with(b"\x7fELF") {
        return Err(
            "Arquivo binário/executável ou configuração excessivamente grande; requer revisão."
                .into(),
        );
    }
    let text = std::str::from_utf8(bytes).map_err(|_| {
        "A instalação automática aceita apenas configuração UTF-8; binários exigem revisão."
    })?;
    if text.contains('\0')
        || text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err("Conteúdo de configuração inválido.".into());
    }
    let lower = text.to_ascii_lowercase();
    for marker in [
        "#!/",
        "<!doctype",
        "<!entity",
        "<script",
        "javascript:",
        "data:",
        "$(`",
        "$(",
        "`",
        ";exec",
        "execute",
        "exec=",
        "command=",
        "command =",
        "exec ",
        "include=",
        "include =",
        "preload",
        "loadlibrary",
        "dlloverride",
        "plugin=",
        "script=",
        "shell=",
        "process.start",
        "os.system",
        "http://",
        "https://",
    ] {
        if lower.contains(marker) {
            return Err("A configuração contém instrução de execução/carregamento não suportada; requer revisão.".into());
        }
    }
    let forbidden_key = |key: &str| {
        let key = key.to_ascii_lowercase();
        [
            "exec", "command", "script", "shell", "plugin", "library", "include", "preload", "dll",
            "bind", "launch",
        ]
        .iter()
        .any(|part| key.contains(part))
    };
    if Path::new(target)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("json"))
    {
        fn inspect(v: &serde_json::Value, forbidden: &dyn Fn(&str) -> bool) -> bool {
            match v {
                serde_json::Value::Object(xs) => xs
                    .iter()
                    .all(|(k, v)| !forbidden(k) && inspect(v, forbidden)),
                serde_json::Value::Array(xs) => xs.iter().all(|v| inspect(v, forbidden)),
                serde_json::Value::String(s) => !s.contains(['$', '`', ';', '|', '&', '<', '>']),
                _ => true,
            }
        }
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|_| "JSON de configuração malformado")?;
        if !value.is_object() || !inspect(&value, &forbidden_key) {
            return Err("JSON contém instruções não suportadas; requer revisão.".into());
        }
    } else {
        for line in text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.starts_with(['#', ';']) && !s.starts_with("//"))
        {
            if line.starts_with('[')
                && line.ends_with(']')
                && !line.contains(['$', '`', ';', '|', '&', '<', '>'])
            {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or("O arquivo não é uma configuração de dados chave=valor suportada.")?;
            if key.trim().is_empty()
                || forbidden_key(key.trim())
                || value.contains(['$', '`', ';', '|', '&', '<', '>'])
                || !key
                    .trim()
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
            {
                return Err(
                    "Diretiva de configuração potencialmente executável; requer revisão.".into(),
                );
            }
        }
    }
    Ok(text.to_owned())
}

pub trait Transport: Send + Sync {
    fn get(
        &self,
        url: &str,
        limit: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<Vec<u8>, String>;
}

pub struct HttpsTransport;
impl Transport for HttpsTransport {
    fn get(
        &self,
        url: &str,
        limit: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<Vec<u8>, String> {
        if !allowed_url(url) {
            return Err("URL de download não permitida.".into());
        }
        let client = reqwest::blocking::Client::builder()
            .https_only(true)
            .user_agent(crate::net::USER_AGENT)
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(6))
            .redirect(reqwest::redirect::Policy::custom(|a| {
                if a.previous().len() >= 4 || !allowed_url(a.url().as_str()) {
                    a.error("Redirecionamento não permitido")
                } else {
                    a.follow()
                }
            }))
            .build()
            .map_err(|e| e.to_string())?;
        let mut response = client
            .get(url)
            .send()
            .map_err(|e| format!("Download falhou: {e}"))?
            .error_for_status()
            .map_err(|e| format!("Download recusado: {e}"))?;
        let total = response.content_length();
        if total.is_some_and(|n| n > limit as u64) {
            return Err("Download excede o tamanho permitido.".into());
        }
        let mut bytes = Vec::new();
        let mut buffer = [0; 16384];
        loop {
            cancelled(cancel)?;
            let n = response
                .read(&mut buffer)
                .map_err(|e| format!("Download interrompido: {e}"))?;
            if n == 0 {
                break;
            }
            if bytes.len() + n > limit {
                return Err("Download excede o tamanho permitido.".into());
            }
            bytes.extend_from_slice(&buffer[..n]);
            progress(bytes.len() as u64, total);
        }
        cancelled(cancel)?;
        Ok(bytes)
    }
}

pub fn cancelled(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::SeqCst) {
        Err("CANCELADO: download cancelado antes de alterar o jogo.".into())
    } else {
        Ok(())
    }
}

fn verify_publication(a: &ArtifactCandidate, bytes: &[u8]) -> Result<(), String> {
    let v: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "Metadados da release inválidos")?;
    if v["id"].as_u64() != Some(a.release_id)
        || v["tag_name"].as_str() != Some(&a.release_tag)
        || v["html_url"].as_str() != Some(&a.release_url)
        || v["draft"] == true
        || v["author"]["login"].as_str() != a.publisher.as_deref()
    {
        return Err("A identidade da release mudou; atualize a descoberta.".into());
    }
    let asset = v["assets"]
        .as_array()
        .and_then(|xs| xs.iter().find(|v| v["id"].as_u64() == Some(a.asset_id)))
        .ok_or("O asset foi removido da release.")?;
    if asset["name"].as_str() != Some(&a.asset_name)
        || asset["browser_download_url"].as_str() != Some(&a.asset_url)
        || asset["size"].as_u64() != Some(a.size_bytes)
        || asset["digest"].as_str() != a.sha256.as_ref().map(|s| format!("sha256:{s}")).as_deref()
        || asset["state"].as_str() != Some("uploaded")
    {
        return Err("Os metadados ou SHA-256 publicados do artefato mudaram.".into());
    }
    Ok(())
}

/// Destino, hash esperado e se é binário, para um membro selecionado.
fn selection<'a>(a: &'a ArtifactCandidate, member: &str) -> Option<(&'a str, &'a str, bool)> {
    a.files
        .iter()
        .find(|f| f.source_path.as_deref() == Some(member))
        .map(|f| {
            (
                f.target_path.as_str(),
                f.sha256.as_deref().unwrap_or(""),
                false,
            )
        })
        .or_else(|| {
            a.dlls
                .iter()
                .find(|d| d.source_path.as_deref() == Some(member))
                .map(|d| {
                    (
                        d.destination_relative_path.as_str(),
                        d.expected_sha256.as_str(),
                        true,
                    )
                })
        })
}

fn verified(target: &str, sha: &str, binary: bool, content: &[u8]) -> Result<Payload, String> {
    if sha256_hex(content) != sha {
        return Err("Arquivo extraído não confere com o SHA-256 publicado.".into());
    }
    Ok(if binary {
        Payload::Binary(validate_dll(target, content)?)
    } else {
        Payload::Text(validate_text(target, content)?)
    })
}

pub fn contents(
    a: &ArtifactCandidate,
    bytes: &[u8],
    cancel: &AtomicBool,
) -> Result<BTreeMap<String, Payload>, String> {
    if bytes.len() as u64 != a.size_bytes || a.sha256.as_deref() != Some(sha256_hex(bytes).as_str())
    {
        return Err("SHA-256 ou tamanho do download não confere; nada será instalado.".into());
    }
    let mut result = BTreeMap::new();
    match a.format {
        ArtifactFormat::File => {
            if let Some(d) = a.dlls.first() {
                let path = &d.destination_relative_path;
                result.insert(
                    path.clone(),
                    verified(path, &d.expected_sha256, true, bytes)?,
                );
            } else {
                let f = &a.files[0];
                result.insert(
                    f.target_path.clone(),
                    Payload::Text(validate_text(&f.target_path, bytes)?),
                );
            }
        }
        ArtifactFormat::Zip => {
            if !bytes.starts_with(b"PK\x03\x04") && !bytes.starts_with(b"PK\x05\x06") {
                return Err("Assinatura de formato ZIP inválida.".into());
            }
            let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
                .map_err(|e| format!("ZIP inválido: {e}"))?;
            if archive.len() > MAX_ENTRIES {
                return Err("ZIP tem entradas demais.".into());
            }
            let mut seen = BTreeSet::new();
            let mut expanded = 0u64;
            for n in 0..archive.len() {
                cancelled(cancel)?;
                let mut entry = archive
                    .by_index(n)
                    .map_err(|e| format!("Entrada ZIP inválida: {e}"))?;
                let name = entry
                    .name()
                    .map_err(|e| format!("Nome ZIP inválido: {e}"))?
                    .into_owned();
                if !safe_member(name.trim_end_matches('/'))
                    || !seen.insert(name.to_ascii_lowercase())
                    || entry.is_symlink()
                    || entry.unix_mode().is_some_and(|m| {
                        m & 0o170000 != 0 && ![0o100000, 0o040000].contains(&(m & 0o170000))
                    })
                {
                    return Err(
                        "ZIP contém caminho inválido, link, arquivo especial ou nome duplicado."
                            .into(),
                    );
                }
                expanded = expanded
                    .checked_add(entry.size())
                    .ok_or("ZIP excessivamente grande")?;
                if expanded > expanded_limit(a)
                    || entry.size() > entry_limit(a)
                    || entry.size() > entry.compressed_size().max(1).saturating_mul(100)
                {
                    return Err(
                        "ZIP excede limites de expansão/compressão; possível ZIP bomb.".into(),
                    );
                }
                let Some((target, sha, binary)) = selection(a, &name) else {
                    continue;
                };
                if entry.is_dir() {
                    return Err("A seleção aponta para um diretório.".into());
                }
                let mut content = Vec::new();
                (&mut entry)
                    .take(entry_limit(a) + 1)
                    .read_to_end(&mut content)
                    .map_err(|e| format!("Extração/CRC falhou: {e}"))?;
                if content.len() as u64 > entry_limit(a) {
                    return Err("Arquivo extraído maior que o limite.".into());
                }
                result.insert(target.to_owned(), verified(target, sha, binary, &content)?);
            }
            if result.len() != a.files.len() + a.dlls.len() {
                return Err("O ZIP não contém todos os arquivos selecionados.".into());
            }
        }
        ArtifactFormat::Tar => {
            let mut expanded = 0u64;
            for entry in tar_entries(bytes, cancel)? {
                expanded = expanded
                    .checked_add(entry.data.len() as u64)
                    .ok_or("TAR excessivamente grande")?;
                if expanded > expanded_limit(a) || entry.data.len() as u64 > entry_limit(a) {
                    return Err("TAR excede os limites de tamanho.".into());
                }
                let Some((target, sha, binary)) = selection(a, &entry.name) else {
                    continue;
                };
                if entry.dir {
                    return Err("A seleção aponta para um diretório.".into());
                }
                result.insert(
                    target.to_owned(),
                    verified(target, sha, binary, entry.data)?,
                );
            }
            if result.len() != a.files.len() + a.dlls.len() {
                return Err("O TAR não contém todos os arquivos selecionados.".into());
            }
        }
        ArtifactFormat::Unknown => return Err("Formato não suportado.".into()),
    }
    Ok(result)
}

struct TarEntry<'a> {
    name: String,
    dir: bool,
    data: &'a [u8],
}

fn tar_number(field: &[u8]) -> Result<u64, String> {
    // Base-256 (bit alto ligado) e campos vazios não são aceitos.
    let text = std::str::from_utf8(field)
        .map_err(|_| "Cabeçalho TAR inválido")?
        .trim_matches(|c| c == '\0' || c == ' ');
    if text.is_empty() || !text.bytes().all(|b| (b'0'..=b'7').contains(&b)) {
        return Err("Cabeçalho TAR inválido.".into());
    }
    u64::from_str_radix(text, 8).map_err(|_| "Cabeçalho TAR inválido.".into())
}

fn tar_text(field: &[u8]) -> Result<String, String> {
    let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    String::from_utf8(field[..end].to_vec()).map_err(|_| "Nome TAR não é UTF-8.".into())
}

/// Nome do membro de uma extensão PAX. Só `path` muda algo; chaves que mudam
/// tamanho, link ou esparsidade recusam o pacote.
fn pax_path(data: &[u8]) -> Result<Option<String>, String> {
    let mut rest = data;
    let mut path = None;
    while !rest.is_empty() {
        let space = rest
            .iter()
            .position(|&b| b == b' ')
            .ok_or("Extensão PAX inválida.")?;
        let len: usize = std::str::from_utf8(&rest[..space])
            .ok()
            .and_then(|n| n.parse().ok())
            .filter(|&n| n > space + 1 && n <= rest.len())
            .ok_or("Extensão PAX inválida.")?;
        let record = &rest[space + 1..len];
        let record = record.strip_suffix(b"\n").ok_or("Extensão PAX inválida.")?;
        let eq = record
            .iter()
            .position(|&b| b == b'=')
            .ok_or("Extensão PAX inválida.")?;
        let key = std::str::from_utf8(&record[..eq]).map_err(|_| "Extensão PAX inválida.")?;
        let value = &record[eq + 1..];
        match key {
            "path" => {
                path = Some(String::from_utf8(value.to_vec()).map_err(|_| "Nome TAR não é UTF-8.")?)
            }
            "mtime" | "atime" | "ctime" | "uid" | "gid" | "uname" | "gname" | "comment" => {}
            k if k.starts_with("LIBARCHIVE.") || k.starts_with("SCHILY.") => {}
            _ => return Err("O TAR usa uma extensão PAX não suportada.".into()),
        }
        rest = &rest[len..];
    }
    Ok(path)
}

/// Lê um TAR sem compressão sem tocar no disco. Só arquivo comum e diretório:
/// link, hard link, dispositivo, FIFO e PAX global recusam o pacote inteiro.
fn tar_entries<'a>(bytes: &'a [u8], cancel: &AtomicBool) -> Result<Vec<TarEntry<'a>>, String> {
    const BLOCK: usize = 512;
    let mut entries: Vec<TarEntry<'a>> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut long_name: Option<String> = None;
    let mut at = 0;
    let mut ended = false;
    let mut headers = 0usize;
    while at + BLOCK <= bytes.len() {
        cancelled(cancel)?;
        let header = &bytes[at..at + BLOCK];
        if header.iter().all(|&b| b == 0) {
            ended = true;
            break;
        }
        headers += 1;
        if headers > 2 * MAX_ENTRIES + 2 {
            return Err("TAR tem entradas demais.".into());
        }
        let checksum = tar_number(&header[148..156])?;
        let sum: u64 = header
            .iter()
            .enumerate()
            .map(|(i, &b)| {
                if (148..156).contains(&i) {
                    32
                } else {
                    b as u64
                }
            })
            .sum();
        if checksum != sum {
            return Err("Checksum de cabeçalho TAR inválido.".into());
        }
        let size = usize::try_from(tar_number(&header[124..136])?)
            .map_err(|_| "TAR excessivamente grande")?;
        let start = at + BLOCK;
        let end = start.checked_add(size).ok_or("TAR excessivamente grande")?;
        if end > bytes.len() {
            return Err("TAR truncado.".into());
        }
        let data = &bytes[start..end];
        at = start + size.div_ceil(BLOCK) * BLOCK;
        let kind = header[156];
        match kind {
            b'L' | b'x' => {
                if long_name.is_some() || size > 4096 {
                    return Err("Extensões de nome TAR inválidas.".into());
                }
                long_name = Some(if kind == b'L' {
                    tar_text(data)?
                } else {
                    pax_path(data)?.unwrap_or_default()
                });
                continue;
            }
            b'0' | 0 | b'5' => {}
            _ => {
                return Err(
                    "TAR contém link, hard link, dispositivo, FIFO ou entrada não suportada."
                        .into(),
                )
            }
        }
        let mut name = match long_name.take().filter(|n| !n.is_empty()) {
            Some(n) => n,
            None => {
                let base = tar_text(&header[..100])?;
                let prefix = if &header[257..262] == b"ustar" {
                    tar_text(&header[345..500])?
                } else {
                    String::new()
                };
                if prefix.is_empty() {
                    base
                } else {
                    format!("{prefix}/{base}")
                }
            }
        };
        while let Some(rest) = name.strip_prefix("./") {
            name = rest.to_owned();
        }
        let dir = kind == b'5';
        if dir {
            name = name.trim_end_matches('/').to_owned();
            if name.is_empty() || name == "." {
                continue;
            }
        }
        if !safe_member(&name) || !seen.insert(name.to_ascii_lowercase()) {
            return Err("TAR contém caminho inválido ou nome duplicado.".into());
        }
        if entries.len() >= MAX_ENTRIES {
            return Err("TAR tem entradas demais.".into());
        }
        entries.push(TarEntry { name, dir, data });
    }
    if !ended || long_name.is_some() {
        return Err("TAR truncado ou sem fim de arquivo.".into());
    }
    Ok(entries)
}

pub fn prepare(
    a: &ArtifactCandidate,
    staging: &Path,
    transport: &dyn Transport,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, Option<u64>, bool),
) -> Result<BTreeMap<String, Payload>, String> {
    validate(a, &a.app_id)?;
    cancelled(cancel)?;
    let metadata = transport.get(&a.metadata_url, 1024 * 1024, cancel, &mut |_, _| {})?;
    verify_publication(a, &metadata)?;
    let q = crate::fixes::providers::GameQuery {
        app_id: a.app_id.clone(),
        ..Default::default()
    };
    let document = crate::fixes::providers::artifacts::Document {
        source: a.recommendation_source.clone(),
        scope: a.scope.clone(),
        text: String::new(),
    };
    let regenerated = crate::fixes::providers::artifacts::candidates_from_release(
        &q,
        &document,
        &a.repository,
        &a.metadata_url,
        std::str::from_utf8(&metadata).map_err(|_| "Metadados UTF-8 inválidos")?,
    );
    if !regenerated.iter().any(|current| current == a) {
        return Err("As instruções, destino ou requisitos publicados mudaram. Atualize a descoberta antes de instalar.".into());
    }
    let bytes = transport.get(&a.asset_url, download_limit(a), cancel, &mut |n, total| {
        progress(n, total, false)
    })?;
    super::files::private_dir(staging)?;
    let path = staging.join("artifact.download");
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    cancelled(cancel)?;
    progress(bytes.len() as u64, Some(bytes.len() as u64), true);
    contents(a, &bytes, cancel)
}
