import { BookOpen, Boxes, Database, FlaskConical, GitBranch } from 'lucide-react'
import type { Source, SourceKind } from '@/types'
import { cn } from '@/lib/cn'

const ICONS: Record<SourceKind, typeof Database> = {
  protondb: Database,
  github: GitBranch,
  'umu-protonfixes': Boxes,
  wiki: BookOpen,
  'frx-lab': FlaskConical,
}

export function SourceIcon({ kind, className }: { kind: SourceKind; className?: string }) {
  const I = ICONS[kind]
  return <I className={cn('size-3.5', className)} aria-hidden />
}

export function SourceChip({ source }: { source: Source }) {
  return (
    <span className="inline-flex h-6 max-w-full items-center gap-1.5 rounded-xs border border-line bg-bg/40 px-2 text-[12px] text-fg-2" title={source.ref}>
      <SourceIcon kind={source.kind} className="text-fg-3" />
      <span className="font-medium text-fg">{source.label}</span>
      <span className="truncate text-fg-3">· {source.ref}</span>
    </span>
  )
}
