import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open, save } from '@tauri-apps/plugin-dialog'
import { call } from './native'
import type {
  CaptureRequest,
  CaptureStatus,
  ComparisonResult,
  MangoHudStatus,
  Session,
  SessionInput,
  SessionMeta,
  SessionPatch,
} from '@/types/performance'

/** Serviços do Performance Lab (Rust: `src-tauri/src/commands/performance.rs`). */

export const detectMangoHud = () => call<MangoHudStatus>('detect_mangohud')
export const listSessions = (appId?: string | null) => call<SessionMeta[]>('list_benchmark_sessions', { appId: appId ?? null })
export const getSession = (id: string) => call<Session>('get_benchmark_session', { id })
export const compareSessions = (beforeId: string, afterId: string) => call<ComparisonResult>('compare_benchmark_sessions', { beforeId, afterId })
export const updateSession = (id: string, patch: SessionPatch) => call<SessionMeta>('update_benchmark_session', { id, patch })
export const deleteSession = (id: string) => call<void>('delete_benchmark_session', { id })
export const importBenchmark = (path: string, input: SessionInput) => call<SessionMeta>('import_benchmark', { path, input })

export const getCaptureStatus = () => call<CaptureStatus>('get_capture_status')
export const prepareBenchmark = (request: CaptureRequest) => call<CaptureStatus>('prepare_benchmark', { request })
export const startCapture = () => call<CaptureStatus>('start_capture')
export const setCaptureLogging = (on: boolean) => call<void>('capture_logging', { on })
export const finishCapture = () => call<void>('finish_capture')
export const cancelCapture = () => call<CaptureStatus>('cancel_capture')
export const resetCapture = () => call<CaptureStatus>('reset_capture')

export const onCaptureStatus = (cb: (s: CaptureStatus) => void): Promise<UnlistenFn> => listen<CaptureStatus>('frx://capture', (e) => cb(e.payload))
export const onImportProgress = (cb: (stage: string) => void): Promise<UnlistenFn> => listen<{ stage: string }>('frx://import', (e) => cb(e.payload.stage))

/** Diálogo nativo para escolher um log do MangoHud. */
export async function pickLogFile(): Promise<string | null> {
  const r = await open({ multiple: false, directory: false, title: 'Importar benchmark do MangoHud', filters: [{ name: 'Logs do MangoHud (CSV)', extensions: ['csv', 'txt', 'log'] }] })
  return typeof r === 'string' ? r : null
}

/** Exporta uma sessão para um arquivo escolhido pelo usuário. */
export async function exportSession(meta: SessionMeta, format: 'json' | 'csv'): Promise<boolean> {
  const safe = meta.label.replace(/[^\p{L}\p{N} _-]+/gu, '').trim().replace(/\s+/g, '-') || 'sessao'
  const path = await save({ title: 'Exportar sessão', defaultPath: `${safe}.${format}`, filters: [{ name: format.toUpperCase(), extensions: [format] }] })
  if (!path) return false
  await call<void>('export_benchmark_session', { id: meta.id, path })
  return true
}
