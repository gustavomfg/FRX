import { useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { AnimatePresence, motion } from 'motion/react'
import { ArchiveRestore, Check, Gauge, RotateCcw, ShieldCheck } from 'lucide-react'
import type { FixPack } from '@/types'
import { Dialog } from '@/components/ui/Dialog'
import { Button } from '@/components/ui/Button'
import { RiskBadge } from '@/components/ui/Status'
import { Mascot } from '@/mascot/Mascot'
import { ChangeList } from './ChangeList'
import { APPLY_STEPS, REVERT_STEPS, applyFix, revertFix, DemoError } from '@/services/demo'
import { useAppState, useDispatch, useSettings, useToast } from '@/state/store'
import { gameById } from '@/data/games'
import { cn } from '@/lib/cn'

type Phase = 'review' | 'confirm' | 'running' | 'done' | 'error'

interface Props {
  fix: FixPack | null
  mode: 'apply' | 'revert'
  open: boolean
  onClose: () => void
}

/**
 * Fluxo demonstrativo: revisar → confirmar → aplicar → resultado.
 * Tudo acontece apenas no estado da interface.
 */
export function FixFlowDialog({ fix, mode, open, onClose }: Props) {
  const settings = useSettings()
  const { failedAttempts } = useAppState()
  const dispatch = useDispatch()
  const toast = useToast()
  const navigate = useNavigate()
  const [phase, setPhase] = useState<Phase>('review')
  const [step, setStep] = useState(0)
  const [ack, setAck] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const abort = useRef<AbortController | null>(null)

  useEffect(() => {
    if (open) {
      setPhase('review')
      setStep(0)
      setAck(false)
      setError(null)
    }
    return () => abort.current?.abort()
  }, [open, fix?.id, mode])

  if (!fix) return null
  const game = gameById(fix.gameId)
  const steps = mode === 'apply' ? APPLY_STEPS : REVERT_STEPS

  const run = async () => {
    setPhase('running')
    setStep(0)
    setError(null)
    abort.current = new AbortController()
    try {
      if (mode === 'apply') {
        await applyFix(fix, (failedAttempts[fix.id] ?? 0) + 1, (p) => setStep(p.step), abort.current.signal)
        dispatch({ type: 'fixApplied', fixId: fix.id, gameId: fix.gameId })
        toast({ tone: 'ok', title: 'Correção aplicada', body: `${fix.name} · simulado, nada foi alterado no sistema.` })
      } else {
        await revertFix(fix, (p) => setStep(p.step), abort.current.signal)
        dispatch({ type: 'fixReverted', fixId: fix.id, gameId: fix.gameId })
        toast({ tone: 'info', title: 'Correção revertida', body: `${fix.name} · backup restaurado (simulado).` })
      }
      setPhase('done')
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') return
      const msg = e instanceof DemoError ? e.message : 'Erro inesperado na simulação.'
      if (e instanceof DemoError) setStep(e.step)
      setError(msg)
      dispatch({ type: 'fixFailed', fixId: fix.id, gameId: fix.gameId, note: msg })
      setPhase('error')
    }
  }

  const next = () => {
    if (phase === 'review') {
      if (mode === 'apply' && settings.confirmBeforeApply) setPhase('confirm')
      else run()
    } else if (phase === 'confirm') run()
  }

  const title =
    phase === 'done'
      ? mode === 'apply'
        ? 'Correção aplicada'
        : 'Correção revertida'
      : phase === 'error'
        ? 'Não deu certo desta vez'
        : mode === 'apply'
          ? fix.name
          : `Reverter ${fix.name}`

  const description =
    phase === 'running'
      ? mode === 'apply'
        ? 'O Pim está ajustando as peças. Isso é só uma simulação.'
        : 'Restaurando o estado anterior a partir do backup (simulado).'
      : phase === 'done'
        ? mode === 'apply'
          ? 'Nada foi modificado no seu sistema — o estado mudou apenas nesta demonstração.'
          : 'O jogo voltou à configuração anterior nesta demonstração.'
        : phase === 'error'
          ? 'Nenhuma alteração ficou pela metade.'
          : `${game?.title ?? ''} · ${mode === 'apply' ? 'revise antes de aplicar' : 'o backup será restaurado'}`

  const pose = phase === 'running' ? 'fix' : phase === 'done' ? (mode === 'apply' ? 'celebrate' : 'wave') : phase === 'error' ? 'worried' : phase === 'confirm' ? 'idle' : 'inspect'

  const footer = (() => {
    switch (phase) {
      case 'review':
        return (
          <>
            <Button variant="ghost" onClick={onClose}>
              Cancelar
            </Button>
            <Button variant={mode === 'apply' ? 'primary' : 'danger'} onClick={next} icon={mode === 'revert' ? <RotateCcw className="size-4" /> : undefined}>
              {mode === 'apply' ? (settings.confirmBeforeApply ? 'Continuar' : 'Aplicar correção') : 'Reverter agora'}
            </Button>
          </>
        )
      case 'confirm':
        return (
          <>
            <Button variant="ghost" onClick={() => setPhase('review')}>
              Voltar
            </Button>
            <Button variant="primary" onClick={next} disabled={!ack}>
              Aplicar correção
            </Button>
          </>
        )
      case 'running':
        return (
          <Button variant="ghost" onClick={() => abort.current?.abort() || onClose()}>
            Cancelar simulação
          </Button>
        )
      case 'done':
        return (
          <>
            <Button variant="ghost" onClick={onClose}>
              Fechar
            </Button>
            {mode === 'apply' && (
              <Button
                variant="primary"
                icon={<Gauge className="size-4" />}
                onClick={() => {
                  onClose()
                  navigate(`/performance?game=${fix.gameId}&run=1`)
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
            <Button variant="primary" icon={<RotateCcw className="size-4" />} onClick={run}>
              Tentar novamente
            </Button>
          </>
        )
    }
  })()

  return (
    <Dialog
      open={open}
      onClose={onClose}
      locked={phase === 'running'}
      title={title}
      description={description}
      footer={footer}
      width={600}
      aside={
        <div className="-mt-1 -ml-2 hidden sm:block">
          <Mascot pose={pose} scale={3} />
        </div>
      }
    >
      <AnimatePresence mode="wait" initial={false}>
        <motion.div key={phase} initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -4 }} transition={{ duration: 0.16 }}>
          {phase === 'review' && (
            <div className="space-y-5">
              {mode === 'apply' && <p className="text-[14px] text-fg-2">{fix.summary}</p>}
              <div className="flex flex-wrap items-center gap-2">
                <RiskBadge risk={fix.risk} />
                <Fact ok={fix.backup} label={fix.backup ? 'Backup automático' : 'Sem backup de arquivos'} />
                <Fact ok={fix.reversible} label={fix.reversible ? 'Reversível' : 'Não reversível'} />
              </div>
              <div>
                <div className="eyebrow mb-2.5">{mode === 'apply' ? 'O que seria alterado' : 'O que será restaurado'}</div>
                <ChangeList changes={fix.changes} inverted={mode === 'revert'} />
              </div>
              {fix.risk === 'alto' && mode === 'apply' && (
                <p className="rounded-md border border-bad/25 bg-bad-soft px-3.5 py-2.5 text-[13px] text-bad">
                  Relatos divergentes: esta correção pode reduzir o desempenho em alguns sistemas. Compare com um benchmark depois de aplicar.
                </p>
              )}
            </div>
          )}

          {phase === 'confirm' && (
            <div className="space-y-4">
              <ul className="space-y-3 text-[13.5px]">
                <li className="flex gap-3">
                  <ArchiveRestore className="mt-0.5 size-4 shrink-0 text-fg-2" aria-hidden />
                  <span>
                    {fix.backup ? 'Um backup do prefixo e das opções de inicialização seria criado em ' : 'As opções atuais seriam registradas em '}
                    <code className="rounded-xs bg-raised px-1 font-mono text-[12px]">~/.local/share/frx/backups</code>.
                  </span>
                </li>
                <li className="flex gap-3">
                  <RotateCcw className="mt-0.5 size-4 shrink-0 text-fg-2" aria-hidden />
                  <span>Você pode reverter a qualquer momento pela página do jogo ou em Fix Packs.</span>
                </li>
                <li className="flex gap-3">
                  <ShieldCheck className="mt-0.5 size-4 shrink-0 text-fg-2" aria-hidden />
                  <span>Fontes: {fix.sources.map((s) => s.label).join(', ')} · confiança de {fix.confidence}%.</span>
                </li>
              </ul>
              <label className="flex cursor-pointer items-start gap-3 rounded-md border border-line-2 bg-raised/50 px-3.5 py-3 text-[13.5px] transition-colors hover:border-line-3">
                <input type="checkbox" checked={ack} onChange={(e) => setAck(e.target.checked)} className="mt-0.5 size-4 accent-[var(--frx-accent)]" />
                <span>
                  Entendo que esta é uma <strong className="font-semibold">demonstração</strong>: nenhum arquivo será alterado e o ganho de desempenho não é garantido.
                </span>
              </label>
            </div>
          )}

          {(phase === 'running' || phase === 'error') && (
            <div>
              <ol className="space-y-1" aria-live="polite">
                {steps.map((label, i) => {
                  const failed = phase === 'error' && i === step
                  const state = failed ? 'failed' : i < step ? 'done' : i === step && phase === 'running' ? 'active' : 'pending'
                  return (
                    <li key={label} className={cn('flex h-9 items-center gap-3 rounded-md px-2 text-[13.5px] transition-colors', state === 'active' && 'bg-raised')}>
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
                      <span className={cn(state === 'pending' ? 'text-fg-3' : 'text-fg', state === 'failed' && 'text-bad')}>{label}</span>
                    </li>
                  )
                })}
              </ol>
              <div className="mt-4 h-1 overflow-hidden rounded-full bg-line-2">
                <motion.div
                  className={cn('h-full', phase === 'error' ? 'bg-bad' : 'bg-accent')}
                  animate={{ width: `${(Math.min(step, steps.length) / steps.length) * 100}%` }}
                  transition={{ duration: 0.4, ease: [0.2, 0.8, 0.2, 1] }}
                />
              </div>
              {error && <p className="mt-4 rounded-md border border-bad/25 bg-bad-soft px-3.5 py-2.5 text-[13px] text-bad">{error}</p>}
            </div>
          )}

          {phase === 'done' && (
            <div className="space-y-3 text-[13.5px] text-fg-2">
              <div className="flex items-center gap-3 rounded-md border border-good/25 bg-good-soft px-3.5 py-3 text-good">
                <Check className="size-4" strokeWidth={2.5} aria-hidden />
                {mode === 'apply' ? `${fix.changes.length} alteraç${fix.changes.length > 1 ? 'ões registradas' : 'ão registrada'} no estado demo.` : 'Estado anterior restaurado.'}
              </div>
              {mode === 'apply' && <p>Rode um benchmark para comparar com a medição anterior. Melhoras dependem do seu hardware e não são garantidas.</p>}
            </div>
          )}
        </motion.div>
      </AnimatePresence>
    </Dialog>
  )
}

function Fact({ ok, label }: { ok: boolean; label: string }) {
  return (
    <span className={cn('inline-flex h-6 items-center gap-1.5 rounded-xs px-2 text-[12px] font-medium', ok ? 'bg-raised text-fg-2' : 'bg-caution-soft text-caution')}>
      {ok ? <Check className="size-3" strokeWidth={2.6} aria-hidden /> : <span aria-hidden>!</span>}
      {label}
    </span>
  )
}
