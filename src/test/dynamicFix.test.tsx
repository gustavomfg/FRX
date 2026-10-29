/**
 * Gerador dinâmico na interface: análise do núcleo → estado de cada
 * descoberta → "Aplicar correção" abre a prévia do Fix Engine (nunca aplica
 * direto) → confirmação específica de receita gerada → histórico.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { act, useState, type ReactNode } from 'react'
import { createRoot } from 'react-dom/client'
import { MemoryRouter } from 'react-router-dom'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { MotionGlobalConfig } from 'motion/react'
import { AppStoreProvider } from '@/state/store'
import { LibraryProvider } from '@/state/library'
import { FixReportsProvider } from '@/state/fixes'
import { FixEngineProvider } from '@/state/fixEngine'
import { ActionableFindings } from '@/components/fix/ActionableFindings'
import { FindingCard } from '@/components/fix/FindingCard'
import { ApplyFixDialog, type ApplyTarget } from '@/components/fix/ApplyFixDialog'
import { OperationHistory } from '@/components/fix/OperationHistory'
import { useAssessments } from '@/components/fix/useAssessments'
import type { Assessment, FixPlan, Operation, Recipe } from '@/types/fixEngine'
import { finding, report, scan } from './fixtures'

;(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
MotionGlobalConfig.skipAnimations = true
const g = globalThis as unknown as { isTauri: boolean; ResizeObserver: unknown }
g.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
}

const GEN = 'gen.200260.0123456789abcdef'
const SNIPPET = 'it was due to D3D11 detection errors'
const SOURCE = { provider: 'github' as const, label: 'Comentário em ValveSoftware/Proton#309', url: 'https://github.com/ValveSoftware/Proton/issues/309#issuecomment-435132818', snippet: SNIPPET, publishedAt: '2018-11-01T18:09:34Z' }
const generated = { generatorVersion: '1.0.0', rulesVersion: '2026.10.26-1', interpreter: 'deterministic', findingIds: ['f-gen'], evidence: 'single' as const, evidenceNote: 'Um único relato, sem confirmação de outras pessoas.', provenance: [SOURCE], discoveredAt: SOURCE.publishedAt }

const recipe: Recipe = {
  schema: 1,
  id: GEN,
  version: '1.0.0',
  trust: 'generated',
  title: 'Desativar Direct3D 11',
  summary: 'Define PROTON_NO_D3D11=1 nas opções de inicialização deste jogo.',
  description: 'Gerada automaticamente.',
  category: 'graphics',
  purpose: 'compatibility',
  risk: 'moderate',
  scope: { appIds: ['200260'], requiresProton: true, gpuVendor: null },
  actions: [{ type: 'set_launch_env', key: 'PROTON_NO_D3D11', value: '1' }],
  alreadyIncludedByProtonfixes: false,
  conflictsWith: [],
  relatedKeys: ['env:PROTON_NO_D3D11=1'],
  effects: ['O d3d11.dll fica indisponível, para o jogo cair no modo Direct3D 9.'],
  sideEffects: ['Se o jogo não tiver modo Direct3D 9, ele deixa de abrir até você desfazer.'],
  sources: [{ label: SOURCE.label, url: SOURCE.url }],
  review: null,
  generated,
}

const base = { evidence: 'single' as const, evidenceNote: generated.evidenceNote, provenance: [SOURCE], interpreter: 'deterministic', generatedAt: '2026-10-26T10:00:00Z', updatedAt: '2026-10-26T10:00:00Z', discoveredAt: null, reevaluated: null, appId: '200260' }
const assessments: Assessment[] = [
  { ...base, findingId: 'f-gen', availability: { state: 'available', reasons: [], checkedAt: '2026-10-27T00:00:00Z', planStatus: 'ready', planFingerprint: 'fixture-plan' }, class: 'applicable', status: 'generated', reasons: ['Receita gerada automaticamente a partir da descoberta.', `Experimental: ${generated.evidenceNote}`], experimental: true, actions: [{ kind: 'launch-option', raw: 'PROTON_NO_D3D11=1', supported: true, note: 'O d3d11.dll fica indisponível.' }], recipeId: GEN, recipe },
  { ...base, findingId: 'f-wt', class: 'needs-validation', status: 'needs-validation', reasons: ['Instala “d3dx9_43” no prefixo do Proton com winetricks: baixa arquivos e executa instaladores, o que o FRX não faz automaticamente.'], experimental: false, actions: [{ kind: 'winetricks', raw: 'd3dx9_43', supported: false, note: 'winetricks' }], recipeId: null, recipe: null },
]
const findings = [
  finding({ id: 'f-gen', title: 'Opção de inicialização sugerida', kind: 'tweak', actions: [{ kind: 'launch-option', label: 'Opções de inicialização na Steam', value: 'PROTON_NO_D3D11=1 %command%', conditional: false }] }),
  finding({ id: 'f-wt' }),
]

const plan: FixPlan = {
  planId: 'plan-g',
  fingerprint: 'fp-g',
  recipeId: GEN,
  recipeVersion: '1.0.0',
  recipeTitle: recipe.title,
  appId: '200260',
  gameName: 'Batman: Arkham City GOTY',
  status: 'ready',
  mode: 'automatic',
  reasons: [],
  checks: [{ label: 'Receita gerada', state: 'warn', detail: 'Validada pelo núcleo, não revisada por pessoas.' }],
  changes: [{ kind: 'launch-options', target: 'Steam › Batman › Opções de inicialização (localconfig.vdf)', before: 'PROTON_NVIDIA_LIBS=1 %command%', after: 'PROTON_NO_D3D11=1 PROTON_NVIDIA_LIBS=1 %command%', summary: 'adiciona PROTON_NO_D3D11=1' }],
  manualLine: null,
  backupNote: 'Backup verificado antes de gravar.',
  steamRunning: false,
  gameRunning: false,
  risk: 'moderate',
  purpose: 'compatibility',
  trust: 'generated',
  effects: recipe.effects,
  sideEffects: recipe.sideEffects,
  sources: recipe.sources,
  generated,
  pack: null,
  createdAt: '2026-10-26T10:00:00Z',
  expiresAt: 9999999999,
}

const appliedOp = (): Operation => ({
  schema: 1,
  id: 'op-g',
  recipeId: GEN,
  recipeVersion: '1.0.0',
  recipeTitle: recipe.title,
  appId: '200260',
  gameName: 'Batman: Arkham City GOTY',
  state: 'applied',
  mode: 'automatic',
  steps: [],
  planFingerprint: 'fp-g',
  createdAt: '2026-10-26T10:00:00Z',
  updatedAt: '2026-10-26T10:00:01Z',
  appliedAt: '2026-10-26T10:00:01Z',
  rolledBackAt: null,
  message: 'Aplicada e verificada no disco.',
  pending: null,
  events: [],
  recipeTrust: 'generated',
  findingIds: ['f-gen'],
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
  return { unmount: () => act(() => root.unmount()) }
}

const flush = (ms = 40) => act(() => new Promise((r) => setTimeout(r, ms)))
const byText = (t: string | RegExp) => {
  const body = document.body.textContent ?? ''
  return typeof t === 'string' ? body.includes(t) : t.test(body)
}
const button = (text: string) => [...document.body.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === text)
const click = async (el: HTMLElement | undefined) => {
  await act(async () => el!.click())
  await flush()
}

function Harness() {
  const a = useAssessments('200260')
  const [target, setTarget] = useState<ApplyTarget | null>(null)
  return (
    <>
      <ul>
        {findings.map((f) => (
          <FindingCard key={f.id} finding={f} defaultOpen assessment={a.byFinding[f.id] ?? null} analyzing={a.loading} onApply={(x) => setTarget({ kind: 'apply', expectedFingerprint: x.availability?.planFingerprint ?? undefined, recipeId: x.recipeId!, appId: x.appId, title: x.recipe?.title ?? f.title })} />
        ))}
      </ul>
      {target && <ApplyFixDialog target={target} onClose={() => setTarget(null)} />}
    </>
  )
}

describe('Gerador dinâmico na interface', () => {
  let calls: { cmd: string; args: Record<string, unknown> }[] = []
  let operations: Operation[] = []
  let prepareFailure: string | null = null

  beforeEach(() => {
    calls = []
    operations = []
    prepareFailure = null
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
            return [{ ...report, findings }]
          case 'get_fix_engine_status':
            return { catalogVersion: 't', catalogSha256: 'x', recipes: 4, reviewed: 3, issues: [], recoveryRequired: 0, pendingManual: 0, busy: false, dataDir: '/data/fix-engine' }
          case 'list_fix_operations':
            return operations
          case 'get_applied_fixes':
            return operations.map((o) => ({ operationId: o.id, recipeId: o.recipeId, recipeVersion: o.recipeVersion, recipeTitle: o.recipeTitle, appId: o.appId, gameName: o.gameName, appliedAt: o.appliedAt, mode: o.mode, effective: 'active', detail: '' }))
          case 'analyze_fix_findings':
            return assessments
          case 'prepare_fix_plan':
            if (prepareFailure) throw prepareFailure
            return plan
          case 'apply_fix_plan':
            operations = [appliedOp()]
            return appliedOp()
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

  it('mostra o estado e o motivo de cada descoberta, e aplica só pela prévia', async () => {
    const m = mount(<Harness />)
    await flush(80)
    // React → núcleo: só o AppID sai da interface.
    const analyze = calls.find((c) => c.cmd === 'analyze_fix_findings')!
    expect(Object.keys(analyze.args)).toEqual(['appId'])
    expect(byText('Receita gerada automaticamente')).toBe(true)
    expect(byText('Experimental')).toBe(true)
    expect(byText('Requer validação adicional')).toBe(true)
    expect(byText(/baixa arquivos e executa instaladores/)).toBe(true)
    expect(button('Requer validação adicional')).toBeUndefined()
    expect(byText('Receita indisponível')).toBe(false)

    // Clicar não aplica: abre a prévia calculada no núcleo.
    await click(button('Aplicar correção'))
    expect(calls.some((c) => c.cmd === 'apply_fix_plan')).toBe(false)
    expect(calls.find((c) => c.cmd === 'prepare_fix_plan')!.args).toEqual({ recipeId: GEN, appId: '200260', expectedFingerprint: 'fixture-plan' })
    expect(byText('Por que foi sugerida')).toBe(true)
    expect(byText(SNIPPET)).toBe(true)
    expect(byText(/deixa de abrir até você desfazer/)).toBe(true)

    await click(button('Continuar'))
    expect(byText(/gerada automaticamente a partir de relatos públicos, não foi revisada por pessoas/)).toBe(true)
    await click(document.body.querySelector<HTMLInputElement>('input[type=checkbox]')!)
    const dialog = document.body.querySelector('[role="dialog"]')!
    const confirm = [...dialog.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === 'Aplicar correção')!
    expect(confirm.disabled).toBe(false)
    await act(async () => confirm.click())
    await flush(80)
    const apply = calls.find((c) => c.cmd === 'apply_fix_plan')!
    expect(Object.keys(apply.args).sort()).toEqual(['appId', 'fingerprint', 'planId'])
    expect(byText('Aplicada e verificada no disco.')).toBe(true)
    m.unmount()
  })

  it('só apresenta planos completos na lista principal; pesquisas e receitas legadas ficam separadas', async () => {
    const legacy = finding({ id: 'legacy', title: 'Fixture cache antigo' })
    const missing = finding({ id: 'missing', title: 'Fixture sem receita' })
    const foreign = finding({ id: 'foreign', gameAppId: '999', title: 'Fixture de outro jogo' })
    const m = mount(<ActionableFindings appId="200260" findings={[...findings, legacy, missing, foreign]} assessments={Object.fromEntries([...assessments, { ...assessments[0], findingId: legacy.id, availability: undefined }, { ...assessments[0], findingId: missing.id, recipeId: null }, { ...assessments[0], findingId: foreign.id, appId: '999' }].map((a) => [a.findingId, a]))} loading={false} error={null} refresh={() => {}} onApply={() => {}} />)
    await flush()
    const main = document.querySelector('[aria-label="Correções disponíveis"]')!
    expect(main.textContent).toContain(findings[0].title)
    expect(main.textContent).not.toContain(legacy.title)
    expect(main.textContent).not.toContain(missing.title)
    expect(byText(foreign.title)).toBe(false)
    expect(byText('Descobertas em investigação (3)')).toBe(true)
    const card = main.querySelector<HTMLButtonElement>('button')!
    await click(card)
    expect(button('Aplicar correção')?.disabled).toBe(false)
    m.unmount()
  })

  it('condição temporária orienta fechar o jogo e revalidar; não oferece aplicação falsa', async () => {
    let refreshes = 0
    const temporary: Assessment = { ...assessments[0], availability: { state: 'temporary', reasons: [{ code: 'game-running', message: 'Feche o jogo para continuar.' }], checkedAt: '2026-10-27', planStatus: 'conflict', planFingerprint: null } }
    const m = mount(<ActionableFindings appId="200260" findings={[findings[0]]} assessments={{ 'f-gen': temporary }} loading={false} error={null} refresh={() => { refreshes++ }} onApply={() => { throw Error('must not apply') }} />)
    await flush()
    expect(byText('Tudo certo: nenhuma correção automática é necessária')).toBe(true)
    const pending = document.querySelector('[aria-label="Aguardando condição local"]')!
    await click(pending.querySelector<HTMLButtonElement>('button')!)
    expect(pending.textContent).toContain('Feche o jogo para continuar.')
    expect(pending.textContent).not.toContain('Correção aplicável')
    expect(button('Aplicar correção')).toBeUndefined()
    await click(button('Revalidar condição')); expect(refreshes).toBe(1)
    m.unmount()
  })

  it('nova validação incompatível retira a ação principal e preserva a razão na pesquisa', async () => {
    function ChangingContext() {
      const [current, setCurrent] = useState(assessments[0])
      return <><button onClick={() => setCurrent({ ...current, availability: { state: 'incompatible', checkedAt: '2026-10-27', planStatus: 'incompatible', planFingerprint: null, reasons: [{ code: 'hardware', message: 'A GPU atual não atende ao requisito NVIDIA desta receita.' }] } })}>Trocar ambiente sintético</button><ActionableFindings appId="200260" findings={[findings[0]]} assessments={{ 'f-gen': current }} loading={false} error={null} refresh={() => {}} onApply={() => {}} /></>
    }
    const m = mount(<ChangingContext />); await flush()
    expect(document.querySelector('[aria-label="Correções disponíveis"]')?.textContent).toContain(findings[0].title)
    await click(button('Trocar ambiente sintético'))
    expect(document.querySelector('[aria-label="Correções disponíveis"]')?.textContent).not.toContain(findings[0].title)
    expect(byText('Descobertas em investigação (1)')).toBe(true)
    await click([...document.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.startsWith(findings[0].title)))
    expect(byText('A GPU atual não atende')).toBe(true)
    expect(button('Aplicar correção')).toBeUndefined()
    m.unmount()
  })

  it('mudança de fonte entre listagem e prévia preserva o motivo do núcleo e não executa', async () => {
    prepareFailure = 'A fonte ou o ambiente mudou desde a listagem. Revalide as correções.'
    const m = mount(<Harness />); await flush(80)
    await click(button('Aplicar correção'))
    expect(calls.find((c) => c.cmd === 'prepare_fix_plan')?.args.expectedFingerprint).toBe('fixture-plan')
    expect(byText('A fonte ou o ambiente mudou')).toBe(true)
    expect(calls.some((c) => c.cmd === 'apply_fix_plan')).toBe(false)
    expect(document.querySelector('[role="dialog"]')?.textContent).not.toContain('Continuar')
    expect(button('Revisar de novo')).toBeUndefined()
    await click(button('Fechar e revalidar'))
    expect(document.querySelector('[role="dialog"]')).toBeNull()
    expect(calls.filter((c) => c.cmd === 'prepare_fix_plan')).toHaveLength(1)
    m.unmount()
  })

  it('histórico identifica correções geradas e o cartão mostra “aplicada”', async () => {
    operations = [appliedOp()]
    const m = mount(
      <>
        <Harness />
        <OperationHistory onAct={() => {}} />
      </>,
    )
    await flush(80)
    expect(byText(/receita gerada automaticamente v1\.0\.0/)).toBe(true)
    expect(byText('Correção aplicada · backup disponível')).toBe(true)
    expect(button('Aplicar correção')).toBeUndefined()
    m.unmount()
  })
})
