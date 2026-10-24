import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from 'react'
import type { FixReport } from '@/types/fixes'
import { getCachedFixReports, searchGameFixes } from '@/services/fixes'
import { errorMessage } from '@/services/native'
import { useLibrary } from './library'
import { useDispatch, useSettings } from './store'

/** Relatórios de correções descobertas (somente leitura), por AppID. */
interface FixReportsState {
  reports: Record<string, FixReport>
  searching: Record<string, boolean>
  errors: Record<string, string>
  search: (appId: string, name: string, force?: boolean) => Promise<FixReport | null>
  reload: () => Promise<void>
}

const Ctx = createContext<FixReportsState | null>(null)

export function FixReportsProvider({ children }: { children: ReactNode }) {
  const { mode } = useLibrary()
  const { fixCacheHours } = useSettings()
  const dispatch = useDispatch()
  const [reports, setReports] = useState<Record<string, FixReport>>({})
  const [searching, setSearching] = useState<Record<string, boolean>>({})
  const [errors, setErrors] = useState<Record<string, string>>({})

  const reload = useCallback(async () => {
    if (mode !== 'real') {
      setReports({})
      return
    }
    try {
      const list = await getCachedFixReports(fixCacheHours)
      setReports(Object.fromEntries(list.map((r) => [r.appId, r])))
    } catch {
      /* sem cache ainda */
    }
  }, [mode, fixCacheHours])

  useEffect(() => {
    void reload()
  }, [reload])

  const search = useCallback(
    async (appId: string, name: string, force = false) => {
      if (mode !== 'real') return null
      setSearching((s) => ({ ...s, [appId]: true }))
      setErrors((e) => {
        const { [appId]: _, ...rest } = e
        return rest
      })
      try {
        const r = await searchGameFixes({ appId, name, force, maxAgeHours: fixCacheHours })
        setReports((all) => ({ ...all, [appId]: r }))
        const n = r.findings.length
        dispatch({ type: 'activity', kind: 'analise', gameId: appId, text: `${name}: ${n} descoberta${n === 1 ? '' : 's'} de correção${r.fromCache ? ' (cache)' : ''}` })
        return r
      } catch (e) {
        setErrors((all) => ({ ...all, [appId]: errorMessage(e) }))
        return null
      } finally {
        setSearching((s) => ({ ...s, [appId]: false }))
      }
    },
    [mode, fixCacheHours, dispatch],
  )

  const value = useMemo(() => ({ reports, searching, errors, search, reload }), [reports, searching, errors, search, reload])
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>
}

export function useFixReports() {
  const v = useContext(Ctx)
  if (!v) throw new Error('FixReportsProvider ausente')
  return v
}
