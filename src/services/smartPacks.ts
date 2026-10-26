import { call } from './native'
import type { SessionMeta } from '@/types/performance'
import type { PackComparison, Profile, SmartPackReport, SwitchPlan } from '@/types/smartPacks'

/**
 * Smart Packs (Rust: `src-tauri/src/commands/smart_packs.rs`). Aplicar e
 * desfazer passam pelos comandos do Fix Engine com o `applyRecipeId` do
 * pacote — só IDs saem daqui.
 */
export const getSmartPacks = (appId: string, profile: Profile) => call<SmartPackReport>('get_smart_packs', { appId, profile })
export const compareSmartPacks = (appId: string, a: string, b: string) => call<PackComparison>('compare_smart_packs', { appId, a, b })
export const planSmartPackSwitch = (appId: string, packId: string) => call<SwitchPlan>('plan_smart_pack_switch', { appId, packId })
export const declareBenchmarkPack = (sessionId: string, packId: string | null) => call<SessionMeta>('declare_benchmark_pack', { sessionId, packId })
