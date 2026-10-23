import { useEffect, useState } from 'react'
import { useSettings } from '@/state/store'

/** Lê tokens CSS para bibliotecas que exigem cores literais (ex.: Recharts). */
export function useCssVars<T extends string>(names: T[]): Record<T, string> {
  const { accent, theme } = useSettings()
  const read = () => {
    const cs = getComputedStyle(document.documentElement)
    return Object.fromEntries(names.map((n) => [n, cs.getPropertyValue(n).trim()])) as Record<T, string>
  }
  const [vals, setVals] = useState(read)
  useEffect(() => {
    // espera o atributo data-accent ser aplicado
    const id = requestAnimationFrame(() => setVals(read()))
    return () => cancelAnimationFrame(id)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [accent, theme])
  return vals
}
