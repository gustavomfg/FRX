/** Fix Engine — espelho de `src-tauri/src/fix_engine/models.rs`. */
import type { ArtifactCandidate, FixCategory, GpuReq, ProviderId } from './fixes'
import type { EnvironmentContext, HardwareMatch, HardwareRequirements } from './steam'

export type Trust = 'discovered' | 'candidate' | 'reviewed' | 'unsupported' | 'revoked' | 'generated' | 'pack'
export type Purpose = 'diagnostic' | 'compatibility' | 'performance' | 'quality'
export type Risk = 'low' | 'moderate' | 'high'

export type RecipeAction =
  | { type: 'set_launch_env'; key: string; value: string }
  | { type: 'add_launch_args'; args: string[] }
  | { type: 'write_game_file'; path: string; content: string }
  | { type: 'install_artifact'; artifact: ArtifactCandidate }
  | { type: 'set_dxvk_option'; path: string; key: string; value: string }
  | { type: 'patch_known_config'; path: string; expectedSha256: string; replacements: { find: string; replace: string }[] }

export interface Recipe {
  hardware?: HardwareRequirements
  schema: number
  id: string
  version: string
  trust: Trust
  title: string
  summary: string
  description: string
  category: FixCategory
  purpose: Purpose
  risk: Risk
  scope: { appIds: string[]; requiresProton: boolean; gpuVendor: GpuReq | null }
  actions: RecipeAction[]
  alreadyIncludedByProtonfixes: boolean
  conflictsWith: string[]
  relatedKeys: string[]
  effects: string[]
  sideEffects: string[]
  sources: { label: string; url: string }[]
  review: { by: string; at: string; notes: string } | null
  /** Só em receitas geradas pelo FRX a partir de uma descoberta. */
  generated?: GeneratedInfo
}

export type EvidenceLevel = 'documented' | 'corroborated' | 'single' | 'contested' | 'none'

export interface Provenance {
  provider: ProviderId
  label: string
  url: string
  snippet: string | null
  publishedAt: string | null
}

/** Public game-specific reports, never a local measurement. */
export interface GamePerformanceEvidence {
  appId: string | null
  positive: number
  negative: number
  authoritativePositive?: boolean
  sourceUrls: string[]
  contextCompatible: boolean
  limitations: string[]
}

export interface GeneratedInfo {
  gameAppId?: string | null
  performanceEvidence?: GamePerformanceEvidence
  generatorVersion: string
  rulesVersion: string
  interpreter: string
  findingIds: string[]
  evidence: EvidenceLevel
  evidenceNote: string
  provenance: Provenance[]
  discoveredAt: string | null
}

export type DynStatus = 'generated' | 'reviewed' | 'already-included' | 'incompatible' | 'needs-validation' | 'blocked' | 'contested' | 'no-actions' | 'insufficient-data' | 'invalid'

export interface Availability {
  state: 'available' | 'temporary' | 'investigation' | 'incompatible' | 'applied'
  reasons: { code: string; message: string }[]
  checkedAt: string
  planStatus: PlanStatus | null
  planFingerprint: string | null
}

export type PlanStatus = 'ready' | 'requires-manual-step' | 'conflict' | 'incompatible' | 'already-applied' | 'already-included' | 'not-executable'
export type ExecMode = 'automatic' | 'manual'
export type CheckState = 'ok' | 'warn' | 'fail'

export interface Check {
  label: string
  state: CheckState
  detail: string
}

export interface PlannedChange {
  kind: 'launch-options' | 'game-file'
  target: string
  before: string | null
  after: string | null
  summary: string
}

export interface FixPlan {
  artifacts?: ArtifactCandidate[]
  environment?: EnvironmentContext | null
  hardwareMatch?: HardwareMatch | null
  performanceWarnings?: string[]
  planId: string
  fingerprint: string
  recipeId: string
  recipeVersion: string
  recipeTitle: string
  appId: string
  gameName: string
  status: PlanStatus
  mode: ExecMode | null
  reasons: string[]
  checks: Check[]
  changes: PlannedChange[]
  manualLine: string | null
  backupNote: string | null
  steamRunning: boolean
  gameRunning: boolean
  risk: Risk
  purpose: Purpose
  trust: Trust
  effects: string[]
  sideEffects: string[]
  sources: { label: string; url: string }[]
  generated: GeneratedInfo | null
  pack: { members: { recipeId: string; version: string; fingerprint: string; trust: Trust }[]; goal: string } | null
  createdAt: string
  expiresAt: number
}

export type OpState =
  | 'checking'
  | 'downloading'
  | 'verifying-artifacts'
  | 'backing-up'
  | 'applying'
  | 'verifying'
  | 'applied'
  | 'failed'
  | 'conflict'
  | 'requires-manual-step'
  | 'rolling-back'
  | 'rolled-back'
  | 'recovery-required'
  | 'cancelled'

export interface EnvWrite {
  key: string
  written: string
  previous: string | null
}

export type Step =
  | { type: 'launch-options'; config: string; appId: string; fileSha: string; before: string | null; after: string; env: EnvWrite[]; args: string[] }
  | { type: 'game-file'; root: string; rel: string; target: string; beforeSha: string | null; content: string; artifact?: ArtifactCandidate | null; identity?: { rootDev: number; rootIno: number; parentDev: number; parentIno: number } | null }

export interface BackupRef {
  path: string
  existed: boolean
  sha256: string | null
  size: number
  mode: number | null
}

export interface StepRecord {
  step: Step
  status: 'pending' | 'writing' | 'backed-up' | 'written' | 'verified' | 'reverted'
  backup: BackupRef | null
  afterSha: string | null
  rollbackBackup: BackupRef | null
}

export interface Operation {
  manualRecoveryRequired?: boolean
  artifacts?: ArtifactCandidate[]
  artifactProgress?: { downloadedBytes: number; totalBytes?: number | null; assetName: string } | null
  recipeFingerprint?: string | null
  schema: number
  id: string
  recipeId: string
  recipeVersion: string
  recipeTitle: string
  appId: string
  gameName: string
  state: OpState
  mode: ExecMode
  steps: StepRecord[]
  planFingerprint: string
  createdAt: string
  updatedAt: string
  appliedAt: string | null
  rolledBackAt: string | null
  message: string | null
  pending: { kind: 'apply' | 'rollback'; line: string } | null
  events: { at: string; state: OpState; message: string }[]
  recipeTrust: Trust | null
  findingIds: string[]
}

export type Effective = 'active' | 'drifted' | 'unknown'

export interface AppliedFix {
  operationId: string
  recipeId: string
  recipeVersion: string
  recipeTitle: string
  appId: string
  gameName: string
  appliedAt: string | null
  mode: ExecMode
  effective: Effective
  detail: string
}

export interface ActiveFixRef {
  recipeId: string
  recipeVersion: string
  operationId: string
  title: string
}

export interface RollbackPlan {
  operationId: string
  fingerprint: string
  status: PlanStatus
  mode: ExecMode | null
  reasons: string[]
  changes: PlannedChange[]
  manualLine: string | null
  steamRunning: boolean
  gameRunning: boolean
}

export interface EngineStatus {
  catalogVersion: string
  catalogSha256: string
  recipes: number
  reviewed: number
  issues: string[]
  recoveryRequired: number
  pendingManual: number
  busy: boolean
  dataDir: string
}
