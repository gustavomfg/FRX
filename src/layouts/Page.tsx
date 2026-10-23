import type { ReactNode } from 'react'
import { cn } from '@/lib/cn'

/** Contêiner padrão das páginas: largura máxima e respiro consistentes. */
export function Page({ children, className = '', narrow }: { children: ReactNode; className?: string; narrow?: boolean }) {
  return <div className={cn('mx-auto w-full px-8 pt-8 pb-16 xl:px-10', narrow ? 'max-w-[920px]' : 'max-w-[1180px]', className)}>{children}</div>
}
