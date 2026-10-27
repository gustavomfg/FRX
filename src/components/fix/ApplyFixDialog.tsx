import { useCallback, useEffect, useRef, useState } from 'react'
import { AnimatePresence, motion } from 'motion/react'
import { ArchiveRestore, Check, ClipboardCopy, ExternalLink, FlaskConical, Gauge, RefreshCw, RotateCcw, ShieldCheck, TriangleAlert, Wrench } from 'lucide-react'
import { useNavigate } from 'react-router-dom'
import { Dialog } from '@/components/ui/Dialog'
import { Button } from '@/components/ui/Button'
import { Mascot } from '@/mascot/Mascot'
import { Pill } from './FindingCard'
import { PerformanceWarning } from './PerformanceWarning'
import { APPLY_STAGES, EVIDENCE, OP_STATE, PLAN_STATUS, PURPOSE, RISK, ROLLBACK_STAGES, TRUST } from './engineMeta'
import { openExternal } from '@/services/opener'
import { applyFixPlan, onFixProgress, prepareFixPlan, prepareFixRollback, rollbackFix, verifyFixOperation } from '@/services/fixEngine'
import { errorMessage } from '@/services/native'
import { useFixEngine } from '@/state/fixEngine'
import { useToast } from '@/state/store'
import type { Check as PlanCheck, FixPlan, OpState, Operation, PlannedChange, RollbackPlan } from '@/types/fixEngine'
import { cn } from '@/lib/cn'

type Phase = 'loading' | 'review' | 'confirm' | 'running' | 'manual' | 'done' | 'error'

export type ApplyTarget = { kind: 'apply'; recipeId: string; appId: string; title: string } | { kind: 'rollback'; operation: Operation }

/**
 * Fluxo real: prévia calculada no núcleo → confirmação → execução com
 * progresso emitido pelo Rust → resultado conferido no disco.
 * A interface só envia IDs e a impressão digital da prévia.
 */
export function ApplyFixDialog({ target, onClose }: { target: ApplyTarget | null; onClose: () => void }) {
  if (!target) return null
  const identity = target.kind === 'apply' ? `${target.appId}:${target.recipeId}` : target.operation.id
  return <ScopedApplyFixDialog key={`${target.kind}:${identity}`} target={target} onClose={onClose} />
}

