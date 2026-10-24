//! Capas e banners dos jogos.
//!
//! Ordem: cache do FRX → cache local da própria Steam (`appcache/librarycache`)
//! → CDN pública da Steam. Tudo é copiado para `<cache>/art/<appid>/` porque o
//! protocolo de assets do Tauri só libera esse diretório para a interface.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::net::{Fetcher, HttpRequest};
use crate::time::now_unix;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtKind {
    Cover,
    Hero,
    Header,
}

impl ArtKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "cover" => Some(Self::Cover),
            "hero" => Some(Self::Hero),
            "header" => Some(Self::Header),
            _ => None,
        }
    }

    fn file(self) -> &'static str {
        match self {
            Self::Cover => "library_600x900.jpg",
            Self::Hero => "library_hero.jpg",
            Self::Header => "header.jpg",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArtResult {
    pub path: Option<String>,
    /// "cache" | "steam-local" | "steam-cdn" | "missing" | "offline"
    pub source: &'static str,
}

const MAX_IMAGE: usize = 6 * 1024 * 1024;
/// Depois de um 404, não tentamos de novo por uma semana.
const MISSING_TTL: u64 = 7 * 86_400;

pub fn valid_app_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 10 && id.chars().all(|c| c.is_ascii_digit())
}

fn looks_like_image(b: &[u8]) -> bool {
    b.starts_with(&[0xFF, 0xD8, 0xFF])
        || b.starts_with(b"\x89PNG\r\n\x1a\n")
        || (b.len() > 12 && &b[8..12] == b"WEBP")
}

/// Procura a imagem no cache da Steam (layout novo por pasta e o antigo, plano).
fn steam_local(roots: &[PathBuf], app_id: &str, kind: ArtKind) -> Option<PathBuf> {
    roots.iter().find_map(|r| {
        let lc = r.join("appcache/librarycache");
        [
            lc.join(app_id).join(kind.file()),
            lc.join(format!("{app_id}_{}", kind.file())),
        ]
        .into_iter()
        .find(|p| p.is_file())
    })
}

pub async fn resolve<F: Fetcher>(
    fetcher: &F,
    art_root: &Path,
    steam_roots: &[PathBuf],
    app_id: &str,
    kind: ArtKind,
    offline_ok: bool,
) -> ArtResult {
    if !valid_app_id(app_id) {
        return ArtResult {
            path: None,
            source: "missing",
        };
    }
    let dir = art_root.join(app_id);
    let target = dir.join(kind.file());
    if target.is_file() {
        return ArtResult {
            path: Some(target.to_string_lossy().into_owned()),
            source: "cache",
        };
    }
    let missing_marker = dir.join(format!("{}.missing", kind.file()));
    if let Ok(t) = std::fs::read_to_string(&missing_marker) {
        if t.trim()
            .parse::<u64>()
            .is_ok_and(|at| now_unix().saturating_sub(at) < MISSING_TTL)
        {
            return ArtResult {
                path: None,
                source: "missing",
            };
        }
    }
    let _ = std::fs::create_dir_all(&dir);

    if let Some(local) = steam_local(steam_roots, app_id, kind) {
        if let Ok(bytes) = std::fs::read(&local) {
            if bytes.len() <= MAX_IMAGE
                && looks_like_image(&bytes)
                && std::fs::write(&target, &bytes).is_ok()
            {
                return ArtResult {
                    path: Some(target.to_string_lossy().into_owned()),
                    source: "steam-local",
                };
            }
        }
    }
    if offline_ok {
        return ArtResult {
            path: None,
            source: "offline",
        };
    }

    let urls = [
        format!(
            "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/{app_id}/{}",
            kind.file()
        ),
        format!(
            "https://cdn.cloudflare.steamstatic.com/steam/apps/{app_id}/{}",
            kind.file()
        ),
    ];
    let mut saw_404 = false;
    for url in urls {
        match fetcher
            .get(HttpRequest::new(url, "image/*", MAX_IMAGE))
            .await
        {
            Ok(r) if r.status == 200 && looks_like_image(&r.body) => {
                let tmp = target.with_extension("part");
                if std::fs::write(&tmp, &r.body).is_ok() && std::fs::rename(&tmp, &target).is_ok() {
                    return ArtResult {
                        path: Some(target.to_string_lossy().into_owned()),
                        source: "steam-cdn",
                    };
                }
            }
            Ok(r) if r.status == 404 || r.status == 403 => saw_404 = true,
            Ok(_) => {}
            Err(_) => {
                return ArtResult {
                    path: None,
                    source: "offline",
                }
            }
        }
    }
    if saw_404 {
        let _ = std::fs::write(&missing_marker, now_unix().to_string());
    }
    ArtResult {
        path: None,
        source: "missing",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::mock::MockFetcher;
    use crate::net::HttpResponse;

    const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3];

    #[tokio::test]
    async fn prefers_steam_local_cache_then_cdn_and_remembers_404() {
        let tmp = tempfile::tempdir().unwrap();
        let art = tmp.path().join("art");
        let steam = tmp.path().join("Steam");
        std::fs::create_dir_all(steam.join("appcache/librarycache/200260")).unwrap();
        std::fs::write(
            steam.join("appcache/librarycache/200260/library_600x900.jpg"),
            JPEG,
        )
        .unwrap();
        let f = MockFetcher::default();

        let r = resolve(
            &f,
            &art,
            std::slice::from_ref(&steam),
            "200260",
            ArtKind::Cover,
            false,
        )
        .await;
        assert_eq!(r.source, "steam-local");
        assert_eq!(
            f.count("https://"),
            0,
            "não deve usar a rede quando há cópia local"
        );
        let r = resolve(
            &f,
            &art,
            std::slice::from_ref(&steam),
            "200260",
            ArtKind::Cover,
            false,
        )
        .await;
        assert_eq!(r.source, "cache");

        f.on_response(
            "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/library_hero.jpg",
            HttpResponse { status: 200, body: JPEG.to_vec(), ..Default::default() },
        );
        let r = resolve(
            &f,
            &art,
            std::slice::from_ref(&steam),
            "620",
            ArtKind::Hero,
            false,
        )
        .await;
        assert_eq!(r.source, "steam-cdn");

        let r = resolve(
            &f,
            &art,
            std::slice::from_ref(&steam),
            "1",
            ArtKind::Cover,
            false,
        )
        .await;
        assert_eq!(r.source, "missing");
        let before = f.count("https://");
        let r = resolve(&f, &art, &[steam], "1", ArtKind::Cover, false).await;
        assert_eq!(r.source, "missing");
        assert_eq!(f.count("https://"), before, "404 recente fica em cache");
    }

    #[tokio::test]
    async fn rejects_non_images_and_bad_ids() {
        let tmp = tempfile::tempdir().unwrap();
        let f = MockFetcher::default();
        f.on(
            "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/7/header.jpg",
            200,
            "<html>",
        );
        let r = resolve(&f, tmp.path(), &[], "7", ArtKind::Header, false).await;
        assert!(r.path.is_none());
        let r = resolve(&f, tmp.path(), &[], "../7", ArtKind::Header, false).await;
        assert_eq!(r.source, "missing");
    }
}
