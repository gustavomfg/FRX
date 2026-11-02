//! Fixture de desenvolvimento para ver o fluxo de DLL na interface. Só existe
//! em build de debug: o módulo, o comando e o transporte somem do release.
//!
//! A descoberta passa pelo parser estrito real (`candidates_from_release`), e a
//! receita, a prévia, o consentimento, o backup e o rollback são os do Fix
//! Engine. Só o download é servido da memória, e só para as duas URLs do
//! fixture. Nada fica no cache em disco: reiniciar o app remove a injeção.
#![cfg(debug_assertions)]

use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use crate::fix_engine::artifacts::{HttpsTransport, Transport};
use crate::fix_engine::files::sha256_hex;
use crate::fixes::models::{DiscoveryScope, FindingSource, FixFinding, ProviderId};
use crate::fixes::providers::{
    artifacts::{candidates_from_release, finding, Document},
    GameQuery,
};

/// Repositório reconhecido, para a descoberta passar pela política real. A
/// release e a tag não existem: o transporte abaixo responde no lugar delas.
const REPO: &str = "Open-Wine-Components/umu-protonfixes";
const ASSET: &str = "frx-dev-dummy.dll";
/// Na raiz do jogo, porque o FRX não cria pastas: `bin/x64/` só existe em alguns jogos.
pub const DESTINATION: &str = "frx-dev-dummy.dll";
const RELEASE_BASE: u64 = 9_000_000_000_000;

static INJECTED: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// DLL fictícia: cabeçalho PE mínimo, como nos testes. Não tem código.
pub fn dummy_dll() -> Vec<u8> {
    let mut bytes = vec![0u8; 64 * 1024];
    bytes[..2].copy_from_slice(b"MZ");
    bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
    bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
    let note = b"FRX dev fixture: not a real library.";
    bytes[0x100..0x100 + note.len()].copy_from_slice(note);
    bytes
}

fn release_id(app_id: &str) -> Option<u64> {
    app_id.parse::<u64>().ok().map(|n| RELEASE_BASE + n)
}

fn tag(app_id: &str) -> String {
    format!("frx-dev-fixture-{app_id}")
}

fn metadata_url(app_id: &str) -> Option<String> {
    release_id(app_id).map(|id| format!("https://api.github.com/repos/{REPO}/releases/{id}"))
}

fn asset_url(app_id: &str) -> String {
    format!(
        "https://github.com/{REPO}/releases/download/{}/{ASSET}",
        tag(app_id)
    )
}

/// Metadados no formato da API do GitHub, com o manifesto estrito do FRX.
fn release_json(app_id: &str) -> Option<String> {
    let bytes = dummy_dll();
    let manifest = serde_json::json!({
        "schema": 1, "appId": app_id, "asset": ASSET,
        "platform": "any", "architecture": "any", "files": [],
        "dlls": [{
            "sourcePath": null,
            "destinationRelativePath": DESTINATION,
            "expectedSha256": sha256_hex(&bytes),
            "expectedOriginalSha256": null,
        }],
    });
    let tag = tag(app_id);
    Some(
        serde_json::json!({
            "id": release_id(app_id)?, "tag_name": tag,
            "html_url": format!("https://github.com/{REPO}/releases/tag/{tag}"),
            "draft": false, "prerelease": false,
            "author": {"login": "frx-dev-fixture"},
            "published_at": "2026-11-02T00:00:00Z",
            "body": format!("```frx-file-fix\n{manifest}\n```"),
            "assets": [{
                "id": 1, "name": ASSET, "browser_download_url": asset_url(app_id),
                "size": bytes.len(), "digest": format!("sha256:{}", sha256_hex(&bytes)),
                "content_type": "application/octet-stream", "state": "uploaded",
            }],
        })
        .to_string(),
    )
}

