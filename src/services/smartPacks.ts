import { call } from './native'
import type { ExperiencePack, PackEvaluation } from '@/types/smartPacks'

/**
 * Pacotes (Rust: `src-tauri/src/commands/smart_packs.rs`). Aplicar e desfazer
 * passam pelos comandos do Fix Engine com o `applyRecipeId` do pacote — só
 * IDs saem daqui.
 */

/** Avaliação automática de uma sessão contra o baseline (preliminar com poucas sessões). */
export const evaluateBenchmarkSession = (sessionId: string) => call<PackEvaluation>('evaluate_benchmark_session', { sessionId })
/** Complete Experience Pack do jogo; `without` = IDs de receitas que o usuário desligou. */
export const getExperiencePack = (appId: string, without: string[] = []) => call<ExperiencePack>('get_experience_pack', { appId, without })
