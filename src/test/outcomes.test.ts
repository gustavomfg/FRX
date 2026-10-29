import { describe, expect, it } from 'vitest'
import { problemReports } from '@/lib/outcomes'
import type { FindingSource } from '@/types/fixes'

const source = (snippet: string | null, label = 'Comentário'): FindingSource => ({ provider: 'github', label, url: 'https://github.com/ValveSoftware/Proton/issues/9595#issuecomment-1', snippet, publishedAt: null } as FindingSource)

describe('relatos de problema por ajuste', () => {
  it('mostra só trechos que descrevem falha, com a citação e a fonte', () => {
    const found = problemReports([
      source('I also tried VKD3D_CONFIG=descriptor_heap, but that caused badly distorted cloth and a crash.'),
      source('This setting works great for me, no issues at all.'),
      source(null),
    ])
    expect(found).toHaveLength(1)
    expect(found[0].quote).toContain('distorted cloth')
    expect(found[0].source).toBe('Comentário')
    expect(found[0].url).toContain('issuecomment')
  })
})
