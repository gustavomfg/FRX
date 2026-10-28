/**
 * Game-specific integration fixtures. Every identity, source and conclusion
 * below is synthetic. IPC is mocked; these tests never scan Steam or record
 * invented benchmarks as real Performance Lab sessions.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { act, useState, type ReactNode } from 'react'
import { createRoot } from 'react-dom/client'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { MotionGlobalConfig } from 'motion/react'
import { AppStoreProvider } from '@/state/store'
import { LibraryProvider } from '@/state/library'
import { FixReportsProvider } from '@/state/fixes'
import { FixEngineProvider } from '@/state/fixEngine'
import { PerformanceProvider } from '@/state/performance'
import { SmartPacksPanel } from '@/components/smart/SmartPacksPanel'
import { GameDetailPage } from '@/pages/GameDetail'
import { FindingCard } from '@/components/fix/FindingCard'
import { GameEvidence } from '@/components/fix/GameEvidence'
import { useAssessments } from '@/components/fix/useAssessments'
import type { Assessment, GamePerformanceEvidence } from '@/types/fixEngine'
import type { FixReport } from '@/types/fixes'
import type { RankEntry, SmartPack, SmartPackReport } from '@/types/smartPacks'
import { batman, finding, report, scan } from './fixtures'

;(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
MotionGlobalConfig.skipAnimations = true
const globals = globalThis as unknown as { isTauri: boolean; ResizeObserver: unknown }
globals.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
}

const A = '910001'
const B = '910002'
const SHARED = 'synthetic-shared-finding'
const TITLE = 'PROTON_NO_D3D11=1 · fixture sintética'
const WARNING = 'Risco técnico geral da fixture; impacto depende do AppID.'
const POSITIVE = 'Possível melhoria específica para este jogo — requer validação local'
const UNKNOWN = 'Impacto neste jogo desconhecido'
const sourceUrl = (appId: string) => `https://github.com/ValveSoftware/Proton/issues/${appId}`
const scope = (appId: string) => ({ kind: 'verified-game' as const, appId, basis: `Fixture: vínculo explícito ao AppID ${appId}` })
const evidence = (appId: string, positive = 0): GamePerformanceEvidence => ({
  appId, positive, negative: 0, authoritativePositive: false,
  sourceUrls: positive ? [`${sourceUrl(appId)}#issuecomment-1`, `${sourceUrl(appId)}#issuecomment-2`] : [],
  contextCompatible: true, limitations: ['Fixture: relatos públicos sintéticos; nenhum benchmark local.'],
})
const currentFinding = (appId: string) => finding({
  id: SHARED, gameAppId: appId, scope: scope(appId), title: TITLE,
  actions: [{ kind: 'env', label: 'Ação sintética', value: 'PROTON_NO_D3D11=1', conditional: false }],
  sources: [{ provider: 'github', label: `Origem sintética ${appId}`, url: sourceUrl(appId), referenceId: null, snippet: null, publishedAt: null }],
})
const discovery = (appId: string): FixReport => ({
  ...report, appId, gameName: `Jogo sintético ${appId}`, findings: [currentFinding(appId)], identity: { umuId: null, aliases: [] },
})
const assessment = (appId: string): Assessment => ({
  appId, findingId: SHARED, class: 'informational', status: 'no-actions', reasons: [`Análise exclusiva da fixture ${appId}`],
  evidence: 'none', evidenceNote: 'Fixture sem benchmark local.', experimental: false, actions: [], recipeId: null, recipe: null,
  provenance: [], interpreter: 'synthetic-fixture', generatedAt: '2026-10-27T10:00:00Z', updatedAt: '2026-10-27T10:00:00Z', discoveredAt: null, reevaluated: null,
  performanceEvidence: evidence(appId, appId === A ? 2 : 0), performanceWarnings: [WARNING],
})

function smartPack(appId: string, baseline = false): SmartPack {
  const recipeId = `gen.${appId}.synthetic`
  return {
    availability: { state: baseline ? 'applied' : 'available', reasons: [], checkedAt: '2026-10-27', planStatus: 'ready', planFingerprint: 'fixture-pack-plan' },
    id: `pack.${appId}.${baseline ? 'baseline' : 'synthetic'}`, appId, version: '1.0.0', name: baseline ? 'Baseline sintético' : TITLE,
    kind: baseline ? 'baseline' : 'single', goal: baseline ? 'reference' : 'compatibility', goalText: 'Fixture; não representa desempenho real.',
    members: baseline ? [] : [{ recipeId, version: '1.0.0', fingerprint: `fixture-${appId}`, trust: 'generated', title: TITLE, env: [{ key: 'PROTON_NO_D3D11', value: '1' }], evidence: 'corroborated', discoveredAt: null, protonVersions: [] }],
    env: baseline ? [] : [{ key: 'PROTON_NO_D3D11', value: '1' }], applyRecipeId: baseline ? null : recipeId, fingerprint: `synthetic-${appId}`,
    requirements: [], compatibility: [], conflictsAvoided: [], sources: baseline ? [] : [{ label: `Origem sintética ${appId}`, url: sourceUrl(appId) }],
    evidence: baseline ? 'none' : 'corroborated', evidenceNote: 'Relatos de fixture.', risk: 'moderate', effects: [], sideEffects: [], reversible: true,
    state: baseline ? 'verified' : 'ready', stateDetail: '', planStatus: baseline ? null : 'ready', operationId: null,
    performanceEvidence: evidence(appId, !baseline && appId === A ? 2 : 0), performanceWarnings: baseline ? [] : [WARNING],
  }
}
function smartReport(appId: string): SmartPackReport {
  const baseline = smartPack(appId, true)
  const pack = smartPack(appId)
  const rank: RankEntry = { packId: pack.id, title: pack.name, tier: appId === A ? 'candidate' : 'performance-risk', position: null, outcome: 'not-measured', headline: appId === A ? POSITIVE : 'Risco potencial — impacto neste jogo desconhecido', reasons: [`Justificativa exclusiva da fixture ${appId}`], vsBaseline: null }
  return {
    appId, gameName: `Jogo sintético ${appId}`, profile: 'balanced', generatedAt: '2026-10-27T10:00:00Z', composerVersion: 'fixture', rulesVersion: 'fixture',
    limits: { maxUnits: 8, maxPacks: 12, maxPackSize: 4 }, referenceSessionId: null, baselineId: baseline.id, packs: [baseline, pack], units: [], graph: [], excluded: [], measurements: [],
    ranking: { profile: 'balanced', version: 'fixture', entries: [rank], winner: null, summary: `Classificação sintética exclusiva ${appId}`, nextTest: null, criteria: [] },
    history: [], unassignedSessions: [], currentPackId: baseline.id, currentNote: '', windows: null, evaluations: [], notes: [],
  }
}
const syntheticScan = {
  ...scan, steamInstalls: [{ path: '/synthetic/Steam', kind: 'native' as const }],
  libraries: [A, B].map((appId) => ({ ...scan.libraries[1], path: `/synthetic/library-${appId}`, steamInstall: '/synthetic/Steam', declaredAppIds: [appId] })),
  games: [A, B].map((appId) => ({ ...batman, appId, name: `Jogo sintético ${appId}`, steamInstall: '/synthetic/Steam', installPath: `/synthetic/library-${appId}/game`, libraryPath: `/synthetic/library-${appId}` })),
}

function Providers({ children }: { children: ReactNode }) {
  return <AppStoreProvider><LibraryProvider><FixReportsProvider><FixEngineProvider><PerformanceProvider><MemoryRouter>{children}</MemoryRouter></PerformanceProvider></FixEngineProvider></FixReportsProvider></LibraryProvider></AppStoreProvider>
}
/** A página de um jogo, pela própria rota `/biblioteca/:gameId`, sem outro roteador. */
function GameRoute({ entry }: { entry: string }) {
  return <AppStoreProvider><LibraryProvider><FixReportsProvider><FixEngineProvider><PerformanceProvider><MemoryRouter initialEntries={[entry]}><Routes><Route path="/biblioteca/:gameId" element={<GameDetailPage />} /></Routes></MemoryRouter></PerformanceProvider></FixEngineProvider></FixReportsProvider></LibraryProvider></AppStoreProvider>
}
function mount(node: ReactNode) {
  const el = document.createElement('div')
  document.body.appendChild(el)
  const root = createRoot(el)
  act(() => root.render(<Providers>{node}</Providers>))
  return { unmount: () => act(() => root.unmount()) }
}
const flush = (ms = 35) => act(() => new Promise((resolve) => setTimeout(resolve, ms)))
const text = () => document.body.textContent ?? ''
const button = (label: string, parent: ParentNode = document.body) => [...parent.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === label)
const click = async (el: HTMLElement | undefined) => { expect(el).toBeDefined(); await act(async () => el!.click()); await flush() }
function Picker({ children }: { children: (appId: string) => ReactNode }) {
  const [appId, setAppId] = useState(A)
  return <><button onClick={() => setAppId(A)}>Abrir fixture A</button><button onClick={() => setAppId(B)}>Abrir fixture B</button>{children(appId)}</>
}
function AssessedFinding({ appId }: { appId: string }) {
  const result = useAssessments(appId)
  return <ul><FindingCard defaultOpen finding={currentFinding(appId)} assessment={result.byFinding[SHARED] ?? null} analyzing={result.loading} /></ul>
}
function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((r) => { resolve = r })
  return { promise, resolve }
}

