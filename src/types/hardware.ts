/** Rust: `system/dmi.rs` (`LaptopIdentity`), com `simulated` vindo de `identify_laptop`. */
export interface LaptopIdentity {
  vendor: string | null
  model: string | null
  boardName: string | null
  productVersion: string | null
  /** `productVersion` na Lenovo, senão `model`. Use este para exibir. */
  displayName: string | null
  /** Laptop de teste do fixture de desenvolvimento (só em build de debug). */
  simulated?: boolean
}

/** Rust: `system/mod.rs` (`SystemProfile`). */
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

/** Rust: `system/hardware.rs` (`GpuInfo`). */
export interface GpuInfo {
  pciAddress: string | null
  gpuFamily: string | null
  vramBytes: number | null
  vendor: 'nvidia' | 'amd' | 'intel' | 'other'
  vendorId: string
  deviceId: string
  name: string | null
  driver: string | null
  driverVersion: string | null
  bootVga: boolean
}

/** Rust: `HardwareSymptom` em `fixes/providers/arch_wiki.rs`. */
export type HardwareSymptom = 'wifi' | 'audio' | 'touchpad' | 'suspend' | 'firmware'

/** A sugestão veio da página do modelo detectado ou da página geral do sintoma. */
export type Fit = 'model' | 'general'

/** Coluna "Working?" da tabela de hardware da página do modelo. */
export type Support = 'works' | 'partial' | 'broken' | 'unknown'

/** Rust: `fixes/models.rs` (`ProviderId`, `ProviderState`, `ProviderStatus`, `ProviderInfo`). */
export type ProviderId = 'arch-wiki'

export type ProviderState = 'ok' | 'cached' | 'stale' | 'no-data' | 'unavailable' | 'rate-limited'

export interface ProviderStatus {
  id: ProviderId
  name: string
  state: ProviderState
  message: string | null
  fetchedAt: string | null
  resetAt: string | null
}

export interface ProviderInfo {
  id: ProviderId
  name: string
  enabled: boolean
  description: string
  reason: string | null
}

export interface WikiSuggestion {
  id: string
  title: string
  page: string
  section: string | null
  url: string
  /** Texto simples para citação. Nunca é executado. */
  excerpt: string | null
  fit: Fit
  /** Instruções da seção que o Fix Engine sabe aplicar. */
  actions: WikiAction[]
}

/** Rust: `fix_engine::system::SystemAction`. Só exibido; a interface nunca o envia. */
export type SystemAction =
  | { type: 'add_modprobe_option'; module: string; options: string[] }
  | { type: 'add_systemd_service'; unit: { template: 'reload_module_around_sleep'; module: string } }
  | { type: 'add_kernel_parameters'; params: string[] }

export interface WikiAction {
  /** Hash da ação; é o que a interface envia de volta. */
  id: string
  action: SystemAction
  /** Trecho literal reconhecido na wiki. */
  snippet: string
  /** Frase da wiki em volta da instrução. */
  context: string
  /** A frase impõe condições ("só em Intel", "se…"). */
  conditional: boolean
  /** Checagem leve feita na busca (hoje: GRUB existe?). `null` = sem checagem prévia. */
  check: { applicable: boolean; detail: string } | null
}

/** Rust: `commands/hardware.rs` (`HardwareReport`). */
export interface HardwareReport {
  identity: LaptopIdentity
  simulated: boolean
  symptom: HardwareSymptom
  retrievedAt: string
  status: ProviderStatus
  modelPage: string | null
  support: Support | null
  suggestions: WikiSuggestion[]
}
