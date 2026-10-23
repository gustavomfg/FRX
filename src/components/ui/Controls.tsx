import { useId, useRef, type KeyboardEvent, type ReactNode } from 'react'
import { ChevronDown } from 'lucide-react'
import { cn } from '@/lib/cn'

export function Toggle({ checked, onChange, label, description, disabled }: { checked: boolean; onChange: (v: boolean) => void; label: string; description?: ReactNode; disabled?: boolean }) {
  const id = useId()
  return (
    <div className="flex items-center justify-between gap-6">
      <div className="min-w-0">
        <label htmlFor={id} className="block text-[14px] font-medium text-fg">
          {label}
        </label>
        {description && <p className="mt-0.5 text-[13px] text-fg-2">{description}</p>}
      </div>
      <button
        id={id}
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={cn(
          'relative h-6 w-10 shrink-0 rounded-[6px] border transition-colors duration-(--frx-dur) ease-frx disabled:opacity-40',
          checked ? 'border-accent bg-accent' : 'border-line-2 bg-raised hover:border-line-3',
        )}
      >
        <span
          className={cn(
            'absolute top-[3px] left-[3px] size-4 rounded-[4px] transition-transform duration-(--frx-dur) ease-spring',
            checked ? 'translate-x-4 bg-accent-ink' : 'translate-x-0 bg-fg-2',
          )}
        />
      </button>
    </div>
  )
}

export interface SegmentOption<T extends string> {
  value: T
  label: ReactNode
  icon?: ReactNode
  title?: string
}

/** Controle segmentado com navegação por setas (padrão radiogroup). */
export function Segmented<T extends string>({ value, onChange, options, label, size = 'md', className }: { value: T; onChange: (v: T) => void; options: SegmentOption<T>[]; label: string; size?: 'sm' | 'md'; className?: string }) {
  const refs = useRef<(HTMLButtonElement | null)[]>([])
  const onKey = (e: KeyboardEvent, i: number) => {
    const dir = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 0
    if (!dir) return
    e.preventDefault()
    const n = (i + dir + options.length) % options.length
    onChange(options[n].value)
    refs.current[n]?.focus()
  }
  return (
    <div role="radiogroup" aria-label={label} className={cn('inline-flex rounded-md border border-line bg-surface p-[3px]', className)}>
      {options.map((o, i) => {
        const active = o.value === value
        return (
          <button
            key={o.value}
            ref={(el) => {
              refs.current[i] = el
            }}
            role="radio"
            aria-checked={active}
            tabIndex={active ? 0 : -1}
            title={o.title}
            onClick={() => onChange(o.value)}
            onKeyDown={(e) => onKey(e, i)}
            className={cn(
              'inline-flex items-center justify-center gap-1.5 rounded-[7px] font-medium whitespace-nowrap transition-colors duration-(--frx-dur-fast)',
              size === 'sm' ? 'h-7 px-2.5 text-[12.5px]' : 'h-8 px-3 text-[13px]',
              active ? 'bg-raised text-fg shadow-[0_1px_0_rgb(255_255_255/0.04)_inset]' : 'text-fg-3 hover:text-fg-2',
            )}
          >
            {o.icon}
            {o.label}
          </button>
        )
      })}
    </div>
  )
}

export function Select<T extends string>({ value, onChange, options, label, className, hideLabel }: { value: T; onChange: (v: T) => void; options: { value: T; label: string }[]; label: string; className?: string; hideLabel?: boolean }) {
  const id = useId()
  return (
    <div className={cn('flex items-center gap-2', className)}>
      <label htmlFor={id} className={cn('text-[12.5px] text-fg-3', hideLabel && 'sr-only')}>
        {label}
      </label>
      <div className="relative">
        <select
          id={id}
          value={value}
          onChange={(e) => onChange(e.target.value as T)}
          className="h-8 appearance-none rounded-sm border border-line-2 bg-surface pr-8 pl-2.5 text-[13px] text-fg transition-colors hover:border-line-3 focus-visible:outline-2"
        >
          {options.map((o) => (
            <option key={o.value} value={o.value} className="bg-raised">
              {o.label}
            </option>
          ))}
        </select>
        <ChevronDown className="pointer-events-none absolute top-1/2 right-2 size-3.5 -translate-y-1/2 text-fg-3" aria-hidden />
      </div>
    </div>
  )
}

export function Kbd({ children }: { children: ReactNode }) {
  return <kbd className="inline-flex h-5 min-w-5 items-center justify-center rounded-[4px] border border-line-2 bg-raised px-1 font-mono text-[10.5px] text-fg-2">{children}</kbd>
}

export function SectionHeader({ eyebrow, title, action, className, id }: { eyebrow?: string; title: ReactNode; action?: ReactNode; className?: string; id?: string }) {
  return (
    <div className={cn('mb-4 flex items-end justify-between gap-4', className)}>
      <div>
        {eyebrow && <div className="eyebrow mb-2">{eyebrow}</div>}
        <h2 id={id} className="font-display text-[19px] font-semibold tracking-[-0.02em] text-fg">
          {title}
        </h2>
      </div>
      {action}
    </div>
  )
}

export function Meter({ value, max = 100, color = 'var(--frx-text)', label, className }: { value: number; max?: number; color?: string; label: string; className?: string }) {
  const pct = Math.max(0, Math.min(100, (value / max) * 100))
  return (
    <div className={cn('h-1.5 w-full overflow-hidden rounded-full bg-line-2', className)} role="meter" aria-label={label} aria-valuenow={Math.round(value)} aria-valuemin={0} aria-valuemax={max}>
      <div className="h-full rounded-full transition-[width] duration-(--frx-dur-slow) ease-frx" style={{ width: `${pct}%`, background: color }} />
    </div>
  )
}