function ScopedApplyFixDialog({ target, onClose }: { target: ApplyTarget; onClose: () => void }) {
  const engine = useFixEngine()
  const toast = useToast()
  const navigate = useNavigate()
  const [phase, setPhase] = useState<Phase>('loading')
  const [plan, setPlan] = useState<FixPlan | null>(null)
  const [rb, setRb] = useState<RollbackPlan | null>(null)
  const [op, setOp] = useState<Operation | null>(null)
  const [stage, setStage] = useState<OpState | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [ack, setAck] = useState(false)
  const [verifying, setVerifying] = useState(false)
  const alive = useRef(true)

  const rollback = target?.kind === 'rollback'
  const appId = target ? (target.kind === 'apply' ? target.appId : target.operation.appId) : ''

  const load = useCallback(async () => {
    if (!target) return
    setPhase('loading')
    setError(null)
    setAck(false)
    setStage(null)
    try {
      if (target.kind === 'apply') {
        setPlan(await prepareFixPlan(target.recipeId, target.appId))
      } else if (target.operation.state === 'requires-manual-step' || target.operation.state === 'recovery-required') {
        // Passo manual pendente ou operação interrompida: começa pela conferência.
        setOp(target.operation)
        setRb(await prepareFixRollback(target.operation.id).catch(() => null))
        setPhase(target.operation.state === 'requires-manual-step' ? 'manual' : 'review')
        return
      } else {
        setRb(await prepareFixRollback(target.operation.id))
      }
      setPhase('review')
    } catch (e) {
      setError(errorMessage(e))
      setPhase('error')
    }
  }, [target])

  useEffect(() => {
    alive.current = true
    setOp(null)
    setPlan(null)
    setRb(null)
    void load()
    return () => void (alive.current = false)
  }, [load])

  const finish = (result: Operation) => {
    setOp(result)
    void engine.refresh()
    if (result.state === 'applied') {
      setPhase('done')
      toast({ tone: 'ok', title: 'Correção aplicada', body: `${result.recipeTitle} · conferida no disco.` })
    } else if (result.state === 'rolled-back') {
      setPhase('done')
      toast({ tone: 'info', title: 'Correção desfeita', body: `${result.recipeTitle} · só o que o FRX adicionou foi removido.` })
    } else if (result.state === 'requires-manual-step') {
      setPhase('manual')
    } else {
      setError(result.message ?? OP_STATE[result.state].label)
      setPhase('error')
    }
  }

  const run = async () => {
    if (!target) return
    setPhase('running')
    setError(null)
    setStage(rollback ? 'rolling-back' : 'checking')
    const un = await onFixProgress((o) => {
      if (alive.current && o.appId === appId) setStage(o.state)
    })
    try {
      if (target.kind === 'apply' && plan) finish(await applyFixPlan(plan))
      else if (rb) finish(await rollbackFix(rb))
    } catch (e) {
      setError(errorMessage(e))
      setPhase('error')
      void engine.refresh()
    } finally {
      un()
    }
  }

  const verify = async () => {
    if (!op) return
    setVerifying(true)
    try {
      const r = await verifyFixOperation(op.id)
      if (r.state === 'requires-manual-step' || r.state === 'recovery-required') {
        setOp(r)
        void engine.refresh()
      } else finish(r)
    } catch (e) {
      setError(errorMessage(e))
    } finally {
      setVerifying(false)
    }
  }

  // Ao trocar de etapa o botão focado some: leva o foco para o controle
  // principal da nova etapa (teclado e leitores de tela não ficam perdidos).
  useEffect(() => {
    const t = setTimeout(() => {
      const dialog = document.querySelector<HTMLElement>('[role="dialog"]')
      const box = dialog?.querySelector<HTMLElement>('input[type="checkbox"]')
      if (box) return box.focus()
      if (!dialog || (dialog.contains(document.activeElement) && document.activeElement !== dialog)) return
      const buttons = [...dialog.querySelectorAll<HTMLButtonElement>('button:not([disabled])')]
      buttons[buttons.length - 1]?.focus()
    }, 260) // depois da transição entre etapas (AnimatePresence em modo "wait")
    return () => clearTimeout(t)
  }, [phase])

  if (!target) return null
  const title =
    target.kind === 'apply'
      ? target.title
      : target.operation.state === 'requires-manual-step' || target.operation.state === 'recovery-required'
        ? `Conferir ${target.operation.recipeTitle}`
        : `Desfazer ${target.operation.recipeTitle}`
  const status = rollback ? rb?.status : plan?.status
  const executable = status === 'ready' || status === 'requires-manual-step'
  const manualMode = status === 'requires-manual-step'
  const changes = (rollback ? rb?.changes : plan?.changes) ?? []
  const reasons = (rollback ? rb?.reasons : plan?.reasons) ?? []
  const stages = rollback ? ROLLBACK_STAGES : APPLY_STAGES
  const stageIndex = stage ? stages.findIndex((s) => s.state === stage) : -1
  // No resultado vale o estado real da operação, não o tipo de diálogo.
  const undone = op?.state === 'rolled-back'

  const pose =
    phase === 'loading' ? 'inspect' : phase === 'running' ? 'fix' : phase === 'done' ? (undone ? 'wave' : 'celebrate') : phase === 'error' ? 'worried' : phase === 'manual' ? 'confused' : 'inspect'

  const description = (() => {
    switch (phase) {
      case 'loading':
        return 'Lendo o estado real do jogo e da Steam…'
      case 'review':
        return rollback ? 'Prévia do rollback — nada foi alterado ainda.' : `${plan?.gameName ?? ''} · prévia calculada agora, nada foi alterado ainda.`
      case 'confirm':
        return 'Confirme para o FRX gravar exatamente o que a prévia mostrou.'
      case 'running':
        return rollback ? 'O Pim está desfazendo só o que esta correção adicionou.' : 'O Pim está aplicando a correção. Não feche a Steam nem abra o jogo agora.'
      case 'manual':
        return 'Falta um passo seu na Steam. O FRX só marca como concluído depois de conferir no arquivo.'
      case 'done':
        return undone ? 'Conferido no disco: a correção foi removida.' : 'Conferido no disco. Vale a partir da próxima vez que o jogo abrir.'
      case 'error':
        return 'Nada foi marcado como concluído.'
    }
  })()

  const footer = (() => {
    switch (phase) {
      case 'loading':
        return (
          <Button variant="ghost" onClick={onClose}>
            Cancelar
          </Button>
        )
      case 'review':
        return (
          <>
            <Button variant="ghost" onClick={onClose}>
              Cancelar
            </Button>
            {target.kind === 'rollback' && target.operation.state === 'recovery-required' ? (
              <Button variant="outline" loading={verifying} icon={<ShieldCheck className="size-3.5" />} onClick={() => void verify()}>
                Conferir estado real
              </Button>
            ) : (
              <Button variant="outline" icon={<RefreshCw className="size-3.5" />} onClick={() => void load()}>
                Conferir de novo
              </Button>
            )}
            {executable && (
              <Button variant={rollback ? 'danger' : 'primary'} icon={rollback ? <RotateCcw className="size-4" /> : undefined} onClick={() => (rollback ? void run() : setPhase('confirm'))}>
                {rollback ? (manualMode ? 'Desfazer manualmente' : changes.length ? 'Desfazer agora' : 'Encerrar operação') : manualMode ? 'Continuar no modo manual' : 'Continuar'}
              </Button>
            )}
          </>
        )
      case 'confirm':
        return (
          <>
            <Button variant="ghost" onClick={() => setPhase('review')}>
              Voltar
            </Button>
            <Button variant="primary" disabled={!ack} onClick={() => void run()}>
              {manualMode ? 'Registrar e mostrar a linha' : 'Aplicar correção'}
            </Button>
          </>
        )
      case 'running':
        return (
          <Button variant="ghost" disabled>
            Aplicando…
          </Button>
        )
      case 'manual':
        return (
          <>
            <Button variant="ghost" onClick={onClose}>
              Conferir depois
            </Button>
            {op?.pending?.kind === 'apply' && rb?.status === 'ready' && rb.changes.length === 0 && (
              <Button variant="outline" onClick={() => void run()}>
                Cancelar operação
              </Button>
            )}
            <Button variant="primary" loading={verifying} icon={<ShieldCheck className="size-4" />} onClick={() => void verify()}>
              Conferir agora
            </Button>
          </>
        )
      case 'done':
        return (
          <>
            <Button variant="ghost" onClick={onClose}>
              Fechar
            </Button>
            {!undone && (
              <Button
                variant="primary"
                icon={<Gauge className="size-4" />}
                onClick={() => {
                  onClose()
                  navigate(`/performance?game=${appId}`)
                }}
              >
                Medir desempenho
              </Button>
            )}
          </>
        )
      case 'error':
        return (
          <>
            <Button variant="ghost" onClick={onClose}>
              Fechar
            </Button>
            <Button variant="primary" icon={<RefreshCw className="size-4" />} onClick={() => void load()}>
              Revisar de novo
            </Button>
          </>
        )
    }
  })()

  return (
    <Dialog
      open
      onClose={onClose}
      locked={phase === 'running'}
      title={title}
      description={description}
      footer={footer}
      width={640}
      aside={
        <div className="-mt-1 -ml-2 hidden sm:block">
          <Mascot pose={pose} scale={3} />
        </div>
      }
    >
      <AnimatePresence mode="wait" initial={false}>
        <motion.div key={phase} initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -4 }} transition={{ duration: 0.16 }}>
          {phase === 'loading' && <div className="h-40 rounded-md skeleton" aria-label="Carregando prévia" />}

          {phase === 'review' && (
            <div className="space-y-5">
              <div className="flex flex-wrap items-center gap-2">
                {status && (
                  <Pill color={PLAN_STATUS[status].color} soft={PLAN_STATUS[status].soft}>
                    {rollback && status === 'ready' ? 'Pode ser desfeita' : rollback && status === 'requires-manual-step' ? 'Desfazer · passo manual' : PLAN_STATUS[status].label}
                  </Pill>
                )}
                {plan && (
                  <>
                    <Pill color={RISK[plan.risk].color} soft={RISK[plan.risk].soft}>
                      {RISK[plan.risk].label}
                    </Pill>
                    {plan.trust === 'generated' && (
                      <Pill color={TRUST.generated.color} soft={TRUST.generated.soft} title={TRUST.generated.help}>
                        <Wrench className="size-3" aria-hidden /> Receita gerada automaticamente
                      </Pill>
                    )}
                    {plan.trust === 'pack' && (
                      <Pill color={TRUST.pack.color} soft={TRUST.pack.soft} title={TRUST.pack.help}>
                        <Wrench className="size-3" aria-hidden /> Smart Pack · {plan.pack?.members.length ?? 0} receitas
                      </Pill>
                    )}
                    {plan.generated && (plan.generated.evidence === 'single' || plan.generated.evidence === 'none') && (
                      <span className="inline-flex h-6 items-center gap-1 rounded-xs bg-caution-soft px-2 text-[12px] font-medium text-caution">
                        <FlaskConical className="size-3" aria-hidden /> Experimental
                      </span>
                    )}
                    <span className="text-[12px] text-fg-3">
                      {PURPOSE[plan.purpose]} · receita v{plan.recipeVersion}
                    </span>
                  </>
                )}
              </div>
              {!rollback && <PerformanceWarning warnings={plan?.performanceWarnings} />}
              {reasons.length > 0 && (
                <ul className={cn('space-y-1.5 rounded-md border px-3.5 py-2.5 text-[13px]', executable ? 'border-caution/25 bg-caution-soft text-caution' : 'border-bad/25 bg-bad-soft text-bad')}>
                  {reasons.map((r) => (
                    <li key={r} className="flex gap-2">
                      <TriangleAlert className="mt-0.5 size-3.5 shrink-0" aria-hidden /> {r}
                    </li>
                  ))}
                </ul>
              )}
              {changes.length > 0 && (
                <div>
                  <div className="eyebrow mb-2.5">{rollback ? 'O que será desfeito' : manualMode ? 'O que você vai colar na Steam' : 'O que será alterado'}</div>
                  <ChangesDiff changes={changes} />
                </div>
              )}
              {plan && <Rationale plan={plan} />}
              {plan && <ChecksList checks={plan.checks} />}
              {plan?.backupNote && (
                <p className="flex gap-2.5 text-[12.5px] text-fg-2">
                  <ArchiveRestore className="mt-0.5 size-3.5 shrink-0 text-fg-3" aria-hidden /> {plan.backupNote}
                </p>
              )}
            </div>
          )}

          {phase === 'confirm' && plan && (
            <div className="space-y-4">
              <PerformanceWarning warnings={plan.performanceWarnings} />
              <ul className="space-y-3 text-[13.5px]">
                <li className="flex gap-3">
                  <ShieldCheck className="mt-0.5 size-4 shrink-0 text-fg-2" aria-hidden />
                  <span>O FRX vai recalcular a prévia antes de gravar. Se algo mudou na Steam ou no jogo desde agora, nada é alterado.</span>
                </li>
                <li className="flex gap-3">
                  <RotateCcw className="mt-0.5 size-4 shrink-0 text-fg-2" aria-hidden />
                  <span>Dá para desfazer pela página do jogo ou pelo histórico em Fix Packs. O rollback remove só o que esta correção adicionou.</span>
                </li>
              </ul>
              <label className="flex cursor-pointer items-start gap-3 rounded-md border border-line-2 bg-raised/50 px-3.5 py-3 text-[13.5px] transition-colors hover:border-line-3">
                <input type="checkbox" checked={ack} onChange={(e) => setAck(e.target.checked)} className="mt-0.5 size-4 accent-[var(--frx-accent)]" />
                <span>
                  {manualMode
                    ? 'Entendo que vou colar a linha nas opções de inicialização do jogo na Steam e que a correção só conta como aplicada depois que o FRX confirmar.'
                    : plan.trust === 'pack'
                      ? 'Entendo que este Smart Pack combina receitas validadas uma a uma e em conjunto, que a combinação não comprova ganho de desempenho e que só um benchmark no meu computador mostra o efeito.'
                      : plan.trust === 'generated'
                      ? 'Entendo que esta correção foi gerada automaticamente a partir de relatos públicos, não foi revisada por pessoas e pode não resolver o problema no meu sistema.'
                      : plan.purpose === 'diagnostic'
                      ? 'Entendo que esta correção serve para diagnóstico e não melhora o desempenho.'
                      : 'Entendo que o resultado depende do meu sistema e que a melhora não é garantida.'}
                </span>
              </label>
            </div>
          )}

          {(phase === 'running' || (phase === 'error' && stageIndex >= 0)) && (
            <div>
              <ol className="space-y-1" aria-live="polite">
                {stages.map((s, i) => {
                  const failed = phase === 'error' && i === stageIndex
                  const state = failed ? 'failed' : i < stageIndex ? 'done' : i === stageIndex && phase === 'running' ? 'active' : 'pending'
                  return (
                    <li key={s.state} className={cn('flex h-9 items-center gap-3 rounded-md px-2 text-[13.5px] transition-colors', state === 'active' && 'bg-raised')}>
                      <span
                        className={cn(
                          'grid size-5 place-items-center rounded-[5px] border',
                          state === 'done' && 'border-good/40 bg-good-soft text-good',
                          state === 'active' && 'border-accent-line text-accent',
                          state === 'pending' && 'border-line-2 text-fg-3',
                          state === 'failed' && 'border-bad/40 bg-bad-soft text-bad',
                        )}
                        aria-hidden
                      >
                        {state === 'done' ? <Check className="size-3" strokeWidth={3} /> : state === 'failed' ? '!' : state === 'active' ? <span className="anim-pulse-dot size-1.5 bg-accent" /> : null}
                      </span>
                      <span className={cn(state === 'pending' ? 'text-fg-3' : 'text-fg', state === 'failed' && 'text-bad')}>{s.label}</span>
                    </li>
                  )
                })}
              </ol>
            </div>
          )}

          {phase === 'manual' && op && (
            <div className="space-y-4 text-[13.5px]">
              <ol className="list-decimal space-y-1.5 pl-5 text-fg-2">
                <li>Na Steam, abra Propriedades do jogo › Geral › Opções de inicialização.</li>
                <li>{op.pending?.kind === 'rollback' ? 'Substitua o conteúdo pela linha abaixo (sem a correção):' : 'Substitua o conteúdo pela linha abaixo:'}</li>
              </ol>
              <ManualLine line={op.pending?.line ?? ''} />
              <p className="text-[12.5px] text-fg-3">A Steam grava as opções no disco ao fechar. Depois de colar, feche a Steam e clique em “Conferir agora”.</p>
              {op.message && <p className="rounded-md border border-line-2 bg-raised/50 px-3.5 py-2.5 text-[12.5px] text-fg-2">{op.message}</p>}
              {error && <p className="rounded-md border border-bad/25 bg-bad-soft px-3.5 py-2.5 text-[13px] text-bad">{error}</p>}
            </div>
          )}

          {phase === 'done' && op && (
            <div className="space-y-3 text-[13.5px] text-fg-2">
              <div className="flex items-center gap-3 rounded-md border border-good/25 bg-good-soft px-3.5 py-3 text-good">
                <Check className="size-4" strokeWidth={2.5} aria-hidden />
                {op.message}
              </div>
              {!undone && op.recipeId !== 'frx.diagnostics.proton-log' && <p>Para saber se fez diferença, compare um benchmark de antes com um de depois. O FRX não atribui mudanças de desempenho à correção.</p>}
              {op.recipeId === 'frx.diagnostics.proton-log' && !undone && <p>O log fica em ~/steam-{op.appId}.log depois que o jogo abrir. Desfaça quando terminar o diagnóstico.</p>}
            </div>
          )}

          {phase === 'error' && error && <p className="mt-4 rounded-md border border-bad/25 bg-bad-soft px-3.5 py-2.5 text-[13px] text-bad">{error}</p>}
        </motion.div>
      </AnimatePresence>
    </Dialog>
  )
}

