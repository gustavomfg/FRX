import { useMemo } from 'react'
import { fixPacks } from '@/data/fixpacks'
import { games } from '@/data/games'
import type { Game } from '@/types'
import { useAppState } from './store'
import { useFixReports } from './fixes'
import { isRelevant } from '@/lib/findings'

export function useGameFixes(gameId: string | undefined) {
  const { fixStates } = useAppState()
  return useMemo(() => {
    const packs = fixPacks.filter((f) => f.gameId === gameId)
    return {
      packs,
      available: packs.filter((f) => fixStates[f.id] === 'disponivel'),
      applied: packs.filter((f) => fixStates[f.id] === 'aplicado'),
    }
  }, [gameId, fixStates])
}

/** Jogos com problemas e correções ainda não aplicadas. */
export function useAttention() {
  const { fixStates } = useAppState()
  return useMemo(
    () =>
      games.filter(
        (g) => (g.status === 'ajustes' || g.status === 'problematico') && g.fixPackIds.some((id) => fixStates[id] === 'disponivel'),
      ),
    [fixStates],
  )
}

/** Quantidade exibida no selo do cartão: correções demo ou descobertas reais (em cache). */
export function useGameFixCount(game: Game) {
  const { fixStates } = useAppState()
  const { reports } = useFixReports()
  if (game.origin === 'steam') return reports[game.id]?.findings.filter((f) => isRelevant(f) && f.kind !== 'report').length ?? 0
  return game.fixPackIds.filter((id) => fixStates[id] === 'disponivel').length
}
