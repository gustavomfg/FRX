import type { InstalledGame, ScanResult } from '@/types/steam'
import type { FixFinding, FixReport } from '@/types/fixes'

/** Fixtures no formato exato devolvido pelo Rust (ver `cargo test real_system_scan`). */
export const batman: InstalledGame = {
  appId: '200260',
  name: 'Batman: Arkham City GOTY',
  installPath: '/mnt/Games/SteamLibrary/steamapps/common/Batman Arkham City GOTY',
  libraryPath: '/mnt/Games/SteamLibrary',
  installationStatus: 'installed',
  stateFlags: ['fully-installed'],
  sizeOnDisk: 19948606783,
  lastUpdated: '2026-10-22T22:49:31Z',
  lastPlayed: '2026-10-23T21:55:36Z',
  buildId: '3718300',
  playtimeMinutes: 46,
  compatibility: {
    tool: {
      internalName: 'proton-cachyos-slr',
      displayName: 'proton-cachyos-11.0-20261015 (steam linux runtime)',
      kind: 'custom',
      version: 'cachyos-11.0-20261015-slr',
      installPath: '/usr/share/steam/compatibilitytools.d/proton-cachyos-slr',
      includesProtonfixes: true,
      origin: 'compatibility-tools-d',
    },
    toolSource: 'per-game',
    lastUsedTool: null,
    prefixPath: '/mnt/Games/SteamLibrary/steamapps/compatdata/200260/pfx',
    prefixVersion: 'CachyOS-11.0-100',
    usesProton: true,
  },
  source: 'steam',
  steamInstall: '/home/g/.local/share/Steam',
}

export const scan: ScanResult = {
  status: 'ok',
  steamInstalls: [{ path: '/home/g/.local/share/Steam', kind: 'native' }],
  libraries: [
    { path: '/home/g/.local/share/Steam', status: 'available', isPrimary: true, label: null, declaredAppIds: ['1493710'], gameCount: 0, steamInstall: '/home/g/.local/share/Steam' },
    { path: '/mnt/Games/SteamLibrary', status: 'available', isPrimary: false, label: null, declaredAppIds: ['200260'], gameCount: 1, steamInstall: '/home/g/.local/share/Steam' },
  ],
  games: [batman],
  tools: [],
  globalDefaultTool: null,
  warnings: [],
  scannedAt: '2026-10-24T19:44:23Z',
  durationMs: 15,
}

export function finding(over: Partial<FixFinding> = {}): FixFinding {
  return {
    scope: { kind: 'verified-game', appId: '200260', basis: 'Fixture: AppID explícito na issue' },
    performanceEvidence: { positive: 0, negative: 0, reports: [] },
    id: 'f1',
    gameAppId: '200260',
    title: 'Componente d3dx9_43 (winetricks)',
    description: 'Citado em 1 comentário.',
    category: 'graphics',
    kind: 'tweak',
    actions: [{ kind: 'winetricks', label: 'Instala componente: DirectX 9', value: 'd3dx9_43', conditional: false }],
    sources: [{ provider: 'github', label: 'Comentário', url: 'https://github.com/ValveSoftware/Proton/issues/309#issuecomment-1', referenceId: null, snippet: null, publishedAt: null }],
    requirements: { gpuVendor: null, protonVersions: [], gameVersion: null },
    confidence: 'unverified',
    compatibility: 'unknown',
    compatibilityNote: '',
    experimental: true,
    analysis: 'parsed',
    evidence: { positive: 1, negative: 0 },
    publishedAt: null,
    retrievedAt: '2026-10-24T20:00:00Z',
    ...over,
  }
}

export const report: FixReport = {
  appId: '200260',
  gameName: 'Batman: Arkham City GOTY',
  findings: [finding()],
  providers: [{ id: 'github', name: 'GitHub', state: 'ok', message: null, fetchedAt: null, resetAt: null }],
  identity: { umuId: 'umu-200260', aliases: [] },
  context: { gpuVendor: 'nvidia', toolName: 'proton-cachyos-11.0-20261015', toolIncludesProtonfixes: true },
  retrievedAt: '2026-10-24T20:00:00Z',
  fromCache: false,
  stale: false,
}
