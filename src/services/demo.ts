import type { BenchmarkRun, FixPack, Game, MetricSample } from '@/types'
import { generateSamples, summarize, seedBenchmarks } from '@/data/benchmarks'
import { hashString } from '@/lib/random'
import { uid } from '@/lib/format'

/**
 * Serviços DEMONSTRATIVOS. Simulam latência e progresso para que a interface
 * possa ser exercitada. Nenhum arquivo é lido ou escrito, nenhum comando é
 * executado e nada é baixado. Em uma fase futura, estes métodos serão
 * substituídos por chamadas ao backend Tauri (ver services/platform.ts).
 */

export class DemoError extends Error {
  constructor(message: string, public step: number) {
    super(message)
  }
}

const wait = (ms: number, signal?: AbortSignal) =>
  new Promise<void>((resolve, reject) => {
    const t = setTimeout(resolve, ms)
    signal?.addEventListener('abort', () => {
      clearTimeout(t)
      reject(new DOMException('cancelado', 'AbortError'))
    })
  })

export interface StepProgress {
  step: number
  label: string
}

async function runSteps(labels: string[], totalMs: number, onStep: (p: StepProgress) => void, signal?: AbortSignal, failAt?: { step: number; message: string }) {
  const per = totalMs / labels.length
  for (let i = 0; i < labels.length; i++) {
    onStep({ step: i, label: labels[i] })
    await wait(per * (0.75 + Math.random() * 0.5), signal)
    if (failAt && failAt.step === i) throw new DemoError(failAt.message, i)
  }
  onStep({ step: labels.length, label: 'Concluído' })
}

export const ANALYZE_STEPS = [
  'Lendo configuração do Proton',
  'Consultando ProtonDB (cache demo)',
  'Procurando relatos no GitHub (demo)',
  'Cruzando com UMU-Protonfixes (demo)',
  'Montando recomendações',
]

export function analyzeGame(_game: Game, onStep: (p: StepProgress) => void, signal?: AbortSignal) {
  return runSteps(ANALYZE_STEPS, 2600, onStep, signal)
}

export const APPLY_STEPS = ['Criando backup do prefixo', 'Gravando alterações (simulado)', 'Verificando integridade']
export const REVERT_STEPS = ['Localizando backup', 'Restaurando arquivos (simulado)', 'Verificando integridade']

export function applyFix(fix: FixPack, attempt: number, onStep: (p: StepProgress) => void, signal?: AbortSignal) {
  const fail = fix.demoFailsOnce && attempt === 1
  return runSteps(APPLY_STEPS, fix.estimatedSeconds * 650, onStep, signal, fail ? { step: 2, message: 'O Steam estava com o prefixo em uso. Nenhuma alteração foi mantida — o backup foi restaurado. (falha simulada)' } : undefined)
}

export function revertFix(fix: FixPack, onStep: (p: StepProgress) => void, signal?: AbortSignal) {
  return runSteps(REVERT_STEPS, Math.max(1800, fix.estimatedSeconds * 450), onStep, signal)
}

const PERF_FIXES = new Set(['fp-ferrous-shader', 'fp-ferrous-ge', 'fp-glass-audio'])
const REGRESSING_FIXES = new Set(['fp-ashen-nvapi'])

/**
 * Gera um benchmark fictício. A tendência depende das correções aplicadas,
 * mas sempre com ruído — o resultado pode sair estável ou inconclusivo.
 */
export async function runBenchmark(
  game: Game,
  appliedFixIds: string[],
  onSample: (s: MetricSample, progress: number) => void,
  signal?: AbortSignal,
): Promise<BenchmarkRun> {
  const baseline = seedBenchmarks.find((b) => b.gameId === game.id && b.fixIds.length === 0)
  const baseFps = baseline?.avgFps ?? 60 + (hashString(game.id) % 70)
  let factor = 1 + (Math.random() - 0.5) * 0.04
  for (const id of appliedFixIds) {
    if (PERF_FIXES.has(id)) factor *= id === 'fp-ferrous-shader' ? 2.1 + Math.random() * 0.2 : 1.06 + Math.random() * 0.05
    if (REGRESSING_FIXES.has(id)) factor *= 0.86 + Math.random() * 0.04
  }
  const noisy = game.id === 'glasswing'
  const seed = Math.floor(Math.random() * 1e9)
  const samples = generateSamples({
    seed,
    fps: baseFps * factor,
    jitter: noisy ? 0.22 : 0.06,
    spikes: factor > 1.5 ? 0 : noisy ? 4 : 2,
    cpu: factor > 1.5 ? 48 : 70,
    gpu: factor > 1.5 ? 94 : 72,
    ram: 11.4,
    vram: 9.8,
    duration: 60,
  })
  for (let i = 0; i < samples.length; i++) {
    await wait(70, signal)
    onSample(samples[i], (i + 1) / samples.length)
  }
  const s = summarize(samples)
  return {
    id: uid('b'),
    gameId: game.id,
    label: appliedFixIds.length ? `Com ${appliedFixIds.length} correç${appliedFixIds.length > 1 ? 'ões' : 'ão'}` : 'Configuração padrão',
    at: new Date().toISOString(),
    preset: 'Ultra · 1440p',
    durationS: 60,
    fixIds: appliedFixIds,
    stdev: noisy ? 20 + Math.random() * 6 : 1.5 + Math.random(),
    samples,
    ...s,
  }
}

/** Fluxo de monitor ao vivo (simulado): uma amostra por intervalo. */
export function startLiveMonitor(baseFps: number, onSample: (s: MetricSample) => void, intervalMs = 500) {
  let t = 0
  let seed = Math.floor(Math.random() * 1e9)
  const id = setInterval(() => {
    const [s] = generateSamples({ seed: seed++, fps: baseFps, jitter: 0.08, spikes: Math.random() < 0.05 ? 1 : 0, cpu: 55, gpu: 82, ram: 11.3, vram: 9.6, duration: 1 })
    onSample({ ...s, t: t++ })
  }, intervalMs)
  return () => clearInterval(id)
}
