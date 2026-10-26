/**
 * Smart Packs na interface. Os relatórios aqui são **fixtures sintéticas**
 * no formato exato do Rust; os números são escolhidos para o teste, não
 * medidos.
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
import { SmartPacksPanel } from '@/components/smart/SmartPacksPanel'
import { SessionHero } from '@/components/performance/SessionView'
import type { FixPlan } from '@/types/fixEngine'
import type { Measurements, RankEntry, SmartPack, SmartPackReport } from '@/types/smartPacks'
import { scan } from './fixtures'
import { meta } from './perfFixtures'

;(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
MotionGlobalConfig.skipAnimations = true
const g = globalThis as unknown as { isTauri: boolean; ResizeObserver: unknown }
g.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
}

const BASE = 'pack.200260.0000000000000000'
const D3D = 'pack.200260.1111111111111111'
const COMBO = 'pack.200260.2222222222222222'

const pack = (id: string, over: Partial<SmartPack> = {}): SmartPack => ({
  id,
  appId: '200260',
  version: '1.0.0',
  name: id === BASE ? 'Baseline — sem correções do FRX' : id === D3D ? 'Desativar Direct3D 11' : 'Pack de compatibilidade',
  kind: id === BASE ? 'baseline' : id === D3D ? 'single' : 'combination',
  goal: id === BASE ? 'reference' : 'compatibility',
  goalText: 'objetivo',
  members: id === BASE ? [] : [{ recipeId: 'gen.200260.aaaaaaaaaaaaaaaa', version: '1.0.0', fingerprint: 'f'.repeat(64), trust: 'generated', title: 'Desativar Direct3D 11', env: [{ key: 'PROTON_NO_D3D11', value: '1' }], evidence: 'single', discoveredAt: '2018-11-01T18:09:34Z', protonVersions: ['Proton 3.16'] }],
  env: id === BASE ? [] : [{ key: 'PROTON_NO_D3D11', value: '1' }],
  applyRecipeId: id === BASE ? null : id === D3D ? 'gen.200260.aaaaaaaaaaaaaaaa' : COMBO,
  fingerprint: 'abcdef0123456789',
  requirements: ['Jogo rodando com Proton.'],
  compatibility: ['“Desativar Direct3D 11” foi relatada com Proton 3.16 em 2018-11-01.'],
  conflictsAvoided: [],
  sources: [{ label: 'Comentário em ValveSoftware/Proton#309', url: 'https://github.com/ValveSoftware/Proton/issues/309#issuecomment-435132818' }],
  evidence: id === BASE ? 'none' : 'single',
  evidenceNote: 'Um único relato.',
  risk: 'moderate',
  effects: ['O d3d11.dll fica indisponível.'],
  sideEffects: ['Se o jogo não tiver modo Direct3D 9, ele deixa de abrir.'],
  reversible: true,
  state: id === BASE ? 'verified' : 'ready',
  stateDetail: '',
  planStatus: id === BASE ? null : 'ready',
  operationId: null,
  ...over,
})

const entry = (packId: string, over: Partial<RankEntry>): RankEntry => ({ packId, title: pack(packId).name, tier: 'candidate', position: null, outcome: 'not-measured', headline: 'Experimental — ainda não testado', reasons: ['Sem benchmark com este pacote.'], vsBaseline: null, ...over })

const meas = (packId: string, avg: number, n = 2): Measurements => ({
  packId,
  sessions: Array.from({ length: n }, (_, i) => ({ id: `${packId}-${i}`, label: `Sessão ${i}`, capturedAt: '2026-10-26T10:00:00Z', source: 'assisted', verified: true, durationS: 60, avgFps: avg, low1: avg / 2, excluded: null, notes: [] })),
  avg: { n, median: avg, min: avg, max: avg, spreadPct: n > 1 ? 0 : null },
  low1: { n, median: avg / 2, min: avg / 2, max: avg / 2, spreadPct: n > 1 ? 0 : null },
  low01: null,
  ftP99: null,
  spikesPerMin: null,
})

function report(over: Partial<SmartPackReport> = {}): SmartPackReport {
  return {
    appId: '200260',
    gameName: 'Batman: Arkham City GOTY',
    profile: 'balanced',
    generatedAt: '2026-10-26T10:00:00Z',
    composerVersion: '1.0.0',
    rulesVersion: '2026.10.26-1',
    limits: { maxUnits: 8, maxPacks: 12, maxPackSize: 4 },
    referenceSessionId: null,
    baselineId: BASE,
    packs: [pack(BASE), pack(D3D)],
    units: [],
    graph: [],
    excluded: [],
    measurements: [],
    ranking: {
      profile: 'balanced',
      version: '1.0.0',
      entries: [entry(BASE, { headline: 'Referência ainda não medida' }), entry(D3D, {})],
      winner: null,
      summary: 'Ainda não há dados suficientes para recomendar um vencedor.',
      nextTest: { packId: BASE, reason: 'Meça o baseline primeiro.' },
      criteria: ['Nenhuma regressão em FPS médio, 1% low ou frametime P99.'],
    },
    history: [{ at: '2026-10-26T10:00:00Z', profile: 'balanced', version: '1.0.0', reason: 'Primeira classificação.', entries: [], sessions: 0 }],
    unassignedSessions: [],
    currentPackId: BASE,
    currentNote: 'Baseline ativo.',
    windows: null,
    notes: [],
    ...over,
  }
}

/** Vencedor medido (sintético): 2 × 2 sessões, +20% no FPS médio. */
function measuredReport(): SmartPackReport {
  const vs = { packId: D3D, limited: false, notes: [], deltas: [{ metric: 'fpsAvg', label: 'FPS médio', deltaPct: 20, thresholdPct: 3, outcome: 'improvement' as const, limited: false }, { metric: 'fpsLow1', label: '1% low', deltaPct: 20, thresholdPct: 5, outcome: 'improvement' as const, limited: false }] }
  const r = report({
    packs: [pack(BASE), pack(D3D), pack(COMBO, { state: 'ready' })],
    measurements: [meas(BASE, 100), meas(D3D, 120)],
  })
  r.ranking = {
    ...r.ranking,
    entries: [
      entry(BASE, { tier: 'measured', outcome: 'no-difference', headline: 'Referência medida (2 sessões)' }),
      entry(D3D, { tier: 'measured', position: 1, outcome: 'improvement', headline: 'Melhor que o baseline nas medições', reasons: ['Melhorou ao menos uma métrica sem piorar FPS médio, 1% low ou P99.', 'FPS médio: +20,0% (melhora; limiar ±3,0%).'], vsBaseline: vs }),
      entry(COMBO, { headline: 'Candidato promissor — ainda não testado' }),
      { packId: 'github-200260-a6a11c0dab', title: 'Opção de inicialização sugerida', tier: 'not-recommended', position: null, outcome: 'not-measured', headline: 'Fora dos pacotes', reasons: ['O FRX não conhece o efeito de PROTON_USE_WINED3D11.'], vsBaseline: null },
    ],
    winner: D3D,
    summary: '#1 — Melhor equilíbrio medido para o perfil escolhido: Desativar Direct3D 11.',
    nextTest: null,
  }
  return r
}

