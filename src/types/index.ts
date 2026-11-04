export type { FixPlan, Operation, RollbackPlan } from './fixEngine'
export type { HardwareReport, LaptopIdentity, SystemProfile } from './hardware'

export type MotionLevel = 'full' | 'calm' | 'off'

/** Preferências do app, guardadas no localStorage. */
export interface Settings {
  theme: 'frx-dark'
  accent: 'manteiga' | 'menta' | 'coral' | 'ceu'
  mascots: boolean
  motion: MotionLevel
  reduceMotion: 'sistema' | 'sempre'
  compactSidebar: boolean
  /** Validade do cache de respostas da Arch Wiki, em horas. */
  fixCacheHours: 6 | 24 | 72 | 168
}
