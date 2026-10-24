/**
 * Tipos espelhados de `src-tauri/src/steam/models.rs` e `src-tauri/src/system`.
 * Mantenha os dois lados em sincronia (serde usa camelCase/kebab-case).
 */

export type ScanStatus = 'ok' | 'no-steam'
export type InstallKind = 'native' | 'flatpak' | 'snap' | 'custom'
export type LibraryStatus = 'available' | 'unavailable' | 'inaccessible'
export type InstallationStatus = 'installed' | 'incomplete' | 'unavailable' | 'unknown'
export type ToolKind = 'official' | 'experimental' | 'hotfix' | 'ge-proton' | 'custom' | 'runtime' | 'unknown'
export type ToolOrigin = 'steam-apps' | 'compatibility-tools-d' | 'referenced'
export type ToolSource = 'per-game' | 'global-default' | 'last-used' | 'none'

export interface SteamInstall {
  path: string
  kind: InstallKind
}

export interface LibraryInfo {
  path: string
  status: LibraryStatus
  isPrimary: boolean
  label: string | null
  declaredAppIds: string[]
  gameCount: number
  steamInstall: string
}

export interface CompatTool {
  internalName: string | null
  displayName: string
  kind: ToolKind
  version: string | null
  installPath: string | null
  includesProtonfixes: boolean
  origin: ToolOrigin
}

export interface GameCompatibility {
  tool: CompatTool | null
  toolSource: ToolSource
  lastUsedTool: CompatTool | null
  prefixPath: string | null
  prefixVersion: string | null
  usesProton: boolean | null
}

export interface InstalledGame {
  appId: string
  name: string
  installPath: string | null
  libraryPath: string
  installationStatus: InstallationStatus
  stateFlags: string[]
  sizeOnDisk: number | null
  lastUpdated: string | null
  lastPlayed: string | null
  buildId: string | null
  playtimeMinutes: number | null
  compatibility: GameCompatibility
  source: 'steam'
  steamInstall: string
}

export interface ScanResult {
  status: ScanStatus
  steamInstalls: SteamInstall[]
  libraries: LibraryInfo[]
  games: InstalledGame[]
  tools: CompatTool[]
  globalDefaultTool: CompatTool | null
  warnings: string[]
  scannedAt: string
  durationMs: number
}

export type GpuVendor = 'nvidia' | 'amd' | 'intel' | 'other'

export interface GpuInfo {
  vendor: GpuVendor
  vendorId: string
  deviceId: string
  name: string | null
  driver: string | null
  driverVersion: string | null
  bootVga: boolean
}

export interface SystemProfile {
  distro: string | null
  distroId: string | null
  kernel: string | null
  arch: string
  sessionType: string | null
  desktop: string | null
  cpu: string | null
  cpuThreads: number | null
  ramBytes: number | null
  gpus: GpuInfo[]
  primaryGpu: GpuInfo | null
  vulkanDrivers: string[]
}

export interface ArtResult {
  path: string | null
  source: 'cache' | 'steam-local' | 'steam-cdn' | 'missing' | 'offline'
}
