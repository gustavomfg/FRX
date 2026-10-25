import { createContext, useCallback, useContext, useEffect, useMemo, useReducer, type ReactNode } from 'react'
import type { Activity, BenchmarkRun, FixHistoryEntry, FixState, Settings } from '@/types'
import { fixPacks } from '@/data/fixpacks'
import { seedBenchmarks } from '@/data/benchmarks'
import { seedActivity } from '@/data/activity'
import { ago } from '@/data/games'
import { uid } from '@/lib/format'

/**
 * Estado global do protótipo. Apenas as preferências são persistidas
 * (localStorage); correções e benchmarks voltam ao estado inicial ao recarregar.
 */

export const DEFAULT_SETTINGS: Settings = {
  theme: 'frx-dark',
  accent: 'manteiga',
  mascots: true,
  motion: 'full',
  reduceMotion: 'sistema',
  scanOnStartup: true,
  confirmBeforeApply: true,
  backupRetentionDays: 30,
  tempUnit: 'C',
  compactSidebar: false,
  useDemoData: false,
  fixCacheHours: 24,
}

const SETTINGS_KEY = 'frx.settings.v1'

function loadSettings(): Settings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY)
    if (raw) return { ...DEFAULT_SETTINGS, ...JSON.parse(raw) }
  } catch {
    /* armazenamento indisponível — segue com o padrão */
  }
  return DEFAULT_SETTINGS
}

export interface Toast {
  id: string
  tone: 'ok' | 'info' | 'warn' | 'error'
  title: string
  body?: string
}

export interface AppState {
  settings: Settings
  fixStates: Record<string, FixState>
  fixHistory: FixHistoryEntry[]
  failedAttempts: Record<string, number>
  activity: Activity[]
  benchmarks: BenchmarkRun[]
  analyzedAt: Record<string, string>
  toasts: Toast[]
}

function initialState(): AppState {
  const fixStates: Record<string, FixState> = {}
  for (const f of fixPacks) fixStates[f.id] = 'disponivel'
  fixStates['fp-ashen-nvapi'] = 'aplicado'
  fixStates['fp-glass-audio'] = 'aplicado'
  return {
    settings: loadSettings(),
    fixStates,
    fixHistory: [
      { id: 'h1', fixId: 'fp-ferrous-shader', gameId: 'ferrous-drift', action: 'aplicado', at: ago(29) },
      { id: 'h2', fixId: 'fp-ferrous-shader', gameId: 'ferrous-drift', action: 'revertido', at: ago(20), note: 'Revertido para repetir o teste' },
      { id: 'h3', fixId: 'fp-ashen-nvapi', gameId: 'ashen-choir', action: 'aplicado', at: ago(52) },
      { id: 'h4', fixId: 'fp-glass-audio', gameId: 'glasswing', action: 'aplicado', at: ago(76) },
    ].reverse() as FixHistoryEntry[],
    failedAttempts: {},
    activity: seedActivity,
    benchmarks: seedBenchmarks,
    analyzedAt: { 'ferrous-drift': ago(3), 'ashen-choir': ago(50) },
    toasts: [],
  }
}

type Action =
  | { type: 'settings'; patch: Partial<Settings> }
  | { type: 'resetSettings' }
  | { type: 'fixApplied'; fixId: string; gameId: string }
  | { type: 'fixReverted'; fixId: string; gameId: string }
  | { type: 'fixFailed'; fixId: string; gameId: string; note: string }
  | { type: 'analyzed'; gameId: string; text: string }
  | { type: 'benchmark'; run: BenchmarkRun; text: string }
  | { type: 'activity'; kind: Activity['kind']; text: string; gameId?: string }
  | { type: 'toast'; toast: Omit<Toast, 'id'> }
  | { type: 'dismissToast'; id: string }
  | { type: 'resetDemo' }

const act = (kind: Activity['kind'], text: string, gameId?: string): Activity => ({ id: uid('a'), at: new Date().toISOString(), kind, text, gameId })
const hist = (fixId: string, gameId: string, action: FixHistoryEntry['action'], note?: string): FixHistoryEntry => ({ id: uid('h'), fixId, gameId, action, at: new Date().toISOString(), note })

