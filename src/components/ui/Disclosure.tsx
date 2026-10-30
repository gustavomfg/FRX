import { useState, type ReactNode } from 'react'
import { ChevronDown } from 'lucide-react'
import { cn } from '@/lib/cn'

/** Bloco recolhível, fechado por padrão: detalhes ficam fora da tela principal. */
export function Disclosure({ title, className, titleClassName, children }: { title: ReactNode; className?: string; titleClassName?: string; children: ReactNode }) {
  const [open, setOpen] = useState(false)
  return (
    <div className={className}>
      <button type="button" onClick={() => setOpen((o) => !o)} aria-expanded={open} className={cn('group flex w-full items-center gap-2 text-left', titleClassName)}>
        <ChevronDown className={cn('size-4 shrink-0 text-fg-3 transition-transform duration-(--frx-dur) group-hover:text-fg', open && 'rotate-180')} aria-hidden />
        {title}
      </button>
      {open && children}
    </div>
  )
}
