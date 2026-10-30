import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { call } from './native'
import type { AppliedFix, EngineStatus, FixPlan, Operation, RollbackPlan } from '@/types/fixEngine'

/**
 * Fix Engine (Rust: `src-tauri/src/commands/fix_engine.rs`).
 * Só identificadores e a impressão digital da prévia saem daqui — nunca
 * caminhos, valores ou comandos. O núcleo recalcula tudo antes de gravar.
 */

export const getFixEngineStatus = () => call<EngineStatus>('get_fix_engine_status')
export const prepareFixPlan = (recipeId: string, appId: string, expectedFingerprint?: string) => call<FixPlan>('prepare_fix_plan', { recipeId, appId, ...(expectedFingerprint ? { expectedFingerprint } : {}) })
export const applyFixPlan = (plan: Pick<FixPlan, 'planId' | 'fingerprint' | 'appId'>) =>
  call<Operation>('apply_fix_plan', { planId: plan.planId, fingerprint: plan.fingerprint, appId: plan.appId })
export const getFixOperation = (operationId: string) => call<Operation>('get_fix_operation', { operationId })
export const listFixOperations = (appId?: string | null) => call<Operation[]>('list_fix_operations', { appId: appId ?? null })
export const getAppliedFixes = (appId?: string | null) => call<AppliedFix[]>('get_applied_fixes', { appId: appId ?? null })
export const prepareFixRollback = (operationId: string) => call<RollbackPlan>('prepare_fix_rollback', { operationId })
export const rollbackFix = (plan: Pick<RollbackPlan, 'operationId' | 'fingerprint'>) => call<Operation>('rollback_fix', { operationId: plan.operationId, fingerprint: plan.fingerprint })
export const verifyFixOperation = (operationId: string) => call<Operation>('verify_fix_operation', { operationId })
export const cancelFixOperation = (operationId: string) => call<boolean>('cancel_fix_operation', { operationId })

export const onFixProgress = (cb: (op: Operation) => void): Promise<UnlistenFn> => listen<Operation>('fix-engine://progress', (e) => cb(e.payload))
/** Tecnologias gráficas que os arquivos do jogo trazem (DLSS, frame generation, PhysX…). */
export const gameTechnologies = (appId: string) => call<string[]>('game_technologies', { appId })
