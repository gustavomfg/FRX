import { Check, FlaskConical, Gauge, RotateCcw, Wrench } from 'lucide-react'
import { Button } from '@/components/ui/Button'
import { Dialog } from '@/components/ui/Dialog'
import { Pill } from '@/components/fix/FindingCard'
import { Mascot } from '@/mascot/Mascot'
import type { Pose } from '@/mascot/sprites'
import { fmtDateTime, fmtNum } from '@/lib/format'
import { cn } from '@/lib/cn'
import type { CaptureStatus } from '@/types/performance'
import type { EvalVerdict, PackEvaluation, SmartPack, SmartPackReport } from '@/types/smartPacks'

type Meta = { label: string; color: string; soft: string; pose: Pose }

/** Rótulo curto do resultado (cards e Performance Lab). */
export const VERDICT: Record<EvalVerdict, Meta> = {
  improvement: { label: 'Melhoria observada', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)', pose: 'celebrate' },
  regression: { label: 'Regressão observada', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)', pose: 'worried' },
  'no-difference': { label: 'Sem diferença significativa', color: 'var(--frx-text-2)', soft: 'var(--frx-raised)', pose: 'observe' },
  inconclusive: { label: 'Resultado inconclusivo', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)', pose: 'confused' },
  'not-comparable': { label: 'Resultado inconclusivo', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)', pose: 'confused' },
  'no-baseline': { label: 'Aguardando baseline', color: 'var(--frx-text-2)', soft: 'var(--frx-raised)', pose: 'prepare' },
  reference: { label: 'Referência registrada', color: 'var(--frx-info)', soft: 'var(--frx-info-soft)', pose: 'wave' },
}

const ACTIVE: CaptureStatus['state'][] = ['checking_dependencies', 'ready', 'waiting_for_game', 'recording', 'processing']
export const captureActive = (c: CaptureStatus | null) => !!c && ACTIVE.includes(c.state)

export function baselineCount(report: SmartPackReport) {
  return report.measurements.find((m) => m.packId === report.baselineId)?.sessions.filter((s) => !s.excluded).length ?? 0
}

/**
 * Estado de teste de um pacote: aguardando baseline, pronto para teste,
 * capturando, analisando ou o resultado da última sessão.
 */
export function testStatus(pack: SmartPack, report: SmartPackReport, capture: CaptureStatus | null, testing: string | null): { label: string; color: string; soft: string } {
  const grey = { color: 'var(--frx-text-2)', soft: 'var(--frx-raised)' }
  if (testing === pack.id && captureActive(capture)) {
    return capture!.state === 'processing' ? { label: 'Analisando resultados', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)' } : { label: 'Capturando desempenho', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)' }
  }
  const base = baselineCount(report)
  if (pack.kind === 'baseline') return base ? { label: `Referência · ${base} ${base === 1 ? 'sessão' : 'sessões'}`, color: 'var(--frx-info)', soft: 'var(--frx-info-soft)' } : { label: 'Aguardando baseline', ...grey }
  const ev = report.evaluations.find((e) => e.packId === pack.id)
  if (ev && ev.verdict !== 'no-baseline') {
    const v = VERDICT[ev.verdict]
    return { label: `${v.label}${ev.preliminary && ev.verdict !== 'reference' ? ' · preliminar' : ''}`, color: v.color, soft: v.soft }
  }
  if (!base) return { label: 'Aguardando baseline', ...grey }
  return { label: 'Pronto para teste', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)' }
}

const num = (v: number, unit: string) => `${fmtNum(v, 1)} ${unit}`
const signed = (v: number, digits = 1) => `${v > 0 ? '+' : v < 0 ? '−' : '±'}${fmtNum(Math.abs(v), digits)}`

