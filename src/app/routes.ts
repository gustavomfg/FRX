import { House, Settings2, type LucideIcon } from 'lucide-react'

export interface NavItem {
  to: string
  label: string
  icon: LucideIcon
  shortcut: string
}

/** Rotas do Hardware Fixer. */
export const NAV: NavItem[] = [
  { to: '/', label: 'Início', icon: House, shortcut: '1' },
  { to: '/configuracoes', label: 'Configurações', icon: Settings2, shortcut: '2' },
]
