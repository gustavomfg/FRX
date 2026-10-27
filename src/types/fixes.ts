/**
 * Descobertas de correções — espelho de `src-tauri/src/fixes/models.rs`.
 * Nesta versão tudo é somente leitura: nada aqui é aplicado ao sistema.
 */

export type FixCategory = 'compatibility' | 'performance' | 'graphics' | 'audio' | 'video' | 'physics' | 'launcher' | 'input' | 'network' | 'other'
export type FindingKind = 'protonfix' | 'tweak' | 'report' | 'release-note'
export type Confidence = 'verified' | 'curated' | 'community-reported' | 'unverified'
export type Compatibility = 'compatible' | 'incompatible' | 'unknown' | 'already-included'
export type AnalysisState = 'parsed' | 'partial' | 'not-verified'
export type ProviderId = 'umu-protonfixes' | 'umu-database' | 'github' | 'protondb'
export type ProviderState = 'ok' | 'cached' | 'stale' | 'no-data' | 'unavailable' | 'rate-limited' | 'disabled'
export type GpuReq = 'nvidia' | 'amd' | 'intel'

export interface FixAction {
  kind: 'winetricks' | 'env' | 'dll-override' | 'launch-option' | 'config-file' | 'registry' | 'proton-version' | 'other'
  label: string
  value: string
  /** Executada apenas sob alguma condição no script original. */
  conditional: boolean
}

export interface FindingSource {
  provider: ProviderId
  label: string
  url: string
  referenceId: string | null
  snippet: string | null
  publishedAt: string | null
}

export interface DiscoveryScope {
  kind: 'verified-game' | 'general-technical' | 'unknown-scope'
  appId: string | null
  basis: string
}

export interface PerformanceReport {
  appId: string
  actionKey: string
  sourceUrl: string
  author: string | null
  reportedAt: string | null
  statement: string
  outcome: 'positive' | 'negative'
  protonVersion: string | null
  gpuVendor: GpuReq | null
  confidence: Confidence
  limitations: string[]
}

export interface FixFinding {
  scope?: DiscoveryScope
  performanceEvidence?: { positive: number; negative: number; reports: PerformanceReport[] }
  id: string
  gameAppId: string
  title: string
  description: string
  category: FixCategory
  kind: FindingKind
  actions: FixAction[]
  sources: FindingSource[]
  requirements: { gpuVendor: GpuReq | null; protonVersions: string[]; gameVersion: string | null }
  confidence: Confidence
  compatibility: Compatibility
  compatibilityNote: string
  experimental: boolean
  analysis: AnalysisState
  evidence: { positive: number; negative: number }
  publishedAt: string | null
  retrievedAt: string
}

export interface ProviderStatus {
  id: ProviderId
  name: string
  state: ProviderState
  message: string | null
  fetchedAt: string | null
  resetAt: string | null
}

export interface FixReport {
  appId: string
  gameName: string
  findings: FixFinding[]
  providers: ProviderStatus[]
  identity: { umuId: string | null; aliases: string[] }
  context: { gpuVendor: GpuReq | null; toolName: string | null; toolIncludesProtonfixes: boolean }
  retrievedAt: string
  /** Algum provedor respondeu a partir do cache local. */
  fromCache: boolean
  /** Algum dado está além da validade configurada (não é confirmação atual). */
  stale: boolean
}

export interface ProviderInfo {
  id: ProviderId
  name: string
  enabled: boolean
  description: string
  reason: string | null
}
