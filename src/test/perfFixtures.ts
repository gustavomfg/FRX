import type { ComparisonResult, MangoHudStatus, Session, SessionMeta } from '@/types/performance'

/** Sessões no formato exato do Rust (números conhecidos: 50 → 112 FPS). */
export function meta(id: string, fps: number, over: Partial<SessionMeta> = {}): SessionMeta {
  return {
    schemaVersion: 1,
    analysisVersion: 4,
    id,
    appId: '200260',
    gameName: 'Batman: Arkham City GOTY',
    label: `Sessão ${id}`,
    notes: '',
    createdAt: '2026-10-24T18:00:00Z',
    capturedAt: '2026-10-24T18:00:00',
    source: 'import',
    originalName: `${id}.csv`,
    originalPath: `/x/${id}.csv`,
    rawHash: id,
    rawBytes: 100,
    format: 'mangohud-v1',
    mangohudVersion: '0.8.4-1+',
    conditions: { proton: null, gpu: 'RTX 4060', driver: null, kernel: null, os: null, cpu: null, cpuScheduler: null, resolution: '1920x1080', preset: 'Alto', fpsLimit: null, vsync: null },
    stats: {
      mode: 'per-frame',
      durationS: 60,
      rows: fps * 60,
      validRows: fps * 60,
      frames: fps * 60,
      fps: { avg: fps, median: fps, min: fps * 0.5, max: fps * 1.2, low1: fps * 0.7, low01: fps * 0.6, lowsEstimated: false },
      frametime: { avg: 1000 / fps, median: 1000 / fps, p90: null, p95: null, p99: 1500 / fps, p999: null, max: 2000 / fps, stdev: 1, histogram: [] },
      stability: { spikes: 0, spikesPerMin: 0, unstableIntervals: [], firstThirdFps: null, lastThirdFps: null, trendPct: null, cv: 0.05, perSecondFpsStdev: 1, seconds: 60, resolutionNote: null },
      resources: { cpuLoad: { avg: 40, min: 30, max: 50, p95: 48, unit: '%', samples: 100 } },
      available: ['fps', 'frametime', 'cpuLoad'],
    },
    quality: { level: 'good', issues: [], totalRows: fps * 60, excludedRows: 0 },
    fixIds: [],
    ...over,
  }
}

export const session = (m: SessionMeta): Session => ({
  meta: m,
  series: [{ key: 'fps', metric: 'fps', unit: 'FPS', points: [[0, m.stats.fps.avg ?? 0], [30, m.stats.fps.avg ?? 0], [60, m.stats.fps.avg ?? 0]] }],
})

export const comparison: ComparisonResult = {
  beforeId: 'a',
  afterId: 'b',
  rows: [{ key: 'fpsAvg', label: 'FPS médio', unit: 'FPS', before: 50, after: 112, absDiff: 62, pctDiff: 124, better: 'higher', direction: 'improved', estimated: false }],
  verdict: { kind: 'improvement', avgDeltaPct: 124, low1DeltaPct: 124, noisePct: 1, reasons: ['FPS médio +124,0%, acima da variação esperada (±3,0%).'] },
  comparability: { controlled: true, differences: [], notes: [] },
}

export const mangohud: MangoHudStatus = {
  installed: true,
  executable: '/usr/bin/mangohud',
  version: '0.8.4-1+',
  layers: [],
  has64bit: true,
  has32bit: true,
  openglShim: true,
  mangohudctl: null,
  mangoapp: null,
  gamescope: null,
  loggingSupported: true,
  flatpakSteam: false,
  flatpakLayer: [],
  notes: [],
  installHint: null,
}
