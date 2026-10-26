/**
 * Fix Engine na interface: listagem, prévia, confirmação, progresso real
 * por evento, falha, passo manual, histórico, rollback, incompatibilidade,
 * modo demonstrativo e integração com o Performance Lab.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { act, type ReactNode } from 'react'
import { createRoot } from 'react-dom/client'
import { MemoryRouter } from 'react-router-dom'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { emit } from '@tauri-apps/api/event'
import { MotionGlobalConfig } from 'motion/react'
import { AppStoreProvider, useDispatch } from '@/state/store'
import { LibraryProvider } from '@/state/library'
import { FixReportsProvider } from '@/state/fixes'
import { FixEngineProvider, useFixEngine } from '@/state/fixEngine'
import { ApplyFixDialog, type ApplyTarget } from '@/components/fix/ApplyFixDialog'
import { RecipesPanel } from '@/components/fix/RecipesPanel'
import { OperationHistory } from '@/components/fix/OperationHistory'
import { ComparisonView } from '@/components/performance/ComparisonView'
import type { FixPlan, Operation, RecipeView, RollbackPlan } from '@/types/fixEngine'
import { batman, scan } from './fixtures'
import { comparison, meta, session } from './perfFixtures'

;(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
// Transições instantâneas: o teste verifica estados, não animações.
MotionGlobalConfig.skipAnimations = true
const g = globalThis as unknown as { isTauri: boolean; ResizeObserver: unknown }
g.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
}

const recipe = (id: string, trust: RecipeView['trust'], extra: Partial<RecipeView> = {}): RecipeView => ({
  schema: 1,
  id,
  version: '1.0.0',
  trust,
  title: id === 'frx.diagnostics.proton-log' ? 'Log de diagnóstico do Proton' : 'Forçar o modo Direct3D 9 (sem d3d11)',
  summary: 's',
  description: 'd',
  category: 'other',
  purpose: id === 'frx.diagnostics.proton-log' ? 'diagnostic' : 'compatibility',
  risk: trust === 'reviewed' ? 'low' : 'high',
  scope: { appIds: [], requiresProton: true, gpuVendor: null },
  actions: [{ type: 'set_launch_env', key: id === 'frx.diagnostics.proton-log' ? 'PROTON_LOG' : 'PROTON_NO_D3D11', value: '1' }],
  alreadyIncludedByProtonfixes: false,
  conflictsWith: [],
  relatedKeys: [],
  effects: [],
  sideEffects: [],
  sources: [{ label: 'Proton README', url: 'https://github.com/ValveSoftware/Proton' }],
  review: trust === 'reviewed' ? { by: 'FRX', at: '2026-10-25', notes: 'ok' } : null,
  executable: trust === 'reviewed',
  integrity: true,
  relatedFindings: [],
  ...extra,
})

const BEFORE = 'PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command%'
const AFTER = 'PROTON_LOG=1 PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command%'

const plan = (status: FixPlan['status'], extra: Partial<FixPlan> = {}): FixPlan => ({
  planId: 'plan-1',
  fingerprint: 'fp-1',
  recipeId: 'frx.diagnostics.proton-log',
  recipeVersion: '1.0.0',
  recipeTitle: 'Log de diagnóstico do Proton',
  appId: '200260',
  gameName: 'Batman: Arkham City GOTY',
  status,
  mode: status === 'ready' ? 'automatic' : status === 'requires-manual-step' ? 'manual' : null,
  reasons: status === 'ready' ? [] : ['A Steam está aberta.'],
  checks: [{ label: 'Receita revisada', state: 'ok', detail: 'Catálogo local' }],
  changes: [{ kind: 'launch-options', target: 'Steam › Batman › Opções de inicialização (localconfig.vdf)', before: BEFORE, after: AFTER, summary: 'adiciona PROTON_LOG=1' }],
  manualLine: status === 'requires-manual-step' ? AFTER : null,
  backupNote: status === 'ready' ? 'Backup verificado antes de gravar.' : null,
  steamRunning: status === 'requires-manual-step',
  gameRunning: false,
  risk: 'low',
  purpose: 'diagnostic',
  trust: 'reviewed',
  effects: [],
  sideEffects: [],
  sources: [],
  generated: null,
  pack: null,
  createdAt: '2026-10-25T10:00:00Z',
  expiresAt: 9999999999,
  ...extra,
})

const op = (state: Operation['state'], extra: Partial<Operation> = {}): Operation => ({
  schema: 1,
  id: 'op-1',
  recipeId: 'frx.diagnostics.proton-log',
  recipeVersion: '1.0.0',
  recipeTitle: 'Log de diagnóstico do Proton',
  appId: '200260',
  gameName: 'Batman: Arkham City GOTY',
  state,
  mode: 'automatic',
  steps: [
    {
      step: { type: 'launch-options', config: '/x/localconfig.vdf', appId: '200260', fileSha: 'a', before: BEFORE, after: AFTER, env: [{ key: 'PROTON_LOG', written: '1', previous: null }], args: [] },
      status: 'verified',
      backup: { path: '/data/fix-engine/backups/op-1/0-localconfig.vdf', existed: true, sha256: 'abcdef0123456789', size: 10, mode: 420 },
      afterSha: 'b',
      rollbackBackup: null,
    },
  ],
  planFingerprint: 'fp-1',
  createdAt: '2026-10-25T10:00:00Z',
  updatedAt: '2026-10-25T10:00:01Z',
  appliedAt: state === 'applied' ? '2026-10-25T10:00:01Z' : null,
  rolledBackAt: null,
  message: state === 'applied' ? 'Aplicada e verificada.' : 'msg',
  pending: state === 'requires-manual-step' ? { kind: 'apply', line: AFTER } : null,
  events: [{ at: '2026-10-25T10:00:01Z', state, message: 'evento' }],
  recipeTrust: 'reviewed',
  findingIds: [],
  ...extra,
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
              <MemoryRouter>{node}</MemoryRouter>
            </FixEngineProvider>
          </FixReportsProvider>
        </LibraryProvider>
      </AppStoreProvider>,
    ),
  )
  return { el, unmount: () => act(() => root.unmount()) }
}

const flush = (ms = 30) => act(() => new Promise((r) => setTimeout(r, ms)))
/** O texto aparece em algum lugar da tela (inclui o portal dos diálogos). */
const byText = (text: string | RegExp) => {
  const body = document.body.textContent ?? ''
  return typeof text === 'string' ? body.includes(text) || undefined : text.test(body) || undefined
}
const button = (text: string) => [...document.body.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === text)
const click = async (el: HTMLElement | undefined) => {
  await act(async () => el!.click())
  await flush()
}

