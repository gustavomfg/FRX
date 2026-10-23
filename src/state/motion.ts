import { useEffect, useSyncExternalStore } from 'react'
import type { MotionLevel } from '@/types'
import { useSettings } from './store'

const QUERY = '(prefers-reduced-motion: reduce)'

function subscribe(cb: () => void) {
  const mq = window.matchMedia(QUERY)
  mq.addEventListener('change', cb)
  return () => mq.removeEventListener('change', cb)
}
const getSnapshot = () => window.matchMedia(QUERY).matches

/** Nível de movimento efetivo: combina a preferência do app com a do sistema. */
export function useMotion(): MotionLevel {
  const { motion, reduceMotion } = useSettings()
  const systemReduce = useSyncExternalStore(subscribe, getSnapshot, () => false)
  if (reduceMotion === 'sempre' || systemReduce) return 'off'
  return motion
}

export function useSystemReducedMotion() {
  return useSyncExternalStore(subscribe, getSnapshot, () => false)
}

/** Espelha o nível efetivo em <html data-motion>, consumido pelo CSS. */
export function useMotionAttribute() {
  const level = useMotion()
  useEffect(() => {
    document.documentElement.dataset.motion = level
  }, [level])
  return level
}
