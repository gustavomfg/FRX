/**
 * Integração TypeScript ↔ Rust: a ponte `invoke` é simulada com o
 * `mockIPC` oficial do Tauri, devolvendo dados no formato real do Rust.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { AppStoreProvider } from '@/state/store'
import { LibraryProvider, useLibrary } from '@/state/library'
import { FixReportsProvider, useFixReports } from '@/state/fixes'
import { call, NativeError } from '@/services/native'
import { report, scan } from './fixtures'

;(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true

type Probe = { lib?: ReturnType<typeof useLibrary>; fixes?: ReturnType<typeof useFixReports> }

function render(probe: Probe) {
  function Inner() {
    probe.lib = useLibrary()
    probe.fixes = useFixReports()
    return null
  }
  const el = document.createElement('div')
  const root = createRoot(el)
  act(() =>
    root.render(
      <AppStoreProvider>
        <LibraryProvider>
          <FixReportsProvider>
            <Inner />
          </FixReportsProvider>
        </LibraryProvider>
      </AppStoreProvider>,
    ),
  )
  return root
}

const flush = () => act(() => new Promise((r) => setTimeout(r, 20)))

describe('ponte IPC', () => {
  const calls: string[] = []
  let failSearch = false

  beforeEach(() => {
    calls.length = 0
    failSearch = false
    localStorage.clear()
    ;(globalThis as unknown as { isTauri: boolean }).isTauri = true
    mockIPC((cmd) => {
      calls.push(cmd)
      switch (cmd) {
        case 'get_installed_games':
          return null
        case 'scan_installed_games':
          return scan
        case 'get_system_profile':
          return { distro: 'CachyOS', gpus: [], primaryGpu: null, vulkanDrivers: [], arch: 'x86_64' }
        case 'get_cached_fix_reports':
          return []
        case 'search_game_fixes':
          if (failSearch) throw 'sem conexão (simulado)'
          return report
        default:
          throw `comando inesperado: ${cmd}`
      }
    })
  })

  afterEach(() => {
    clearMocks()
    ;(globalThis as unknown as { isTauri: boolean }).isTauri = false
  })

  it('carrega a biblioteca real ao abrir e não mistura dados demonstrativos', async () => {
    const probe: Probe = {}
    const root = render(probe)
    await flush()
    expect(calls).toContain('get_installed_games')
    expect(calls).toContain('scan_installed_games')
    expect(probe.lib!.mode).toBe('real')
    expect(probe.lib!.phase).toBe('ready')
    expect(probe.lib!.games.map((g) => [g.id, g.origin])).toEqual([['200260', 'steam']])
    act(() => root.unmount())
  })

  it('pesquisa correções, guarda o relatório e expõe erros', async () => {
    const probe: Probe = {}
    const root = render(probe)
    await flush()
    await act(async () => {
      await probe.fixes!.search('200260', 'Batman: Arkham City GOTY')
    })
    expect(probe.fixes!.reports['200260'].findings).toHaveLength(1)

    failSearch = true
    await act(async () => {
      await probe.fixes!.search('200260', 'Batman: Arkham City GOTY', true)
    })
    expect(probe.fixes!.errors['200260']).toContain('sem conexão')
    expect(probe.fixes!.reports['200260']).toBeDefined() // o relatório anterior continua visível
    act(() => root.unmount())
  })

  it('no navegador usa a demonstração e não chama o Rust', async () => {
    ;(globalThis as unknown as { isTauri: boolean }).isTauri = false
    const probe: Probe = {}
    const root = render(probe)
    await flush()
    expect(probe.lib!.mode).toBe('demo')
    expect(probe.lib!.games.every((g) => g.origin === 'demo')).toBe(true)
    expect(calls).toHaveLength(0)
    await expect(call('scan_installed_games')).rejects.toBeInstanceOf(NativeError)
    act(() => root.unmount())
  })
})
