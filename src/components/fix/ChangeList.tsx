import { FileCode2, KeyRound, Package, Rocket, SquareTerminal } from 'lucide-react'
import type { FileChange } from '@/types'
import { cn } from '@/lib/cn'

const KIND: Record<FileChange['kind'], { label: string; icon: typeof FileCode2 }> = {
  env: { label: 'Variável de ambiente', icon: SquareTerminal },
  arquivo: { label: 'Arquivo', icon: FileCode2 },
  launch: { label: 'Inicialização', icon: Rocket },
  registro: { label: 'Registro do Wine', icon: KeyRound },
  dll: { label: 'Componente', icon: Package },
}

const ACTION_MARK = { adicionar: '+', alterar: '~', remover: '−' } as const

/** Lista de mudanças em formato de "diff" legível. */
export function ChangeList({ changes, inverted }: { changes: FileChange[]; inverted?: boolean }) {
  return (
    <ul className="divide-y divide-line overflow-hidden rounded-md border border-line bg-bg/50">
      {changes.map((c, i) => {
        const K = KIND[c.kind]
        const action = inverted ? (c.action === 'adicionar' ? 'remover' : c.action === 'remover' ? 'adicionar' : 'alterar') : c.action
        const from = inverted ? c.after : c.before
        const to = inverted ? c.before : c.after
        return (
          <li key={i} className="px-3.5 py-3">
            <div className="flex items-center gap-2.5">
              <span
                className={cn(
                  'grid size-[18px] shrink-0 place-items-center rounded-[4px] font-mono text-[12px] leading-none font-semibold',
                  action === 'adicionar' && 'bg-good-soft text-good',
                  action === 'alterar' && 'bg-accent-soft text-accent',
                  action === 'remover' && 'bg-bad-soft text-bad',
                )}
                title={action}
                aria-label={action}
              >
                {ACTION_MARK[action]}
              </span>
              <K.icon className="size-3.5 shrink-0 text-fg-3" aria-hidden />
              <code className="min-w-0 flex-1 truncate font-mono text-[12.5px] text-fg" title={c.path}>
                {c.path}
              </code>
              <span className="shrink-0 text-[11.5px] text-fg-3">{K.label}</span>
            </div>
            {(from || to) && (
              <div className="mt-2 ml-7 space-y-0.5 font-mono text-[12px]">
                {from && action !== 'adicionar' && (
                  <div className="truncate text-fg-3">
                    <span className="mr-2 select-none text-bad/80">−</span>
                    <span className="line-through decoration-fg-3/40">{from}</span>
                  </div>
                )}
                {to && action !== 'remover' && (
                  <div className="truncate text-fg-2">
                    <span className="mr-2 select-none text-good">+</span>
                    {to}
                  </div>
                )}
              </div>
            )}
          </li>
        )
      })}
    </ul>
  )
}