/** Por que a correção foi sugerida, o que ela faz, riscos e fontes originais. */
function Rationale({ plan }: { plan: FixPlan }) {
  const g = plan.generated
  const open = (url: string) => void openExternal(url).catch(() => {})
  if (!g && !plan.effects.length && !plan.sideEffects.length) return null
  return (
    <div className="space-y-3">
      {g && (
        <div>
          <div className="eyebrow mb-2.5">Por que foi sugerida</div>
          <p className="text-[12.5px] text-fg-2">
            {EVIDENCE[g.evidence]} — {g.evidenceNote}
          </p>
          <ul className="mt-2 space-y-2">
            {g.provenance.map((p) => (
              <li key={p.url} className="rounded-md border border-line bg-bg/40 px-3 py-2">
                <div className="flex items-center gap-2 text-[12.5px]">
                  <span className="min-w-0 flex-1 truncate text-fg">{p.label}</span>
                  <button onClick={() => open(p.url)} className="inline-flex shrink-0 items-center gap-1 rounded-xs text-[12px] font-medium text-fg-2 hover:text-accent" aria-label={`Abrir fonte: ${p.label}`}>
                    Abrir <ExternalLink className="size-3" />
                  </button>
                </div>
                {p.snippet && <p className="mt-1 line-clamp-3 border-l-2 border-line-2 pl-2.5 font-mono text-[11.5px] break-words text-fg-3">{p.snippet}</p>}
              </li>
            ))}
          </ul>
        </div>
      )}
      {(plan.effects.length > 0 || plan.sideEffects.length > 0) && (
        <ul className="space-y-1 text-[12.5px]">
          {plan.effects.map((e) => (
            <li key={e} className="flex gap-2 text-fg-2">
              <span className="w-16 shrink-0 text-fg-3">Efeito</span>
              {e}
            </li>
          ))}
          {plan.sideEffects.map((e) => (
            <li key={e} className="flex gap-2 text-fg-2">
              <span className="w-16 shrink-0 text-caution">Risco</span>
              {e}
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

export function ChangesDiff({ changes }: { changes: PlannedChange[] }) {
  return (
    <ul className="divide-y divide-line overflow-hidden rounded-md border border-line bg-bg/50">
      {changes.map((c, i) => (
        <li key={i} className="px-3.5 py-3">
          <div className="flex items-center gap-2.5">
            <span className="grid size-[18px] shrink-0 place-items-center rounded-[4px] bg-accent-soft font-mono text-[12px] leading-none font-semibold text-accent" aria-hidden>
              ~
            </span>
            <span className="min-w-0 flex-1 truncate text-[12.5px] text-fg" title={c.target}>
              {c.target}
            </span>
          </div>
          <p className="mt-1 ml-7 text-[12px] text-fg-3">{c.summary}</p>
          {(c.before !== null || c.after !== null) && (
            <div className="mt-2 ml-7 space-y-0.5 font-mono text-[12px]">
              <div className="break-all text-fg-3">
                <span className="mr-2 select-none text-bad/80">−</span>
                {c.before ? <span className="line-through decoration-fg-3/40">{c.before}</span> : <span className="italic">(vazio)</span>}
              </div>
              <div className="break-all text-fg-2">
                <span className="mr-2 select-none text-good">+</span>
                {c.after ? c.after : <span className="italic">(vazio)</span>}
              </div>
            </div>
          )}
        </li>
      ))}
    </ul>
  )
}

export function ChecksList({ checks }: { checks: PlanCheck[] }) {
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

function ManualLine({ line }: { line: string }) {
  const toast = useToast()
  const copy = () =>
    navigator.clipboard
      .writeText(line)
      .then(() => toast({ tone: 'ok', title: 'Linha copiada' }))
      .catch(() => toast({ tone: 'error', title: 'Não foi possível copiar', body: 'Selecione o texto e copie manualmente.' }))
  return (
    <div className="flex items-center gap-2 rounded-md border border-line-2 bg-bg px-3 py-2.5">
      <code className="min-w-0 flex-1 font-mono text-[12.5px] break-all text-fg select-all">{line || '(deixe as opções vazias)'}</code>
      <Button size="sm" variant="outline" icon={<ClipboardCopy className="size-3.5" />} onClick={copy} disabled={!line}>
        Copiar
      </Button>
    </div>
  )
}
