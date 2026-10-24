import type { FixFinding } from '@/types/fixes'

/**
 * "Relevante" = útil para este sistema: não é incompatível com o hardware e
 * não é um ajuste isolado que mais gente diz não ter funcionado.
 */
export function isRelevant(f: FixFinding) {
  if (f.compatibility === 'incompatible') return false
  if (f.kind === 'tweak' && f.confidence === 'unverified' && f.evidence.negative >= f.evidence.positive) return false
  return true
}
