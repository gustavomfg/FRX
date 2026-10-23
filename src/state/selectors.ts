import { useMemo } from 'react'
import { fixPacks } from '@/data/fixpacks'
import { games } from '@/data/games'
import { useAppState } from './store'

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
