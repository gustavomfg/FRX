import { Gauge, House, LibraryBig, Settings2, type LucideIcon } from 'lucide-react'

export interface NavItem {
  to: string
  label: string
  icon: LucideIcon
  shortcut: string
}

export const NAV: NavItem[] = [
  { to: '/', label: 'Início', icon: House, shortcut: '1' },
  { to: '/biblioteca', label: 'Biblioteca', icon: LibraryBig, shortcut: '2' },
  { to: '/performance', label: 'Performance Lab', icon: Gauge, shortcut: '3' },
  { to: '/configuracoes', label: 'Configurações', icon: Settings2, shortcut: '4' },
]

/** Sugestões são de um jogo: a visão antiga `/correcoes?game=` vai para a página desse jogo. */
export function gameFixTarget(params: URLSearchParams): string {
  const game = params.get('game')
  if (!game) return '/biblioteca'
  const section: Record<string, string> = { smart: 'smart', receitas: 'receitas', descobertas: 'descobertas' }
  const next = new URLSearchParams()
  const secao = section[params.get('view') ?? '']
  if (secao) next.set('secao', secao)
  for (const key of ['pack', 'recipe', 'finding']) {
    const value = params.get(key)
    if (value) next.set(key, value)
  }
  const query = next.toString()
  return `/biblioteca/${encodeURIComponent(game)}${query ? `?${query}` : ''}`
}
