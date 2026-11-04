import { createContext, useCallback, useContext, useEffect, useReducer, type ReactNode } from 'react'
import type { Settings } from '@/types'
import { uid } from '@/lib/format'

/**
 * Estado global: preferências (persistidas no localStorage) e notificações.
 * As correções e o diário vêm do núcleo nativo, não daqui.
 */

export const DEFAULT_SETTINGS: Settings = {
  theme: 'frx-dark',
  accent: 'manteiga',
  mascots: true,
  motion: 'full',
  reduceMotion: 'sistema',
  compactSidebar: false,
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
  toasts: Toast[]
}

type Action =
  | { type: 'settings'; patch: Partial<Settings> }
  | { type: 'resetSettings' }
  | { type: 'toast'; toast: Omit<Toast, 'id'> }
  | { type: 'dismissToast'; id: string }

function reducer(s: AppState, a: Action): AppState {
  switch (a.type) {
    case 'settings':
      return { ...s, settings: { ...s.settings, ...a.patch } }
    case 'resetSettings':
      return { ...s, settings: DEFAULT_SETTINGS }
    case 'toast':
      return { ...s, toasts: [...s.toasts.slice(-2), { ...a.toast, id: uid('t') }] }
    case 'dismissToast':
      return { ...s, toasts: s.toasts.filter((t) => t.id !== a.id) }
  }
}

const StateCtx = createContext<AppState | null>(null)
const DispatchCtx = createContext<React.Dispatch<Action> | null>(null)

export function AppStoreProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, undefined, (): AppState => ({ settings: loadSettings(), toasts: [] }))

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

/** Identidade estável: seguro em listas de dependências de efeitos. */
export function useToast() {
  const dispatch = useDispatch()
  return useCallback((toast: Omit<Toast, 'id'>) => dispatch({ type: 'toast', toast }), [dispatch])
}
