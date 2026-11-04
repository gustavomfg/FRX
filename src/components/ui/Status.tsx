import type { ReactNode } from 'react'
import { cn } from '@/lib/cn'

/** Marca de modo demonstrativo (navegador, sem camada nativa). */
export function DemoTag({ className, children = 'Demo' }: { className?: string; children?: string }) {
  return (
    <span
      className={cn('inline-flex h-[18px] items-center rounded-[3px] border border-accent-line px-1.5 font-pixel text-[9px] tracking-[0.06em] text-accent uppercase', className)}
      title="Sem camada nativa: nenhuma leitura ou alteração real"
    >
      {children}
    </span>
  )
}

/** Etiqueta colorida de estado. A cor vem sempre acompanhada de texto. */
export function Pill({ color, soft, children, title }: { color: string; soft: string; children: ReactNode; title?: string }) {
  return (
    <span className="inline-flex h-6 items-center gap-1.5 rounded-xs px-2 text-[12px] font-medium whitespace-nowrap" style={{ background: soft, color }} title={title}>
      {children}
    </span>
  )
}
