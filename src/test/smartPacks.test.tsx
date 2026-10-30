/**
 * Performance Lab: configuração e conclusão da sessão em relação à
 * referência. As avaliações aqui são **fixtures sintéticas** no formato
 * exato do Rust; os números são escolhidos para o teste, não medidos.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { act, type ReactNode } from 'react'
import { createRoot } from 'react-dom/client'
import { MemoryRouter } from 'react-router-dom'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { MotionGlobalConfig } from 'motion/react'
import { AppStoreProvider } from '@/state/store'
import { LibraryProvider } from '@/state/library'
import { FixReportsProvider } from '@/state/fixes'
import { FixEngineProvider } from '@/state/fixEngine'
import { PerformanceProvider } from '@/state/performance'
import { SessionHero } from '@/components/performance/SessionView'
import { SessionPackResult } from '@/components/smart/SessionPackResult'
import type { PackEvaluation } from '@/types/smartPacks'
import { scan } from './fixtures'
import { meta } from './perfFixtures'

;(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
MotionGlobalConfig.skipAnimations = true
const g = globalThis as unknown as { isTauri: boolean }

const BASE = 'pack.200260.0000000000000000'
const D3D = 'pack.200260.1111111111111111'

const regression = (over: Partial<PackEvaluation> = {}): PackEvaluation => ({
  appId: '200260',
  packId: D3D,
  packName: 'Desativar Direct3D 11',
  sessionId: 's-new',
  sessionLabel: 'Smart Pack · Desativar Direct3D 11',
  capturedAt: '2026-10-26T21:30:00Z',
  verdict: 'regression',
  preliminary: true,
  headline: 'Indício de regressão — recomendado repetir o teste',
  summary: 'FPS médio 95,0 → 55,0 FPS (−42,1%, limiar ±13,4%)',
  deltas: [
    { metric: 'fpsAvg', label: 'FPS médio', deltaPct: -42.105, thresholdPct: 13.4, outcome: 'regression', limited: true, before: 95, after: 55, unit: 'FPS' },
    { metric: 'fpsLow1', label: '1% low', deltaPct: -45.7, thresholdPct: 5, outcome: 'regression', limited: true, before: 70, after: 38, unit: 'FPS' },
  ],
  baselineSessions: 1,
  packSessions: 1,
  limitations: ['A comparação assume o mesmo trecho do jogo, as mesmas configurações gráficas e duração parecida. Cenas diferentes também mudam o FPS.'],
  nextStep: 'Repita o teste no mesmo trecho: faltam 1 sessão(ões) do baseline e 1 do pacote para uma conclusão.',
  rankingEffect: 'Preliminar: o pacote fica no fim dos candidatos, marcado com indício de regressão, e não entra em combinações.',
  baselineId: BASE,
  currentPackId: D3D,
  ...over,
})

function mount(node: ReactNode) {
  const el = document.createElement('div')
  document.body.appendChild(el)
  const root = createRoot(el)
  act(() =>
    root.render(
      <AppStoreProvider>
        <LibraryProvider>
          <FixReportsProvider>
            <FixEngineProvider>
              <PerformanceProvider>
                <MemoryRouter>{node}</MemoryRouter>
              </PerformanceProvider>
            </FixEngineProvider>
          </FixReportsProvider>
        </LibraryProvider>
      </AppStoreProvider>,
    ),
  )
  return { unmount: () => act(() => root.unmount()) }
}

const flush = (ms = 40) => act(() => new Promise((r) => setTimeout(r, ms)))
const text = () => document.body.textContent ?? ''
const button = (t: string) => [...document.body.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === t)
const click = async (el: HTMLElement | undefined) => {
  await act(async () => el!.click())
  await flush()
}

describe('Performance Lab e pacotes', () => {
  let calls: { cmd: string; args: Record<string, unknown> }[] = []
  let evaluationFailure: string | null = null

  beforeEach(() => {
    calls = []
    evaluationFailure = null
    g.isTauri = true
    mockIPC(
      async (cmd, args) => {
        calls.push({ cmd, args: (args ?? {}) as Record<string, unknown> })
        switch (cmd) {
          case 'get_installed_games':
          case 'scan_installed_games':
            return scan
          case 'get_system_profile':
            return { gpus: [], primaryGpu: null, vulkanDrivers: [], arch: 'x86_64' }
          case 'get_cached_fix_reports':
          case 'list_fix_operations':
          case 'get_applied_fixes':
          case 'list_benchmark_sessions':
            return []
          case 'get_fix_engine_status':
            return { catalogVersion: 't', catalogSha256: 'x', recipes: 4, reviewed: 3, issues: [], recoveryRequired: 0, pendingManual: 0, busy: false, dataDir: '/d' }
          case 'evaluate_benchmark_session':
            if (evaluationFailure) throw new Error(evaluationFailure)
            return regression({ sessionId: (args as { sessionId: string }).sessionId })
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

  it('Performance Lab mostra a configuração do pacote na sessão', () => {
    const s = meta('a', 100, { smartPack: { packId: D3D, env: [{ key: 'PROTON_NO_D3D11', value: '1' }], recipes: [], operationIds: ['op-1'], verified: true, source: 'assisted', recordedAt: 'x' } })
    const m = mount(<SessionHero meta={s} />)
    expect(text()).toContain('PROTON_NO_D3D11=1 · conferido')
    m.unmount()
    const old = mount(<SessionHero meta={meta('b', 100)} />)
    expect(text()).toContain('Não registrado')
    old.unmount()
  })

  it('Performance Lab mostra a conclusão da sessão com os valores do núcleo', async () => {
    const m = mount(<SessionPackResult meta={meta('s-new', 55)} />)
    await flush(80)
    expect(text()).toContain('Pacote · resultado')
    expect(text()).toContain('Indício de regressão — recomendado repetir o teste')
    expect(text()).toContain('FPS médio')
    expect(button('Abrir o pacote do jogo')).toBeTruthy()
    m.unmount()
  })

  it('falha de avaliação no Performance Lab aparece e pode ser repetida', async () => {
    evaluationFailure = 'Histórico indisponível'
    const m = mount(<SessionPackResult meta={meta('s-new', 55)} />)
    await flush(80)
    expect(text()).toContain('Histórico indisponível')
    expect(text()).not.toContain('Regressão observada')
    evaluationFailure = null
    await click(button('Tentar avaliação de novo'))
    await flush(80)
    expect(text()).toContain('Indício de regressão')
    expect(calls.filter((c) => c.cmd === 'evaluate_benchmark_session')).toHaveLength(2)
    m.unmount()
  })
})
