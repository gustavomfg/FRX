import { memo, useMemo, useRef } from 'react'
import { GRID_H, GRID_W, PIXEL_COLORS, POSES, framePaths, type Pose } from './sprites'
import { useTick } from './ticker'
import { useMotion } from '@/state/motion'
import { useSettings } from '@/state/store'
import { cn } from '@/lib/cn'

interface MascotProps {
  pose: Pose
  /** Tamanho de cada pixel em px (múltiplos inteiros preservam a nitidez). */
  scale?: number
  /** Recorta a parte de baixo — útil para o Pim "espiar" atrás de uma borda. */
  rows?: number
  flip?: boolean
  className?: string
  label?: string
  /** Mostra mesmo se mascotes estiverem desativados (ex.: pré-visualização nas configurações). */
  force?: boolean
}

const POSE_LABEL: Record<Pose, string> = {
  idle: 'Pim, o mascote do FRX, parado',
  wave: 'Pim acenando',
  rest: 'Pim descansando',
  inspect: 'Pim investigando com uma lupa',
  fix: 'Pim fazendo manutenção com uma chave',
  observe: 'Pim observando o gráfico',
  celebrate: 'Pim comemorando',
  worried: 'Pim preocupado',
  confused: 'Pim em dúvida',
  sleep: 'Pim dormindo',
  prepare: 'Pim preparando o benchmark com uma prancheta',
  record: 'Pim acompanhando a gravação',
  analyze: 'Pim examinando um gráfico',
}

function frameAt(pose: Pose, t: number) {
  const { ticks } = POSES[pose]
  const total = ticks.reduce((a, b) => a + b, 0)
  let m = t % total
  for (let i = 0; i < ticks.length; i++) {
    if (m < ticks[i]) return i
    m -= ticks[i]
  }
  return 0
}

export const Mascot = memo(function Mascot({ pose, scale = 4, rows = GRID_H, flip, className, label, force }: MascotProps) {
  const settings = useSettings()
  const motion = useMotion()
  const visible = settings.mascots || !!force
  const animate = visible && motion !== 'off'
  const raw = useTick(animate)
  // Em modo "calmo" o Pim anda na metade da velocidade.
  const t = motion === 'calm' ? Math.floor(raw / 2) : raw
  // Deslocamento por instância, para que vários Pims não pisquem em sincronia.
  const offset = useRef(Math.floor(Math.random() * 40)).current
  const index = animate ? frameAt(pose, t + offset) : 0
  const paths = useMemo(() => framePaths(pose, index, rows), [pose, index, rows])

  if (!visible) return null
  return (
    <svg
      role="img"
      aria-label={label ?? POSE_LABEL[pose]}
      viewBox={`0 0 ${GRID_W} ${rows}`}
      width={GRID_W * scale}
      height={rows * scale}
      className={cn('pixelated block shrink-0 select-none', className)}
      style={flip ? { transform: 'scaleX(-1)' } : undefined}
      shapeRendering="crispEdges"
    >
      {paths.map((p) => (
        <path key={p.key} d={p.d} style={{ fill: PIXEL_COLORS[p.key] }} />
      ))}
    </svg>
  )
})
