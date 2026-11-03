import { useCallback, useEffect, useRef, useState } from 'react'
import { ArchiveRestore, BookOpen, ClipboardCopy, ExternalLink, FileCode2, RefreshCw, RotateCcw, ShieldAlert, TerminalSquare, TriangleAlert } from 'lucide-react'
import { Dialog } from '@/components/ui/Dialog'
import { Button } from '@/components/ui/Button'
import { Pill } from '@/components/ui/Status'
import { ChecksList } from '@/components/fix/ApplyFixDialog'
import { OP_STATE, PLAN_STATUS, RISK } from '@/components/fix/engineMeta'
import { applySystemFix, prepareSystemFix } from '@/services/hardware'
import { prepareFixRollback, rollbackFix, verifyFixOperation } from '@/services/fixEngine'
import { errorMessage } from '@/services/native'
import { openExternal } from '@/services/opener'
import { useSettings, useToast } from '@/state/store'
import type { FixPlan, Operation, PlannedChange, RollbackPlan } from '@/types/fixEngine'
import type { HardwareSymptom, WikiAction, WikiSuggestion } from '@/types/hardware'
import { cn } from '@/lib/cn'

export type SystemFixTarget =
  | { kind: 'apply'; symptom: HardwareSymptom; suggestion: WikiSuggestion; action: WikiAction }
  | { kind: 'rollback'; operation: Operation }
  /** Operação aguardando o comando `sudo` (aplicar ou desfazer). */
  | { kind: 'pending'; operation: Operation }

type Phase = 'loading' | 'review' | 'running' | 'manual' | 'done' | 'error'

/**
 * Prévia e execução de uma correção de sistema. O núcleo calcula tudo; a
 * interface só envia IDs. Com o sistema real, o FRX não grava em /etc:
 * faz o backup, prepara o arquivo e mostra o comando `sudo` para o usuário.
 */
export function SystemFixDialog({ target, onClose, onChanged }: { target: SystemFixTarget | null; onClose: () => void; onChanged?: () => void }) {
  if (!target) return null
  const key = target.kind === 'apply' ? `${target.suggestion.id}:${target.action.id}` : `${target.kind}:${target.operation.id}`
  return <Scoped key={key} target={target} onClose={onClose} onChanged={onChanged} />
}

