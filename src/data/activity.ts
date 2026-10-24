import type { Activity } from '@/types'
import { ago } from './games'

/** Atividades DEMONSTRATIVAS exibidas na Home. */
export const seedActivity: Activity[] = ([
  { id: 'a1', at: ago(0.4), kind: 'scan', text: 'Biblioteca verificada — 12 jogos encontrados' },
  { id: 'a2', at: ago(3), kind: 'analise', text: 'Ferrous Drift analisado: 2 problemas conhecidos', gameId: 'ferrous-drift' },
  { id: 'a3', at: ago(28), kind: 'benchmark', text: 'Benchmark em Ferrous Drift: +124% FPS médio (demo)', gameId: 'ferrous-drift' },
  { id: 'a4', at: ago(51), kind: 'benchmark', text: 'Ashen Choir regrediu −13% com NVAPI (demo)', gameId: 'ashen-choir' },
  { id: 'a5', at: ago(74), kind: 'benchmark', text: 'Glasswing: resultado inconclusivo, variação alta', gameId: 'glasswing' },
] as Activity[]).map((a) => ({ ...a, demo: true }))
