import { describe, expect, it } from 'vitest'
import { isRelevant } from './findings'
import { finding } from '@/test/fixtures'

describe('isRelevant', () => {
  it('oculta incompatíveis e ajustes contestados', () => {
    expect(isRelevant(finding())).toBe(true)
    expect(isRelevant(finding({ compatibility: 'incompatible' }))).toBe(false)
    expect(isRelevant(finding({ evidence: { positive: 1, negative: 1 } }))).toBe(false)
    expect(isRelevant(finding({ kind: 'protonfix', confidence: 'curated', evidence: { positive: 0, negative: 0 } }))).toBe(true)
  })
})
