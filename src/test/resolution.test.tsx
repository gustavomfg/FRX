/** Synthetic IPC fixtures only. No real game, diagnostic, download or benchmark. */
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
import { ResolutionCenter } from '@/components/resolution/ResolutionCenter'
import type { ResolutionEvidence, ResolutionPlan, ResolutionReport } from '@/types/resolution'
import type { FixPlan, Operation } from '@/types/fixEngine'
import { scan } from './fixtures'

;(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
MotionGlobalConfig.skipAnimations = true
const g = globalThis as unknown as { isTauri: boolean }
const at = '2026-10-27T12:00:00Z'
const evidence: ResolutionEvidence = { id: 'synthetic-source', kind: 'official-documentation', summary: 'Fixture: documentação explica o diagnóstico, não confirma a causa.', sourceUrl: 'https://github.com/ValveSoftware/Proton', publisher: 'Fixture publisher', publishedAt: '2025-01-01T12:00:00Z', updatedAt: at, fetchedAt: at, version: 'fixture-v1', appId: null, technology: 'graphics', stale: false, limitations: ['Fixture: documento geral não prova desempenho.'] }
const resolutionPlan: ResolutionPlan = { availability: { state: 'available', reasons: [], checkedAt: at, planStatus: 'ready', planFingerprint: 'fixture-plan' }, id: 'resolution.200260.fixture', appId: '200260', title: 'Preparar diagnóstico sintético', objective: 'Registrar o sintoma para investigar; não corrige o crash.', problemId: 'fixture-problem', recipeId: 'frx.diagnostics.proton-log', state: 'applicable', actions: ['Definir PROTON_LOG=1'], dependencies: ['Proton confirmado na fixture'], risks: ['Log pode conter dados pessoais.'], sources: [evidence], reversible: true, verificationMethod: 'Conferir configuração; importar log autorizado.', successCriterion: 'Configuração persistida não prova funcionamento.', operationId: null, verification: 'Funcionamento ainda não verificado.', limitations: [] }
const report = (appId = '200260', over: Partial<ResolutionReport> = {}): ResolutionReport => ({ schema: 1, appId, gameName: `Synthetic Game ${appId}`, generatedAt: at, contextFingerprint: `fixture-${appId}`, symptoms: ['startup'], environment: null, technologies: [{ id: 'dlss', name: 'DLSS', hardware: 'supported', gameSupport: 'detected', environment: 'unknown', configured: 'unknown', runtime: 'unknown', evidence: [evidence], limitations: ['A presença da DLL não prova que o jogo implementa o recurso.'] }, { id: 'mfg', name: 'Multi Frame Generation', hardware: 'unsupported', gameSupport: 'unknown', environment: 'unknown', configured: 'unknown', runtime: 'unknown', evidence: [], limitations: ['Fixture RTX 4060 não atende ao requisito RTX 50.'] }], problems: [{ id: 'fixture-problem', technology: 'stability', symptom: `Crash informado no jogo ${appId}`, certainty: 'user-reported', evidence: [evidence], hypotheses: ['Dependência ou driver ainda não confirmados.'], ruledOut: [], limitations: ['Sem observação runtime.'], nextAction: 'Preparar log com confirmação.', planIds: [resolutionPlan.id] }], plans: [{ ...resolutionPlan, appId }], packs: [], knowledge: [evidence], limitations: ['Fixture isolada; nenhum dado desta suíte representa diagnóstico real.'], ...over })
const fixPlan: FixPlan = { planId: 'fixture-plan', fingerprint: 'fixture-fp', recipeId: resolutionPlan.recipeId!, recipeVersion: '1.0.0', recipeTitle: resolutionPlan.title, appId: '200260', gameName: 'Synthetic Game', status: 'ready', mode: 'automatic', reasons: [], checks: [], changes: [{ kind: 'launch-options', target: 'Synthetic LaunchOptions', before: '%command%', after: 'PROTON_LOG=1 %command%', summary: 'Adiciona apenas o diagnóstico' }], manualLine: null, backupNote: 'Backup isolado verificado.', steamRunning: false, gameRunning: false, risk: 'low', purpose: 'diagnostic', trust: 'reviewed', effects: [], sideEffects: [], sources: [], generated: null, pack: null, createdAt: at, expiresAt: 9999999999 }
const operation: Operation = { schema: 1, id: 'fixture-operation', recipeId: resolutionPlan.recipeId!, recipeVersion: '1.0.0', recipeTitle: resolutionPlan.title, appId: '200260', gameName: 'Synthetic Game', state: 'applied', mode: 'automatic', steps: [], planFingerprint: 'fixture-fp', createdAt: at, updatedAt: at, appliedAt: at, rolledBackAt: null, message: 'Configuração registrada na fixture.', pending: null, events: [], recipeTrust: 'reviewed', findingIds: [] }

const flush = () => act(() => new Promise((resolve) => setTimeout(resolve, 30)))
const text = () => document.body.textContent ?? ''
const button = (label: string) => [...(document.querySelector('[role=dialog]') ?? document.body).querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === label)
const click = async (target: HTMLElement | undefined | null) => { expect(target).toBeTruthy(); await act(async () => target!.click()); await flush() }
function mount(node: ReactNode) {
  const el = document.createElement('div'); document.body.appendChild(el)
  const root = createRoot(el)
  const render = (child: ReactNode) => act(() => root.render(<AppStoreProvider><LibraryProvider><FixReportsProvider><FixEngineProvider><MemoryRouter>{child}</MemoryRouter></FixEngineProvider></FixReportsProvider></LibraryProvider></AppStoreProvider>))
  render(node)
  return { render, unmount: () => act(() => root.unmount()) }
}

describe('Resolution Center — IPC e estados honestos', () => {
  let calls: { cmd: string; args: Record<string, unknown> }[]
  let stored: ResolutionReport | null
  let failure: string | null
  let selectedLog: string | null
  let delayFirst: boolean
  let finishDelayed: ((value: ResolutionReport) => void) | undefined
  beforeEach(() => {
    calls = []; stored = report(); failure = null; selectedLog = null; delayFirst = false; finishDelayed = undefined
    localStorage.clear(); g.isTauri = true
    mockIPC(async (cmd, raw) => {
      const args = (raw ?? {}) as Record<string, unknown>
      calls.push({ cmd, args })
      switch (cmd) {
        case 'get_installed_games': case 'scan_installed_games': return scan
        case 'get_system_profile': return { gpus: [], primaryGpu: null, vulkanDrivers: [], arch: 'x86_64' }
        case 'get_cached_fix_reports': case 'get_applied_fixes': case 'list_fix_operations': return []
        case 'detected_symptoms': return []
        case 'get_fix_engine_status': return { catalogVersion: 'fixture', recipes: 1, reviewed: 1, issues: [], recoveryRequired: 0, pendingManual: 0, busy: false, dataDir: '/synthetic' }
        case 'get_resolution_report':
          if (delayFirst && args.appId === '200260') return new Promise<ResolutionReport>((resolve) => { finishDelayed = resolve })
          return args.appId === '200260' ? stored : report(args.appId as string)
        case 'diagnose_game': case 'research_game_resolutions':
          if (failure) throw failure
          stored = report(args.appId as string, { symptoms: args.symptoms as string[], ...(stored?.plans[0]?.operationId ? { plans: stored.plans } : {}) }); return stored
        case 'plugin:dialog|open': return selectedLog
        case 'import_resolution_log': return report(args.appId as string, { limitations: ['Log sintético importado; ainda não comprova recurso funcional.'] })
        case 'prepare_fix_plan': return fixPlan
        case 'apply_fix_plan': stored = report('200260', { plans: [{ ...resolutionPlan, state: 'applied-unverified', operationId: operation.id }] }); return operation
        case 'get_fix_operation': return operation
        case 'verify_resolution_plan': stored = report('200260', { plans: [{ ...resolutionPlan, state: 'verified-configuration', operationId: operation.id, verification: 'Configuração conferida. Funcionamento ainda não verificado.' }] }); return stored
        case 'prepare_fix_rollback': return { operationId: operation.id, fingerprint: 'fixture-rollback', status: 'ready', mode: 'automatic', reasons: [], changes: [{ kind: 'launch-options', target: 'Synthetic LaunchOptions', before: 'PROTON_LOG=1 %command%', after: '%command%', summary: 'Remover apenas variável adicionada na fixture.' }], manualLine: null, steamRunning: false, gameRunning: false }
        case 'rollback_fix': stored = report('200260', { plans: [{ ...resolutionPlan, state: 'rolled-back', operationId: operation.id }] }); return { ...operation, state: 'rolled-back' }
        default: return null
      }
    }, { shouldMockEvents: true })
  })
  afterEach(() => { clearMocks(); g.isTauri = false; document.body.innerHTML = '' })

  it('distingue suporte, indício estático, runtime desconhecido e limitação de hardware', async () => {
    const m = mount(<ResolutionCenter appId="200260" available />); await flush()
    expect(text()).toContain('Indício estático'); expect(text()).toContain('Desconhecido'); expect(text()).toContain('Não suportado')
    expect(text()).toContain('RTX 4060 não atende'); expect(text()).toContain('Publicada:'); expect(text()).toContain('Atualizada:')
    expect(text()).toContain('Documentação oficial'); expect(text()).toContain('Contexto técnico geral')
    expect(calls.some((c) => c.cmd === 'diagnose_game' || c.cmd === 'apply_fix_plan')).toBe(false)
    m.unmount()
  })

  it('preserva sintomas carregados e pesquisa somente após ação explícita', async () => {
    const m = mount(<ResolutionCenter appId="200260" available />); await flush()
    expect(document.body.querySelector<HTMLInputElement>('input[type=checkbox]')?.checked).toBe(true)
    await click(button('Pesquisar soluções'))
    expect(calls.find((c) => c.cmd === 'research_game_resolutions')?.args).toEqual({ appId: '200260', symptoms: ['startup'] })
    m.unmount()
  })

  it('ignora resposta atrasada após trocar jogo e não transporta seu plano', async () => {
    delayFirst = true
    const m = mount(<ResolutionCenter appId="200260" available />); await flush()
    m.render(<ResolutionCenter appId="123" available />); await flush()
    await act(async () => finishDelayed!(report())); await flush()
    expect(text()).toContain('Crash informado no jogo 123'); expect(text()).not.toContain('Crash informado no jogo 200260')
    m.unmount()
  })

  it('recusa relatório que retorna outro AppID', async () => {
    stored = report('999')
    const m = mount(<ResolutionCenter appId="200260" available />); await flush()
    expect(text()).toContain('diagnóstico retornou outro AppID'); expect(button('Aplicar correção')).toBeUndefined()
    m.unmount()
  })

  it('mantém diagnóstico salvo quando pesquisa falha e permite nova tentativa', async () => {
    failure = 'Fonte indisponível: limite de consultas.'
    const m = mount(<ResolutionCenter appId="200260" available />); await flush(); await click(button('Pesquisar soluções'))
    expect(document.body.querySelector('[role=alert]')?.textContent).toContain('limite de consultas')
    expect(text()).toContain('Crash informado no jogo 200260')
    expect(button('Aplicar correção')).toBeUndefined()
    failure = null; await click(button('Diagnosticar')); expect(document.body.querySelector('[role=alert]')).toBeNull()
    m.unmount()
  })

  it('mantém solução em revisão sem prévia e bloqueia URL hostil', async () => {
    stored = report('200260', { plans: [{ ...resolutionPlan, state: 'review', availability: { state: 'investigation', reasons: [{ code: 'binary-review', message: 'DLL desconhecida sem procedência.' }], checkedAt: at, planStatus: null, planFingerprint: null }, recipeId: null, risks: ['DLL desconhecida sem procedência.'], sources: [{ ...evidence, sourceUrl: 'javascript:alert(1)' }] }] })
    const m = mount(<ResolutionCenter appId="200260" available />); await flush()
    expect(text()).toContain('Requer revisão'); expect(text()).toContain('DLL desconhecida'); expect(text()).toContain('abertura bloqueada')
    expect(button('Aplicar correção')).toBeUndefined(); expect(calls.some((c) => c.cmd === 'prepare_fix_plan')).toBe(false)
    m.unmount()
  })

  it('plano legado sem disponibilidade validada fica na investigação, mesmo com receita e estado antigo aplicável', async () => {
    stored = report('200260', { plans: [{ ...resolutionPlan, availability: undefined }] })
    const m = mount(<ResolutionCenter appId="200260" available />); await flush()
    expect(document.querySelector('[aria-label="Soluções aplicáveis"]')?.textContent).toContain('Nenhuma correção automática disponível')
    expect(text()).toContain('Descobertas em investigação (1)')
    expect(text()).not.toContain('Prévia disponível')
    expect(button('Aplicar correção')).toBeUndefined()
    m.unmount()
  })

  it('rollback antigo não permite reaplicar plano após falha de revalidação', async () => {
    stored = report('200260', { plans: [{ ...resolutionPlan, state: 'rolled-back', operationId: operation.id }] })
    const m = mount(<ResolutionCenter appId="200260" available />); await flush()
    expect(button('Aplicar correção')).toBeDefined()
    failure = 'Fixture: ambiente indisponível para revalidar.'
    await click(button('Pesquisar soluções'))
    expect(button('Aplicar correção')).toBeUndefined()
    expect(text()).toContain('Alteração desfeita')
    selectedLog = null
    await click(button('Importar log deste jogo'))
    expect(button('Aplicar correção')).toBeUndefined()
    failure = null; await click(button('Diagnosticar'))
    expect(button('Aplicar correção')).toBeDefined()
    m.unmount()
  })

  it('separa condição temporária de procedimentos em investigação sem perder o problema identificado', async () => {
    stored = report('200260', { plans: [{ ...resolutionPlan, state: 'review', availability: { state: 'temporary', checkedAt: at, planStatus: 'requires-manual-step', planFingerprint: null, reasons: [{ code: 'steam-running', message: 'Feche a Steam normalmente para permitir esta alteração.' }] } }] })
    const m = mount(<ResolutionCenter appId="200260" available />); await flush()
    expect(text()).toContain('Problemas identificados')
    expect(text()).toContain('Aguardando condição local')
    expect(text()).toContain('Feche a Steam normalmente')
    expect(text()).toContain('Descobertas em investigação (0)')
    expect(button('Aplicar correção')).toBeUndefined()
    await click(button('Revalidar condições'))
    expect(calls.some((c) => c.cmd === 'diagnose_game')).toBe(true)
    m.unmount()
  })

  it('abre prévia, exige confirmação existente e confere configuração sem anunciar recurso resolvido', async () => {
    const m = mount(<ResolutionCenter appId="200260" available />); await flush(); await click(button('Aplicar correção'))
    expect(calls.find((c) => c.cmd === 'prepare_fix_plan')?.args).toEqual({ recipeId: resolutionPlan.recipeId, appId: '200260', expectedFingerprint: 'fixture-plan' })
    expect(calls.some((c) => c.cmd === 'apply_fix_plan')).toBe(false)
    await click(button('Continuar'))
    expect(button('Aplicar correção')?.disabled).toBe(true)
    await click(document.body.querySelector<HTMLElement>('[role=dialog] input[type=checkbox]'))
    await click(button('Aplicar correção'))
    await click(document.body.querySelector<HTMLElement>('[role=dialog] button[aria-label="Fechar"]'))
    expect(text()).toContain('Aplicada — funcionamento ainda não verificado')
    await click(button('Verificar alteração'))
    expect(calls.find((c) => c.cmd === 'verify_resolution_plan')?.args).toEqual({ appId: '200260', planId: resolutionPlan.id })
    expect(text()).toContain('Configuração conferida — funcionamento ainda não verificado')
    m.unmount()
  })

  it('desfazer abre prévia de rollback e só executa após confirmação', async () => {
    stored = report('200260', { plans: [{ ...resolutionPlan, state: 'applied-unverified', operationId: operation.id }] })
    const m = mount(<ResolutionCenter appId="200260" available />); await flush(); await click(button('Desfazer'))
    expect(calls.find((c) => c.cmd === 'get_fix_operation')?.args).toEqual({ operationId: operation.id })
    expect(calls.some((c) => c.cmd === 'rollback_fix')).toBe(false)
    await click(button('Desfazer agora'))
    expect(calls.find((c) => c.cmd === 'rollback_fix')?.args).toEqual({ operationId: operation.id, fingerprint: 'fixture-rollback' })
    m.unmount()
  })

  it('cancelar seletor não importa log e arquivo autorizado usa AppID explícito', async () => {
    const m = mount(<ResolutionCenter appId="200260" available />); await flush(); await click(button('Importar log deste jogo'))
    expect(calls.some((c) => c.cmd === 'import_resolution_log')).toBe(false)
    selectedLog = '/synthetic/steam-200260.log'; await click(button('Importar log deste jogo'))
    expect(calls.find((c) => c.cmd === 'import_resolution_log')?.args).toEqual({ appId: '200260', path: selectedLog })
    expect(text()).toContain('Log sintético importado'); m.unmount()
  })

  it('modo demonstrativo não faz chamadas de Resolution Intelligence', async () => {
    const m = mount(<ResolutionCenter appId="demo" available={false} />); await flush()
    expect(text()).toContain('modo demonstrativo não pesquisa nem aplica')
    expect(button('Diagnosticar')).toBeUndefined()
    expect(calls.some((c) => /resolution|diagnose_game/.test(c.cmd))).toBe(false)
    m.unmount()
  })

  it('datas inválidas de fontes externas não derrubam o diagnóstico', async () => {
    stored = report('200260', { knowledge: [{ ...evidence, publishedAt: 'invalid-date', updatedAt: 'unknown' }] })
    const m = mount(<ResolutionCenter appId="200260" available />); await flush()
    expect(text()).toContain('Data não confirmada pela fonte')
    expect(text()).toContain('Crash informado no jogo 200260')
    m.unmount()
  })
})
