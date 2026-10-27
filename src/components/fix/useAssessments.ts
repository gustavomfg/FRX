import { useCallback, useEffect, useState } from 'react'
import { analyzeFixFindings } from '@/services/fixEngine'
import { useFixEngine } from '@/state/fixEngine'
import { useLibrary } from '@/state/library'
import { useFixReports } from '@/state/fixes'
import type { Assessment } from '@/types/fixEngine'

/**
 * Gerador dinâmico do núcleo para um jogo: diz, para cada descoberta, se o
 * FRX consegue gerar uma correção aplicável e, se não, o motivo exato.
 * Refaz a análise quando a pesquisa ou o histórico do Fix Engine mudam.
 */
export function useAssessments(appId: string | null) {
  const engine = useFixEngine()
  const fixes = useFixReports()
  const library = useLibrary()
  const [result, setResult] = useState<{ appId: string; byFinding: Record<string, Assessment> } | null>(null)
  const [loading, setLoading] = useState(false)
  const [epoch, setEpoch] = useState(0)
  const refresh = useCallback(() => setEpoch((n) => n + 1), [])
  const [error, setError] = useState<string | null>(null)
  const retrieved = appId ? fixes.reports[appId]?.retrievedAt : undefined
  const context = JSON.stringify([library.system, library.games.find((g) => g.id === appId)?.steam])
  useEffect(() => {
    if (!appId || !engine.available) return
    window.addEventListener('focus', refresh)
    return () => window.removeEventListener('focus', refresh)
  }, [appId, engine.available, refresh])
  const stamp = engine.operations.map((o) => o.id + o.state).join()

  useEffect(() => {
    setResult(null)
    setError(null)
    if (!engine.available || !appId || !retrieved) {
      setLoading(false)
      return
    }
    let gone = false
    setLoading(true)
    analyzeFixFindings(appId)
      .then((list) => {
        if (gone) return
        setResult({ appId, byFinding: Object.fromEntries(list.filter((a) => a.appId === appId).map((a) => [a.findingId, a])) })
        setError(null)
      })
      .catch((e) => !gone && setError(String(e?.message ?? e)))
      .finally(() => !gone && setLoading(false))
    return () => void (gone = true)
  }, [engine.available, appId, retrieved, stamp, epoch, context])

  return { byFinding: result?.appId === appId ? result.byFinding : {}, loading, error, refresh }
}