/** Resultado automático de um teste, com valores reais da sessão e da referência. */
export function PackEvaluationCard({ e, onRestore, onRepeat, compact }: { e: PackEvaluation; onRestore?: () => void; onRepeat?: () => void; compact?: boolean }) {
  const v = VERDICT[e.verdict]
  return (
    <section className="rounded-xl border border-line bg-surface p-5" aria-label="Resultado do teste" aria-live="polite">
      <div className="flex items-start gap-4">
        <Mascot pose={v.pose} scale={compact ? 2 : 3} className="shrink-0" />
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <Pill color={v.color} soft={v.soft}>
              {v.label}
            </Pill>
            {e.preliminary && e.verdict !== 'reference' && e.verdict !== 'no-baseline' && e.verdict !== 'not-comparable' && (
              <Pill color="var(--frx-caution)" soft="var(--frx-caution-soft)">
                Preliminar
              </Pill>
            )}
            <span className="text-[12px] text-fg-3">
              {e.packName} · {e.sessionLabel}
              {e.capturedAt && ` · ${fmtDateTime(e.capturedAt)}`}
            </span>
          </div>
          <p className="mt-2 text-[15px] font-medium text-fg">{e.headline}</p>
          {e.summary && <p className="mt-1 text-[12.5px] text-fg-2">{e.summary}</p>}
        </div>
      </div>
      {e.deltas.length > 0 && !compact && (
        <table className="mt-4 w-full text-[12.5px]">
          <thead>
            <tr className="text-left text-fg-3">
              <th className="py-1 font-normal">Métrica</th>
              <th className="py-1 text-right font-normal">Referência</th>
              <th className="py-1 text-right font-normal">Com o pacote</th>
              <th className="py-1 text-right font-normal">Diferença</th>
              <th className="py-1 text-right font-normal">Limiar</th>
            </tr>
          </thead>
          <tbody className="tnum">
            {e.deltas.map((d) => (
              <tr key={d.metric} className="border-t border-line">
                <td className="py-1.5 text-fg-2">{d.label}</td>
                <td className="py-1.5 text-right text-fg-2">{num(d.before, d.unit)}</td>
                <td className="py-1.5 text-right text-fg">{num(d.after, d.unit)}</td>
                <td className="py-1.5 text-right" style={{ color: d.outcome === 'improvement' ? 'var(--frx-good)' : d.outcome === 'regression' ? 'var(--frx-bad)' : 'var(--frx-text-2)' }}>
                  {signed(d.after - d.before)} ({signed(d.deltaPct)}%)
                </td>
                <td className="py-1.5 text-right text-fg-3">±{fmtNum(d.thresholdPct, 1)}%</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      <div className="mt-4 space-y-1 text-[12.5px]">
        {e.nextStep && (
          <p className="flex items-start gap-2 text-fg-2">
            <FlaskConical className="mt-0.5 size-3.5 shrink-0 text-accent" aria-hidden /> {e.nextStep}
          </p>
        )}
        <p className="text-fg-2">
          <span className="text-fg-3">No ranking · </span>
          {e.rankingEffect}
        </p>
        <p className="text-fg-3">
          {e.baselineSessions} sessão(ões) de referência · {e.packSessions} com esta configuração.
        </p>
        {!compact && (
          <ul className="mt-1 space-y-0.5 text-[12px] text-fg-3">
            {e.limitations.map((l) => (
              <li key={l}>{l}</li>
            ))}
          </ul>
        )}
      </div>
      {(onRestore || onRepeat) && (
        <div className="mt-4 flex flex-wrap gap-2">
          {onRestore && e.verdict === 'regression' && (
            <Button size="sm" variant="danger" icon={<RotateCcw className="size-3.5" />} onClick={onRestore}>
              Restaurar configuração anterior
            </Button>
          )}
          {onRepeat && (
            <Button size="sm" variant="outline" icon={<Gauge className="size-3.5" />} onClick={onRepeat}>
              Repetir teste
            </Button>
          )}
        </div>
      )}
    </section>
  )
}

type Step = 'baseline' | 'apply' | 'capture' | 'capturing' | 'result'

const STEP_LABEL: Record<Exclude<Step, 'capturing'>, string> = {
  baseline: 'Referência sem correções',
  apply: 'Aplicar o pacote',
  capture: 'Capturar com o MangoHud',
  result: 'Resultado automático',
}

/**
 * Fluxo guiado "Testar desempenho". Cada ação abre os diálogos existentes
 * (prévia do Fix Engine, troca de pacote, captura do Performance Lab); nada
 * é aplicado ou gravado sem confirmação, e não há monitoramento contínuo.
 */
export function PackTestDialog({
  pack,
  report,
  capture,
  result,
  skipBaseline,
  onSkipBaseline,
  onClose,
  onApply,
  onRestore,
  onCapture,
  onOpenCapture,
  onContinue,
}: {
  pack: SmartPack
  report: SmartPackReport
  capture: CaptureStatus | null
  result: PackEvaluation | null
  skipBaseline: boolean
  onSkipBaseline: () => void
  onClose: () => void
  onApply: () => void
  onRestore: () => void
  onCapture: () => void
  onOpenCapture: () => void
  onContinue: () => void
}) {
  const isBase = pack.kind === 'baseline'
  const base = baselineCount(report)
  const cur = report.currentPackId
  const step: Step = result
    ? 'result'
    : captureActive(capture)
      ? 'capturing'
      : !isBase && base === 0 && !skipBaseline
        ? 'baseline'
        : cur !== pack.id
          ? isBase
            ? 'baseline'
            : 'apply'
          : 'capture'
  const order: Exclude<Step, 'capturing'>[] = isBase ? ['baseline', 'capture', 'result'] : ['baseline', 'apply', 'capture', 'result']
  const at = order.indexOf(step === 'capturing' ? 'capture' : step)
  const pose: Pose = step === 'result' && result ? VERDICT[result.verdict].pose : step === 'capturing' ? (capture?.state === 'processing' ? 'analyze' : 'record') : step === 'apply' ? 'fix' : 'prepare'
  const env = pack.env.map((p) => `${p.key}=${p.value}`).join(' ')

  const body = (() => {
    switch (step) {
      case 'baseline':
        return cur === report.baselineId ? (
          <p>
            {isBase ? 'Capture uma sessão sem nenhuma correção do FRX.' : 'Ainda não há uma sessão de referência. Capture primeiro a configuração normal'} no trecho do jogo que você vai repetir com o pacote. O FRX usa essa sessão como base de comparação.
          </p>
        ) : (
          <p>
            A referência precisa ser medida sem correções do FRX, e agora há correções ativas. Restaure a configuração anterior (com prévia e confirmação) e depois capture a referência.
          </p>
        )
      case 'apply':
        return (
          <div className="space-y-2">
            <p>O pacote altera as opções de inicialização deste jogo:</p>
            <code className="block rounded-sm border border-line bg-bg px-3 py-2 font-mono text-[12px] break-all text-fg">{env}</code>
            <p className="text-fg-3">A prévia mostra a linha completa antes e depois; nada é gravado sem a sua confirmação. Com a Steam aberta, o FRX mostra a linha para você colar.</p>
          </div>
        )
      case 'capture':
        return (
          <div className="space-y-2">
            <p>{isBase ? 'Configuração normal ativa.' : `“${pack.name}” está ativo e conferido no disco.`} Agora capture o desempenho no mesmo trecho e com as mesmas configurações gráficas da referência.</p>
            <p className="text-fg-3">Dica: escolha início manual e comece a gravar quando chegar ao trecho jogável (pelo FRX ou com Shift esquerdo + F2), para não medir menus e carregamentos. A captura só grava pelo tempo que você escolher.</p>
          </div>
        )
      case 'capturing':
        return (
          <div className="space-y-2">
            <p>{capture?.state === 'processing' ? 'Analisando o log do MangoHud…' : (capture?.message ?? 'Captura em andamento.')}</p>
            <p className="text-fg-3">Quando a captura terminar, o FRX associa a sessão ao pacote ativo e compara com a referência automaticamente.</p>
          </div>
        )
      case 'result':
        return result ? <PackEvaluationCard e={result} onRestore={onRestore} /> : null
    }
  })()

  const footer = (() => {
    switch (step) {
      case 'baseline':
        return (
          <>
            {!isBase && (
              <Button variant="ghost" onClick={onSkipBaseline}>
                Testar sem referência
              </Button>
            )}
            {cur === report.baselineId ? (
              <Button variant="primary" icon={<Gauge className="size-4" />} onClick={onCapture}>
                Capturar referência agora
              </Button>
            ) : (
              <Button variant="primary" icon={<RotateCcw className="size-4" />} onClick={onRestore}>
                Restaurar configuração anterior
              </Button>
            )}
          </>
        )
      case 'apply':
        return (
          <>
            <Button variant="ghost" onClick={onClose}>
              Cancelar
            </Button>
            <Button variant="primary" icon={<Wrench className="size-4" />} onClick={onApply}>
              Revisar e aplicar
            </Button>
          </>
        )
      case 'capture':
        return (
          <>
            <Button variant="ghost" onClick={onClose}>
              Depois
            </Button>
            <Button variant="primary" icon={<Gauge className="size-4" />} onClick={onCapture}>
              Preparar captura
            </Button>
          </>
        )
      case 'capturing':
        return (
          <Button variant="secondary" onClick={onOpenCapture}>
            Controles da captura
          </Button>
        )
      case 'result':
        return (
          <>
            <Button variant="ghost" onClick={onClose}>
              Fechar
            </Button>
            <Button variant="primary" onClick={onContinue}>
              {result?.verdict === 'reference' && !isBase ? 'Continuar: aplicar o pacote' : 'Repetir teste'}
            </Button>
          </>
        )
    }
  })()

  return (
    <Dialog
      open
      onClose={onClose}
      width={680}
      title={`Testar desempenho · ${pack.name}`}
      description={isBase ? 'Medir a configuração de referência.' : 'Referência → aplicar → capturar → resultado automático.'}
      footer={footer}
      aside={
        <div className="-mt-1 -ml-2 hidden sm:block">
          <Mascot pose={pose} scale={3} />
        </div>
      }
    >
      <ol className="mb-5 flex flex-wrap gap-x-5 gap-y-1 text-[12.5px]" aria-label="Etapas">
        {order.map((s, i) => (
          <li key={s} className={cn('flex items-center gap-1.5', i === at ? 'text-fg' : i < at ? 'text-good' : 'text-fg-3')}>
            <span className={cn('grid size-4 place-items-center rounded-[4px] border text-[10px]', i < at ? 'border-good/40 bg-good-soft' : i === at ? 'border-accent-line text-accent' : 'border-line-2')} aria-hidden>
              {i < at ? <Check className="size-2.5" strokeWidth={3} /> : i + 1}
            </span>
            {STEP_LABEL[s]}
          </li>
        ))}
      </ol>
      <div className="text-[13.5px] text-fg-2">{body}</div>
    </Dialog>
  )
}
