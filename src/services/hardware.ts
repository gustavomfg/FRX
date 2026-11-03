import { call } from './native'
import type { SystemProfile } from '@/types/steam'
import type { HardwareReport, HardwareSymptom, LaptopIdentity } from '@/types/hardware'
import type { FixPlan, Operation } from '@/types/fixEngine'

/** Serviços de hardware (Rust: `commands/system.rs` e `commands/hardware.rs`). */

export const identifyLaptop = () => call<LaptopIdentity>('identify_laptop')
export const getSystemProfile = () => call<SystemProfile>('get_system_profile')

export interface HardwareSearchArgs {
  symptom: HardwareSymptom
  /** Ignora o cache válido e consulta a fonte de novo. */
  force: boolean
  maxAgeHours: number
}

export const searchHardwareFixes = (args: HardwareSearchArgs) => call<HardwareReport>('search_hardware_fixes', { args })

/** Prévia de uma instrução reconhecida. Só IDs saem da interface. */
export const prepareSystemFix = (args: { symptom: HardwareSymptom; suggestionId: string; actionId: string; maxAgeHours: number }) =>
  call<FixPlan>('prepare_system_fix', { args })
export const applySystemFix = (plan: Pick<FixPlan, 'planId' | 'fingerprint'>) =>
  call<Operation>('apply_system_fix', { planId: plan.planId, fingerprint: plan.fingerprint })

/** Só em build de debug: simula um laptop (`null` volta ao DMI real). */
export const setDevLaptop = (presetId: string | null) => call<void>('set_dev_laptop', { presetId })
