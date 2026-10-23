import { cn } from '@/lib/cn'

/**
 * Símbolo FRX: um direcional (d-pad) que também é um sinal de "+",
 * de primeiros socorros — jogar e consertar no mesmo gesto.
 */
const MARK = [
  '...YYY...',
  '...YYY...',
  '...YYY...',
  'YYYYYYYYY',
  'YYYY.YYYY',
  'HHHYYYHHH',
  '...YYY...',
  '...YYY...',
  '...HHH...',
]

const markPaths = (() => {
  const d: Record<string, string> = { Y: '', H: '' }
  MARK.forEach((row, y) => {
    let x = 0
    while (x < row.length) {
      const c = row[x]
      if (c === '.') {
        x++
        continue
      }
      let w = 1
      while (row[x + w] === c) w++
      d[c] += `M${x} ${y}h${w}v1h-${w}z`
      x += w
    }
  })
  return d
})()

export function LogoMark({ size = 22, className }: { size?: number; className?: string }) {
  return (
    <svg viewBox="0 0 9 9" width={size} height={size} className={cn('pixelated shrink-0', className)} aria-hidden>
      <path d={markPaths.Y} style={{ fill: 'var(--frx-accent)' }} />
      <path d={markPaths.H} style={{ fill: 'color-mix(in oklab, var(--frx-accent) 62%, #3b2a0c)' }} />
    </svg>
  )
}

export function Logo({ compact, className }: { compact?: boolean; className?: string }) {
  return (
    <div className={cn('flex items-center gap-2.5', className)} aria-label="FRX">
      <LogoMark size={compact ? 20 : 22} />
      {!compact && (
        <div className="flex flex-col leading-none">
          <span className="font-display text-[21px] font-extrabold tracking-[-0.05em] text-fg">FRX</span>
        </div>
      )}
    </div>
  )
}
