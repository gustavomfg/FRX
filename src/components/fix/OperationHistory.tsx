import { useState } from 'react'
import { AnimatePresence, motion } from 'motion/react'
import { ChevronDown, RotateCcw, ShieldCheck } from 'lucide-react'
import { Button } from '@/components/ui/Button'
import { EmptyState } from '@/components/ui/EmptyState'
import { Pill } from './FindingCard'
import { OP_STATE } from './engineMeta'
import { ArtifactDetails } from './ArtifactDetails'
import { type ApplyTarget } from './ApplyFixDialog'
import { useFixEngine } from '@/state/fixEngine'
import { fmtDateTime, fmtRelative } from '@/lib/format'
import type { Operation } from '@/types/fixEngine'
import { cn } from '@/lib/cn'

/** Histórico persistente do Fix Engine (diário em fix-engine/operations). */
export function OperationHistory({ appId, onAct, compact }: { appId?: string; onAct: (t: ApplyTarget) => void; compact?: boolean }) {
  const engine = useFixEngine()
  const ops = engine.operations.filter((o) => !appId || o.appId === appId)
  if (!ops.length)
    return compact ? (
      <p className="text-[12.5px] text-fg-3">Nenhuma correção aplicada pelo FRX neste jogo.</p>
    ) : (
      <EmptyState pose="idle" title="Nenhuma operação ainda" body="Cada correção aplicada ou desfeita fica registrada aqui, com backup, etapas e resultado conferido no disco." />
    )
  return (
    <ul className={cn('space-y-2', !compact && 'mt-5')} aria-label="Histórico de correções">
      {ops.map((op) => (
        <OperationRow key={op.id} op={op} onAct={onAct} showGame={!appId} />
      ))}
    </ul>
  )
}

function OperationRow({ op, onAct, showGame }: { op: Operation; onAct: (t: ApplyTarget) => void; showGame: boolean }) {
  const engine = useFixEngine()
  const [open, setOpen] = useState(false)
  const meta = OP_STATE[op.state]
  const applied = engine.applied.find((a) => a.operationId === op.id)
  const backups = op.steps.map((s) => s.backup).filter((b) => b?.existed)
  return (
    <li className="rounded-lg border border-line bg-surface">
      <div className="flex items-center gap-4 px-4 py-3">
        <button onClick={() => setOpen((o) => !o)} aria-expanded={open} className="group flex min-w-0 flex-1 items-center gap-3 text-left">
          <ChevronDown className={cn('size-4 shrink-0 text-fg-3 transition-transform duration-(--frx-dur) group-hover:text-fg', open && 'rotate-180')} aria-hidden />
          <span className="min-w-0 flex-1">
            <span className="block truncate text-[13.5px] font-medium text-fg">
              {op.recipeTitle}
              {showGame && <span className="font-normal text-fg-3"> · {op.gameName}</span>}
            </span>
            <span className="block truncate text-[12px] text-fg-3">
              {fmtRelative(op.updatedAt)} · {op.mode === 'manual' ? 'modo manual' : 'aplicada pelo FRX'} · {op.recipeTrust === 'generated' ? 'receita gerada automaticamente' : op.recipeTrust === 'pack' ? 'Smart Pack' : 'receita'} v{op.recipeVersion}
              {applied && applied.effective !== 'active' && <span className="text-caution"> · {applied.effective === 'drifted' ? 'alterada fora do FRX' : 'estado desconhecido'}</span>}
            </span>
          </span>
        </button>
        <Pill color={meta.color} soft={meta.soft}>
          {meta.label}
        </Pill>
        {(op.state === 'applied' || op.state === 'recovery-required') && (
          <Button size="sm" variant="outline" icon={<RotateCcw className="size-3.5" />} onClick={() => onAct({ kind: 'rollback', operation: op })}>
            {op.state === 'applied' ? 'Desfazer' : 'Resolver'}
          </Button>
        )}
        {op.state === 'requires-manual-step' && (
          <Button size="sm" variant="outline" icon={<ShieldCheck className="size-3.5" />} onClick={() => onAct({ kind: 'rollback', operation: op })}>
            Conferir
          </Button>
        )}
      </div>
      <AnimatePresence initial={false}>
        {open && (
          <motion.div initial={{ height: 0, opacity: 0 }} animate={{ height: 'auto', opacity: 1 }} exit={{ height: 0, opacity: 0 }} transition={{ duration: 0.2 }} className="overflow-hidden">
            <div className="space-y-3 border-t border-line px-4 pt-3 pb-4 text-[12.5px]">
              {op.message && <p className="text-fg-2">{op.message}</p>}
              <ArtifactDetails artifacts={op.artifacts} />
              <ol className="space-y-1">
                {op.events.map((e, i) => (
                  <li key={i} className="flex gap-3">
                    <span className="tnum w-48 shrink-0 font-mono text-[11.5px] whitespace-nowrap text-fg-3">{fmtDateTime(e.at)}</span>
                    <span className="w-40 shrink-0" style={{ color: OP_STATE[e.state].color }}>
                      {OP_STATE[e.state].label}
                    </span>
                    <span className="min-w-0 flex-1 text-fg-2">{e.message}</span>
                  </li>
                ))}
              </ol>
              {op.steps.map((s, i) =>
                s.step.type === 'launch-options' ? (
                  <div key={i} className="space-y-0.5 font-mono text-[12px]">
                    <div className="break-all text-fg-3">
                      <span className="mr-2 select-none text-bad/80">−</span>
                      {s.step.before || '(vazio)'}
                    </div>
                    <div className="break-all text-fg-2">
                      <span className="mr-2 select-none text-good">+</span>
                      {s.step.after || '(vazio)'}
                    </div>
                  </div>
                ) : (
                  <p key={i} className="font-mono text-[12px] text-fg-2">
                    {s.step.rel}
                  </p>
                ),
              )}
              {backups.length > 0 && (
                <p className="text-fg-3">
                  Backup: <code className="font-mono text-[11.5px] break-all">{backups[0]!.path}</code> · SHA-256 {backups[0]!.sha256?.slice(0, 12)}…
                </p>
              )}
              <p className="font-mono text-[11px] text-fg-3">{op.id}</p>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </li>
  )
}
