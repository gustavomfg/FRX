import { useEffect, useState } from 'react'
import { getGameArtUrl, type ArtKind } from '@/services/games'
import { isTauri } from '@/services/platform'

/** Cache em memória da sessão: evita repetir o `invoke` ao navegar entre páginas. */
const memo = new Map<string, Promise<string | null>>()

/** Arte oficial do jogo, carregada de forma assíncrona e só para jogos reais. */
export function useGameArt(appId: string | undefined, kind: ArtKind) {
  const key = appId ? `${appId}:${kind}` : ''
  const [url, setUrl] = useState<string | null>(null)
  const [loaded, setLoaded] = useState(false)

  useEffect(() => {
    setUrl(null)
    setLoaded(false)
    if (!appId || !isTauri()) return
    let alive = true
    let p = memo.get(key)
    if (!p) {
      p = getGameArtUrl(appId, kind)
        .then((r) => {
          // Sem conexão: não memoriza, para tentar de novo mais tarde.
          if (r.source === 'offline') memo.delete(key)
          return r.url
        })
        .catch(() => {
          memo.delete(key)
          return null
        })
      memo.set(key, p)
    }
    p.then((u) => alive && setUrl(u))
    return () => {
      alive = false
    }
  }, [appId, kind, key])

  return { url, loaded, onLoad: () => setLoaded(true), onError: () => setUrl(null) }
}