describe('Fix Engine na interface', () => {
  let calls: { cmd: string; args: Record<string, unknown> }[] = []
  let current: FixPlan = plan('ready')
  let applyResult: () => Operation = () => op('applied')
  let operations: Operation[] = []
  let rollbackPlan: RollbackPlan

  beforeEach(() => {
    calls = []
    current = plan('ready')
    applyResult = () => op('applied')
    operations = []
    rollbackPlan = { operationId: 'op-1', fingerprint: 'rb-1', status: 'ready', mode: 'automatic', reasons: [], changes: [{ kind: 'launch-options', target: 't', before: AFTER, after: BEFORE, summary: 'remove só o que esta correção adicionou' }], manualLine: null, steamRunning: false, gameRunning: false }
    localStorage.clear()
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
            return []
          case 'get_fix_engine_status':
            return { catalogVersion: '2026.10.25', catalogSha256: 'x', recipes: 2, reviewed: 1, issues: [], recoveryRequired: 0, pendingManual: 0, busy: false, dataDir: '/data/fix-engine' }
          case 'list_fix_operations':
            return operations
          case 'get_applied_fixes':
            return operations.filter((o) => o.state === 'applied').map((o) => ({ operationId: o.id, recipeId: o.recipeId, recipeVersion: o.recipeVersion, recipeTitle: o.recipeTitle, appId: o.appId, gameName: o.gameName, appliedAt: o.appliedAt, mode: o.mode, effective: 'active', detail: '' }))
          case 'list_fix_recipes':
            return [recipe('frx.diagnostics.proton-log', 'reviewed'), recipe('frx.compat.no-d3d11', 'candidate')]
          case 'check_fix_compatibility':
            return (args as { recipeId: string }).recipeId === 'frx.compat.no-d3d11' ? plan('not-executable', { recipeId: 'frx.compat.no-d3d11' }) : current
          case 'prepare_fix_plan':
            return current
          case 'apply_fix_plan': {
            for (const s of ['checking', 'backing-up', 'applying', 'verifying'] as const) await emit('fix-engine://progress', op(s))
            return applyResult()
          }
          case 'verify_fix_operation':
            return op('applied', { mode: 'manual', message: 'Confirmado no localconfig.vdf.' })
          case 'prepare_fix_rollback':
            return rollbackPlan
          case 'rollback_fix':
            return op('rolled-back', { message: 'Desfeita e verificada.' })
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

  it('carrega estado, histórico e aplicadas do núcleo', async () => {
    operations = [op('applied')]
    const probe: { e?: ReturnType<typeof useFixEngine> } = {}
    function P() {
      probe.e = useFixEngine()
      return null
    }
    const m = mount(<P />)
    await flush()
    expect(probe.e!.available).toBe(true)
    expect(probe.e!.status?.reviewed).toBe(1)
    expect(probe.e!.applied.map((a) => a.recipeId)).toEqual(['frx.diagnostics.proton-log'])
    m.unmount()
  })

  it('lista receitas com o estado calculado para o jogo: aplicável × candidata', async () => {
    const m = mount(<RecipesPanel games={[{ id: '200260', title: batman.name, origin: 'steam' } as never]} appId="200260" recipeId={null} onSelect={() => {}} onAct={() => {}} onShowFinding={() => {}} />)
    await flush(60)
    expect(byText('Aplicável')).toBeTruthy()
    expect(byText('Candidata')).toBeTruthy()
    // A receita candidata não pode ser aplicada.
    expect(calls.filter((c) => c.cmd === 'check_fix_compatibility')).toHaveLength(2)
    m.unmount()
  })

  it('prévia → confirmação → progresso real → aplicada; envia só IDs e a impressão digital', async () => {
    const target: ApplyTarget = { kind: 'apply', recipeId: 'frx.diagnostics.proton-log', appId: '200260', title: 'Log de diagnóstico do Proton' }
    const m = mount(<ApplyFixDialog target={target} onClose={() => {}} />)
    await flush()
    expect(byText(BEFORE)).toBeTruthy()
    expect(byText(AFTER)).toBeTruthy()
    await click(button('Continuar'))
    expect(button('Aplicar correção')!.disabled).toBe(true)
    await click(document.body.querySelector<HTMLInputElement>('input[type=checkbox]')!)
    expect(button('Aplicar correção')!.disabled).toBe(false)
    await act(async () => button('Aplicar correção')!.click())
    await flush(60)
    expect(byText(/Aplicada e verificada|Confirmado/)).toBeTruthy()
    const apply = calls.find((c) => c.cmd === 'apply_fix_plan')!
    expect(Object.keys(apply.args).sort()).toEqual(['appId', 'fingerprint', 'planId'])
    expect(apply.args.fingerprint).toBe('fp-1')
    m.unmount()
  })

  it('falha no núcleo vira erro visível, nunca “aplicada”', async () => {
    applyResult = () => {
      throw 'O estado do jogo ou da Steam mudou desde a prévia. Nada foi alterado; revise a correção de novo.'
    }
    const m = mount(<ApplyFixDialog target={{ kind: 'apply', recipeId: 'frx.diagnostics.proton-log', appId: '200260', title: 'Log' }} onClose={() => {}} />)
    await flush()
    await click(button('Continuar'))
    await click(document.body.querySelector<HTMLInputElement>('input[type=checkbox]')!)
    await act(async () => button('Aplicar correção')!.click())
    await flush(60)
    expect(byText(/mudou desde a prévia/)).toBeTruthy()
    expect(byText(/Aplicada e verificada/)).toBeFalsy()
    expect(button('Revisar de novo')).toBeTruthy()
    m.unmount()
  })

  it('Steam aberta: mostra a linha para colar e só conclui depois de conferir', async () => {
    current = plan('requires-manual-step')
    applyResult = () => op('requires-manual-step', { mode: 'manual' })
    const m = mount(<ApplyFixDialog target={{ kind: 'apply', recipeId: 'frx.diagnostics.proton-log', appId: '200260', title: 'Log' }} onClose={() => {}} />)
    await flush()
    expect(byText('A Steam está aberta.')).toBeTruthy()
    await click(button('Continuar no modo manual'))
    await click(document.body.querySelector<HTMLInputElement>('input[type=checkbox]')!)
    await act(async () => button('Registrar e mostrar a linha')!.click())
    await flush(60)
    expect([...document.body.querySelectorAll('code')].some((c) => c.textContent === AFTER)).toBe(true)
    expect(byText(/Aplicada e verificada/)).toBeFalsy()
    await act(async () => button('Conferir agora')!.click())
    await flush(60)
    expect(calls.some((c) => c.cmd === 'verify_fix_operation')).toBe(true)
    expect(byText(/Aplicada e verificada|Confirmado/)).toBeTruthy()
    m.unmount()
  })

  it('incompatível: explica o motivo e não oferece aplicar', async () => {
    current = plan('incompatible', { reasons: ['O jogo roda sem Proton; esta correção não se aplica.'] })
    const m = mount(<ApplyFixDialog target={{ kind: 'apply', recipeId: 'frx.diagnostics.proton-log', appId: '200260', title: 'Log' }} onClose={() => {}} />)
    await flush()
    expect(byText(/roda sem Proton/)).toBeTruthy()
    expect(button('Continuar')).toBeFalsy()
    m.unmount()
  })

  it('histórico com rollback; conflito de rollback não oferece desfazer', async () => {
    operations = [op('applied')]
    const acted: ApplyTarget[] = []
    const m = mount(<OperationHistory onAct={(t) => acted.push(t)} />)
    await flush()
    expect(byText('Aplicada')).toBeTruthy()
    await click(button('Desfazer'))
    expect(acted[0]).toMatchObject({ kind: 'rollback' })
    m.unmount()

    const d = mount(<ApplyFixDialog target={acted[0]} onClose={() => {}} />)
    await flush()
    await act(async () => button('Desfazer agora')!.click())
    await flush(60)
    expect(byText('Desfeita e verificada.')).toBeTruthy()
    expect(Object.keys(calls.find((c) => c.cmd === 'rollback_fix')!.args).sort()).toEqual(['fingerprint', 'operationId'])
    d.unmount()

    rollbackPlan = { ...rollbackPlan, status: 'conflict', mode: null, changes: [], reasons: ['PROTON_LOG foi alterada para “0” depois da aplicação. O FRX não sobrescreve mudanças de outros programas.'] }
    const c = mount(<ApplyFixDialog target={{ kind: 'rollback', operation: op('applied') }} onClose={() => {}} />)
    await flush()
    expect(byText(/não sobrescreve/)).toBeTruthy()
    expect(button('Desfazer agora')).toBeFalsy()
    c.unmount()
  })

  it('no modo demonstrativo não consulta o Fix Engine', async () => {
    const probe: { e?: ReturnType<typeof useFixEngine>; d?: ReturnType<typeof useDispatch> } = {}
    g.isTauri = false
    function P() {
      probe.e = useFixEngine()
      return null
    }
    const m = mount(<P />)
    await flush()
    expect(probe.e!.available).toBe(false)
    expect(calls.some((c) => c.cmd.includes('fix'))).toBe(false)
    m.unmount()
  })

  it('comparação mostra correções diferentes sem atribuir causa', () => {
    const c = {
      ...comparison,
      comparability: {
        controlled: true,
        differences: [{ field: 'fixes', label: 'Correções do FRX ativas', before: 'nenhuma', after: 'Renderizar com WineD3D (OpenGL) (v1.0.0)', major: false }],
        notes: ['As sessões têm correções do FRX diferentes. Uma diferença de desempenho entre elas não prova que a correção foi a causa; repita as medições para confirmar.'],
      },
    }
    const m = mount(<ComparisonView result={c} before={session(meta('a', 50))} after={session(meta('b', 112))} />)
    expect(byText(/Correções do FRX ativas/)).toBeTruthy()
    expect(byText(/não prova que a correção foi a causa/)).toBeTruthy()
    m.unmount()
  })
})
