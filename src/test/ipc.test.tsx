/**
 * Ponte IPC: `call` repassa comando e argumentos ao núcleo Rust, converte
 * erros em `NativeError` com o nome do comando, e não tenta nada fora do app.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { call, NativeError } from '@/services/native'

type Globals = { isTauri: boolean }
const globals = globalThis as unknown as Globals

describe('ponte IPC', () => {
  const calls: { cmd: string; args: unknown }[] = []

  beforeEach(() => {
    calls.length = 0
    globals.isTauri = true
  })

  afterEach(() => {
    clearMocks()
    globals.isTauri = false
  })

  it('repassa o comando e os argumentos ao núcleo e devolve o resultado', async () => {
    mockIPC((cmd, args) => {
      calls.push({ cmd, args })
      return { distro: 'CachyOS' }
    })
    const profile = await call<{ distro: string }>('get_system_profile')
    expect(profile).toEqual({ distro: 'CachyOS' })
    await call('clear_fix_cache', { deep: true })
    expect(calls).toEqual([
      { cmd: 'get_system_profile', args: {} },
      { cmd: 'clear_fix_cache', args: { deep: true } },
    ])
  })

  it('erro do núcleo vira NativeError com a mensagem e o nome do comando', async () => {
    mockIPC(() => {
      throw 'sem conexão (simulado)'
    })
    const error = await call('search_hardware_fixes', { args: {} }).catch((e: unknown) => e)
    expect(error).toBeInstanceOf(NativeError)
    expect(error).toMatchObject({ message: 'sem conexão (simulado)', command: 'search_hardware_fixes' })
  })

  it('fora do app desktop recusa sem tentar falar com o núcleo', async () => {
    globals.isTauri = false
    const error = await call('identify_laptop').catch((e: unknown) => e)
    expect(error).toBeInstanceOf(NativeError)
    expect(error).toMatchObject({ message: 'Disponível apenas no app desktop.', command: 'identify_laptop' })
  })
})
