import { convertFileSrc } from '@tauri-apps/api/core'
import { call } from './native'
import type { ArtResult, ScanResult, SystemProfile } from '@/types/steam'

/** Serviços de jogos (Rust: `src-tauri/src/commands/games.rs`). */

export const scanInstalledGames = () => call<ScanResult>('scan_installed_games')
export const getInstalledGames = () => call<ScanResult | null>('get_installed_games')
export const getSystemProfile = () => call<SystemProfile>('get_system_profile')

export type ArtKind = 'cover' | 'hero' | 'header'

/** Devolve uma URL `asset:` servida pelo Tauri, ou `null` se não houver imagem. */
export async function getGameArtUrl(appId: string, kind: ArtKind): Promise<{ url: string | null; source: ArtResult['source'] }> {
  const r = await call<ArtResult>('get_game_art', { appId, kind })
  return { url: r.path ? convertFileSrc(r.path) : null, source: r.source }
}
