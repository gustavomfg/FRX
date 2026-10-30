/** Synthetic IPC fixtures only. No download, game file or performance measurement is real. */
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { act, useState, type ReactNode } from 'react'
import { createRoot } from 'react-dom/client'
import { MemoryRouter } from 'react-router-dom'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { emit } from '@tauri-apps/api/event'
import { MotionGlobalConfig } from 'motion/react'
import { AppStoreProvider } from '@/state/store'
import { LibraryProvider } from '@/state/library'
import { FixReportsProvider } from '@/state/fixes'
import { FixEngineProvider } from '@/state/fixEngine'
import { ApplyFixDialog, type ApplyTarget } from '@/components/fix/ApplyFixDialog'
import { OperationHistory } from '@/components/fix/OperationHistory'
import type { ArtifactCandidate } from '@/types/fixes'
import type { FixPlan, Operation } from '@/types/fixEngine'
import type { EnvironmentContext, GpuInfo } from '@/types/steam'
import { scan } from './fixtures'

;(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
MotionGlobalConfig.skipAnimations = true
const g = globalThis as unknown as { isTauri: boolean }
const source = { provider: 'github' as const, label: 'Synthetic maintained release', url: 'https://github.com/fixture/project/releases/tag/v1', referenceId: null, snippet: 'Synthetic test data', publishedAt: null }
const gpu = (name: string, vendor: GpuInfo['vendor'], deviceId: string): GpuInfo => ({ vendor, vendorId: vendor === 'nvidia' ? '10de' : '1002', deviceId, name, driver: vendor === 'nvidia' ? 'nvidia' : 'amdgpu', driverVersion: 'fixture-driver', bootVga: false })
const environment: EnvironmentContext = { detectedGpus: [gpu('Fixture NVIDIA GPU', 'nvidia', '0001'), gpu('Fixture AMD GPU', 'amd', '0002')], selectedGpu: null, selection: 'unknown', installedVulkanDrivers: ['fixture-icd'], activeVulkanDriver: null, cpuArch: 'x86_64', sessionType: 'wayland', protonVersion: 'Fixture Proton', protonOrigin: 'referenced' }
const artifact: ArtifactCandidate = { id: 'artifact-synthetic', appId: '200260', scope: { kind: 'verified-game', appId: '200260', basis: 'Synthetic exact game reference' }, recommendationSource: source, repository: 'fixture/project', releaseTag: 'v1', releaseUrl: source.url, metadataUrl: 'https://api.github.com/repos/fixture/project/releases/tags/v1', releaseId: 1, publisher: 'fixture', assetId: 2, assetName: 'fixture-config.zip', assetUrl: 'https://github.com/fixture/project/releases/download/v1/fixture-config.zip', sizeBytes: 1024, contentType: 'application/zip', sha256: 'a'.repeat(64), signatureUrl: null, format: 'zip', platform: 'windows', architecture: 'any', files: [{ sourcePath: 'config/fixture.cfg', targetPath: 'Config/fixture.cfg', sha256: 'b'.repeat(64), replace: true, expectedOriginalSha256: 'c'.repeat(64) }], requirements: { gpuVendor: null, protonVersions: [], gameVersion: null }, trust: 'origin-and-integrity', reasons: [], license: 'Synthetic license', publishedAt: null }
const plan: FixPlan = { planId: 'synthetic-plan', fingerprint: 'synthetic-fp', recipeId: 'gen.200260.aaaaaaaaaaaaaaaa', recipeVersion: '1.1.0', recipeTitle: 'Fixture de configuração', appId: '200260', gameName: 'Synthetic Game', status: 'ready', mode: 'automatic', reasons: [], checks: [{ label: 'Destino', state: 'ok', detail: 'Synthetic authorized target' }], changes: [{ kind: 'game-file', target: '/synthetic/steamapps/common/Game/Config/fixture.cfg', before: 'sha256:original', after: 'sha256:published', summary: 'Substituir somente o arquivo autorizado, com backup.' }], manualLine: null, backupNote: 'Backup recuperável antes da instalação.', steamRunning: false, gameRunning: false, risk: 'moderate', purpose: 'compatibility', trust: 'generated', effects: ['Synthetic text configuration'], sideEffects: [], sources: [source], generated: null, pack: null, createdAt: '2026-10-27T12:00:00Z', expiresAt: 9999999999, artifacts: [artifact], environment, hardwareMatch: { state: 'unknown', reasons: [], limitations: ['A GPU de execução não foi confirmada.'] } }
const operation = (state: Operation['state'], message = 'Fixture aplicada e verificada.'): Operation => ({ schema: 1, id: 'synthetic-op', recipeId: plan.recipeId, recipeVersion: plan.recipeVersion, recipeTitle: plan.recipeTitle, appId: plan.appId, gameName: plan.gameName, state, mode: 'automatic', steps: [{ step: { type: 'game-file', root: '/synthetic/steamapps/common/Game', rel: 'Config/fixture.cfg', target: '/synthetic/steamapps/common/Game/Config/fixture.cfg', beforeSha: 'c'.repeat(64), content: '[Fixture]\nenabled=true', artifact }, status: 'verified', backup: { path: '/synthetic/frx/backups/synthetic-op/fixture.cfg', existed: true, sha256: 'c'.repeat(64), size: 100, mode: 384 }, afterSha: 'b'.repeat(64), rollbackBackup: null }], planFingerprint: plan.fingerprint, createdAt: plan.createdAt, updatedAt: plan.createdAt, appliedAt: state === 'applied' ? plan.createdAt : null, rolledBackAt: state === 'rolled-back' ? plan.createdAt : null, message, pending: null, events: [{ at: plan.createdAt, state, message }], recipeTrust: 'generated', findingIds: ['synthetic-finding'], artifacts: [artifact] })

function mount(node: ReactNode) {
  const el = document.createElement('div'); document.body.appendChild(el)
  const root = createRoot(el)
  act(() => root.render(<AppStoreProvider><LibraryProvider><FixReportsProvider><FixEngineProvider><MemoryRouter>{node}</MemoryRouter></FixEngineProvider></FixReportsProvider></LibraryProvider></AppStoreProvider>))
  return { unmount: () => act(() => root.unmount()) }
}
const flush = (ms = 40) => act(() => new Promise((resolve) => setTimeout(resolve, ms)))
const text = () => document.body.textContent ?? ''
const button = (label: string) => [...document.body.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === label)
const click = async (target: HTMLElement | undefined) => { await act(async () => target!.click()); await flush() }
async function confirmDownload() {
  await click(button('Continuar'))
  const checkboxes = [...document.body.querySelectorAll<HTMLInputElement>('input[type=checkbox]')]
  expect(checkboxes.length).toBe(2)
  expect(button('Baixar e instalar correção')?.disabled).toBe(true)
  await click(checkboxes[0])
  expect(button('Baixar e instalar correção')?.disabled).toBe(true)
  await click(checkboxes[1])
  expect(button('Baixar e instalar correção')?.disabled).toBe(false)
  await click(button('Baixar e instalar correção'))
}

describe('File Fix Engine — interface com fixtures isoladas', () => {
  let calls: { cmd: string; args: Record<string, unknown> }[]
  let operations: Operation[]
  let resultState: Operation['state']
  let paused: boolean
  let resolveApply: ((op: Operation) => void) | undefined
  beforeEach(() => {
    calls = []; operations = []; resultState = 'applied'; paused = false; resolveApply = undefined
    localStorage.clear(); g.isTauri = true
    mockIPC(async (cmd, args) => {
      calls.push({ cmd, args: (args ?? {}) as Record<string, unknown> })
      switch (cmd) {
        case 'get_installed_games': case 'scan_installed_games': return scan
        case 'get_system_profile': return { gpus: environment.detectedGpus, primaryGpu: environment.detectedGpus[0], vulkanDrivers: environment.installedVulkanDrivers, arch: 'x86_64' }
        case 'get_cached_fix_reports': case 'get_applied_fixes': return []
        case 'get_fix_engine_status': return { catalogVersion: 'fixture', recipes: 0, reviewed: 0, issues: [], recoveryRequired: 0, pendingManual: 0, busy: false, dataDir: '/synthetic' }
        case 'list_fix_operations': return operations
        case 'prepare_fix_plan': return plan
        case 'apply_fix_plan': {
          await emit('fix-engine://progress', { ...operation('downloading'), artifactProgress: { downloadedBytes: 512, totalBytes: 1024, assetName: artifact.assetName } })
          if (paused) return new Promise<Operation>((resolve) => { resolveApply = resolve })
          await emit('fix-engine://progress', operation('verifying-artifacts'))
          const op = operation(resultState, resultState === 'failed' ? 'SHA-256 diferente. Os destinos não foram instalados.' : undefined)
          operations = [op]
          return op
        }
        case 'cancel_fix_operation': {
          const op = operation('cancelled', 'Download cancelado antes da instalação.')
          operations = [op]; resolveApply?.(op); return true
        }
        case 'prepare_fix_rollback': return { operationId: 'synthetic-op', fingerprint: 'synthetic-rollback-fp', status: 'ready', mode: 'automatic', reasons: [], changes: [{ kind: 'game-file', target: 'Config/fixture.cfg', before: 'sha256:installed', after: 'sha256:original', summary: 'Restaurar o original com backup verificado.' }], manualLine: null, steamRunning: false, gameRunning: false }
        case 'rollback_fix': operations = [operation('rolled-back', 'Arquivo original restaurado e verificado.')]; return operations[0]
        default: return null
      }
    }, { shouldMockEvents: true })
  })
  afterEach(() => { clearMocks(); g.isTauri = false; document.body.innerHTML = '' })

  it('mostra fonte, hash, destinos e hardware, exige autorização de download e permite desfazer após recarregar o histórico', async () => {
    const m = mount(<ApplyFixDialog target={{ kind: 'apply', appId: plan.appId, recipeId: plan.recipeId, title: plan.recipeTitle }} onClose={() => {}} />)
    await flush()
    expect(text()).toContain('Origem e hash identificados')
    expect(text()).toContain(artifact.sha256)
    expect(text()).toContain('Config/fixture.cfg')
    expect(text()).toContain('Substituir com backup')
    expect(text()).toContain('Não confirmada por uma captura')
    expect(text()).toContain('Fixture NVIDIA GPU · Fixture AMD GPU')
    expect(calls.some((c) => c.cmd === 'apply_fix_plan')).toBe(false)
    await confirmDownload()
    expect(text()).toContain('Fixture aplicada e verificada.')
    expect(calls.find((c) => c.cmd === 'apply_fix_plan')?.args).toEqual({ planId: plan.planId, fingerprint: plan.fingerprint, appId: plan.appId })
    m.unmount()
    function History() { const [target, setTarget] = useState<ApplyTarget | null>(null); return <><OperationHistory onAct={setTarget} />{target && <ApplyFixDialog target={target} onClose={() => setTarget(null)} />}</> }
    const reloaded = mount(<History />)
    await flush()
    await click(document.body.querySelector<HTMLElement>('[aria-label="Histórico de correções"] button[aria-expanded]') ?? undefined)
    expect(text()).toContain('Backup:')
    expect(text()).toContain('Config/fixture.cfg')
    await click(button('Desfazer'))
    expect(calls.some((c) => c.cmd === 'rollback_fix')).toBe(false)
    await click(button('Desfazer agora'))
    expect(text()).toContain('Arquivo original restaurado e verificado.')
    reloaded.unmount()
  })

  it('usa o ID da operação para cancelar o staging e não anuncia instalação concluída', async () => {
    paused = true
    const m = mount(<ApplyFixDialog target={{ kind: 'apply', appId: plan.appId, recipeId: plan.recipeId, title: plan.recipeTitle }} onClose={() => {}} />)
    await flush(); await confirmDownload()
    expect(text()).toContain('512 bytes de 1,0 KiB')
    await click(button('Cancelar download'))
    expect(calls.find((c) => c.cmd === 'cancel_fix_operation')?.args).toEqual({ operationId: 'synthetic-op' })
    expect(text()).toContain('Download cancelado antes da instalação.')
    expect(button('Medir desempenho')).toBeUndefined()
    m.unmount()
  })

  it('mostra falha de integridade sem apresentar sucesso ou resultado de FPS', async () => {
    resultState = 'failed'
    const m = mount(<ApplyFixDialog target={{ kind: 'apply', appId: plan.appId, recipeId: plan.recipeId, title: plan.recipeTitle }} onClose={() => {}} />)
    await flush(); await confirmDownload()
    expect(text()).toContain('SHA-256 diferente. Os destinos não foram instalados.')
    expect(button('Medir desempenho')).toBeUndefined()
    expect(text()).not.toContain('Fixture aplicada e verificada.')
    m.unmount()
  })
})
