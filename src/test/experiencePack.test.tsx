/**
 * Complete Experience Pack na página do jogo. Pacote e relatório são
 * **fixtures sintéticas** no formato exato do Rust.
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
import { ExperiencePackSection } from '@/components/pack/ExperiencePack'
import type { ApplyTarget } from '@/components/fix/ApplyFixDialog'
import type { Operation } from '@/types/fixEngine'
import type { ExperiencePack, PackAction } from '@/types/smartPacks'
import { report, scan } from './fixtures'

;(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
MotionGlobalConfig.skipAnimations = true
const g = globalThis as unknown as { isTauri: boolean }

const action = (recipeId: string, over: Partial<PackAction> = {}): PackAction => ({
  recipeId,
  title: recipeId,
  area: 'Gráficos',
  changes: ['PROTON_DLSS_UPGRADE=1'],
  effects: ['Usa a versão mais recente do DLSS.'],
  sideEffects: [],
  risk: 'moderate',
  evidence: 'documented',
  evidenceNote: 'Opção documentada pela ferramenta e conferida nesta instalação.',
  positive: 0,
  negative: 0,
  sources: [{ label: 'README', url: 'https://github.com/CachyOS/proton-cachyos' }],
  selected: true,
  ...over,
})

const pack = (over: Partial<ExperiencePack> = {}): ExperiencePack => ({
  appId: '200260',
  gameName: 'Batman',
  generatedAt: 'x',
  environment: ['GPU NVIDIA', 'proton-cachyos'],
  actions: [action('frx.proton.dlss-upgrade'), action('gen.200260.aaaaaaaaaaaaaaaa', { title: 'Heap', area: 'Compatibilidade', changes: ['PROTON_HEAP_DELAY_FREE=1'] })],
  leftOut: [
    { title: 'VKD3D', reason: '1 relato(s) dizem que não resolveu ou piorou.', kind: 'negative', changes: ['VKD3D_CONFIG=descriptor_heap'], count: 1 },
    { title: 'Relato geral', reason: 'Relato geral sobre o jogo, sem ação técnica estruturada.', kind: 'no-action', changes: [], count: 6 },
    { title: 'HDR', reason: 'O FRX não conhece o efeito de DXVK_HDR.', kind: 'unsupported', changes: ['DXVK_HDR=1'], count: 2 },
  ],
  areas: ['Gráficos', 'Compatibilidade'],
  risks: ['A primeira execução precisa de internet.'],
  compatibility: [],
  packId: 'pack.200260.2222222222222222',
  applyRecipeId: 'pack.200260.2222222222222222',
  availability: { state: 'available', reasons: [], checkedAt: 'x', planStatus: 'ready', planFingerprint: 'fp-pack' },
  state: 'ready',
  stateDetail: '',
  operationId: null,
  partial: true,
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
const text = () => document.body.textContent ?? ''
const button = (t: string) => [...document.body.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === t)

describe('Complete Experience Pack na página do jogo', () => {
  let calls: { cmd: string; args: Record<string, unknown> }[] = []
  let current: () => ExperiencePack = () => pack()
  let cached = true
  let operations: Operation[] = []

  beforeEach(() => {
    calls = []
    current = () => pack()
    cached = true
    operations = []
    g.isTauri = true
    mockIPC(async (cmd, args) => {
      calls.push({ cmd, args: (args ?? {}) as Record<string, unknown> })
      switch (cmd) {
        case 'get_installed_games':
        case 'scan_installed_games':
          return scan
        case 'get_system_profile':
          return { gpus: [], primaryGpu: null, vulkanDrivers: [], arch: 'x86_64' }
        case 'get_cached_fix_reports':
          return cached ? [report] : []
        case 'get_fix_engine_status':
          return { catalogVersion: 't', recipes: 4, reviewed: 3, issues: [], recoveryRequired: 0, pendingManual: 0, busy: false, dataDir: '/d' }
        case 'list_fix_operations':
          return operations
        case 'get_applied_fixes':
          return []
        case 'get_experience_pack':
          return current()
        default:
          return null
      }
    }, { shouldMockEvents: true })
  })

  afterEach(() => {
    clearMocks()
    g.isTauri = false
    document.body.innerHTML = ''
  })

  it('sem pesquisa: uma única ação principal, Buscar melhorias', async () => {
    cached = false
    const m = mount(<ExperiencePackSection appId="200260" title="Batman" onAct={() => {}} />)
    await flush()
    expect(button('Buscar melhorias')).toBeTruthy()
    expect(calls.some((c) => c.cmd === 'get_experience_pack')).toBe(false)
    m.unmount()
  })

  it('resume o pacote, agrupa o que ficou de fora e prepara pela prévia com o ID do pacote', async () => {
    const acted: ApplyTarget[] = []
    const m = mount(<ExperiencePackSection appId="200260" title="Batman" onAct={(t) => acted.push(t)} />)
    await flush(80)
    expect(text()).toContain('Complete Experience Pack')
    expect(text()).toContain('2 melhorias compatíveis')
    expect(text()).toContain('Pacote parcial: 2 possíveis melhorias')
    expect(text()).not.toContain('Fora do pacote')
    await act(async () => button('Detalhes técnicos')!.click())
    expect(text()).toContain('Fora do pacote · 9 descobertas')
    expect(text()).toContain('VKD3D_CONFIG=descriptor_heap')
    await act(async () => button('Preparar pacote')!.click())
    expect(acted[0]).toMatchObject({ kind: 'apply', recipeId: 'pack.200260.2222222222222222', appId: '200260', expectedFingerprint: 'fp-pack' })
    m.unmount()
  })

  it('desmarcar uma ação recompõe o pacote no núcleo só com IDs', async () => {
    const m = mount(<ExperiencePackSection appId="200260" title="Batman" onAct={() => {}} />)
    await flush(80)
    await act(async () => button('Detalhes técnicos')!.click())
    const box = document.body.querySelector<HTMLInputElement>('input[aria-label="Incluir Heap"]')!
    await act(async () => box.click())
    await flush(80)
    const last = calls.filter((c) => c.cmd === 'get_experience_pack').at(-1)!
    expect(last.args).toEqual({ appId: '200260', without: ['gen.200260.aaaaaaaaaaaaaaaa'] })
    m.unmount()
  })

  it('pacote aplicado oferece desfazer o pacote inteiro', async () => {
    operations = [{ id: 'op-pack', appId: '200260', state: 'applied', recipeId: 'pack.200260.2222222222222222', recipeTitle: 'Complete Experience Pack', steps: [], events: [], updatedAt: '2026-10-29T10:00:00Z', mode: 'automatic', recipeVersion: '1.4.0', recipeTrust: 'pack', gameName: 'Batman' } as unknown as Operation]
    current = () => pack({ state: 'verified', operationId: 'op-pack', availability: { state: 'applied', reasons: [], checkedAt: 'x', planStatus: 'already-applied', planFingerprint: null } })
    const acted: ApplyTarget[] = []
    const m = mount(<ExperiencePackSection appId="200260" title="Batman" onAct={(t) => acted.push(t)} />)
    await flush(80)
    expect(text()).toContain('Aplicado')
    expect(button('Preparar pacote')).toBeUndefined()
    await act(async () => button('Desfazer pacote')!.click())
    expect(acted[0]).toMatchObject({ kind: 'rollback', operation: { id: 'op-pack' } })
    m.unmount()
  })

  it('pacote vazio não inventa melhorias e explica o motivo', async () => {
    current = () => pack({ actions: [], areas: [], applyRecipeId: null, packId: null, availability: { state: 'investigation', reasons: [], checkedAt: 'x', planStatus: null, planFingerprint: null } })
    const m = mount(<ExperiencePackSection appId="200260" title="Batman" onAct={() => {}} />)
    await flush(80)
    expect(text()).toContain('Nenhuma melhoria tem evidência e suporte suficientes')
    expect(button('Preparar pacote')).toBeUndefined()
    m.unmount()
  })
})
