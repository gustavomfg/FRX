import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import type { Game } from '@/types'
import type { ScanResult, SystemProfile } from '@/types/steam'
import { games as demoGames } from '@/data/games'
import { getInstalledGames, getSystemProfile, scanInstalledGames } from '@/services/games'
import { errorMessage } from '@/services/native'
import { isTauri } from '@/services/platform'
import { steamToGame } from '@/lib/steamGame'
import { useDispatch, useSettings } from './store'

/**
 * Fonte da biblioteca.
 * - `real`: app desktop lendo a Steam pelo Rust.
 * - `demo`: navegador, ou quando o usuário liga "Usar dados demonstrativos".
 * As duas nunca se misturam.
 */
export type DataMode = 'real' | 'demo'
export type LibraryPhase = 'idle' | 'loading' | 'scanning' | 'ready' | 'error'

interface LibraryState {
  mode: DataMode
  phase: LibraryPhase
  games: Game[]
  scan: ScanResult | null
  /** A lista exibida veio da última varredura salva (a nova ainda não terminou). */
  fromCache: boolean
  error: string | null
  system: SystemProfile | null
  rescan: () => Promise<void>
  byId: (id: string | undefined) => Game | undefined
}

const Ctx = createContext<LibraryState | null>(null)

export function LibraryProvider({ children }: { children: ReactNode }) {
  const settings = useSettings()
  const dispatch = useDispatch()
  const mode: DataMode = isTauri() && !settings.useDemoData ? 'real' : 'demo'
  const [phase, setPhase] = useState<LibraryPhase>(mode === 'real' ? 'loading' : 'ready')
  const [scan, setScan] = useState<ScanResult | null>(null)
  const [fromCache, setFromCache] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [system, setSystem] = useState<SystemProfile | null>(null)
  const running = useRef(false)

  const rescan = useCallback(async () => {
    if (mode !== 'real' || running.current) return
    running.current = true
    setPhase('scanning')
    setError(null)
    try {
      const r = await scanInstalledGames()
      setScan(r)
      setFromCache(false)
      setPhase('ready')
      const playable = r.games.length
      dispatch({
        type: 'activity',
        kind: 'scan',
        text: r.status === 'no-steam' ? 'Nenhuma instalação da Steam encontrada' : `Biblioteca Steam verificada — ${playable} jogo${playable === 1 ? '' : 's'} em ${r.libraries.length} biblioteca${r.libraries.length === 1 ? '' : 's'}`,
      })
    } catch (e) {
      setError(errorMessage(e))
      setPhase('error')
    } finally {
      running.current = false
    }
  }, [mode, dispatch])

  // Abertura: mostra a última varredura salva na hora e atualiza em seguida.
  useEffect(() => {
    if (mode !== 'real') {
      setPhase('ready')
      return
    }
    let alive = true
    setPhase('loading')
    getSystemProfile()
      .then((p) => alive && setSystem(p))
      .catch(() => {})
    getInstalledGames()
      .then((cached) => {
        if (!alive) return
        if (cached) {
          setScan(cached)
          setFromCache(true)
          setPhase('ready')
        }
        if (!cached || settings.scanOnStartup) void rescan()
      })
      .catch(() => alive && void rescan())
    return () => {
      alive = false
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode])

  const games = useMemo(() => (mode === 'demo' ? demoGames : (scan?.games ?? []).map(steamToGame)), [mode, scan])
  const byId = useCallback((id: string | undefined) => games.find((g) => g.id === id), [games])

  const value = useMemo<LibraryState>(
    () => ({ mode, phase, games, scan, fromCache, error, system, rescan, byId }),
    [mode, phase, games, scan, fromCache, error, system, rescan, byId],
  )
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>
}

export function useLibrary() {
  const v = useContext(Ctx)
  if (!v) throw new Error('LibraryProvider ausente')
  return v
}
