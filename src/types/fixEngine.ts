/**
 * Tipos do Fix Engine. Espelham `src-tauri/src/fix_engine/models.rs`
 * (camelCase no JSON, valores kebab-case). A interface nunca monta estes
 * objetos para enviar de volta: só reenvia `planId`/`operationId` e a impressão
 * digital.
 */

export type Risk = 'low' | 'moderate' | 'high'

export type PlanStatus = 'ready' | 'requires-manual-step' | 'conflict' | 'incompatible' | 'already-applied'

/** `automatic`: o FRX grava sozinho (só com raiz fictícia). `manual`: mostra o `sudo`. */
export type ExecMode = 'automatic' | 'manual'

export type CheckState = 'ok' | 'warn' | 'fail'

export interface Check {
  label: string
  state: CheckState
  detail: string
}

export interface PlannedChange {
  kind: 'system-file'
  /** Caminho mostrado ao usuário. */
  target: string
  before: string | null
  after: string | null
  summary: string
}

export interface SourceLink {
  label: string
  url: string
}

/** Prévia calculada pelo núcleo. */
export interface FixPlan {
  planId: string
  fingerprint: string
  title: string
  /** Nome do laptop para o qual a prévia foi feita. */
  laptop: string
  status: PlanStatus
  mode: ExecMode | null
  reasons: string[]
  checks: Check[]
  changes: PlannedChange[]
  /** Comando `sudo` (ou sequência) para o modo manual. */
  manualLine: string | null
  backupNote: string | null
  risk: Risk
  effects: string[]
  sideEffects: string[]
  sources: SourceLink[]
  expiresAt: number
}

/** Identidade de um arquivo confinado (inode e diretório pinados). */
export interface FileIdentity {
  rootDev: number
  rootIno: number
  parentDev: number
  parentIno: number
}

export interface Step {
  /** Raiz do sistema: `/` em produção. */
  systemRoot: string
  /** Caminho absoluto (`/etc/modprobe.d/frx-x.conf`). */
  path: string
  identity: FileIdentity | null
  beforeSha: string | null
  content: string
  postApply: string[]
  preRollback: string[]
  postRollback: string[]
}

export type OpState =
  | 'checking'
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

export interface BackupRef {
  path: string
  /** `false`: o arquivo não existia, e o rollback o remove. */
  existed: boolean
  sha256: string | null
  size: number
  mode: number | null
}

export type StepStatus = 'pending' | 'backed-up' | 'writing' | 'written' | 'verified' | 'reverted'

export interface StepRecord {
  step: Step
  status: StepStatus
  backup: BackupRef | null
  afterSha: string | null
  rollbackBackup: BackupRef | null
}

export interface OpEvent {
  at: string
  state: OpState
  message: string
}

/** Passo manual que o usuário ainda precisa rodar (o `sudo`). */
export interface Pending {
  kind: 'apply' | 'rollback'
  line: string
}

/** Operação no diário. Mais recentes primeiro na listagem. */
export interface Operation {
  schema: number
  id: string
  /** Título da correção. */
  title: string
  /** Nome do laptop. */
  laptop: string
  manualRecoveryRequired: boolean
  state: OpState
  mode: ExecMode
  steps: StepRecord[]
  planFingerprint: string
  createdAt: string
  updatedAt: string
  appliedAt: string | null
  rolledBackAt: string | null
  message: string | null
  pending: Pending | null
  events: OpEvent[]
}

export interface RollbackPlan {
  operationId: string
  fingerprint: string
  status: PlanStatus
  mode: ExecMode | null
  reasons: string[]
  changes: PlannedChange[]
  manualLine: string | null
}
