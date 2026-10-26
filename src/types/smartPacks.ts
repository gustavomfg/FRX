/** Smart Packs — espelho de `src-tauri/src/smart_packs/models.rs`. */
import type { DynStatus, EvidenceLevel, PlanStatus, Purpose, Risk, Trust } from './fixEngine'
import type { GpuReq } from './fixes'

export type Profile = 'performance' | 'balanced' | 'compatibility'
export type PackKind = 'baseline' | 'single' | 'combination'
export type Goal = 'reference' | 'compatibility' | 'performance' | 'balanced'
export type PerfHint = 'unknown' | 'penalty' | 'frame-cap'
export type PackState = 'candidate' | 'ready' | 'conflict' | 'testing' | 'applied' | 'verified' | 'failed' | 'partially_recovered' | 'rolled_back' | 'unsupported'
export type ConflictKind = 'same-key-different-value' | 'mutually-exclusive' | 'redundant' | 'proton-versions' | 'gpu-mismatch' | 'frame-limiter'
export type Outcome = 'improvement' | 'regression' | 'no-difference' | 'mixed' | 'inconclusive' | 'not-measured'
export type Tier = 'measured' | 'candidate' | 'not-recommended'

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

export interface Unit {
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
  id: string
  appId: string
  version: string
  name: string
  kind: PackKind
  goal: Goal
  goalText: string
  members: { recipeId: string; version: string; fingerprint: string; trust: Trust; title: string; env: EnvPair[]; evidence: EvidenceLevel; discoveredAt: string | null; protonVersions: string[] }[]
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

export interface RankEntry {
  packId: string
  title: string
  tier: Tier
  position: number | null
  outcome: Outcome
  headline: string
  reasons: string[]
  vsBaseline: VsBaseline | null
}

export interface Ranking {
  profile: Profile
  version: string
  entries: RankEntry[]
  winner: string | null
  summary: string
  nextTest: { packId: string; reason: string } | null
  criteria: string[]
}

export interface RankingSnapshot {
  at: string
  profile: Profile
  version: string
  reason: string
  entries: [string, number | null, Tier, Outcome][]
  sessions: number
}

export interface Excluded {
  id: string
  title: string
  status: DynStatus | null
  reason: string
}

export interface WindowsReference {
  origin: string
  url: string | null
  avgFps: number | null
  low1: number | null
  conditions: string
}

export interface SmartPackReport {
  appId: string
  gameName: string
  profile: Profile
  generatedAt: string
  composerVersion: string
  rulesVersion: string
  limits: { maxUnits: number; maxPacks: number; maxPackSize: number }
  referenceSessionId: string | null
  baselineId: string
  packs: SmartPack[]
  units: Unit[]
  graph: Edge[]
  excluded: Excluded[]
  measurements: Measurements[]
  ranking: Ranking
  history: RankingSnapshot[]
  unassignedSessions: SessionRef[]
  currentPackId: string | null
  currentNote: string
  windows: WindowsReference | null
  evaluations: PackEvaluation[]
  notes: string[]
}

export interface PackComparison {
  a: string
  b: string
  deltas: MetricDelta[]
  limited: boolean
  summary: string
  notes: string[]
}

export interface SwitchPlan {
  to: string
  from: string | null
  keep: EnvPair[]
  remove: EnvPair[]
  add: EnvPair[]
  steps: { kind: 'rollback' | 'apply'; operationId: string | null; recipeId: string | null; title: string; detail: string }[]
  possible: boolean
  notes: string[]
}

export interface PackSnapshot {
  packId: string
  env: EnvPair[]
  recipes: { recipeId: string; version: string }[]
  operationIds: string[]
  verified: boolean
  source: 'assisted' | 'declared'
  recordedAt: string
}
