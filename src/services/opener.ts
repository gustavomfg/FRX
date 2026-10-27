import { openUrl } from '@tauri-apps/plugin-opener'
import { isTauri } from './platform'

/** Domínios que a interface pode abrir no navegador (espelha a capability do Tauri). */
const ALLOWED = ['github.com', 'raw.githubusercontent.com', 'www.protondb.com', 'store.steampowered.com', 'steamdb.info', 'openwinecomponents.org']

export function isSafeExternalUrl(url: string) {
  try {
    const u = new URL(url)
    return u.protocol === 'https:' && (ALLOWED.includes(u.hostname) || (u.hostname === 'www.nvidia.com' && u.pathname.startsWith('/en-us/geforce/technologies/dlss/'))) && !u.username && !u.password && !u.port
  } catch {
    return false
  }
}

/** Abre uma fonte original no navegador padrão do sistema. */
export async function openExternal(url: string) {
  if (!isSafeExternalUrl(url)) throw new Error('Endereço não permitido.')
  if (isTauri()) await openUrl(url)
  else window.open(url, '_blank', 'noopener,noreferrer')
}
