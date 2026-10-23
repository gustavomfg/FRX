import { useSyncExternalStore } from 'react'

/**
 * Relógio único compartilhado por todos os sprites (8 Hz).
 * Um só setInterval para o app inteiro, pausado quando a janela fica oculta
 * ou quando não há assinantes — mantém o custo de CPU próximo de zero.
 */

export const TICK_MS = 125
let tick = 0
let timer: ReturnType<typeof setInterval> | null = null
const listeners = new Set<() => void>()

function start() {
  if (timer || typeof document === 'undefined' || document.hidden) return
  timer = setInterval(() => {
    tick++
    listeners.forEach((l) => l())
  }, TICK_MS)
}
function stop() {
  if (timer) clearInterval(timer)
  timer = null
}

if (typeof document !== 'undefined') {
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) stop()
    else if (listeners.size) start()
  })
}

function subscribe(l: () => void) {
  listeners.add(l)
  start()
  return () => {
    listeners.delete(l)
    if (!listeners.size) stop()
  }
}

const noopSubscribe = () => () => {}

export function useTick(enabled: boolean) {
  return useSyncExternalStore(
    enabled ? subscribe : noopSubscribe,
    () => (enabled ? tick : 0),
    () => 0,
  )
}