function Scoped({ target, onClose, onChanged }: { target: SystemFixTarget; onClose: () => void; onChanged?: () => void }) {
  const { fixCacheHours } = useSettings()
  const toast = useToast()
  const [phase, setPhase] = useState<Phase>('loading')
  const [plan, setPlan] = useState<FixPlan | null>(null)
  const [rb, setRb] = useState<RollbackPlan | null>(null)
  const [op, setOp] = useState<Operation | null>(target.kind === 'pending' ? target.operation : null)
  const [error, setError] = useState<string | null>(null)
  const [ack, setAck] = useState(false)
  const [verifying, setVerifying] = useState(false)
  const alive = useRef(true)
  useEffect(() => {
    alive.current = true
    return () => {
      alive.current = false
    }
  }, [])

  const load = useCallback(async () => {
    setPhase('loading')
    setError(null)
    try {
      if (target.kind === 'apply') {
        const p = await prepareSystemFix({ symptom: target.symptom, suggestionId: target.suggestion.id, actionId: target.action.id, maxAgeHours: fixCacheHours })
        if (alive.current) setPlan(p)
      } else if (target.kind === 'rollback') {
        const p = await prepareFixRollback(target.operation.id)
        if (alive.current) setRb(p)
      }
      if (alive.current) setPhase(target.kind === 'pending' ? 'manual' : 'review')
    } catch (e) {
      if (alive.current) {
        setError(errorMessage(e))
        setPhase('error')
      }
    }
  }, [target, fixCacheHours])

  useEffect(() => void load(), [load])

  const finish = (o: Operation) => {
    setOp(o)
    onChanged?.()
    setPhase(o.state === 'requires-manual-step' ? 'manual' : 'done')
  }

  const run = async () => {
    setPhase('running')
    setError(null)
    try {
      if (plan) finish(await applySystemFix(plan))
      else if (rb) finish(await rollbackFix(rb))
    } catch (e) {
      setError(errorMessage(e))
      setPhase('error')
    }
  }

  const verify = async () => {
    if (!op) return
    setVerifying(true)
    try {
      const o = await verifyFixOperation(op.id)
      finish(o)
      if (o.state === 'requires-manual-step') toast({ tone: 'info', title: 'Ainda não confere', body: o.message ?? undefined })
    } catch (e) {
      toast({ tone: 'error', title: 'Não foi possível conferir', body: errorMessage(e) })
    } finally {
      setVerifying(false)
    }
  }

  const open = (url: string) => void openExternal(url).catch(() => toast({ tone: 'error', title: 'Não foi possível abrir o link' }))
  const conditional = target.kind === 'apply' && target.action.conditional
  const needsAck = plan ? plan.risk === 'high' || conditional : false
  const executable = plan ? plan.status === 'ready' || plan.status === 'requires-manual-step' : rb ? rb.status === 'ready' || rb.status === 'requires-manual-step' : false
  const manualPlan = plan?.mode === 'manual' || rb?.mode === 'manual'

  const title =
    target.kind === 'apply' ? 'Aplicar correção' : target.kind === 'rollback' ? 'Desfazer correção' : op?.pending?.kind === 'rollback' ? 'Desfazer: passo no terminal' : 'Aplicar: passo no terminal'
  const subject = plan?.recipeTitle ?? (target.kind !== 'apply' ? target.operation.recipeTitle : '')

  const footer =
    phase === 'review' ? (
      <>
        <Button variant="ghost" onClick={onClose}>
          Cancelar
        </Button>
        <Button variant={rb ? 'danger' : 'primary'} disabled={!executable || (needsAck && !ack)} onClick={run} icon={rb ? <RotateCcw className="size-4" /> : undefined}>
          {rb ? (manualPlan ? 'Preparar o desfazer' : 'Desfazer') : manualPlan ? 'Fazer backup e preparar arquivo' : 'Aplicar'}
        </Button>
      </>
    ) : phase === 'manual' ? (
      <>
        <Button variant="ghost" onClick={onClose}>
          Fechar e conferir depois
        </Button>
        <Button variant="primary" loading={verifying} icon={<RefreshCw className="size-4" />} onClick={verify}>
          Já rodei — conferir
        </Button>
      </>
    ) : phase === 'error' ? (
      <>
        <Button variant="ghost" onClick={onClose}>
          Fechar
        </Button>
        <Button onClick={load}>{target.kind === 'apply' ? 'Refazer a prévia' : 'Tentar de novo'}</Button>
      </>
    ) : phase === 'done' ? (
      <Button variant="primary" onClick={onClose}>
        Concluir
      </Button>
    ) : null

  return (
    <Dialog open onClose={onClose} locked={phase === 'running'} width={680} title={title} description={subject} footer={footer}>
      {phase === 'loading' && <div className="h-[220px] rounded-md skeleton" aria-label="Calculando a prévia" />}

      {phase === 'error' && (
        <p role="alert" className="rounded-md border border-bad/25 bg-bad-soft px-3.5 py-3 text-[13px] text-bad">
          {error}
        </p>
      )}

      {phase === 'review' && plan && (
        <div className="space-y-5">
          <div className="flex flex-wrap gap-2">
            <Pill color={PLAN_STATUS[plan.status].color} soft={PLAN_STATUS[plan.status].soft}>
              {PLAN_STATUS[plan.status].label}
            </Pill>
            <Pill color={RISK[plan.risk].color} soft={RISK[plan.risk].soft}>
              {RISK[plan.risk].label}
            </Pill>
            <Pill color="var(--frx-text-2)" soft="var(--frx-raised)">
              {plan.gameName}
            </Pill>
          </div>

          {plan.reasons.length > 0 && (
            <ul className="space-y-1 text-[13px] text-fg-2">
              {plan.reasons.map((r) => (
                <li key={r}>{r}</li>
              ))}
            </ul>
          )}

          <section aria-label="O que a fonte diz">
            <div className="eyebrow mb-2">O que a fonte diz</div>
            <blockquote className="border-l-2 border-line-2 pl-3 text-[13px] text-fg-2">{plan.effects[0]}</blockquote>
            {plan.sources.map((s) => (
              <button key={s.url} type="button" onClick={() => open(s.url)} className="mt-2 inline-flex items-center gap-1.5 text-[12.5px] text-fg underline-offset-2 hover:underline">
                <BookOpen className="size-3.5" aria-hidden /> {s.label} <ExternalLink className="size-3" aria-hidden />
              </button>
            ))}
            {conditional && (
              <p className="mt-3 flex gap-2 rounded-md border border-caution/30 bg-caution-soft px-3 py-2.5 text-[12.5px] text-caution">
                <TriangleAlert className="mt-0.5 size-3.5 shrink-0" aria-hidden />
                A frase impõe condições que o FRX não consegue verificar (modelo da placa, processador, sintoma exato). Confirme que valem para o seu laptop antes de seguir.
              </p>
            )}
          </section>

          <section aria-label="Arquivo alterado">
            <div className="eyebrow mb-2">Arquivo do sistema</div>
            {plan.changes.map((c) => (
              <SystemChange key={c.target} change={c} />
            ))}
          </section>

          <ChecksList checks={plan.checks} />

          {plan.backupNote && (
            <p className="flex gap-2 text-[12.5px] text-fg-2">
              <ArchiveRestore className="mt-0.5 size-3.5 shrink-0 text-fg-3" aria-hidden />
              {plan.backupNote}
            </p>
          )}

          {plan.sideEffects.length > 0 && (
            <ul className="space-y-1 text-[12.5px] text-fg-2">
              {plan.sideEffects.map((s) => (
                <li key={s} className="flex gap-2">
                  <span className="mt-[7px] size-1 shrink-0 bg-fg-3" aria-hidden />
                  {s}
                </li>
              ))}
            </ul>
          )}

          {plan.manualLine && <RootNotice line={plan.manualLine} preview />}

          {needsAck && executable && (
            <label className="flex cursor-pointer items-start gap-2.5 rounded-md border border-line-2 px-3 py-2.5 text-[13px] text-fg">
              <input type="checkbox" className="mt-0.5 accent-(--frx-accent)" checked={ack} onChange={(e) => setAck(e.target.checked)} />
              <span>
                {plan.risk === 'high'
                  ? 'Entendo que um parâmetro do kernel errado pode impedir o sistema de iniciar, e sei como removê-lo pelo menu do GRUB.'
                  : 'Li o trecho da wiki e as condições valem para o meu laptop.'}
              </span>
            </label>
          )}
        </div>
      )}

      {phase === 'review' && rb && (
        <div className="space-y-4">
          <Pill color={PLAN_STATUS[rb.status].color} soft={PLAN_STATUS[rb.status].soft}>
            {PLAN_STATUS[rb.status].label}
          </Pill>
          <ul className="space-y-1 text-[13px] text-fg-2">
            {rb.changes.map((c) => (
              <li key={c.target} className="font-mono text-[12.5px] text-fg">
                {c.summary}
              </li>
            ))}
            {rb.reasons.map((r) => (
              <li key={r}>{r}</li>
            ))}
          </ul>
          {rb.manualLine && <RootNotice line={rb.manualLine} preview />}
        </div>
      )}

      {phase === 'running' && <div className="h-[120px] rounded-md skeleton" aria-label="Executando" />}

      {phase === 'manual' && op && (
        <div className="space-y-4">
          <Pill color={OP_STATE[op.state].color} soft={OP_STATE[op.state].soft}>
            {OP_STATE[op.state].label}
          </Pill>
          {op.message && <p className="text-[13px] text-fg-2">{op.message}</p>}
          <RootNotice line={op.pending?.line ?? ''} />
        </div>
      )}

      {phase === 'done' && op && (
        <div className="space-y-3">
          <Pill color={OP_STATE[op.state].color} soft={OP_STATE[op.state].soft}>
            {OP_STATE[op.state].label}
          </Pill>
          <p className="text-[13.5px] text-fg-2">{op.message}</p>
        </div>
      )}
    </Dialog>
  )
}

