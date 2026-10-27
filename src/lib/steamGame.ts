import type { CoverArt, Game } from '@/types'
import type { InstalledGame, CompatTool } from '@/types/steam'
import { hashString } from './random'

/**
 * Converte um jogo detectado pelo Rust no modelo usado pela interface.
 * Campos que a Steam não informa ficam vazios — nada é inventado.
 */

const PALETTES: CoverArt['palette'][] = [
  ['#16151a', '#3a3440', '#6e6474', '#c9b8a8', '#0c0b0e'],
  ['#18211b', '#2f4a35', '#5f8a5a', '#d9d3a3', '#0d140f'],
  ['#1d2a33', '#3f6573', '#89b2b0', '#f0d9a8', '#0e171c'],
  ['#2b1d24', '#7a3b2e', '#d8743e', '#f2b25b', '#1a1012'],
  ['#0b0c14', '#1b1f3a', '#3d4a7a', '#e8c48a', '#06070c'],
  ['#221e18', '#5a4a36', '#a88e64', '#efe2c4', '#14110d'],
]
const SCENES: CoverArt['scene'][] = ['montanhas', 'mar', 'cidade', 'floresta', 'dunas', 'espaco', 'caverna']
const ORBS: NonNullable<CoverArt['orb']>[] = ['sol', 'lua', 'planeta', 'nenhum']

/** Arte provisória (determinística) enquanto a capa oficial não carrega. */
export function placeholderCover(appId: string): CoverArt {
  const h = hashString(appId)
  return { scene: SCENES[h % SCENES.length], palette: PALETTES[(h >>> 3) % PALETTES.length], seed: h % 1000, orb: ORBS[(h >>> 7) % ORBS.length] }
}

/** Nome curto para ferramentas com nomes longos (ex.: proton-cachyos). */
export function toolLabel(tool: CompatTool | null | undefined) {
  if (!tool) return null
  return tool.displayName.replace(/\s*\(steam linux runtime\)\s*$/i, ' (SLR)')
}

export function steamToGame(s: InstalledGame): Game {
  const c = s.compatibility
  const tool = toolLabel(c.tool)
  return {
    id: s.appId,
    title: s.name,
    origin: 'steam',
    sizeGb: s.sizeOnDisk ? s.sizeOnDisk / 1e9 : undefined,
    playtimeH: s.playtimeMinutes != null ? s.playtimeMinutes / 60 : undefined,
    lastPlayed: s.lastPlayed ?? undefined,
    store: 'Steam',
    runner: c.usesProton ? 'Proton' : 'Desconhecido',
    protonVersion: tool ?? (c.prefixVersion ? `Prefixo ${c.prefixVersion}` : 'Ferramenta não identificada'),
    status: 'nao-testado',
    cover: placeholderCover(s.appId),
    issueIds: [],
    fixPackIds: [],
    steam: s,
  }
}

export const INSTALL_LABEL: Record<InstalledGame['installationStatus'], string> = {
  installed: 'Instalado',
  incomplete: 'Incompleto',
  unavailable: 'Indisponível',
  unknown: 'Estado desconhecido',
}

export const TOOL_SOURCE_LABEL: Record<InstalledGame['compatibility']['toolSource'], string> = {
  'per-game': 'Definida nas propriedades do jogo',
  'global-default': 'Padrão global do Steam Play',
  'last-used': 'Deduzida do prefixo (última execução)',
  none: 'Não identificada',
}
