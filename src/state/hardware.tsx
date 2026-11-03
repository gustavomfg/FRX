import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from 'react'
import { getSystemProfile, identifyLaptop } from '@/services/hardware'
import { errorMessage } from '@/services/native'
import type { LaptopIdentity } from '@/types/hardware'
import type { SystemProfile } from '@/types/steam'

interface HardwareState {
  identity: LaptopIdentity | null
  system: SystemProfile | null
  loading: boolean
  error: string | null
  reload: () => void
}

const Ctx = createContext<HardwareState | null>(null)

/** Identidade do laptop (DMI) e perfil do sistema, lidos uma vez ao abrir. */
export function HardwareProvider({ children }: { children: ReactNode }) {
  const [identity, setIdentity] = useState<LaptopIdentity | null>(null)
  const [system, setSystem] = useState<SystemProfile | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const reload = useCallback(() => {
    setLoading(true)
    setError(null)
    Promise.all([identifyLaptop(), getSystemProfile()])
      .then(([id, sys]) => {
        setIdentity(id)
        setSystem(sys)
      })
      .catch((e) => setError(errorMessage(e)))
      .finally(() => setLoading(false))
  }, [])

  useEffect(reload, [reload])

  const value = useMemo(() => ({ identity, system, loading, error, reload }), [identity, system, loading, error, reload])
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>
}

export function useHardware() {
  const v = useContext(Ctx)
  if (!v) throw new Error('useHardware fora do HardwareProvider')
  return v
}

/** Nome do laptop para exibir; cai para o fabricante quando o modelo é desconhecido. */
export function laptopName(id: LaptopIdentity | null) {
  return id?.displayName ?? id?.model ?? null
}
