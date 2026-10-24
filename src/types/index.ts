/** Tipos de domínio do FRX. Tudo nesta etapa é alimentado por dados demonstrativos. */

export type CompatStatus = 'perfeito' | 'jogavel' | 'ajustes' | 'problematico' | 'nao-testado'

export type CoverScene = 'montanhas' | 'mar' | 'cidade' | 'floresta' | 'dunas' | 'espaco' | 'caverna'

export interface CoverArt {
  scene: CoverScene
  /** Paleta do céu → horizonte → primeiro plano (5 cores). */
  palette: [string, string, string, string, string]
  seed: number
  /** Elemento celeste: sol, lua, planeta ou nenhum. */
  orb?: 'sol' | 'lua' | 'planeta' | 'nenhum'
}

export interface Game {
  id: string
  title: string
  /** Origem dos dados: fictício (navegador/demo) ou detectado na Steam. */
  origin: 'demo' | 'steam'
  studio?: string
  genre?: string
  year?: number
  sizeGb?: number
  playtimeH?: number
  lastPlayed?: string // ISO
  store: 'Steam' | 'GOG' | 'Epic' | 'Local'
  runner: 'Proton' | 'Wine' | 'Nativo' | 'Desconhecido'
  protonVersion: string
  status: CompatStatus
  /** Resumo agregado (fictício) de relatos da comunidade. */
  communityReports?: number
  launchOptions?: string
  cover: CoverArt
  issueIds: string[]
  fixPackIds: string[]
  /** Dados reais da instalação (apenas quando origin === 'steam'). */
  steam?: import('./steam').InstalledGame
}

export type Severity = 'baixa' | 'media' | 'alta'

export type SourceKind = 'protondb' | 'github' | 'umu-protonfixes' | 'wiki' | 'frx-lab'

export interface Source {
  kind: SourceKind
  label: string
  /** Referência textual, sem links reais nesta etapa. */
  ref: string
}

export interface Issue {
  id: string
  gameId: string
  title: string
  description: string
  severity: Severity
  symptom: 'desempenho' | 'crash' | 'audio' | 'video' | 'input' | 'rede' | 'inicializacao'
  sources: Source[]
}

export type RiskLevel = 'baixo' | 'moderado' | 'alto'

export interface FileChange {
  path: string
  kind: 'env' | 'arquivo' | 'launch' | 'registro' | 'dll'
  action: 'adicionar' | 'alterar' | 'remover'
  before?: string
  after?: string
}

export interface FixPack {
  id: string
  gameId: string
  name: string
  summary: string
  solves: string[] // issue ids
  confidence: number // 0–100
  risk: RiskLevel
  backup: boolean
  reversible: boolean
  hardware: string[]
  proton: string[]
  sources: Source[]
  changes: FileChange[]
  estimatedSeconds: number
  /** Simula uma falha na primeira tentativa para demonstrar o estado de erro. */
  demoFailsOnce?: boolean
}

export type FixState = 'disponivel' | 'aplicado'

export interface FixHistoryEntry {
  id: string
  fixId: string
  gameId: string
  action: 'aplicado' | 'revertido' | 'falhou'
  at: string
  note?: string
}

export interface MetricSample {
  t: number // segundos
  fps: number
  frametime: number // ms
  cpu: number // %
  gpu: number // %
  ram: number // GB
  vram: number // GB
  tempGpu: number // °C (simulado)
  powerW: number // W (simulado)
}

export type BenchmarkVerdict = 'melhora' | 'regressao' | 'estavel' | 'inconclusivo'

export interface BenchmarkRun {
  id: string
  gameId: string
  label: string
  at: string
  preset: string
  durationS: number
  fixIds: string[]
  avgFps: number
  low1: number
  frametimeAvg: number
  frametimeP99: number
  /** Desvio padrão do FPS médio entre passadas — usado para medir confiança. */
  stdev: number
  samples: MetricSample[]
}

export interface Activity {
  id: string
  /** Eventos fictícios da demonstração — ocultos quando o app usa dados reais. */
  demo?: boolean
  at: string
  kind: 'scan' | 'fix' | 'revert' | 'benchmark' | 'analise' | 'erro'
  text: string
  gameId?: string
}

export interface SystemInfo {
  distro: string
  kernel: string
  desktop: string
  cpu: string
  gpu: string
  driver: string
  vulkan: string
  ramGb: number
  vramGb: number
  steamRuntime: string
  protonVersions: string[]
  defaultProton: string
}

export type MotionLevel = 'full' | 'calm' | 'off'

export interface Settings {
  theme: 'frx-dark'
  accent: 'manteiga' | 'menta' | 'coral' | 'ceu'
  mascots: boolean
  motion: MotionLevel
  reduceMotion: 'sistema' | 'sempre'
  scanOnStartup: boolean
  confirmBeforeApply: boolean
  backupRetentionDays: 7 | 30 | 90
  tempUnit: 'C' | 'F'
  compactSidebar: boolean
  /** No app desktop, usa a biblioteca fictícia em vez da Steam real. */
  useDemoData: boolean
  /** Validade do cache de correções, em horas. */
  fixCacheHours: 6 | 24 | 72 | 168
}
