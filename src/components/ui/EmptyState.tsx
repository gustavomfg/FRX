import type { ReactNode } from 'react'
import { Mascot } from '@/mascot/Mascot'
import type { Pose } from '@/mascot/sprites'
import { cn } from '@/lib/cn'

export function EmptyState({ pose = 'confused', title, body, action, className }: { pose?: Pose; title: string; body?: ReactNode; action?: ReactNode; className?: string }) {
  return (
    <div className={cn('flex flex-col items-center justify-center px-6 py-14 text-center', className)}>
      <Mascot pose={pose} scale={4} />
      <div className="mt-2 h-px w-24 bg-line-2" aria-hidden />
      <h3 className="mt-5 font-display text-[18px] font-semibold tracking-[-0.02em]">{title}</h3>
      {body && <p className="mt-1.5 max-w-sm text-[13.5px] text-fg-2">{body}</p>}
      {action && <div className="mt-5">{action}</div>}
    </div>
  )
}
