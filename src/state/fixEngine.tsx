import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from 'react'
import type { AppliedFix, EngineStatus, Operation } from '@/types/fixEngine'
import { getAppliedFixes, getFixEngineStatus, listFixOperations, onFixProgress } from '@/services/fixEngine'
import { useLibrary } from './library'

/**
 * Estado do Fix Engine na interface: histórico de operações, correções
 * aplicadas (relidas do disco pelo núcleo) e progresso em tempo real.
 * No modo demonstrativo nada disso existe — não há motor para consultar.
 */
interface FixEngineState {
  available: boolean
  status: EngineStatus | null
  operations: Operation[]
  applied: AppliedFix[]
  /** Última atualização de progresso recebida do núcleo. */
  live: Operation | null
  refresh: () => Promise<void>
}

const Ctx = createContext<FixEngineState | null>(null)

export function FixEngineProvider({ children }: { children: ReactNode }) {
  const { mode } = useLibrary()
  const available = mode === 'real'
  const [status, setStatus] = useState<EngineStatus | null>(null)
  const [operations, setOperations] = useState<Operation[]>([])
  const [applied, setApplied] = useState<AppliedFix[]>([])
  const [live, setLive] = useState<Operation | null>(null)

  const refresh = useCallback(async () => {
    if (!available) {
      setStatus(null)
      setOperations([])
      setApplied([])
      return
    }
    try {
      const [s, ops, ap] = await Promise.all([getFixEngineStatus(), listFixOperations(), getAppliedFixes()])
      setStatus(s)
      setOperations(ops)
      setApplied(ap)
    } catch {
      /* núcleo indisponível: a página mostra o erro ao tentar agir */
    }
  }, [available])

  useEffect(() => {
    void refresh()
  }, [refresh])

  useEffect(() => {
    if (!available) return
    let un: (() => void) | undefined
    let gone = false
    void onFixProgress((op) => {
      setLive(op)
      setOperations((all) => [op, ...all.filter((o) => o.id !== op.id)])
    }).then((u) => (gone ? u() : (un = u)))
    return () => {
      gone = true
      un?.()
    }
  }, [available])

  const value = useMemo(() => ({ available, status, operations, applied, live, refresh }), [available, status, operations, applied, live, refresh])
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>
}

export function useFixEngine() {
  const v = useContext(Ctx)
  if (!v) throw new Error('FixEngineProvider ausente')
  return v
}
