/**
 * Dashboard do Hardware Fixer: DMI na tela inicial e o fluxo sintoma → busca
 * na Arch Wiki → cartões, com a ponte `invoke` simulada no formato do Rust.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { act } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { MotionGlobalConfig } from 'motion/react'
import { App } from '@/App'
import type { HardwareReport } from '@/types/hardware'
import type { FixPlan, Operation } from '@/types/fixEngine'

;(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
MotionGlobalConfig.skipAnimations = true

const identity = {
  vendor: 'LENOVO',
  model: '20XWCTO1WW',
  boardName: '20XWCTO1WW',
  productVersion: 'ThinkPad X1 Carbon Gen 9',
  displayName: 'ThinkPad X1 Carbon Gen 9',
}

const ACTION = {
  id: 'b0c311d42e52',
  action: { type: 'add_modprobe_option' as const, module: 'snd_hda_intel', options: ['model=dell-headset-multi'] },
  snippet: 'options snd_hda_intel model=dell-headset-multi',
  context: 'If the headset jack does not work, set the model.',
  conditional: true,
}

const PATH = '/etc/modprobe.d/frx-snd-hda-intel.conf'
const LINE = `sudo install -m 0644 -o root -g root -- '/home/u/.local/share/io.frx.FRX/fix-engine/system-staged/op-1/frx-snd-hda-intel.conf' '${PATH}'`

const plan: FixPlan = {
  planId: 'op-1',
  fingerprint: 'fp-1',
  recipeId: 'system.modprobe.b0c311d42e52',
  recipeVersion: '1',
  recipeTitle: 'Audio: options snd_hda_intel model=dell-headset-multi',
  appId: 'system',
  gameName: 'ThinkPad X1 Carbon Gen 9',
  status: 'requires-manual-step',
  mode: 'manual',
  reasons: [],
  checks: [{ label: 'Destino', state: 'ok', detail: `${PATH} está na lista de arquivos de sistema permitidos.` }],
  changes: [{ kind: 'system-file', target: PATH, before: null, after: '# Gerado pelo FRX\noptions snd_hda_intel model=dell-headset-multi\n', summary: `cria ${PATH}` }],
  manualLine: LINE,
  backupNote: `${PATH} ainda não existe. Desfazer remove o arquivo criado.`,
  steamRunning: false,
  gameRunning: false,
  risk: 'moderate',
  purpose: 'compatibility',
  trust: 'generated',
  effects: [ACTION.context],
  sideEffects: ['Vale depois de reiniciar.'],
  sources: [{ label: 'Arch Wiki · Lenovo ThinkPad X1 Carbon (Gen 9) › Audio', url: 'https://wiki.archlinux.org/title/Lenovo_ThinkPad_X1_Carbon_(Gen_9)#Audio' }],
  generated: null,
  pack: null,
  createdAt: '2026-11-03T00:00:00Z',
  expiresAt: 4102444800,
}

const op = (state: Operation['state'], pending: Operation['pending']): Operation => ({
  schema: 1,
  id: 'op-1',
  recipeId: plan.recipeId,
  recipeVersion: '1',
  recipeTitle: plan.recipeTitle,
  appId: 'system',
  gameName: plan.gameName,
  state,
  mode: 'manual',
  steps: [],
  planFingerprint: 'fp-1',
  createdAt: '',
  updatedAt: '',
  appliedAt: null,
  rolledBackAt: null,
  message: state === 'applied' ? `Confirmado em ${PATH}.` : 'Rode o comando com sudo.',
  pending,
  events: [],
  recipeTrust: 'generated',
  findingIds: [],
})

const report = (symptom: string): HardwareReport => ({
  identity,
  simulated: false,
  symptom: symptom as HardwareReport['symptom'],
  retrievedAt: '2026-11-02T00:00:00Z',
  status: { id: 'arch-wiki', name: 'Arch Wiki', state: 'ok', message: null, fetchedAt: null, resetAt: null },
  modelPage: 'Lenovo ThinkPad X1 Carbon (Gen 9)',
  support: 'works',
  suggestions: [
    {
      id: 'a',
      title: 'Audio',
      page: 'Lenovo ThinkPad X1 Carbon (Gen 9)',
      section: 'Audio',
      url: 'https://wiki.archlinux.org/title/Lenovo_ThinkPad_X1_Carbon_(Gen_9)#Audio',
      excerpt: 'This laptop requires Sound Open Firmware.',
      fit: 'model',
      actions: [ACTION],
    },
    {
      id: 'b',
      title: 'Advanced Linux Sound Architecture/Troubleshooting',
      page: 'Advanced Linux Sound Architecture/Troubleshooting',
      section: null,
      url: 'https://wiki.archlinux.org/title/Advanced_Linux_Sound_Architecture/Troubleshooting',
      excerpt: 'Solução de problemas de áudio no ALSA.',
      fit: 'general',
      actions: [],
    },
  ],
})

const flush = () => act(() => new Promise((r) => setTimeout(r, 20)))

describe('Dashboard', () => {
  const calls: { cmd: string; args: unknown }[] = []
  let el: HTMLDivElement
  let root: Root

  beforeEach(() => {
    calls.length = 0
    localStorage.clear()
    localStorage.setItem('frx.settings.v1', JSON.stringify({ motion: 'off' }))
    window.location.hash = ''
    ;(globalThis as unknown as { isTauri: boolean }).isTauri = true
    mockIPC((cmd, args) => {
      calls.push({ cmd, args })
      switch (cmd) {
        case 'identify_laptop':
          return identity
        case 'get_system_profile':
          return {
            kernel: '6.18.2-cachyos',
            arch: 'x86_64',
            gpus: [],
            primaryGpu: { vendor: 'intel', name: 'Iris Xe Graphics' },
            vulkanDrivers: [],
          }
        case 'search_hardware_fixes':
          return report((args as { args: { symptom: string } }).args.symptom)
        case 'list_fix_operations':
          return []
        case 'prepare_system_fix':
          return plan
        case 'apply_system_fix':
          return op('requires-manual-step', { kind: 'apply', line: LINE })
        case 'verify_fix_operation':
          return op('applied', null)
        default:
          return null
      }
    })
    el = document.createElement('div')
    document.body.appendChild(el)
    root = createRoot(el)
  })

  afterEach(() => {
    act(() => root.unmount())
    el.remove()
    clearMocks()
  })

  it('mostra modelo, GPU e kernel lidos do sistema e nunca toca nas telas de jogos', async () => {
    act(() => root.render(<App />))
    await flush()
    expect(el.textContent).toContain('Seu sistema:')
    expect(el.textContent).toContain('ThinkPad X1 Carbon Gen 9')
    expect(el.textContent).toContain('Iris Xe Graphics')
    expect(el.textContent).toContain('Linux 6.18.2-cachyos')
    expect(calls.map((c) => c.cmd)).not.toContain('scan_installed_games')
    expect(el.textContent).not.toContain('Biblioteca')
    expect(el.textContent).not.toContain('Performance Lab')
  })

  it('clicar em um sintoma pesquisa e mostra os cartões com o selo do modelo', async () => {
    act(() => root.render(<App />))
    await flush()
    const audio = [...el.querySelectorAll('button[role="radio"]')].find((b) => b.textContent?.includes('Áudio'))!
    act(() => (audio as HTMLButtonElement).click())
    await flush()
    const search = calls.find((c) => c.cmd === 'search_hardware_fixes')
    expect(search?.args).toMatchObject({ args: { symptom: 'audio', force: false } })
    expect(el.textContent).toContain('Para o seu ThinkPad X1 Carbon Gen 9')
    expect(el.textContent).toContain('Geral')
    expect(el.textContent).toContain('Funciona segundo a wiki')
    expect(el.querySelectorAll('section[aria-labelledby="resultados"] > ul > li')).toHaveLength(2)
  })

  it('aplicar abre a prévia com o arquivo em /etc, o backup e o comando sudo, e só envia IDs', async () => {
    act(() => root.render(<App />))
    await flush()
    const audio = [...el.querySelectorAll('button[role="radio"]')].find((b) => b.textContent?.includes('Áudio'))!
    act(() => (audio as HTMLButtonElement).click())
    await flush()
    const apply = [...el.querySelectorAll('button')].find((b) => b.textContent === 'Aplicar correção')!
    expect(el.textContent).toContain('options snd_hda_intel model=dell-headset-multi')
    expect(el.textContent).toContain('A fonte impõe condições')
    act(() => apply.click())
    await flush()

    const prepare = calls.find((c) => c.cmd === 'prepare_system_fix')!
    expect(prepare.args).toEqual({ args: { symptom: 'audio', suggestionId: 'a', actionId: ACTION.id, maxAgeHours: expect.any(Number) } })
    const dialog = document.body.querySelector('[role="dialog"]')!
    expect(dialog.textContent).toContain(PATH)
    expect(dialog.textContent).toContain('options snd_hda_intel model=dell-headset-multi')
    expect(dialog.textContent).toContain('Desfazer remove o arquivo criado')
    expect(dialog.textContent).toContain('o FRX não usa sudo')
    expect(dialog.textContent).toContain('sudo install -m 0644 -o root -g root')

    // Instrução condicional: exige a confirmação de leitura.
    const confirm = [...dialog.querySelectorAll('button')].find((b) => b.textContent?.includes('Fazer backup e preparar arquivo'))!
    expect(confirm.disabled).toBe(true)
    act(() => (dialog.querySelector('input[type="checkbox"]') as HTMLInputElement).click())
    expect(confirm.disabled).toBe(false)
    act(() => confirm.click())
    await flush()
    expect(calls.find((c) => c.cmd === 'apply_system_fix')?.args).toEqual({ planId: 'op-1', fingerprint: 'fp-1' })
    expect(document.body.textContent).toContain('Já rodei — conferir')

    const check = [...document.body.querySelectorAll('button')].find((b) => b.textContent?.includes('Já rodei — conferir'))!
    act(() => check.click())
    await flush()
    expect(calls.some((c) => c.cmd === 'verify_fix_operation')).toBe(true)
    expect(document.body.textContent).toContain(`Confirmado em ${PATH}.`)
  })

  it('o fixture de desenvolvimento simula um laptop pelo núcleo', async () => {
    act(() => root.render(<App />))
    await flush()
    const fw = [...el.querySelectorAll('button')].find((b) => b.textContent?.startsWith('Framework Laptop 13'))!
    act(() => fw.click())
    await flush()
    expect(calls.find((c) => c.cmd === 'set_dev_laptop')?.args).toEqual({ presetId: 'framework-13' })
    expect(calls.filter((c) => c.cmd === 'identify_laptop').length).toBeGreaterThan(1)
  })
})
