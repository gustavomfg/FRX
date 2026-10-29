import { open } from '@tauri-apps/plugin-dialog'
import { call } from './native'
import type { ResolutionReport } from '@/types/resolution'

export const getResolutionReport = (appId: string) => call<ResolutionReport | null>('get_resolution_report', { appId })
/** Problemas que os relatos deste jogo citam: pré-marcam os checkboxes, sem pesquisar nada. */
export const detectedSymptoms = (appId: string) => call<string[]>('detected_symptoms', { appId })
/** Tecnologias gráficas que os arquivos do jogo trazem (DLSS, frame generation, PhysX…). */
export const gameTechnologies = (appId: string) => call<string[]>('game_technologies', { appId })
export const diagnoseGame = (appId: string, symptoms: string[]) => call<ResolutionReport>('diagnose_game', { appId, symptoms })
export const researchGameResolutions = (appId: string, symptoms: string[]) => call<ResolutionReport>('research_game_resolutions', { appId, symptoms })
export const importResolutionLog = (appId: string, path: string) => call<ResolutionReport>('import_resolution_log', { appId, path })
export const verifyResolutionPlan = (appId: string, planId: string) => call<ResolutionReport>('verify_resolution_plan', { appId, planId })

/** User-initiated file selection; never searches personal logs automatically. */
export async function chooseResolutionLog(): Promise<string | null> {
  const path = await open({ multiple: false, directory: false, title: 'Selecionar log deste jogo', filters: [{ name: 'Logs do Proton, DXVK ou VKD3D', extensions: ['log', 'txt'] }] })
  return typeof path === 'string' ? path : null
}