const plan = { planId: 'p', fingerprint: 'fp', recipeId: 'gen.200260.aaaaaaaaaaaaaaaa', recipeVersion: '1.0.0', recipeTitle: 'Desativar Direct3D 11', appId: '200260', gameName: 'Batman', status: 'ready', mode: 'automatic', reasons: [], checks: [], changes: [], manualLine: null, backupNote: null, steamRunning: false, gameRunning: false, risk: 'moderate', purpose: 'compatibility', trust: 'generated', effects: [], sideEffects: [], sources: [], generated: null, pack: null, createdAt: 'x', expiresAt: 9999999999 } satisfies FixPlan

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

describe('Smart Packs na interface', () => {
  let calls: { cmd: string; args: Record<string, unknown> }[] = []
  let current: () => SmartPackReport = () => report()

  beforeEach(() => {
    calls = []
    current = () => report()
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
          case 'detect_mangohud':
            return { installed: true, version: '0.8', path: '/usr/bin/mangohud', vulkanLayer: true, openglLib: true, notes: [] }
          case 'get_capture_status':
            return { phase: 'idle' }
          case 'get_smart_packs':
            return current()
          case 'prepare_fix_plan':
            return plan
          case 'plan_smart_pack_switch':
            return { to: BASE, from: D3D, keep: [], remove: [{ key: 'PROTON_NO_D3D11', value: '1' }], add: [], steps: [{ kind: 'rollback', operationId: 'op-1', recipeId: null, title: 'Desfazer “Desativar Direct3D 11”', detail: 'Remove só as variáveis que esta operação adicionou.' }], possible: true, notes: ['Cada passo abre a própria prévia e pede confirmação.'] }
          case 'compare_smart_packs':
            return { a: BASE, b: D3D, deltas: [{ metric: 'fpsAvg', label: 'FPS médio', deltaPct: 20, thresholdPct: 3, outcome: 'improvement', limited: false }], limited: false, summary: 'B teve FPS médio maior que A além da variação medida.', notes: [] }
          case 'declare_benchmark_pack':
            return meta('old', 100)
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

  it('sem evidência: nenhum vencedor, candidatos em ordem de teste e referência Windows indisponível', async () => {
    const m = mount(<SmartPacksPanel appId="200260" />)
    await flush(80)
    expect(text()).toContain('Ainda não há dados suficientes para recomendar um vencedor.')
    expect(text()).toContain('Experimental — ainda não testado')
    expect(text()).toContain('Ordem sugerida de teste — não é ordem de desempenho.')
    expect(text()).not.toMatch(/#1/)
    expect(text()).toContain('Referência Windows indisponível')
    expect(text()).toContain('Próximo teste: Meça o baseline primeiro.')
    // Valores medidos só aparecem quando existem.
    expect(text()).not.toMatch(/\d+,\d FPS/)
    const req = calls.find((c) => c.cmd === 'get_smart_packs')!
    expect(req.args).toEqual({ appId: '200260', profile: 'balanced' })
    m.unmount()
  })

  it('troca de perfil recalcula no núcleo e o filtro mostra só o ranking medido', async () => {
    current = measuredReport
    const m = mount(<SmartPacksPanel appId="200260" />)
    await flush(80)
    await click(document.body.querySelector<HTMLElement>('[role="radio"][aria-checked="false"]') ?? button('Máximo desempenho'))
    await flush(60)
    expect(calls.filter((c) => c.cmd === 'get_smart_packs').map((c) => c.args.profile)).toContain('performance')
    await click(button('Ranking medido'))
    expect(text()).not.toContain('Candidato promissor')
    expect(text()).toContain('Desativar Direct3D 11')
    m.unmount()
  })

  it('vencedor medido: destaque, números reais do relatório, detalhes e aplicação pela prévia', async () => {
    current = measuredReport
    const m = mount(<SmartPacksPanel appId="200260" />)
    await flush(80)
    expect(text()).toContain('#1 — Melhor resultado medido')
    expect(text()).toContain('120,0')
    expect(text()).toContain('+20,0%')
    expect(text()).toContain('Fora dos pacotes')
    const card = document.body.querySelector('li[aria-label="Desativar Direct3D 11"]')!
    await click([...card.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === 'Detalhes'))
    expect(text()).toContain('Por que esta classificação')
    expect(text()).toContain('FPS médio: +20,0% (melhora; limiar ±3,0%).')
    await click(button('Fechar'))
    await click(button('Aplicar'))
    await flush(60)
    const prep = calls.find((c) => c.cmd === 'prepare_fix_plan')!
    expect(prep.args).toEqual({ recipeId: 'gen.200260.aaaaaaaaaaaaaaaa', appId: '200260' })
    expect(calls.some((c) => c.cmd === 'apply_fix_plan')).toBe(false)
    m.unmount()
  })

  it('comparação entre pacotes e restauração passo a passo', async () => {
    current = () => ({ ...measuredReport(), currentPackId: D3D })
    const m = mount(<SmartPacksPanel appId="200260" />)
    await flush(80)
    await click(button('Comparar'))
    expect(calls.find((c) => c.cmd === 'compare_smart_packs')!.args).toEqual({ appId: '200260', a: BASE, b: D3D })
    expect(text()).toContain('B teve FPS médio maior que A além da variação medida.')
    await click(button('Restaurar estado anterior'))
    expect(calls.find((c) => c.cmd === 'plan_smart_pack_switch')!.args).toEqual({ appId: '200260', packId: BASE })
    expect(text()).toContain('Desfazer “Desativar Direct3D 11”')
    expect(text()).toContain('Cada passo abre a própria prévia')
    m.unmount()
  })

  it('sessões sem registro podem ser declaradas e erros do núcleo aparecem', async () => {
    current = () => report({ unassignedSessions: [{ id: 's-old', label: 'Sessão antiga', capturedAt: null, source: 'declared', verified: false, durationS: 60, avgFps: 118.2, low1: 21.5, excluded: 'Configuração do FRX não registrada.', notes: [] }] })
    const m = mount(<SmartPacksPanel appId="200260" />)
    await flush(80)
    await click(button('Declarar'))
    expect(calls.find((c) => c.cmd === 'declare_benchmark_pack')!.args).toEqual({ sessionId: 's-old', packId: BASE })
    m.unmount()

    current = () => {
      throw 'Jogo não encontrado na biblioteca da Steam.'
    }
    const e = mount(<SmartPacksPanel appId="200260" />)
    await flush(80)
    expect(text()).toContain('Não foi possível montar os Smart Packs')
    expect(text()).toContain('Jogo não encontrado')
    e.unmount()
  })

  it('modo demonstrativo não consulta o núcleo', async () => {
    g.isTauri = false
    const m = mount(<SmartPacksPanel appId="200260" />)
    await flush(60)
    expect(calls.some((c) => c.cmd === 'get_smart_packs')).toBe(false)
    m.unmount()
  })

  it('Performance Lab mostra a configuração Smart Pack da sessão', () => {
    const s = meta('a', 100, { smartPack: { packId: D3D, env: [{ key: 'PROTON_NO_D3D11', value: '1' }], recipes: [], operationIds: ['op-1'], verified: true, source: 'assisted', recordedAt: 'x' } })
    const m = mount(<SessionHero meta={s} />)
    expect(text()).toContain('PROTON_NO_D3D11=1 · conferido')
    m.unmount()
    const old = mount(<SessionHero meta={meta('b', 100)} />)
    expect(text()).toContain('Não registrado')
    old.unmount()
  })
})