describe('Game-specific fix intelligence — integração sintética', () => {
  let calls: { cmd: string; args: Record<string, unknown> }[]
  let analyze: (appId: string) => Assessment[] | Promise<Assessment[]>
  let cachedReports: FixReport[]
  beforeEach(() => {
    calls = []
    cachedReports = [discovery(A), discovery(B)]
    analyze = (appId) => [assessment(appId)]
    globals.isTauri = true
    mockIPC(async (cmd, raw) => {
      const args = (raw ?? {}) as Record<string, unknown>
      calls.push({ cmd, args })
      switch (cmd) {
        case 'get_installed_games': case 'scan_installed_games': return syntheticScan
        case 'get_system_profile': return { gpus: [], primaryGpu: null, vulkanDrivers: [], arch: 'x86_64' }
        case 'get_cached_fix_reports': return cachedReports
        case 'detected_symptoms': return []
        case 'get_fix_engine_status': return { catalogVersion: 'fixture', catalogSha256: 'fixture', recipes: 0, reviewed: 0, issues: [], recoveryRequired: 0, pendingManual: 0, busy: false, dataDir: '/synthetic/fix-engine' }
        case 'list_fix_operations': case 'get_applied_fixes': case 'list_benchmark_sessions': case 'list_fix_recipes': return []
        case 'detect_mangohud': return { installed: false, version: null, path: null, vulkanLayer: false, openglLib: false, notes: [] }
        case 'get_capture_status': return { state: 'idle' }
        case 'get_smart_packs': return { ...smartReport(String(args.appId)), profile: args.profile }
        case 'analyze_fix_findings': return analyze(String(args.appId))
        default: return null
      }
    }, { shouldMockEvents: true })
  })
  afterEach(() => {
    clearMocks()
    globals.isTauri = false
    document.body.innerHTML = ''
  })

  it('a mesma ação pode ser candidata no jogo A e risco desconhecido no B, sem carregar a origem ou o modal anterior', async () => {
    const mounted = mount(<Picker>{(appId) => <SmartPacksPanel appId={appId} />}</Picker>)
    await flush(90)
    const candidate = document.body.querySelector<HTMLElement>('section[aria-label="Candidatos para testar"]')!
    expect(candidate.textContent).toContain(TITLE)
    expect(candidate.textContent).toContain(POSITIVE)
    expect(candidate.textContent).toContain(`AppID ${A}`)
    expect(button('Testar desempenho', candidate)).toBeDefined()
    expect(text()).not.toMatch(/\d+[,.]\d+ FPS/)
    await click(button('Detalhes', candidate))
    const detail = document.body.querySelector('[role="dialog"]')!
    expect(detail.textContent).toContain(WARNING)
    expect(detail.textContent).toContain(`Origem sintética ${A}`)
    expect(detail.textContent).toContain('Nenhuma sessão com este pacote.')

    await click(button('Abrir fixture B'))
    await flush(60)
    expect(document.body.querySelector('[role="dialog"]')).toBeNull()
    const risks = document.body.querySelector<HTMLElement>('section[aria-label="Risco de perda de desempenho"]')!
    expect(risks.textContent).toContain(TITLE)
    expect(risks.textContent).toContain(UNKNOWN)
    expect(risks.textContent).toContain(`AppID ${B}`)
    expect(button('Testar desempenho', risks)).toBeUndefined()
    expect(button('Revisar compatibilidade', risks)).toBeDefined()
    expect(text()).not.toContain(POSITIVE)
    expect(text()).not.toContain(`AppID ${A}`)
    expect(text()).not.toContain(`Origem sintética ${A}`)
    await click(button('Detalhes', risks))
    expect(document.body.querySelector('[role="dialog"]')!.textContent).toContain(WARNING)
    expect(text()).toContain(`Origem sintética ${B}`)
    expect(calls.filter((c) => c.cmd === 'get_smart_packs').map((c) => c.args.appId)).toEqual([A, B])
    expect(calls.some((c) => ['prepare_fix_plan', 'apply_fix_plan', 'prepare_benchmark', 'start_capture'].includes(c.cmd))).toBe(false)
    mounted.unmount()
  })

  it('uma análise atrasada do jogo A não substitui a do B mesmo com findingId idêntico', async () => {
    const pendingA = deferred<Assessment[]>()
    analyze = (appId) => appId === A ? pendingA.promise : [assessment(B), assessment(A)]
    const mounted = mount(<Picker>{(appId) => <AssessedFinding appId={appId} />}</Picker>)
    await flush(90)
    expect(calls.some((c) => c.cmd === 'analyze_fix_findings' && c.args.appId === A)).toBe(true)
    await click(button('Abrir fixture B'))
    expect(text()).toContain(`Análise exclusiva da fixture ${B}`)
    expect(text()).not.toContain(`Análise exclusiva da fixture ${A}`)
    await act(async () => pendingA.resolve([assessment(A)]))
    await flush()
    expect(text()).toContain(`Análise exclusiva da fixture ${B}`)
    expect(text()).not.toContain(`Análise exclusiva da fixture ${A}`)
    expect(text()).not.toContain(POSITIVE)
    mounted.unmount()
  })

  it('oculta a análise já carregada do A enquanto a resposta do B ainda está pendente', async () => {
    const pendingB = deferred<Assessment[]>()
    analyze = (appId) => appId === B ? pendingB.promise : [assessment(A)]
    const mounted = mount(<Picker>{(appId) => <AssessedFinding appId={appId} />}</Picker>)
    await flush(90)
    expect(text()).toContain(`Análise exclusiva da fixture ${A}`)
    expect(text()).toContain(POSITIVE)
    await click(button('Abrir fixture B'))
    expect(text()).not.toContain(`Análise exclusiva da fixture ${A}`)
    expect(text()).not.toContain(POSITIVE)
    await act(async () => pendingB.resolve([assessment(B)]))
    await flush()
    expect(text()).toContain(`Análise exclusiva da fixture ${B}`)
    expect(text()).toContain(UNKNOWN)
    mounted.unmount()
  })

  it('a página de um jogo mostra só as correções dele, e Smart Packs e pesquisa técnica abrem nessa mesma página', async () => {
    cachedReports = [A, B].map((appId) => ({ ...discovery(appId), findings: [{ ...currentFinding(appId), id: `fixture-${appId}-d3d11` }] }))
    analyze = (appId) => [{ ...assessment(appId), findingId: `fixture-${appId}-d3d11` }]
    const runtimeErrors: unknown[] = []
    const element = document.createElement('div')
    document.body.appendChild(element)
    const root = createRoot(element, { onUncaughtError: (error) => runtimeErrors.push(error) })
    act(() => root.render(<GameRoute entry={`/biblioteca/${A}`} />))
    await flush(120)
    expect(runtimeErrors).toEqual([])
    expect(text()).toContain(`Jogo sintético ${A}`)
    expect(text()).not.toContain(`Jogo sintético ${B}`)
    expect(document.querySelector('[aria-label="Correções disponíveis"]')).not.toBeNull()
    expect(calls.some((c) => c.cmd === 'analyze_fix_findings')).toBe(true)
    await click(button('Smart Packs'))
    await flush(60)
    expect(runtimeErrors).toEqual([])
    expect(text()).toContain(`Classificação sintética exclusiva ${A}`)
    expect(text()).toContain(POSITIVE)
    expect(text()).not.toContain(`Classificação sintética exclusiva ${B}`)
    await click(button('Visão do jogo'))
    await flush(40)
    await click(button('Pesquisa técnica'))
    await flush(60)
    expect(runtimeErrors).toEqual([])
    expect(document.querySelector('[aria-label="Descobertas"]')).not.toBeNull()
    expect(text()).not.toContain('Todos os jogos')
    act(() => root.unmount())
  })

  it('não transforma dados legados sem atribuição confirmada nem evidência de outro AppID em melhoria deste jogo', async () => {
    const mounted = mount(<>
      <GameEvidence appId={B} scope={{ kind: 'unknown-scope', appId: null, basis: 'Fixture legada sem AppID verificável.' }} evidence={evidence(B, 2)} />
      <GameEvidence appId={B} scope={scope(B)} evidence={evidence(A, 2)} />
    </>)
    await flush()
    expect(text()).toContain('Atribuição ao jogo não confirmada')
    expect(text()).toContain('Esta origem não autoriza gerar uma correção específica')
    expect(text()).toContain(UNKNOWN)
    expect(text()).not.toContain(POSITIVE)
    expect(text()).not.toContain('Relatos públicos: 2')
    mounted.unmount()
  })
})
