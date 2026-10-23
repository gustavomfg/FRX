import { Link } from 'react-router-dom'
import { Wrench } from 'lucide-react'
import type { Game } from '@/types'
import { GameArt } from './GameArt'
import { StatusBadge } from '@/components/ui/Status'
import { useGameFixes } from '@/state/selectors'
import { fmtRelative } from '@/lib/format'

export function GameCard({ game }: { game: Game }) {
  const { available } = useGameFixes(game.id)
  return (
    <Link
      to={`/biblioteca/${game.id}`}
      className="group block rounded-lg focus-visible:outline-offset-4"
      aria-label={`${game.title} — abrir detalhes`}
    >
      <div className="relative aspect-[3/4] overflow-hidden rounded-lg border border-line bg-surface transition-[transform,border-color,box-shadow] duration-(--frx-dur-slow) ease-frx group-hover:-translate-y-1 group-hover:border-line-3 group-hover:shadow-[0_18px_30px_-18px_rgb(0_0_0/0.9)]">
        <div className="absolute inset-0 transition-transform duration-[600ms] ease-frx group-hover:scale-[1.04]">
          <GameArt art={game.cover} title={game.title} />
        </div>
        <div className="absolute inset-x-0 bottom-0 h-1/2 bg-gradient-to-t from-black/75 to-transparent" aria-hidden />
        <div className="absolute inset-x-3 bottom-3">
          <p className="font-display text-[19px] leading-[1.05] font-bold tracking-[-0.03em] text-white drop-shadow">{game.title}</p>
        </div>
        {available.length > 0 && (
          <span className="absolute top-2.5 right-2.5 inline-flex h-6 items-center gap-1 rounded-[5px] bg-accent px-1.5 text-[11.5px] font-semibold text-accent-ink" title={`${available.length} correções disponíveis`}>
            <Wrench className="size-3" strokeWidth={2.4} aria-hidden />
            <span className="tnum">{available.length}</span>
            <span className="sr-only">correções disponíveis</span>
          </span>
        )}
      </div>
      <div className="mt-3 flex items-start justify-between gap-2 px-0.5">
        <div className="min-w-0">
          <p className="truncate text-[13px] text-fg-2">{game.protonVersion}</p>
          <p className="truncate text-[12px] text-fg-3">{game.genre}</p>
        </div>
        <StatusBadge status={game.status} size="sm" className="mt-0.5" />
      </div>
    </Link>
  )
}

export function GameRow({ game }: { game: Game }) {
  const { available, applied } = useGameFixes(game.id)
  return (
    <Link
      to={`/biblioteca/${game.id}`}
      className="group grid grid-cols-[48px_minmax(0,2.2fr)_minmax(0,1.3fr)_minmax(0,1.1fr)_minmax(0,1fr)_90px] items-center gap-5 rounded-md px-3 py-2.5 transition-colors duration-(--frx-dur-fast) hover:bg-surface"
    >
      <span className="block size-12 overflow-hidden rounded-[6px] border border-line">
        <GameArt art={game.cover} variant="thumb" />
      </span>
      <span className="min-w-0">
        <span className="block truncate text-[14px] font-medium text-fg group-hover:text-accent">{game.title}</span>
        <span className="block truncate text-[12px] text-fg-3">
          {game.studio} · {game.genre}
        </span>
      </span>
      <span className="truncate text-[13px] text-fg-2">{game.protonVersion}</span>
      <span>
        <StatusBadge status={game.status} size="sm" />
      </span>
      <span className="text-[13px]">
        {available.length > 0 ? (
          <span className="inline-flex items-center gap-1.5 text-accent">
            <Wrench className="size-3.5" aria-hidden /> {available.length} disponíve{available.length > 1 ? 'is' : 'l'}
          </span>
        ) : applied.length > 0 ? (
          <span className="text-fg-2">{applied.length} aplicada{applied.length > 1 ? 's' : ''}</span>
        ) : (
          <span className="text-fg-3">—</span>
        )}
      </span>
      <span className="text-right text-[12px] text-fg-3">{fmtRelative(game.lastPlayed)}</span>
    </Link>
  )
}