/** Arquivo novo: conteúdo completo. Edição de linha (GRUB): antes e depois. */
function SystemChange({ change }: { change: PlannedChange }) {
  const lines = (text: string | null, sign: '+' | '−') =>
    text?.split('\n').filter((l, i, all) => l !== '' || i < all.length - 1).map((l, i) => (
      <div key={i} className={cn('whitespace-pre-wrap break-all', sign === '+' ? 'text-fg-2' : 'text-fg-3 line-through decoration-fg-3/40')}>
        <span className={cn('mr-2 select-none', sign === '+' ? 'text-good' : 'text-bad/80')}>{sign}</span>
        {l}
      </div>
    ))
  return (
    <div className="overflow-hidden rounded-md border border-line bg-bg/50">
      <div className="flex items-center gap-2 border-b border-line px-3.5 py-2.5">
        <FileCode2 className="size-3.5 text-fg-3" aria-hidden />
        <code className="min-w-0 flex-1 truncate font-mono text-[12.5px] text-fg">{change.target}</code>
      </div>
      <p className="px-3.5 pt-2.5 text-[12px] text-fg-3">{change.summary}</p>
      {(change.before || change.after) && (
        <div className="space-y-0.5 px-3.5 pt-2 pb-3 font-mono text-[12px]">
          {lines(change.before, '−')}
          {lines(change.after, '+')}
        </div>
      )}
    </div>
  )
}

