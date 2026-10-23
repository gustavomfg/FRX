import { useEffect, useRef, useState } from 'react'
import { animate } from 'motion/react'
import { useMotion } from '@/state/motion'
import { fmtNum } from '@/lib/format'

/** Contador que sobe até o valor. Sem movimento, mostra o número final direto. */
export function AnimatedNumber({ value, digits = 0, duration = 1.1, prefix = '', suffix = '', className, delay = 0 }: { value: number; digits?: number; duration?: number; prefix?: string; suffix?: string; className?: string; delay?: number }) {
  const motion = useMotion()
  const [display, setDisplay] = useState(motion === 'off' ? value : 0)
  const from = useRef(0)

  useEffect(() => {
    if (motion === 'off') {
      setDisplay(value)
      return
    }
    const ctrl = animate(from.current, value, {
      duration: motion === 'calm' ? duration * 0.6 : duration,
      delay,
      ease: [0.16, 1, 0.3, 1],
      onUpdate: (v) => setDisplay(v),
    })
    from.current = value
    return () => ctrl.stop()
  }, [value, motion, duration, delay])

  return (
    <span className={className}>
      {prefix}
      {fmtNum(display, digits)}
      {suffix}
    </span>
  )
}
