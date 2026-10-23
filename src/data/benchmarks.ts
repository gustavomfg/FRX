import type { BenchmarkRun, MetricSample } from '@/types'
import { clamp, rng } from '@/lib/random'
import { ago } from './games'

/**
 * Corridas de benchmark DEMONSTRATIVAS. As amostras são geradas por um
 * PRNG com semente fixa; nenhuma é uma medição real.
 */

interface GenOpts {
  seed: number
  fps: number
  jitter: number // variação relativa (0–1)
  spikes: number // quantidade de engasgos
  cpu: number
  gpu: number
  ram: number
  vram: number
  duration?: number
}

export function generateSamples(o: GenOpts): MetricSample[] {
  const r = rng(o.seed)
  const n = o.duration ?? 60
  const spikeAt = new Set(Array.from({ length: o.spikes }, () => Math.floor(r() * n)))
  const out: MetricSample[] = []
  let drift = 0
  for (let t = 0; t < n; t++) {
    drift = drift * 0.8 + (r() - 0.5) * o.jitter * 0.9
    let fps = o.fps * (1 + drift + Math.sin(t / 6) * o.jitter * 0.35)
    if (spikeAt.has(t)) fps *= 0.42 + r() * 0.2
    fps = clamp(fps, 8, 400)
    const ft = 1000 / fps
    const gpu = clamp(o.gpu + (r() - 0.5) * 8, 5, 100)
    out.push({
      t,
      fps: Math.round(fps * 10) / 10,
      frametime: Math.round(ft * 100) / 100,
      cpu: Math.round(clamp(o.cpu + (r() - 0.5) * 10 + (spikeAt.has(t) ? 18 : 0), 3, 100)),
      gpu: Math.round(gpu),
      ram: Math.round((o.ram + (r() - 0.5) * 0.3) * 10) / 10,
      vram: Math.round((o.vram + (r() - 0.5) * 0.2) * 10) / 10,
      tempGpu: Math.round(48 + gpu * 0.26 + r() * 2),
      powerW: Math.round(70 + gpu * 1.9 + r() * 6),
    })
  }
  return out
}

export function summarize(samples: MetricSample[]) {
  const fps = samples.map((s) => s.fps)
  const avg = fps.reduce((a, b) => a + b, 0) / fps.length
  const sorted = [...fps].sort((a, b) => a - b)
  const lowCount = Math.max(1, Math.round(sorted.length * 0.01))
  const low1 = sorted.slice(0, lowCount).reduce((a, b) => a + b, 0) / lowCount
  const fts = samples.map((s) => s.frametime).sort((a, b) => a - b)
  const ftAvg = fts.reduce((a, b) => a + b, 0) / fts.length
  const p99 = fts[Math.min(fts.length - 1, Math.floor(fts.length * 0.99))]
  return { avgFps: avg, low1, frametimeAvg: ftAvg, frametimeP99: p99 }
}

function makeRun(
  base: Omit<BenchmarkRun, 'avgFps' | 'low1' | 'frametimeAvg' | 'frametimeP99' | 'samples'>,
  gen: GenOpts,
  override?: Partial<Pick<BenchmarkRun, 'avgFps' | 'low1'>>,
): BenchmarkRun {
  let samples = generateSamples(gen)
  if (override?.avgFps) {
    // Reescala as amostras para que o gráfico concorde com o valor exibido.
    const k = override.avgFps / summarize(samples).avgFps
    samples = samples.map((p) => {
      const fps = Math.round(p.fps * k * 10) / 10
      return { ...p, fps, frametime: Math.round((1000 / fps) * 100) / 100 }
    })
  }
  const s = summarize(samples)
  return { ...base, ...s, ...override, samples }
}

const sys = { ram: 11.2, vram: 9.4 }

