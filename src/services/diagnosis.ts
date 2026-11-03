import { call } from './native'
import type { DiagnosisResult, Symptom } from '@/types/diagnosis'

/** Diagnóstico por regras (Rust: `src-tauri/src/commands/diagnosis.rs`). Não aplica nada. */
export const diagnoseGame = (appId: string, name: string, symptom: Symptom, maxAgeHours: number) =>
  call<DiagnosisResult>('diagnose_game', { appId, name, symptom, maxAgeHours })

/**
 * Só em `npm run dev`: liga a DLL fictícia para este jogo, em memória. O comando
 * não existe no build de release. Não grava nada; a instalação segue o fluxo normal.
 */
export const injectDevFixtureDll = (appId: string) => call<string>('inject_dev_fixture_dll', { appId })
