/**
 * Performance Lab: ponte TS ↔ Rust (mockIPC), estados, histórico, seleção,
 * comparação, erros, modo demonstrativo e preferências de animação.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, type ReactNode } from 'react'
import { createRoot } from 'react-dom/client'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { emit } from '@tauri-apps/api/event'
import { AppStoreProvider, useAppState, useDispatch } from '@/state/store'
import { LibraryProvider } from '@/state/library'
import { FixReportsProvider } from '@/state/fixes'
import { PerformanceProvider, usePerformance } from '@/state/performance'
import { HistoryTable } from '@/components/performance/HistoryTable'
import { ComparisonView } from '@/components/performance/ComparisonView'
import { Mascot } from '@/mascot/Mascot'
import { duration, num } from '@/components/performance/format'
import { scan } from './fixtures'
import { comparison, mangohud, meta, session } from './perfFixtures'

;(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
const g = globalThis as unknown as { isTauri: boolean; ResizeObserver: unknown }
g.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
}

function mount(node: ReactNode) {
  const el = document.createElement('div')
  document.body.appendChild(el)
  const root = createRoot(el)
  act(() => root.render(node))
  return { el, unmount: () => act(() => root.unmount()) }
}

const providers = (child: ReactNode) => (
  <AppStoreProvider>
    <LibraryProvider>
      <FixReportsProvider>
        <PerformanceProvider>{child}</PerformanceProvider>
      </FixReportsProvider>
    </LibraryProvider>
  </AppStoreProvider>
)

const flush = () => act(() => new Promise((r) => setTimeout(r, 30)))

describe('PerformanceProvider (IPC)', () => {
  let sessions = [meta('a', 50)]
  let calls: { cmd: string; args: unknown }[] = []
  let importFails = false

  beforeEach(() => {
    sessions = [meta('a', 50)]
    calls = []
    importFails = false
    localStorage.clear()
    g.isTauri = true
    mockIPC(
      (cmd, args) => {
        calls.push({ cmd, args })
        switch (cmd) {
          case 'get_installed_games':
            return scan
          case 'scan_installed_games':
            return scan
          case 'get_system_profile':
            return { gpus: [], primaryGpu: null, vulkanDrivers: [], arch: 'x86_64' }
          case 'get_cached_fix_reports':
            return []
          case 'detect_mangohud':
            return mangohud
          case 'list_benchmark_sessions':
            return sessions
          case 'get_capture_status':
            return { state: 'idle', plan: null, hudAttached: false, controllable: false, frames: 0, message: null, sessionId: null, interrupted: false, updatedAt: '' }
          case 'import_benchmark':
            if (importFails) throw 'O log tem cabeçalho, mas nenhuma linha de dados.'
            sessions = [meta('b', 112), ...sessions]
            return sessions[0]
          default:
            return null
        }
      },
      { shouldMockEvents: true },
    )
  })
  afterEach(() => {
    clearMocks()
    g.isTauri = false
    document.body.innerHTML = ''
  })

  it('carrega histórico e MangoHud do Rust ao abrir', async () => {
    const probe: { p?: ReturnType<typeof usePerformance> } = {}
    function P() {
      probe.p = usePerformance()
      return null
    }
    const m = mount(providers(<P />))
    await flush()
    expect(probe.p!.enabled).toBe(true)
    expect(probe.p!.sessions.map((s) => s.id)).toEqual(['a'])
    expect(probe.p!.mangohud?.version).toBe('0.8.4-1+')
    m.unmount()
  })

  it('importa, recarrega o histórico e trata erros sem travar', async () => {
    const probe: { p?: ReturnType<typeof usePerformance>; toasts?: number } = {}
    function P() {
      probe.p = usePerformance()
      probe.toasts = useAppState().toasts.filter((t) => t.tone === 'error').length
      return null
    }
    const m = mount(providers(<P />))
    await flush()
    let r: unknown
    await act(async () => {
      r = await probe.p!.importLog('/x/b.csv', { appId: '200260' })
    })
    expect((r as { id: string }).id).toBe('b')
    expect(probe.p!.sessions).toHaveLength(2)
    const imp = calls.find((c) => c.cmd === 'import_benchmark')!.args as { path: string; input: { appId: string } }
    expect(imp.path).toBe('/x/b.csv')
    expect(imp.input.appId).toBe('200260')

    importFails = true
    await act(async () => {
      r = await probe.p!.importLog('/x/vazio.csv', {})
    })
    expect(r).toBeNull()
    expect(probe.toasts).toBe(1)
    m.unmount()
  })

  it('recebe o estado da captura por evento e recarrega ao concluir', async () => {
    const probe: { p?: ReturnType<typeof usePerformance> } = {}
    function P() {
      probe.p = usePerformance()
      return null
    }
    const m = mount(providers(<P />))
    await flush()
    const before = calls.filter((c) => c.cmd === 'list_benchmark_sessions').length
    await act(async () => {
      await emit('frx://capture', { state: 'recording', plan: null, hudAttached: true, controllable: true, frames: 500, recordedS: 5, message: 'Gravando', sessionId: null, interrupted: false, updatedAt: '' })
    })
    await flush()
    expect(probe.p!.capture?.state).toBe('recording')
    await act(async () => {
      await emit('frx://capture', { state: 'completed', plan: null, hudAttached: true, controllable: false, frames: 900, recordedS: 30, message: 'Sessão salva', sessionId: 'b', interrupted: false, updatedAt: '' })
    })
    await flush()
    expect(probe.p!.capture?.state).toBe('completed')
    expect(calls.filter((c) => c.cmd === 'list_benchmark_sessions').length).toBeGreaterThan(before)
    m.unmount()
  })

  it('no modo demonstrativo não chama o Rust', async () => {
    g.isTauri = false
    const probe: { p?: ReturnType<typeof usePerformance> } = {}
    function P() {
      probe.p = usePerformance()
      return null
    }
    const m = mount(providers(<P />))
    await flush()
    expect(probe.p!.enabled).toBe(false)
    expect(calls).toHaveLength(0)
    await act(async () => {
      expect(await probe.p!.importLog('/x/b.csv', {})).toBeNull()
      expect(await probe.p!.update('a', { label: 'teste' })).toBe(false)
      expect(await probe.p!.remove('a')).toBe(false)
      await probe.p!.start()
    })
    expect(calls).toHaveLength(0)
    m.unmount()
  })

  it('limpa o histórico real ao ativar a demonstração', async () => {
    const probe: { p?: ReturnType<typeof usePerformance> } = {}
    function P() {
      probe.p = usePerformance()
      const dispatch = useDispatch()
      return <button onClick={() => dispatch({ type: 'settings', patch: { useDemoData: true } })}>Demo</button>
    }
    const m = mount(providers(<P />))
    await flush()
    expect(probe.p!.sessions).toHaveLength(1)
    act(() => m.el.querySelector('button')!.click())
    await flush()
    expect(probe.p!.enabled).toBe(false)
    expect(probe.p!.sessions).toEqual([])
    expect(probe.p!.capture).toBeNull()
    m.unmount()
  })
})

describe('Histórico e comparação', () => {
  beforeEach(() => {
    // happy-dom não calcula layout; fornece o tamanho realista do contêiner ao Recharts.
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue({
      x: 0, y: 0, top: 0, left: 0, right: 800, bottom: 260, width: 800, height: 260,
      toJSON: () => ({}),
    })
  })
  afterEach(() => {
    vi.restoreAllMocks()
    document.body.innerHTML = ''
  })

  it('seleciona antes/depois e impede usar a mesma sessão nos dois', () => {
    const onBefore = vi.fn()
    const onAfter = vi.fn()
    const noop = () => {}
    const m = mount(
      <HistoryTable
        sessions={[meta('b', 112), meta('a', 50)]}
        viewId={null}
        beforeId={null}
        afterId="b"
        onView={noop}
        onBefore={onBefore}
        onAfter={onAfter}
        onEdit={noop}
        onDelete={noop}
        onExport={noop}
      />,
    )
    const rows = [...m.el.querySelectorAll('tbody tr')]
    const btn = (row: Element, label: string) => [...row.querySelectorAll('button')].find((b) => b.textContent === label) as HTMLButtonElement
    expect(btn(rows[0], 'Antes').disabled).toBe(true)
    expect(btn(rows[0], 'Depois').getAttribute('aria-pressed')).toBe('true')
    act(() => btn(rows[1], 'Antes').click())
    expect(onBefore).toHaveBeenCalledWith('a')
    act(() => btn(rows[0], 'Depois').click())
    expect(onAfter).toHaveBeenCalledWith(null)
    m.unmount()
  })

  it('mostra só números reais e o veredito sem depender da animação', () => {
    const m = mount(
      <AppStoreProvider>
        <ComparisonView result={comparison} before={session(meta('a', 50))} after={session(meta('b', 112))} />
      </AppStoreProvider>,
    )
    const text = m.el.textContent ?? ''
    expect(text).toContain('Melhoria observada')
    expect(text).toContain('+124,0%')
    expect(text).toContain('não prova')
    m.unmount()
  })

  it('formata valores ausentes como “—”, nunca como zero', () => {
    expect(num(null)).toBe('—')
    expect(num(undefined, 1)).toBe('—')
    expect(num(0)).toBe('0')
    expect(duration(75)).toBe('1 min 15 s')
  })

  it('a comparação não transforma FPS ausente em zero', () => {
    const before = meta('a', 50)
    before.stats.fps.avg = null
    const m = mount(
      <AppStoreProvider>
        <ComparisonView result={{ ...comparison, verdict: { ...comparison.verdict, kind: 'inconclusive', avgDeltaPct: null } }} before={session(before)} after={session(meta('b', 112))} />
      </AppStoreProvider>,
    )
    expect(m.el.textContent).toContain('FPS médio · — → 112,0')
    m.unmount()
  })
})

describe('Preferências de animação', () => {
  afterEach(() => {
    localStorage.clear()
    document.body.innerHTML = ''
  })

  const frames = () => document.querySelector('svg[role="img"]')?.innerHTML
  it('com animações desligadas o Pim fica parado; com mascotes desligados, some', async () => {
    localStorage.setItem('frx.settings.v1', JSON.stringify({ motion: 'off', mascots: true }))
    const m = mount(
      <AppStoreProvider>
        <Mascot pose="record" />
      </AppStoreProvider>,
    )
    const first = frames()
    await act(() => new Promise((r) => setTimeout(r, 600)))
    expect(frames()).toBe(first)
    m.unmount()

    localStorage.setItem('frx.settings.v1', JSON.stringify({ mascots: false }))
    const m2 = mount(
      <AppStoreProvider>
        <Mascot pose="celebrate" />
      </AppStoreProvider>,
    )
    expect(document.querySelector('svg[role="img"]')).toBeNull()
    m2.unmount()
  })
})