function reducer(s: AppState, a: Action): AppState {
  switch (a.type) {
    case 'settings':
      return { ...s, settings: { ...s.settings, ...a.patch } }
    case 'resetSettings':
      return { ...s, settings: DEFAULT_SETTINGS }
    case 'fixApplied': {
      const name = fixPacks.find((f) => f.id === a.fixId)?.name ?? a.fixId
      return {
        ...s,
        fixStates: { ...s.fixStates, [a.fixId]: 'aplicado' },
        fixHistory: [hist(a.fixId, a.gameId, 'aplicado'), ...s.fixHistory],
        activity: [act('fix', `Correção aplicada: ${name} (simulado)`, a.gameId), ...s.activity],
      }
    }
    case 'fixReverted': {
      const name = fixPacks.find((f) => f.id === a.fixId)?.name ?? a.fixId
      return {
        ...s,
        fixStates: { ...s.fixStates, [a.fixId]: 'disponivel' },
        fixHistory: [hist(a.fixId, a.gameId, 'revertido'), ...s.fixHistory],
        activity: [act('revert', `Correção revertida: ${name} (simulado)`, a.gameId), ...s.activity],
      }
    }
    case 'fixFailed':
      return {
        ...s,
        failedAttempts: { ...s.failedAttempts, [a.fixId]: (s.failedAttempts[a.fixId] ?? 0) + 1 },
        fixHistory: [hist(a.fixId, a.gameId, 'falhou', a.note), ...s.fixHistory],
        activity: [act('erro', 'Falha simulada ao aplicar correção — backup restaurado', a.gameId), ...s.activity],
      }
    case 'analyzed':
      return { ...s, analyzedAt: { ...s.analyzedAt, [a.gameId]: new Date().toISOString() }, activity: [act('analise', a.text, a.gameId), ...s.activity] }
    case 'benchmark':
      return { ...s, benchmarks: [...s.benchmarks, a.run], activity: [act('benchmark', a.text, a.run.gameId), ...s.activity] }
    case 'activity':
      return { ...s, activity: [act(a.kind, a.text, a.gameId), ...s.activity] }
    case 'toast':
      return { ...s, toasts: [...s.toasts.slice(-2), { ...a.toast, id: uid('t') }] }
    case 'dismissToast':
      return { ...s, toasts: s.toasts.filter((t) => t.id !== a.id) }
    case 'resetDemo': {
      const fresh = initialState()
      return { ...fresh, settings: s.settings, toasts: s.toasts }
    }
  }
}

const StateCtx = createContext<AppState | null>(null)
const DispatchCtx = createContext<React.Dispatch<Action> | null>(null)

export function AppStoreProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, undefined, initialState)

  useEffect(() => {
    try {
      localStorage.setItem(SETTINGS_KEY, JSON.stringify(state.settings))
    } catch {
      /* ignora */
    }
    const el = document.documentElement
    el.dataset.theme = state.settings.theme
    if (state.settings.accent === 'manteiga') delete el.dataset.accent
    else el.dataset.accent = state.settings.accent
  }, [state.settings])

  return (
    <DispatchCtx.Provider value={dispatch}>
      <StateCtx.Provider value={state}>{children}</StateCtx.Provider>
    </DispatchCtx.Provider>
  )
}

export function useAppState() {
  const s = useContext(StateCtx)
  if (!s) throw new Error('AppStoreProvider ausente')
  return s
}
export function useDispatch() {
  const d = useContext(DispatchCtx)
  if (!d) throw new Error('AppStoreProvider ausente')
  return d
}
export const useSettings = () => useAppState().settings

/** Helpers derivados mais usados pelas páginas. */
export function useFixSummary() {
  const { fixStates } = useAppState()
  return useMemo(() => {
    const available = fixPacks.filter((f) => fixStates[f.id] === 'disponivel')
    const applied = fixPacks.filter((f) => fixStates[f.id] === 'aplicado')
    return { available, applied }
  }, [fixStates])
}

/** Identidade estável: seguro em listas de dependências de efeitos. */
export function useToast() {
  const dispatch = useDispatch()
  return useCallback((toast: Omit<Toast, 'id'>) => dispatch({ type: 'toast', toast }), [dispatch])
}
