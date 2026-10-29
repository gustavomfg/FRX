import type { FindingSource } from '@/types/fixes'

/** Trecho de texto que descreve falha ou efeito colateral (em inglês ou português). */
const PROBLEM = /\b(crash\w*|freez\w*|froze|hang\w*|stutter\w*|artifact\w*|flicker\w*|distort\w*|black screen|regress\w*|worse|broke\w*|did(?:n't| not) (?:fix|help|work)|does(?:n't| not) fix|xid \d+|não resolv\w*|piorou|trava\w*|congel\w*|quebr\w*|artefat\w*|distorc\w*)\b/i

export interface ProblemReport {
  quote: string
  source: string
  url: string
}

/** Relatos das fontes que descrevem problema com o ajuste. Só exibe; não interpreta nem executa nada. */
export function problemReports(sources: FindingSource[], limit = 3): ProblemReport[] {
  const out: ProblemReport[] = []
  for (const s of sources) {
    const text = (s.snippet ?? '').replace(/\s+/g, ' ').trim()
    if (!text || !PROBLEM.test(text)) continue
    out.push({ quote: text.length > 220 ? `${text.slice(0, 217)}…` : text, source: s.label, url: s.url })
    if (out.length === limit) break
  }
  return out
}