export const seedBenchmarks: BenchmarkRun[] = [
  // Ferrous Drift — a melhora "vitrine": 50 → 112
  makeRun(
    { id: 'b-fd-1', gameId: 'ferrous-drift', label: 'Padrão · Proton 9.0-4', at: ago(30), preset: 'Ultra · 1440p', durationS: 60, fixIds: [], stdev: 2.1 },
    { seed: 101, fps: 52, jitter: 0.12, spikes: 9, cpu: 78, gpu: 64, ...sys },
    { avgFps: 50, low1: 21 },
  ),
  makeRun(
    { id: 'b-fd-2', gameId: 'ferrous-drift', label: 'Com cache de shaders', at: ago(28), preset: 'Ultra · 1440p', durationS: 60, fixIds: ['fp-ferrous-shader'], stdev: 2.6 },
    { seed: 102, fps: 112, jitter: 0.05, spikes: 0, cpu: 46, gpu: 94, ram: 11.6, vram: 10.1 },
    { avgFps: 112, low1: 88 },
  ),
  // Ashen Choir — regressão após NVAPI
  makeRun(
    { id: 'b-ac-1', gameId: 'ashen-choir', label: 'Padrão · GE-Proton9-20', at: ago(52), preset: 'Alto · 1440p', durationS: 60, fixIds: [], stdev: 1.8 },
    { seed: 201, fps: 71, jitter: 0.06, spikes: 1, cpu: 52, gpu: 91, ram: 13.1, vram: 12.2 },
    { avgFps: 71, low1: 54 },
  ),
  makeRun(
    { id: 'b-ac-2', gameId: 'ashen-choir', label: 'Com NVAPI exposta', at: ago(51), preset: 'Alto · 1440p', durationS: 60, fixIds: ['fp-ashen-nvapi'], stdev: 2.0 },
    { seed: 202, fps: 62, jitter: 0.08, spikes: 3, cpu: 55, gpu: 97, ram: 13.4, vram: 12.9 },
    { avgFps: 62, low1: 41 },
  ),
  // Moss & Mortar — estável
  makeRun(
    { id: 'b-mm-1', gameId: 'moss-mortar', label: 'Wine 10.2 · padrão', at: ago(100), preset: 'Alto · 1440p', durationS: 60, fixIds: [], stdev: 1.1 },
    { seed: 301, fps: 88, jitter: 0.04, spikes: 0, cpu: 41, gpu: 58, ram: 7.8, vram: 4.4 },
    { avgFps: 88, low1: 71 },
  ),
  makeRun(
    { id: 'b-mm-2', gameId: 'moss-mortar', label: 'Wine 10.2 Staging', at: ago(99), preset: 'Alto · 1440p', durationS: 60, fixIds: [], stdev: 1.2 },
    { seed: 302, fps: 89, jitter: 0.04, spikes: 0, cpu: 40, gpu: 59, ram: 7.9, vram: 4.4 },
    { avgFps: 89, low1: 72 },
  ),
  // Glasswing — inconclusivo (variância alta)
  makeRun(
    { id: 'b-gw-1', gameId: 'glasswing', label: 'Padrão · Proton 9.0-4', at: ago(75), preset: 'Médio · 1080p', durationS: 60, fixIds: [], stdev: 21 },
    { seed: 401, fps: 140, jitter: 0.22, spikes: 4, cpu: 33, gpu: 41, ram: 5.2, vram: 2.6 },
    { avgFps: 140, low1: 62 },
  ),
  makeRun(
    { id: 'b-gw-2', gameId: 'glasswing', label: 'Com latência de áudio fixa', at: ago(74), preset: 'Médio · 1080p', durationS: 60, fixIds: ['fp-glass-audio'], stdev: 24 },
    { seed: 402, fps: 151, jitter: 0.24, spikes: 4, cpu: 34, gpu: 43, ram: 5.2, vram: 2.6 },
    { avgFps: 151, low1: 60 },
  ),
]

export interface Comparison {
  before: BenchmarkRun
  after: BenchmarkRun
  deltaPct: number
  lowDeltaPct: number
  frametimeDeltaPct: number
  verdict: import('@/types').BenchmarkVerdict
}

/**
 * Classifica uma comparação. Se a diferença não supera a variação entre
 * passadas, o resultado é tratado como inconclusivo — nunca como ganho.
 */
export function compareRuns(before: BenchmarkRun, after: BenchmarkRun): Comparison {
  const deltaPct = ((after.avgFps - before.avgFps) / before.avgFps) * 100
  const lowDeltaPct = ((after.low1 - before.low1) / before.low1) * 100
  const frametimeDeltaPct = ((after.frametimeAvg - before.frametimeAvg) / before.frametimeAvg) * 100
  const noise = Math.sqrt(before.stdev ** 2 + after.stdev ** 2) * 1.5
  const diff = after.avgFps - before.avgFps
  let verdict: Comparison['verdict']
  if (Math.abs(diff) <= noise && noise > before.avgFps * 0.08) verdict = 'inconclusivo'
  else if (Math.abs(deltaPct) < 3) verdict = 'estavel'
  else verdict = diff > 0 ? 'melhora' : 'regressao'
  return { before, after, deltaPct, lowDeltaPct, frametimeDeltaPct, verdict }
}
