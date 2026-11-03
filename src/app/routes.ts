import { House, Settings2, type LucideIcon } from 'lucide-react'

export interface NavItem {
  to: string
  label: string
  icon: LucideIcon
  shortcut: string
}

/**
 * Rotas do Hardware Fixer. As telas de jogos (Biblioteca, Performance Lab,
 * Smart Packs, Correções) continuam no repositório, mas fora das rotas.
 */
export const NAV: NavItem[] = [
  { to: '/', label: 'Início', icon: House, shortcut: '1' },
  { to: '/configuracoes', label: 'Configurações', icon: Settings2, shortcut: '2' },
]

/** A visão antiga `/correcoes?game=` vai para a página do jogo, onde fica o pacote. */
export function gameFixTarget(params: URLSearchParams): string {
  const game = params.get('game')
  return game ? `/biblioteca/${encodeURIComponent(game)}` : '/biblioteca'
}
