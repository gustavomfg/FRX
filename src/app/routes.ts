import { Gauge, House, LibraryBig, Settings2, Wrench, type LucideIcon } from 'lucide-react'

export interface NavItem {
  to: string
  label: string
  icon: LucideIcon
  shortcut: string
}

export const NAV: NavItem[] = [
  { to: '/', label: 'Início', icon: House, shortcut: '1' },
  { to: '/biblioteca', label: 'Biblioteca', icon: LibraryBig, shortcut: '2' },
  { to: '/correcoes', label: 'Fix Packs', icon: Wrench, shortcut: '3' },
  { to: '/performance', label: 'Performance Lab', icon: Gauge, shortcut: '4' },
  { to: '/configuracoes', label: 'Configurações', icon: Settings2, shortcut: '5' },
]
