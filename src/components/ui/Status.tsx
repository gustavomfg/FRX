import type { CompatStatus, RiskLevel, Severity } from '@/types'
import { cn } from '@/lib/cn'

/** Glifos pixel 5×5 — o estado nunca depende só da cor. */
const GLYPHS: Record<CompatStatus, string[]> = {
  perfeito: ['.....', '....#', '...#.', '#.#..', '.#...'],
  jogavel: ['.....', '.###.', '.###.', '.###.', '.....'],
  ajustes: ['..#..', '..#..', '#####', '..#..', '..#..'],
  problematico: ['..#..', '..#..', '..#..', '.....', '..#..'],
  'nao-testado': ['.###.', '#...#', '..##.', '.....', '..#..'],
}

export const STATUS_META: Record<CompatStatus, { label: string; short: string; color: string; soft: string; description: string }> = {
  perfeito: { label: 'Perfeito', short: 'Perfeito', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)', description: 'Roda sem ajustes conhecidos.' },
  jogavel: { label: 'Jogável', short: 'Jogável', color: 'var(--frx-text-2)', soft: 'var(--frx-raised)', description: 'Roda bem, com pequenos detalhes.' },
  ajustes: { label: 'Pede ajustes', short: 'Ajustes', color: 'var(--frx-adjust)', soft: 'var(--frx-adjust-soft)', description: 'Melhora com correções disponíveis.' },
  problematico: { label: 'Problemático', short: 'Problemático', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)', description: 'Problemas sérios relatados.' },
  'nao-testado': { label: 'Não testado', short: 'Sem dados', color: 'var(--frx-text-3)', soft: 'var(--frx-raised)', description: 'Ainda sem relatos suficientes.' },
}

export function PixelGlyph({ rows, color, size = 10, className }: { rows: string[]; color: string; size?: number; className?: string }) {
  let d = ''
  rows.forEach((r, y) => [...r].forEach((c, x) => c === '#' && (d += `M${x} ${y}h1v1h-1z`)))
  return (
    <svg viewBox={`0 0 ${rows[0].length} ${rows.length}`} width={size} height={size} className={cn('pixelated shrink-0', className)} aria-hidden>
      <path d={d} style={{ fill: color }} />
    </svg>
  )
}

export function StatusBadge({ status, size = 'md', className }: { status: CompatStatus; size?: 'sm' | 'md'; className?: string }) {
  const m = STATUS_META[status]
  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 rounded-xs font-medium whitespace-nowrap',
        size === 'sm' ? 'h-5 px-1.5 text-[11.5px]' : 'h-6 px-2 text-[12px]',
        className,
      )}
      style={{ background: m.soft, color: m.color }}
    >
      <PixelGlyph rows={GLYPHS[status]} color={m.color} size={size === 'sm' ? 8 : 10} />
      {m.label}
    </span>
  )
}

export function StatusDot({ status }: { status: CompatStatus }) {
  return <PixelGlyph rows={GLYPHS[status]} color={STATUS_META[status].color} size={10} />
}

const RISK: Record<RiskLevel, { label: string; color: string; soft: string; bars: number }> = {
  baixo: { label: 'Risco baixo', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)', bars: 1 },
  moderado: { label: 'Risco moderado', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)', bars: 2 },
  alto: { label: 'Risco alto', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)', bars: 3 },
}

export function RiskBadge({ risk, className }: { risk: RiskLevel; className?: string }) {
  const r = RISK[risk]
  return (
    <span className={cn('inline-flex h-6 items-center gap-1.5 rounded-xs px-2 text-[12px] font-medium whitespace-nowrap', className)} style={{ background: r.soft, color: r.color }}>
      <span className="flex items-end gap-px" aria-hidden>
        {[0, 1, 2].map((i) => (
          <span key={i} className="w-[3px]" style={{ height: 4 + i * 2, background: i < r.bars ? r.color : 'currentColor', opacity: i < r.bars ? 1 : 0.25 }} />
        ))}
      </span>
      {r.label}
    </span>
  )
}

const SEV: Record<Severity, { label: string; color: string }> = {
  baixa: { label: 'Baixa', color: 'var(--frx-text-2)' },
  media: { label: 'Média', color: 'var(--frx-caution)' },
  alta: { label: 'Alta', color: 'var(--frx-bad)' },
}

export function SeverityMark({ severity }: { severity: Severity }) {
  const s = SEV[severity]
  const n = severity === 'alta' ? 3 : severity === 'media' ? 2 : 1
  return (
    <span className="inline-flex items-center gap-1.5 text-[12px] font-medium" style={{ color: s.color }}>
      <span className="flex gap-[2px]" aria-hidden>
        {[0, 1, 2].map((i) => (
          <span key={i} className="size-[5px]" style={{ background: i < n ? s.color : 'var(--frx-line-2)' }} />
        ))}
      </span>
      Severidade {s.label.toLowerCase()}
    </span>
  )
}

/** Confiança em 10 blocos — leitura rápida, sem gráfico. */
export function ConfidenceMeter({ value, className, showLabel = true }: { value: number; className?: string; showLabel?: boolean }) {
  const filled = Math.round(value / 10)
  const tone = value >= 75 ? 'var(--frx-good)' : value >= 55 ? 'var(--frx-accent)' : 'var(--frx-caution)'
  return (
    <span className={cn('inline-flex items-center gap-2', className)} role="meter" aria-valuenow={value} aria-valuemin={0} aria-valuemax={100} aria-label={`Confiança ${value}%`}>
      <span className="flex gap-[2px]" aria-hidden>
        {Array.from({ length: 10 }, (_, i) => (
          <span key={i} className="h-2.5 w-[5px] rounded-[1px]" style={{ background: i < filled ? tone : 'var(--frx-line-2)' }} />
        ))}
      </span>
      {showLabel && <span className="tnum font-mono text-[12px] text-fg-2">{value}%</span>}
    </span>
  )
}

export function DemoTag({ className, children = 'Demo' }: { className?: string; children?: string }) {
  return (
    <span
      className={cn('inline-flex h-[18px] items-center rounded-[3px] border border-accent-line px-1.5 font-pixel text-[9px] tracking-[0.06em] text-accent uppercase', className)}
      title="Dados demonstrativos — nenhuma medição ou alteração real"
    >
      {children}
    </span>
  )
}
