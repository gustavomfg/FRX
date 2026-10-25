import { fmtNum } from '@/lib/format'
import type { Metric, QualityLevel } from '@/types/performance'

/** Número com casas decimais ou "—" quando a métrica não existe no log. */
export const num = (v: number | null | undefined, digits = 0) => (v == null || !Number.isFinite(v) ? '—' : fmtNum(v, digits))

export function duration(s: number) {
  if (s < 60) return `${fmtNum(s, s < 10 ? 1 : 0)} s`
  const m = Math.floor(s / 60)
  const r = Math.round(s % 60)
  return r ? `${m} min ${r} s` : `${m} min`
}

export const QUALITY: Record<QualityLevel, { label: string; color: string; soft: string; help: string }> = {
  good: { label: 'Boa para comparar', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)', help: 'Sem problemas de dados detectados.' },
  usable: { label: 'Utilizável, com ressalvas', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)', help: 'Veja os avisos antes de tirar conclusões.' },
  unsuitable: { label: 'Inadequada para comparar', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)', help: 'Os dados não sustentam uma comparação confiável.' },
}

export const METRIC_LABEL: Record<Metric, string> = {
  fps: 'FPS',
  frametime: 'Frametime',
  cpuLoad: 'CPU',
  gpuLoad: 'GPU',
  cpuTemp: 'Temp. CPU',
  gpuTemp: 'Temp. GPU',
  cpuPower: 'Consumo CPU',
  gpuPower: 'Consumo GPU',
  gpuCoreClock: 'Clock GPU',
  gpuMemClock: 'Clock memória GPU',
  cpuMhz: 'Clock CPU',
  vram: 'VRAM',
  ram: 'RAM',
  swap: 'Swap',
  processRss: 'Memória do jogo',
}

export const DIGITS: Partial<Record<Metric, number>> = { vram: 1, ram: 1, swap: 2, processRss: 1, frametime: 1 }
