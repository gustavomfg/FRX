import { useEffect, useState } from 'react'
import { getAppInfo, type AppInfo } from './platform'

export function useAppInfo() {
  const [info, setInfo] = useState<AppInfo | null>(null)
  useEffect(() => {
    let alive = true
    getAppInfo().then((i) => alive && setInfo(i))
    return () => {
      alive = false
    }
  }, [])
  return info
}
