import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import type { CaptureRequest, CaptureStatus, MangoHudStatus, Session, SessionInput, SessionMeta, SessionPatch } from '@/types/performance'
import * as perf from '@/services/performance'
import { errorMessage } from '@/services/native'
import { useLibrary } from './library'
import { useDispatch, useToast } from './store'

/**
 * Estado do Performance Lab real: histórico, MangoHud e captura assistida.
 * Fica no topo do app para a captura continuar enquanto o usuário navega.
 */
interface PerformanceState {
  enabled: boolean
  mangohud: MangoHudStatus | null
  checkMangoHud: () => Promise<void>
  sessions: SessionMeta[]
  loading: boolean
  error: string | null
  reload: () => Promise<void>
  getSession: (id: string) => Promise<Session>
  importLog: (path: string, input: SessionInput) => Promise<SessionMeta | null>
  importStage: string | null
  update: (id: string, patch: SessionPatch) => Promise<boolean>
  remove: (id: string) => Promise<boolean>
  capture: CaptureStatus | null
  prepare: (r: CaptureRequest) => Promise<CaptureStatus | null>
  start: () => Promise<void>
  logging: (on: boolean) => Promise<void>
  finish: () => Promise<void>
  cancel: () => Promise<void>
  reset: () => Promise<void>
}

const Ctx = createContext<PerformanceState | null>(null)

export function PerformanceProvider({ children }: { children: ReactNode }) {
  const { mode } = useLibrary()
  const enabled = mode === 'real'
  const enabledRef = useRef(enabled)
  enabledRef.current = enabled
  const toast = useToast()
  const dispatch = useDispatch()
  const [mangohud, setMangohud] = useState<MangoHudStatus | null>(null)
  const [sessions, setSessions] = useState<SessionMeta[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [importStage, setImportStage] = useState<string | null>(null)
  const [capture, setCapture] = useState<CaptureStatus | null>(null)
  const cache = useRef(new Map<string, Session>())
  const lastCompleted = useRef<string | null>(null)

  const fail = useCallback((title: string, e: unknown) => toast({ tone: 'error', title, body: errorMessage(e) }), [toast])

  const reload = useCallback(async () => {
    if (!enabled) return
    setLoading(true)
    try {
      const list = await perf.listSessions()
      if (!enabledRef.current) return
      setSessions(list)
      setError(null)
    } catch (e) {
      setError(errorMessage(e))
    } finally {
      setLoading(false)
    }
  }, [enabled])

  const checkMangoHud = useCallback(async () => {
    if (!enabled) return
    try {
      const status = await perf.detectMangoHud()
      if (enabledRef.current) setMangohud(status)
    } catch {
      setMangohud(null)
    }
  }, [enabled])

  useEffect(() => {
    if (!enabled) {
      setSessions([])
      setMangohud(null)
      setCapture(null)
      setImportStage(null)
      setError(null)
      cache.current.clear()
      return
    }
    let alive = true
    void reload()
    void checkMangoHud()
    perf.getCaptureStatus().then((s) => alive && setCapture(s)).catch(() => {})
    const subs = [
      perf.onCaptureStatus((s) => { if (alive) setCapture(s) }),
      perf.onImportProgress((s) => { if (alive) setImportStage(s) }),
    ]
    return () => {
      alive = false
      subs.forEach((p) => p.then((un) => un()).catch(() => {}))
    }
  }, [enabled, reload, checkMangoHud])

  // Captura concluída: atualiza o histórico e avisa (uma vez por sessão).
  useEffect(() => {
    if (capture?.state === 'completed' && capture.sessionId && lastCompleted.current !== capture.sessionId) {
      lastCompleted.current = capture.sessionId
      void reload()
      toast({ tone: 'ok', title: 'Benchmark registrado', body: capture.message ?? undefined })
      dispatch({ type: 'activity', kind: 'benchmark', gameId: capture.plan?.appId, text: `Benchmark real em ${capture.plan?.gameName ?? 'jogo'}: ${capture.message ?? 'sessão salva'}` })
    }
  }, [capture, reload, toast, dispatch])

  const getSession = useCallback(async (id: string) => {
    if (!enabledRef.current) throw new Error('Disponível apenas no modo real do app desktop.')
    const hit = cache.current.get(id)
    if (hit) return hit
    const s = await perf.getSession(id)
    cache.current.set(id, s)
    return s
  }, [])

  const importLog = useCallback(
    async (path: string, input: SessionInput) => {
      if (!enabledRef.current) return null
      try {
        const meta = await perf.importBenchmark(path, input)
        await reload()
        dispatch({ type: 'activity', kind: 'benchmark', gameId: meta.appId ?? undefined, text: `Benchmark importado: ${meta.label}` })
        return meta
      } catch (e) {
        fail('Não foi possível importar', e)
        return null
      } finally {
        setImportStage(null)
      }
    },
    [reload, dispatch, fail],
  )

  const update = useCallback(
    async (id: string, patch: SessionPatch) => {
      if (!enabledRef.current) return false
      try {
        await perf.updateSession(id, patch)
        cache.current.delete(id)
        await reload()
        return true
      } catch (e) {
        fail('Não foi possível salvar', e)
        return false
      }
    },
    [reload, fail],
  )

  const remove = useCallback(
    async (id: string) => {
      if (!enabledRef.current) return false
      try {
        await perf.deleteSession(id)
        cache.current.delete(id)
        await reload()
        return true
      } catch (e) {
        fail('Não foi possível excluir', e)
        return false
      }
    },
    [reload, fail],
  )

  const wrap = useCallback(
    <T,>(title: string, fn: () => Promise<T>) =>
      async () => {
        if (!enabledRef.current) return
        try {
          const r = await fn()
          if (r && typeof r === 'object' && 'state' in (r as object)) setCapture(r as unknown as CaptureStatus)
        } catch (e) {
          fail(title, e)
        }
      },
    [fail],
  )

  const prepare = useCallback(
    async (r: CaptureRequest) => {
      if (!enabledRef.current) return null
      try {
        const s = await perf.prepareBenchmark(r)
        setCapture(s)
        return s
      } catch (e) {
        fail('Não foi possível preparar a sessão', e)
        return null
      }
    },
    [fail],
  )

  const value = useMemo<PerformanceState>(
    () => ({
      enabled,
      mangohud,
      checkMangoHud,
      sessions,
      loading,
      error,
      reload,
      getSession,
      importLog,
      importStage,
      update,
      remove,
      capture,
      prepare,
      start: wrap('Não foi possível começar', perf.startCapture),
      logging: (on: boolean) => wrap(on ? 'Não foi possível iniciar a gravação' : 'Não foi possível parar a gravação', () => perf.setCaptureLogging(on))(),
      finish: wrap('Não foi possível encerrar', perf.finishCapture),
      cancel: wrap('Não foi possível cancelar', perf.cancelCapture),
      reset: wrap('Não foi possível limpar', perf.resetCapture),
    }),
    [enabled, mangohud, checkMangoHud, sessions, loading, error, reload, getSession, importLog, importStage, update, remove, capture, prepare, wrap],
  )
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>
}

export function usePerformance() {
  const v = useContext(Ctx)
  if (!v) throw new Error('PerformanceProvider ausente')
  return v
}
