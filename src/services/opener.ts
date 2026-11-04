import { openUrl } from '@tauri-apps/plugin-opener'
import { isTauri } from './platform'

/** Só as páginas da Arch Wiki citadas nas sugestões (espelha a capability `opener:allow-open-url`). */
export function isSafeExternalUrl(url: string) {
  try {
    const u = new URL(url)
    return u.protocol === 'https:' && u.hostname === 'wiki.archlinux.org' && u.pathname.startsWith('/title/') && !u.username && !u.password && !u.port
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
