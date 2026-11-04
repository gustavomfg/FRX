import type { Check } from '@/types/fixEngine'
import { cn } from '@/lib/cn'

/** Verificações feitas na prévia: cada linha diz o que foi conferido e o resultado. */
export function ChecksList({ checks }: { checks: Check[] }) {
  return (
    <div>
      <div className="eyebrow mb-2.5">Verificações</div>
      <ul className="space-y-1">
        {checks.map((c, i) => (
          <li key={i} className="flex items-start gap-2.5 text-[12.5px]">
            <span
              className={cn('mt-[3px] grid size-3.5 shrink-0 place-items-center rounded-[3px] text-[9px] font-bold', c.state === 'ok' ? 'bg-good-soft text-good' : c.state === 'warn' ? 'bg-caution-soft text-caution' : 'bg-bad-soft text-bad')}
              aria-label={c.state === 'ok' ? 'ok' : c.state === 'warn' ? 'atenção' : 'bloqueio'}
            >
              {c.state === 'ok' ? '✓' : '!'}
            </span>
            <span className="w-36 shrink-0 text-fg-3">{c.label}</span>
            <span className="min-w-0 flex-1 break-words text-fg-2">{c.detail}</span>
          </li>
        ))}
      </ul>
    </div>
  )
}
