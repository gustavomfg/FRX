import type { ReactNode } from 'react'
import { DemoTag } from '@/components/ui/Status'

export function InfoBlock({ title, children, tag }: { title: string; children: ReactNode; tag?: boolean | ReactNode }) {
  return (
    <section>
      <div className="mb-2 flex items-center justify-between">
        <h3 className="eyebrow">{title}</h3>
        {tag === true ? <DemoTag /> : tag || null}
      </div>
      <div className="divide-y divide-line border-t border-line">{children}</div>
    </section>
  )
}

export function InfoRow({ k, v, mono }: { k: string; v: string; mono?: boolean }) {
  return (
    <div className="flex items-center justify-between gap-4 py-2 text-[13px]">
      <span className="shrink-0 text-fg-3">{k}</span>
      <span className={mono ? 'truncate text-right font-mono text-[12px] text-fg' : 'truncate text-right text-fg'} title={v}>
        {v}
      </span>
    </div>
  )
}
