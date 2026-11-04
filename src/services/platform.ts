import { invoke, isTauri as tauriDetected } from '@tauri-apps/api/core'

/**
 * Ponte de plataforma: detecta o app desktop e expõe as informações do app.
 * No navegador (`npm run dev:web`) não há camada nativa; as chamadas avisam
 * que o recurso é do app desktop.
 */

export const isTauri = () => tauriDetected()

export interface AppInfo {
  name: string
  version: string
  tauriVersion: string | null
  runtime: 'tauri' | 'browser'
  os: string
  arch: string
  kernel: string | null
  distro: string | null
  session: string | null
  desktop: string | null
}

export const APP_VERSION = '0.5.0'

const browserInfo: AppInfo = {
  name: 'FRX',
  version: APP_VERSION,
  tauriVersion: null,
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
