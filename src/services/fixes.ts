import { call } from './native'
import type { FixReport, ProviderInfo } from '@/types/fixes'

/** Serviços de descoberta de correções (Rust: `src-tauri/src/commands/fixes.rs`). */

export interface SearchArgs {
  appId: string
  name: string
  /** Ignora o cache válido e consulta as fontes de novo. */
  force: boolean
  maxAgeHours: number
}

export const searchGameFixes = (args: SearchArgs) => call<FixReport>('search_game_fixes', { args })
export const getCachedFixReports = (maxAgeHours: number) => call<FixReport[]>('get_cached_fix_reports', { maxAgeHours })
export const getFixProviders = () => call<ProviderInfo[]>('get_fix_providers')
export const clearFixCache = () => call<void>('clear_fix_cache')
export const getCacheSize = () => call<number>('get_cache_size')
