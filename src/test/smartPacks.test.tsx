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
import { emit } from '@tauri-apps/api/event'
import { MotionGlobalConfig } from 'motion/react'
import { AppStoreProvider } from '@/state/store'
import { LibraryProvider } from '@/state/library'
import { FixReportsProvider } from '@/state/fixes'
import { FixEngineProvider } from '@/state/fixEngine'
import { PerformanceProvider } from '@/state/performance'
import { SmartPacksPanel } from '@/components/smart/SmartPacksPanel'
import { SessionHero } from '@/components/performance/SessionView'
import { SessionPackResult } from '@/components/smart/SessionPackResult'
import type { FixPlan } from '@/types/fixEngine'
import type { Measurements, PackEvaluation, RankEntry, SmartPack, SmartPackReport } from '@/types/smartPacks'
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
    evaluations: [],
    notes: [],
    ...over,
  }
}

/** Vencedor medido (sintético): 2 × 2 sessões, +20% no FPS médio. */
function measuredReport(): SmartPackReport {
  const vs = { packId: D3D, limited: false, notes: [], deltas: [{ metric: 'fpsAvg', label: 'FPS médio', deltaPct: 20, thresholdPct: 3, outcome: 'improvement' as const, limited: false, before: 100, after: 120, unit: 'FPS' }, { metric: 'fpsLow1', label: '1% low', deltaPct: 20, thresholdPct: 5, outcome: 'improvement' as const, limited: false, before: 50, after: 60, unit: 'FPS' }] }
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

/**
 * FIXTURE do cenário relatado pelo usuário (observação informal, não
 * benchmark registrado): baseline ~95 FPS, com PROTON_NO_D3D11 ~55 FPS.
 */
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
          case 'evaluate_benchmark_session':
            return regression({ sessionId: (args as { sessionId: string }).sessionId })
          case 'prepare_benchmark':
            return { state: 'ready', plan: null, hudAttached: false, hudVersion: null, device: null, controllable: false, logFile: null, frames: 0, recordedS: null, message: null, sessionId: null, interrupted: false, updatedAt: 'x' }
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
  it('Testar desempenho sem referência: guia a captura do baseline com as condições pré-preenchidas', async () => {
    const m = mount(<SmartPacksPanel appId="200260" />)
    await flush(80)
    expect(text()).toContain('Aguardando baseline')
    await click(button('Testar desempenho'))
    expect(text()).toContain('Testar desempenho · Desativar Direct3D 11')
    expect(text()).toContain('Ainda não há uma sessão de referência')
    await click(button('Capturar referência agora'))
    expect(text()).toContain('Teste de desempenho: Desativar Direct3D 11')
    const label = document.body.querySelector<HTMLInputElement>('input[value="Referência (sem correções do FRX)"], input[value^="Smart Pack"]')
    expect(label).toBeTruthy()
    expect(calls.some((c) => c.cmd === 'apply_fix_plan')).toBe(false)
    m.unmount()
  })

  it('pack ativo: captura concluída é analisada sozinha e a regressão aparece com valores e restauração', async () => {
    current = () => {
      const r = report({ currentPackId: D3D, measurements: [meas(BASE, 95, 1)] })
      r.packs = [pack(BASE, { state: 'ready' }), pack(D3D, { state: 'verified' })]
      return r
    }
    const m = mount(<SmartPacksPanel appId="200260" />)
    await flush(80)
    expect(text()).toContain('Pronto para teste')
    await click(button('Testar desempenho'))
    expect(text()).toContain('está ativo e conferido no disco')
    expect(button('Preparar captura')).toBeTruthy()
    // O núcleo avisa que a captura terminou (evento real do Performance Lab).
    await act(async () => {
      await emit('frx://capture', { state: 'completed', plan: { appId: '200260', gameName: 'Batman' }, hudAttached: true, hudVersion: null, device: null, controllable: true, logFile: null, frames: 3300, recordedS: 60, message: 'Sessão salva', sessionId: 's-new', interrupted: false, updatedAt: 'x' })
    })
    await flush(120)
    expect(calls.find((c) => c.cmd === 'evaluate_benchmark_session')!.args).toEqual({ sessionId: 's-new' })
    expect(text()).toContain('Indício de regressão — recomendado repetir o teste')
    expect(text()).toContain('Preliminar')
    expect(text()).toContain('95,0 FPS')
    expect(text()).toContain('55,0 FPS')
    expect(text()).toContain('−40,0 (−42,1%)')
    expect(text()).toContain('faltam 1 sessão(ões) do baseline')
    expect(calls.some((c) => c.cmd.startsWith('rollback'))).toBe(false)
    await click(button('Restaurar configuração anterior'))
    expect(calls.find((c) => c.cmd === 'plan_smart_pack_switch')!.args).toEqual({ appId: '200260', packId: BASE })
    m.unmount()
  })

  it('resultado do último teste e estado do card vêm das avaliações do núcleo', async () => {
    current = () => report({ evaluations: [regression()], measurements: [meas(BASE, 95, 1), meas(D3D, 55, 1)] })
    const m = mount(<SmartPacksPanel appId="200260" />)
    await flush(80)
    expect(text()).toContain('Último teste')
    expect(text()).toContain('Regressão observada · preliminar')
    m.unmount()
  })

  it('Performance Lab mostra a conclusão da sessão e oferece restaurar', async () => {
    const m = mount(<SessionPackResult meta={meta('s-new', 55)} />)
    await flush(80)
    expect(text()).toContain('Smart Pack · resultado')
    expect(text()).toContain('Indício de regressão — recomendado repetir o teste')
    await click(button('Restaurar configuração anterior'))
    expect(calls.find((c) => c.cmd === 'plan_smart_pack_switch')!.args).toEqual({ appId: '200260', packId: BASE })
    m.unmount()
  })
})
