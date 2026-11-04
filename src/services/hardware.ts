import { call } from './native'
import type { FixPlan, Operation } from '@/types/fixEngine'
import type { HardwareReport, HardwareSymptom, LaptopIdentity, ProviderInfo, SystemProfile } from '@/types/hardware'

/** Serviços de hardware (Rust: `commands/system.rs`, `commands/hardware.rs`). */

export const identifyLaptop = () => call<LaptopIdentity>('identify_laptop')
export const getSystemProfile = () => call<SystemProfile>('get_system_profile')

export interface HardwareSearchArgs {
  symptom: HardwareSymptom
  /** Ignora o cache válido e consulta a fonte de novo. */
  force: boolean
  maxAgeHours: number
}

export const searchHardwareFixes = (args: HardwareSearchArgs) => call<HardwareReport>('search_hardware_fixes', { args })

export interface SystemFixArgs {
  symptom: HardwareSymptom
  suggestionId: string
  actionId: string
  maxAgeHours: number
}

/** Prévia de uma instrução reconhecida. Só IDs saem da interface. */
export const prepareSystemFix = (args: SystemFixArgs) => call<FixPlan>('prepare_system_fix', { args })

export const applySystemFix = (plan: Pick<FixPlan, 'planId' | 'fingerprint'>) =>
  call<Operation>('apply_system_fix', { planId: plan.planId, fingerprint: plan.fingerprint })

/** Fontes consultadas, para a tela de ajustes. */
export const getFixProviders = () => call<ProviderInfo[]>('get_fix_providers')
/** Tamanho, em bytes, das respostas guardadas em cache. */
export const getCacheSize = () => call<number>('get_cache_size')
export const clearFixCache = () => call<void>('clear_fix_cache')

/** Só em build de debug: simula um laptop (`null` volta ao DMI real). */
export const setDevLaptop = (presetId: string | null) => call<void>('set_dev_laptop', { presetId })