fn fixture_finding(app_id: &str) -> Option<FixFinding> {
    let json = release_json(app_id)?;
    let q = GameQuery {
        app_id: app_id.into(),
        retrieved_at: crate::time::now_iso(),
        ..Default::default()
    };
    let tag = tag(app_id);
    let document = Document {
        source: FindingSource {
            provider: ProviderId::Github,
            label: "FRX · fixture de desenvolvimento (não é uma fonte real)".into(),
            url: format!("https://github.com/{REPO}/releases/tag/{tag}"),
            reference_id: None,
            snippet: Some("DLL fictícia injetada pelo botão de desenvolvimento.".into()),
            published_at: None,
        },
        scope: DiscoveryScope::verified(app_id, "Fixture de desenvolvimento do FRX"),
        text: String::new(),
    };
    let candidate = candidates_from_release(&q, &document, REPO, &metadata_url(app_id)?, &json)
        .into_iter()
        .next()?;
    let mut f = finding(&q, candidate);
    f.title = "DLL de teste (dev) · frx-dev-dummy.dll".into();
    Some(f)
}

/// Descobertas injetadas para este jogo, somadas às do cache em `game_ctx`.
pub fn findings(app_id: &str) -> Vec<FixFinding> {
    let injected = INJECTED.lock().is_ok_and(|s| s.contains(app_id));
    if !injected {
        return vec![];
    }
    fixture_finding(app_id).into_iter().collect()
}

/// Serve da memória só as URLs do fixture de um jogo injetado; o resto vai
/// para o transporte HTTPS normal, com a mesma allowlist.
pub struct DevTransport {
    inner: HttpsTransport,
}

impl Default for DevTransport {
    fn default() -> Self {
        Self {
            inner: HttpsTransport,
        }
    }
}

impl DevTransport {
    fn fixture(url: &str) -> Option<Vec<u8>> {
        let injected = INJECTED.lock().ok()?.clone();
        injected.iter().find_map(|app| {
            if metadata_url(app).as_deref() == Some(url) {
                release_json(app).map(String::into_bytes)
            } else if asset_url(app) == url {
                Some(dummy_dll())
            } else {
                None
            }
        })
    }
}

impl Transport for DevTransport {
    fn get(
        &self,
        url: &str,
        limit: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<Vec<u8>, String> {
        let Some(bytes) = Self::fixture(url) else {
            return self.inner.get(url, limit, cancel, progress);
        };
        crate::fix_engine::artifacts::cancelled(cancel)?;
        if bytes.len() > limit {
            return Err("Download excede o tamanho permitido.".into());
        }
        progress(bytes.len() as u64, Some(bytes.len() as u64));
        Ok(bytes)
    }
}

/// Liga o fixture para um jogo. Não escreve nada: a instalação só acontece
/// pelo fluxo normal, com prévia, três confirmações e backup.
#[tauri::command]
pub fn inject_dev_fixture_dll(app_id: String) -> Result<String, String> {
    if !crate::art::valid_app_id(&app_id) || release_id(&app_id).is_none() {
        return Err("AppID inválido".into());
    }
    if fixture_finding(&app_id).is_none() {
        return Err("O fixture não passou pelo parser de descobertas.".into());
    }
    INJECTED
        .lock()
        .map_err(|_| "estado indisponível")?
        .insert(app_id);
    Ok(DESTINATION.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fixture_goes_through_the_strict_parser_as_a_consented_dll() {
        let f = fixture_finding("200260").unwrap();
        let [a] = f.artifacts.as_slice() else {
            panic!("{:?}", f.artifacts)
        };
        assert_eq!(
            a.trust,
            crate::fixes::models::ArtifactTrust::OriginAndIntegrity,
            "{:?}",
            a.reasons
        );
        assert_eq!(a.dlls[0].destination_relative_path, DESTINATION);
        assert_eq!(a.dlls[0].expected_sha256, sha256_hex(&dummy_dll()));
        assert!(a.dlls[0].requires_explicit_user_consent);
        assert!(crate::fix_engine::artifacts::validate_dll(DESTINATION, &dummy_dll()).is_ok());
    }

    #[test]
    fn only_injected_games_get_the_finding_or_the_served_urls() {
        let app = "999001";
        assert!(findings(app).is_empty());
        assert!(DevTransport::fixture(&asset_url(app)).is_none());
        inject_dev_fixture_dll(app.into()).unwrap();
        assert_eq!(findings(app).len(), 1);
        assert_eq!(DevTransport::fixture(&asset_url(app)).unwrap(), dummy_dll());
        assert!(DevTransport::fixture(&metadata_url(app).unwrap()).is_some());
        assert!(
            DevTransport::fixture("https://github.com/x/y/releases/download/t/a.dll").is_none()
        );
        assert!(inject_dev_fixture_dll("../1".into()).is_err());
    }
}
