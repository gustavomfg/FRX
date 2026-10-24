import type { Game } from '@/types'
import { GameArt } from './GameArt'
import { useGameArt } from '@/hooks/useGameArt'
import type { ArtKind } from '@/services/games'
import { cn } from '@/lib/cn'

/**
 * Arte do jogo com layout estável: a cena pixelada ocupa o espaço desde o
 * primeiro quadro e a imagem oficial da Steam (quando existe) entra por cima
 * com um fade curto. Jogos de demonstração usam só a arte gerada.
 */
export function CoverImage({ game, kind = 'cover', variant = 'cover', className, onReady }: { game: Game; kind?: ArtKind; variant?: 'cover' | 'banner' | 'thumb'; className?: string; onReady?: (ok: boolean) => void }) {
  const art = useGameArt(game.origin === 'steam' ? game.id : undefined, kind)
  return (
    <div className={cn('relative h-full w-full', className)}>
      <GameArt art={game.cover} variant={variant} title={art.url ? undefined : game.title} />
      {art.url && (
        <img
          src={art.url}
          alt={`Arte oficial de ${game.title}`}
          loading="lazy"
          decoding="async"
          draggable={false}
          onLoad={() => {
            art.onLoad()
            onReady?.(true)
          }}
          onError={() => {
            art.onError()
            onReady?.(false)
          }}
          className={cn('absolute inset-0 h-full w-full object-cover transition-opacity duration-(--frx-dur-slow) ease-frx', art.loaded ? 'opacity-100' : 'opacity-0')}
        />
      )}
    </div>
  )
}
