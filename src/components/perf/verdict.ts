import type { BenchmarkVerdict } from '@/types'
import type { Pose } from '@/mascot/sprites'

export const VERDICT: Record<BenchmarkVerdict, { label: string; color: string; soft: string; pose: Pose; text: string }> = {
  melhora: {
    label: 'Melhora',
    color: 'var(--frx-good)',
    soft: 'var(--frx-good-soft)',
    pose: 'celebrate',
    text: 'A diferença ficou bem acima da variação entre passadas.',
  },
  regressao: {
    label: 'Regressão',
    color: 'var(--frx-bad)',
    soft: 'var(--frx-bad-soft)',
    pose: 'worried',
    text: 'O desempenho caiu depois da mudança. Vale considerar reverter a correção.',
  },
  estavel: {
    label: 'Estável',
    color: 'var(--frx-text-2)',
    soft: 'var(--frx-raised)',
    pose: 'idle',
    text: 'Diferença dentro de ±3% — sem ganho perceptível, mas também sem perdas.',
  },
  inconclusivo: {
    label: 'Inconclusivo',
    color: 'var(--frx-caution)',
    soft: 'var(--frx-caution-soft)',
    pose: 'confused',
    text: 'A variação entre passadas é maior que a diferença medida. Repita o teste antes de concluir.',
  },
}
