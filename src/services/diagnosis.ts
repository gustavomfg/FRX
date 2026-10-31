import { call } from './native'
import type { DiagnosisResult, Symptom } from '@/types/diagnosis'

/** Diagnóstico por regras (Rust: `src-tauri/src/commands/diagnosis.rs`). Não aplica nada. */
export const diagnoseGame = (appId: string, name: string, symptom: Symptom, maxAgeHours: number) =>
  call<DiagnosisResult>('diagnose_game', { appId, name, symptom, maxAgeHours })
