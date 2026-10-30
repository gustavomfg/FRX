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

/** A visão antiga `/correcoes?game=` vai para a página do jogo, onde fica o pacote. */
export function gameFixTarget(params: URLSearchParams): string {
  const game = params.get('game')
  return game ? `/biblioteca/${encodeURIComponent(game)}` : '/biblioteca'
}