/** Comandos que exigem root. O FRX só mostra e copia; nunca executa. */
function RootNotice({ line, preview }: { line: string; preview?: boolean }) {
  const toast = useToast()
  const commands = line.split(' && ').filter(Boolean)
  const copy = () =>
    navigator.clipboard
      .writeText(line)
      .then(() => toast({ tone: 'ok', title: 'Comando copiado' }))
      .catch(() => toast({ tone: 'error', title: 'Não foi possível copiar', body: 'Selecione o texto e copie manualmente.' }))
  return (
    <section aria-label="Comando com sudo" className="rounded-md border border-line-2 bg-bg">
      <div className="flex items-start gap-2.5 border-b border-line px-3.5 py-3 text-[12.5px] text-fg-2">
        <ShieldAlert className="mt-0.5 size-4 shrink-0 text-caution" aria-hidden />
        <p>
          {preview
            ? 'Gravar em /etc exige root, e o FRX não usa sudo. Ao confirmar, ele faz o backup e prepara o arquivo numa pasta privada; você roda o comando abaixo no terminal e o FRX confere o resultado.'
            : 'Rode no terminal. O FRX não executa este comando; depois, clique em “Já rodei — conferir” para ele verificar o arquivo.'}
        </p>
      </div>
      <div className="flex items-start gap-2 px-3.5 py-3">
        <TerminalSquare className="mt-0.5 size-4 shrink-0 text-fg-3" aria-hidden />
        <div className="min-w-0 flex-1 space-y-1 font-mono text-[12px] break-all text-fg select-all">
          {commands.map((c, i) => (
            <div key={i}>
              {c}
              {i < commands.length - 1 && <span className="text-fg-3"> &&</span>}
            </div>
          ))}
        </div>
        {!preview && (
          <Button size="sm" variant="outline" icon={<ClipboardCopy className="size-3.5" />} onClick={copy}>
            Copiar
          </Button>
        )}
      </div>
    </section>
  )
}
