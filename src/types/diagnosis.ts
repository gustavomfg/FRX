import type { EvidenceLevel } from '@/types/fixEngine'
import type { FindingSource, GpuReq, ProviderStatus } from '@/types/fixes'

/** Espelha `src-tauri/src/diagnosis/models.rs`. */
export type Symptom = 'unspecified' | 'crash-or-wont-start' | 'performance' | 'graphics' | 'other'

export type Outcome = 'positive' | 'not-resolved' | 'worsened' | 'mixed'
export type ClaimKind = { type: 'reported-result'; outcome: Outcome } | { type: 'documented' } | { type: 'mention-only' }

export interface Claim {
  kind: ClaimKind
  option: string
  disabled: boolean
  combinedWith: string[]
  quote: string | null
  source: FindingSource
  gameVerified: boolean
}

export interface ConfigKey {
  id: string
  items: { kind: string; name: string; value: string | null }[]
}

export type SuggestionState = 'applicable' | 'temporary' | 'already-applied' | 'already-included' | 'review'
export type CompatStatus = 'compatible' | 'unknown'

export interface Suggestion {
  key: ConfigKey
  title: string
  optionalImprovement: boolean
  state: SuggestionState
  confidence: EvidenceLevel
  reason: string
  compatibility: CompatStatus
  compatibilityDetail: string
  /** Fabricante que a variável ou a fonte exige. `null` não prova que vale em qualquer GPU. */
  gpuVendor: GpuReq | null
  positive: number
  negative: number
  claims: Claim[]
  recipeId: string | null
  expectedFingerprint: string | null
}

export type ExcludedKind = 'incompatible' | 'negative' | 'wrong-purpose'

export interface Excluded {
  key: ConfigKey
  title: string
  kind: ExcludedKind
  reason: string
  claims: Claim[]
}

export type Verdict = 'suggestions' | 'only-in-review' | 'no-evidence' | 'sources-unavailable'

export interface DiagnosisContext {
  gpuVendor: GpuReq | null
  gpuName: string | null
  driver: string | null
  protonTool: string | null
  usesProton: boolean | null
  toolIncludesProtonfixes: boolean
  gameFiles: string[]
  installKnown: boolean
  currentEnv: [string, string][]
}

export interface DiagnosisResult {
  appId: string
  symptom: Symptom
  verdict: Verdict
  message: string
  context: DiagnosisContext
  suggestions: Suggestion[]
  inReview: Suggestion[]
  excluded: Excluded[]
  providers: ProviderStatus[]
  limitations: string[]
  generatedAt: string
  origin: { method: string; learnedComponents: string[] }
  automaticChanges: boolean
}
