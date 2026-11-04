import type { OpState, PlanStatus, Risk } from '@/types/fixEngine'

/** Rótulo, cor e fundo de um estado na interface. */
type Meta = { label: string; color: string; soft: string }

export const PLAN_STATUS: Record<PlanStatus, Meta> = {
  ready: { label: 'Aplicável', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)' },
  'requires-manual-step': { label: 'Aplicável · passo manual', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)' },
  conflict: { label: 'Conflito', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)' },
  incompatible: { label: 'Incompatível', color: 'var(--frx-text-3)', soft: 'var(--frx-raised)' },
  'already-applied': { label: 'Já aplicada', color: 'var(--frx-info)', soft: 'var(--frx-info-soft)' },
}

export const OP_STATE: Record<OpState, Meta> = {
  checking: { label: 'Conferindo', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)' },
  'backing-up': { label: 'Criando backup', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)' },
  applying: { label: 'Aplicando', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)' },
  verifying: { label: 'Verificando', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)' },
  applied: { label: 'Aplicada', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)' },
  failed: { label: 'Não aplicada', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)' },
  conflict: { label: 'Conflito', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)' },
  'requires-manual-step': { label: 'Aguardando passo manual', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)' },
  'rolling-back': { label: 'Desfazendo', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)' },
  'rolled-back': { label: 'Desfeita', color: 'var(--frx-text-2)', soft: 'var(--frx-raised)' },
  'recovery-required': { label: 'Requer recuperação', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)' },
}

export const RISK: Record<Risk, Meta> = {
  low: { label: 'Risco baixo', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)' },
  moderate: { label: 'Risco moderado', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)' },
  high: { label: 'Risco alto', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)' },
}
