/**
 * Performance Lab — espelho de `src-tauri/src/performance/{models,comparison,capture,detection}.rs`.
 * Todos os números vêm de logs reais do MangoHud; nada aqui é gerado.
 */

export type Metric =
  | 'fps'
  | 'frametime'
  | 'cpuLoad'
  | 'gpuLoad'
  | 'cpuTemp'
  | 'gpuTemp'
  | 'cpuPower'
  | 'gpuPower'
  | 'gpuCoreClock'
  | 'gpuMemClock'
  | 'cpuMhz'
  | 'vram'
  | 'ram'
  | 'swap'
  | 'processRss'

export type LogFormat = 'mangohud-v1' | 'mangohud' | 'generic-frametime'
export type CaptureMode = 'per-frame' | 'periodic'
export type Severity = 'info' | 'warning' | 'error'
export type QualityLevel = 'good' | 'usable' | 'unsuitable'
export type SessionSource = 'import' | 'assisted'

export interface Issue {
  severity: Severity
  code: string
  message: string
}

export interface Quality {
  level: QualityLevel
  issues: Issue[]
  totalRows: number
  excludedRows: number
}

export interface FpsStats {
  avg: number | null
  median: number | null
  min: number | null
  max: number | null
  low1: number | null
  low01: number | null
  lowsEstimated: boolean
}

export interface HistogramBucket {
  label: string
  fromMs: number
  toMs: number | null
  timeShare: number
  count: number
}

export interface FrametimeStats {
  avg: number | null
  median: number | null
  p90: number | null
  p95: number | null
  p99: number | null
  p999: number | null
  max: number | null
  stdev: number | null
  histogram: HistogramBucket[]
}

export interface Interval {
  startS: number
  endS: number
  spikes: number
  worstMs: number
  cv: number
}

export interface Stability {
  spikes: number
  spikesPerMin: number | null
  unstableIntervals: Interval[]
  firstThirdFps: number | null
  lastThirdFps: number | null
  trendPct: number | null
  cv: number | null
  perSecondFpsStdev: number | null
  seconds: number
  resolutionNote: string | null
}

export interface ResourceStat {
  avg: number
  min: number
  max: number
  p95: number
  unit: string
  samples: number
}

export interface SessionStats {
  mode: CaptureMode
  durationS: number
  rows: number
  validRows: number
  frames: number | null
  fps: FpsStats
  frametime: FrametimeStats
  stability: Stability
  resources: Partial<Record<Metric, ResourceStat>>
  available: Metric[]
}

export interface Series {
  key: string
  metric: Metric
  unit: string
  points: [number, number][]
}

export interface Conditions {
  proton: string | null
  gpu: string | null
  driver: string | null
  kernel: string | null
  os: string | null
  cpu: string | null
  cpuScheduler: string | null
  resolution: string | null
  preset: string | null
  fpsLimit: string | null
  vsync: boolean | null
}

export interface SessionMeta {
  schemaVersion: number
  analysisVersion: number
  id: string
  appId: string | null
  gameName: string | null
  label: string
  notes: string
  createdAt: string
  capturedAt: string | null
  source: SessionSource
  originalName: string | null
  originalPath: string | null
  rawHash: string
  rawBytes: number
  format: LogFormat
  mangohudVersion: string | null
  conditions: Conditions
  stats: SessionStats
  quality: Quality
  fixIds: string[]
}

export interface Session {
  meta: SessionMeta
  series: Series[]
}

export interface SessionInput {
  appId?: string | null
  gameName?: string | null
  label?: string | null
  notes?: string | null
  conditions?: Partial<Conditions>
}

export interface SessionPatch {
  label?: string
  notes?: string
  appId?: string
  gameName?: string
  conditions?: Conditions
}

// --- Comparação -------------------------------------------------------------

export type Better = 'higher' | 'lower' | 'neutral'
export type VerdictKind = 'improvement' | 'regression' | 'small-difference' | 'inconclusive'

export interface CompareRow {
  key: string
  label: string
  unit: string
  before: number
  after: number
  absDiff: number
  pctDiff: number | null
  better: Better
  direction: 'improved' | 'worsened' | 'same'
  estimated: boolean
}

export interface Verdict {
  kind: VerdictKind
  avgDeltaPct: number | null
  low1DeltaPct: number | null
  noisePct: number | null
  reasons: string[]
}

export interface Difference {
  field: string
  label: string
  before: string | null
  after: string | null
  major: boolean
}

export interface ComparisonResult {
  beforeId: string
  afterId: string
  rows: CompareRow[]
  verdict: Verdict
  comparability: { controlled: boolean; differences: Difference[]; notes: string[] }
}

// --- MangoHud e captura -------------------------------------------------------

export interface LayerInfo {
  manifest: string
  arch: string
  library: string
  libraryExists: boolean
}

export interface MangoHudStatus {
  installed: boolean
  executable: string | null
  version: string | null
  layers: LayerInfo[]
  has64bit: boolean
  has32bit: boolean
  openglShim: boolean
  mangohudctl: string | null
  mangoapp: string | null
  gamescope: string | null
  loggingSupported: boolean
  flatpakSteam: boolean
  flatpakLayer: string[]
  notes: string[]
  installHint: { distro: string; commands: string[] } | null
}

export type CaptureState =
  | 'idle'
  | 'checking_dependencies'
  | 'ready'
  | 'waiting_for_game'
  | 'recording'
  | 'processing'
  | 'completed'
  | 'failed'
  | 'cancelled'

export type StartMode = 'auto' | 'manual'

export interface CaptureRequest {
  appId: string
  gameName: string
  durationS: number
  start: StartMode
  startDelayS: number
  showHud: boolean
  input: SessionInput
}

export interface CapturePlan {
  appId: string
  gameName: string
  durationS: number
  start: StartMode
  startDelayS: number
  showHud: boolean
  configPath: string
  outputDir: string
  socketName: string
  launchOptions: string
  launchOptionsFlatpak: string
  existingLaunchOptions: string | null
  flatpakOverride: string | null
  createdAt: string
  input: SessionInput
}

export interface CaptureStatus {
  state: CaptureState
  plan: CapturePlan | null
  hudAttached: boolean
  hudVersion: string | null
  device: string | null
  controllable: boolean
  logFile: string | null
  frames: number
  recordedS: number | null
  message: string | null
  sessionId: string | null
  interrupted: boolean
  updatedAt: string
}
