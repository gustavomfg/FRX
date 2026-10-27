import type { EnvironmentContext } from './steam'
import type { Availability } from './fixEngine'

/** IPC mirror of resolution/models.rs. Unknown states remain unknown in the UI. */
export interface ResolutionEvidence {
  id: string
  kind: string
  summary: string
  sourceUrl: string | null
  publisher: string | null
  publishedAt: string | null
  updatedAt: string | null
  fetchedAt: string
  version: string | null
  appId: string | null
  technology: string | null
  stale: boolean
  limitations: string[]
}

export interface ResolutionTechnology {
  id: string
  name: string
  hardware: string
  gameSupport: string
  environment: string
  configured: string
  runtime: string
  evidence: ResolutionEvidence[]
  limitations: string[]
}

export interface ResolutionProblem {
  id: string
  technology: string
  symptom: string
  certainty: string
  evidence: ResolutionEvidence[]
  hypotheses: string[]
  ruledOut: string[]
  limitations: string[]
  nextAction: string
  planIds: string[]
}

export interface ResolutionPlan {
  availability?: Availability
  id: string
  appId: string
  title: string
  objective: string
  problemId: string
  recipeId: string | null
  state: string
  actions: string[]
  dependencies: string[]
  risks: string[]
  sources: ResolutionEvidence[]
  reversible: boolean
  verificationMethod: string
  successCriterion: string
  operationId: string | null
  verification: string
  limitations: string[]
}

export interface ResolutionPack {
  id: string
  appId: string
  purpose: string
  title: string
  planIds: string[]
  smartPackId: string | null
  explanation: string
}

export interface ResolutionReport {
  schema: number
  appId: string
  gameName: string
  generatedAt: string
  contextFingerprint: string
  symptoms?: string[]
  environment: EnvironmentContext | null
  technologies: ResolutionTechnology[]
  problems: ResolutionProblem[]
  plans: ResolutionPlan[]
  packs: ResolutionPack[]
  knowledge: ResolutionEvidence[]
  limitations: string[]
}
