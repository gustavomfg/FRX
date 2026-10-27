import type { EvidenceLevel } from '@/types/fixEngine'
import type { Goal, Outcome, PackState, Profile, Tier } from '@/types/smartPacks'

type Meta = { label: string; color: string; soft: string }

export const PROFILE: Record<Profile, { label: string; short: string; help: string }> = {
  performance: { label: 'Máximo desempenho', short: 'Desempenho', help: 'Prioriza o FPS médio, sem ignorar o 1% low.' },
  balanced: { label: 'Equilibrado', short: 'Equilibrado', help: 'Nenhuma piora em FPS médio, 1% low ou frametime; valoriza menos quedas.' },
  compatibility: { label: 'Máxima compatibilidade', short: 'Compatibilidade', help: 'Prioriza estabilidade e evidência documental; FPS menor é aceito e informado.' },
}

export const GOAL: Record<Goal, string> = {
  reference: 'Referência',
  compatibility: 'Compatibilidade',
  performance: 'Desempenho',
  balanced: 'Equilibrado',
}

export const STATE: Record<PackState, Meta> = {
  candidate: { label: 'Candidato', color: 'var(--frx-text-2)', soft: 'var(--frx-raised)' },
  ready: { label: 'Pronto para aplicar', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)' },
  conflict: { label: 'Conflito', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)' },
  testing: { label: 'Em andamento', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)' },
  applied: { label: 'Aplicado', color: 'var(--frx-info)', soft: 'var(--frx-info-soft)' },
  verified: { label: 'Ativo · conferido', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)' },
  failed: { label: 'Falhou', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)' },
  partially_recovered: { label: 'Requer recuperação', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)' },
  rolled_back: { label: 'Desfeito', color: 'var(--frx-text-2)', soft: 'var(--frx-raised)' },
  unsupported: { label: 'Incompatível', color: 'var(--frx-text-3)', soft: 'var(--frx-raised)' },
}

export const OUTCOME: Record<Outcome, Meta> = {
  improvement: { label: 'Melhora medida', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)' },
  regression: { label: 'Regressão', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)' },
  'no-difference': { label: 'Sem diferença', color: 'var(--frx-text-2)', soft: 'var(--frx-raised)' },
  mixed: { label: 'Sinais mistos', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)' },
  inconclusive: { label: 'Inconclusivo', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)' },
  'not-measured': { label: 'Sem benchmark', color: 'var(--frx-text-3)', soft: 'var(--frx-raised)' },
}

export const TIER: Record<Tier, string> = {
  measured: 'Ranking medido',
  candidate: 'Candidatos para testar',
  'performance-risk': 'Risco de perda de desempenho',
  'not-recommended': 'Não recomendados',
}

export const EVIDENCE_LABEL: Record<EvidenceLevel, string> = {
  documented: 'Documentada',
  corroborated: 'Vários relatos',
  single: 'Um relato',
  contested: 'Contestada',
  none: 'Nenhuma',
}
