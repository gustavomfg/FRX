import type { InstallationStatus } from '@/types/steam'
import { PixelGlyph } from '@/components/ui/Status'
import { INSTALL_LABEL } from '@/lib/steamGame'
import { cn } from '@/lib/cn'

const META: Record<InstallationStatus, { color: string; soft: string; glyph: string[] }> = {
  installed: { color: 'var(--frx-good)', soft: 'var(--frx-good-soft)', glyph: ['.....', '....#', '...#.', '#.#..', '.#...'] },
  incomplete: { color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)', glyph: ['.....', '##...', '###..', '####.', '.....'] },
  unavailable: { color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)', glyph: ['#...#', '.#.#.', '..#..', '.#.#.', '#...#'] },
  unknown: { color: 'var(--frx-text-3)', soft: 'var(--frx-raised)', glyph: ['.###.', '#...#', '..##.', '.....', '..#..'] },
}

/** Estado da instalação na Steam — sempre com glifo e texto, nunca só cor. */
export function InstallBadge({ status, className, solid }: { status: InstallationStatus; className?: string; solid?: boolean }) {
  const m = META[status]
  return (
    <span className={cn('inline-flex h-5 items-center gap-1.5 rounded-xs px-1.5 text-[11.5px] font-medium whitespace-nowrap backdrop-blur-sm', className)} style={{ background: solid ? `linear-gradient(${m.soft}, ${m.soft}), var(--frx-bg)` : m.soft, color: m.color }}>
      <PixelGlyph rows={m.glyph} color={m.color} size={8} />
      {INSTALL_LABEL[status]}
    </span>
  )
}
