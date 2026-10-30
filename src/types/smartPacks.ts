/** Smart Packs — espelho de `src-tauri/src/smart_packs/models.rs`. */
import type { Availability, DynStatus, EvidenceLevel, GamePerformanceEvidence, PlanStatus, Purpose, RecipeAction, Risk, Trust } from './fixEngine'
import type { ArtifactCandidate, GpuReq } from './fixes'
import type { HardwareRequirements } from './steam'

export type PackKind = 'baseline' | 'single' | 'combination'
export type Goal = 'reference' | 'compatibility' | 'performance' | 'balanced'
export type PerfHint = 'unknown' | 'penalty' | 'frame-cap'
export type PackState = 'candidate' | 'ready' | 'conflict' | 'testing' | 'applied' | 'verified' | 'failed' | 'partially_recovered' | 'rolled_back' | 'unsupported'
export type ConflictKind = 'same-key-different-value' | 'mutually-exclusive' | 'redundant' | 'proton-versions' | 'gpu-mismatch' | 'frame-limiter' | 'game-scope' | 'same-file'
export type Outcome = 'improvement' | 'regression' | 'no-difference' | 'mixed' | 'inconclusive' | 'not-measured'

export interface EnvPair {
  key: string
  value: string
}

export interface Edge {
  a: string
  b: string
  relation: 'no-known-conflict' | 'excludes'
  kind: ConflictKind | null
  reason: string
}

export interface FileFixRef {
  targetPath: string
  sha256: string
  artifactId: string | null
  artifactVersion: string | null
  artifactSha256: string | null
}

export interface Unit {
  hardware?: HardwareRequirements
  fileActions?: RecipeAction[]
  appId?: string | null
  performanceEvidence?: GamePerformanceEvidence
  recipeId: string
  version: string
  fingerprint: string
  trust: Trust
  title: string
  env: EnvPair[]
  purpose: Purpose
  risk: Risk
  evidence: EvidenceLevel
  evidenceNote: string
  findingIds: string[]
  sources: { label: string; url: string }[]
  protonVersions: string[]
  discoveredAt: string | null
  gpu: GpuReq | null
  alreadyIncluded: boolean
  perf: PerfHint
  effects: string[]
  sideEffects: string[]
}

export interface SmartPack {
  availability?: Availability
  artifacts?: ArtifactCandidate[]
  fileFixes?: FileFixRef[]
  performanceEvidence?: GamePerformanceEvidence
  performanceWarnings?: string[]
  id: string
  appId: string
  version: string
  name: string
  kind: PackKind
  goal: Goal
  goalText: string
  members: { recipeId: string; version: string; fingerprint: string; trust: Trust; title: string; env: EnvPair[]; fileActions?: RecipeAction[]; evidence: EvidenceLevel; discoveredAt: string | null; protonVersions: string[] }[]
  env: EnvPair[]
  applyRecipeId: string | null
  fingerprint: string
  requirements: string[]
  compatibility: string[]
  conflictsAvoided: Edge[]
  sources: { label: string; url: string }[]
  evidence: EvidenceLevel
  evidenceNote: string
  risk: Risk
  effects: string[]
  sideEffects: string[]
  reversible: boolean
  state: PackState
  stateDetail: string
  planStatus: PlanStatus | null
  operationId: string | null
}

export interface MetricStats {
  n: number
  median: number
  min: number
  max: number
  spreadPct: number | null
}

export interface SessionRef {
  id: string
  label: string
  capturedAt: string | null
  source: 'assisted' | 'declared'
  verified: boolean
  durationS: number
  avgFps: number | null
  low1: number | null
  excluded: string | null
  notes: string[]
}

export interface Measurements {
  packId: string
  sessions: SessionRef[]
  avg: MetricStats | null
  low1: MetricStats | null
  low01: MetricStats | null
  ftP99: MetricStats | null
  spikesPerMin: MetricStats | null
}

export interface MetricDelta {
  metric: string
  label: string
  deltaPct: number
  percentageAvailable?: boolean
  thresholdPct: number
  outcome: Outcome
  limited: boolean
  before: number
  after: number
  unit: string
}

export type EvalVerdict = 'improvement' | 'regression' | 'no-difference' | 'inconclusive' | 'not-comparable' | 'no-baseline' | 'reference'

export interface PackEvaluation {
  conditions?: import('./performance').Conditions
  appId: string
  packId: string
  packName: string
  sessionId: string
  sessionLabel: string
  capturedAt: string | null
  verdict: EvalVerdict
  preliminary: boolean
  headline: string
  summary: string
  deltas: MetricDelta[]
  baselineSessions: number
  packSessions: number
  limitations: string[]
  nextStep: string | null
  rankingEffect: string
  baselineId: string
  currentPackId: string | null
}

export interface VsBaseline {
  packId: string
  deltas: MetricDelta[]
  limited: boolean
  notes: string[]
}

export interface Excluded {
  performanceWarnings?: string[]
  id: string
  title: string
  status: DynStatus | null
  reason: string
}

export interface PackSnapshot {
  fileFixes?: FileFixRef[]
  appId?: string | null
  fingerprint?: string | null
  packId: string
  env: EnvPair[]
  recipes: { recipeId: string; version: string; fingerprint?: string | null }[]
  operationIds: string[]
  verified: boolean
  source: 'assisted' | 'declared'
  recordedAt: string
}

/** Complete Experience Pack — espelho de `smart_packs/experience.rs`. */
export type LeftOutKind = 'negative' | 'incompatible' | 'conflict' | 'risky' | 'already-included' | 'weak-evidence' | 'unsupported' | 'no-action'

export interface PackAction {
  recipeId: string
  title: string
  area: string
  changes: string[]
  effects: string[]
  sideEffects: string[]
  risk: Risk
  evidence: EvidenceLevel
  evidenceNote: string
  positive: number
  negative: number
  sources: { label: string; url: string }[]
  selected: boolean
}

export interface LeftOut {
  title: string
  reason: string
  kind: LeftOutKind
  changes: string[]
  count: number
}

export interface ExperiencePack {
  appId: string
  gameName: string
  generatedAt: string
  environment: string[]
  actions: PackAction[]
  leftOut: LeftOut[]
  areas: string[]
  risks: string[]
  compatibility: string[]
  packId: string | null
  applyRecipeId: string | null
  availability: Availability
  state: PackState
  stateDetail: string
  operationId: string | null
  partial: boolean
}
