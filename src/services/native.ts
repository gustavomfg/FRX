import { invoke } from '@tauri-apps/api/core'
import { isTauri } from './platform'

/** Erro padronizado das chamadas nativas, pronto para exibir na interface. */
export class NativeError extends Error {
  constructor(
    message: string,
    public command: string,
  ) {
    super(message)
  }
}

/** `invoke` com mensagens de erro legíveis e proteção fora do Tauri. */
export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) throw new NativeError('Disponível apenas no app desktop.', command)
  try {
    return await invoke<T>(command, args)
  } catch (e) {
    const msg = typeof e === 'string' ? e : e instanceof Error ? e.message : 'Erro desconhecido na camada nativa.'
    throw new NativeError(msg, command)
  }
}

export function errorMessage(e: unknown) {
  if (e instanceof Error) return e.message
  return typeof e === 'string' ? e : 'Erro desconhecido.'
}
