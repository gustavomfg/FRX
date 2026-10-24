import { invoke, isTauri as tauriDetected } from '@tauri-apps/api/core'

/**
 * Ponte de plataforma entre a interface e a camada nativa (Rust/Tauri).
 *
 * Dentro do app desktop, as chamadas vão para os comandos definidos em
 * `src-tauri/src/commands.rs`. No navegador (`npm run dev:web`), um
 * substituto demonstrativo responde no lugar, para que a interface continue
 * funcionando sem o Tauri.
 */

export const isTauri = () => tauriDetected()

export interface AppInfo {
  name: string
  version: string
  tauriVersion: string | null
  mode: 'demo' | 'native'
  runtime: 'tauri' | 'browser'
  os: string
  arch: string
  kernel: string | null
  distro: string | null
  session: string | null
  desktop: string | null
}

export const APP_VERSION = '0.2.0'

const browserInfo: AppInfo = {
  name: 'FRX',
  version: APP_VERSION,
  tauriVersion: null,
  mode: 'demo',
  runtime: 'browser',
  os: 'navegador',
  arch: '—',
  kernel: null,
  distro: null,
  session: null,
  desktop: null,
}

let cached: Promise<AppInfo> | null = null

export function getAppInfo(): Promise<AppInfo> {
  if (!cached) {
    cached = isTauri()
      ? invoke<Omit<AppInfo, 'runtime'>>('app_info')
          .then((info) => ({ ...info, runtime: 'tauri' as const }))
          .catch(() => browserInfo)
      : Promise.resolve(browserInfo)
  }
  return cached
}

/** Teste de ida e volta da ponte IPC. */
export function ping(message: string) {
  return isTauri() ? invoke<string>('ping', { message }) : Promise.resolve(`pong: ${message} (navegador)`)
}

export const platform = {
  mode: 'demo' as const,
  version: APP_VERSION,
}
