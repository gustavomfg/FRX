import { call } from './native'
import type { Operation, RollbackPlan } from '@/types/fixEngine'

/**
 * Diário e desfazer do Fix Engine (Rust: `commands/fix_engine.rs`).
 * Só identificadores e a impressão digital da prévia saem daqui: o núcleo
 * recalcula tudo antes de qualquer alteração.
 */

/** Operações registradas, mais recentes primeiro. */
export const listFixOperations = () => call<Operation[]>('list_fix_operations')
export const prepareFixRollback = (operationId: string) => call<RollbackPlan>('prepare_fix_rollback', { operationId })
export const rollbackFix = (plan: Pick<RollbackPlan, 'operationId' | 'fingerprint'>) =>
  call<Operation>('rollback_fix', { operationId: plan.operationId, fingerprint: plan.fingerprint })
/** Confere no disco um passo manual já rodado, ou uma operação interrompida. */
export const verifyFixOperation = (operationId: string) => call<Operation>('verify_fix_operation', { operationId })
