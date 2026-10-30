import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { FlaskConical } from 'lucide-react'
import { Button } from '@/components/ui/Button'
import { Pill } from '@/components/ui/Status'
import { Mascot } from '@/mascot/Mascot'
import type { Pose } from '@/mascot/sprites'
import { evaluateBenchmarkSession } from '@/services/smartPacks'
import { errorMessage } from '@/services/native'
import { useFixEngine } from '@/state/fixEngine'
import { usePerformance } from '@/state/performance'
import { fmtDateTime, fmtNum } from '@/lib/format'
import type { SessionMeta } from '@/types/performance'
import type { EvalVerdict, PackEvaluation } from '@/types/smartPacks'


type Meta = { label: string; color: string; soft: string; pose: Pose }

/** Rótulo curto do resultado (cards e Performance Lab). */
export const VERDICT: Record<EvalVerdict, Meta> = {
  improvement: { label: 'Melhoria observada', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)', pose: 'celebrate' },
  regression: { label: 'Regressão observada', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)', pose: 'worried' },
  'no-difference': { label: 'Sem diferença relevante', color: 'var(--frx-text-2)', soft: 'var(--frx-raised)', pose: 'observe' },
  inconclusive: { label: 'Resultado inconclusivo', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)', pose: 'confused' },
  'not-comparable': { label: 'Não comparável', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)', pose: 'confused' },
  'no-baseline': { label: 'Aguardando baseline', color: 'var(--frx-text-2)', soft: 'var(--frx-raised)', pose: 'prepare' },
  reference: { label: 'Referência registrada', color: 'var(--frx-info)', soft: 'var(--frx-info-soft)', pose: 'wave' },
}

const num = (v: number, unit: string) => `${fmtNum(v, 1)} ${unit}`
const signed = (v: number, digits = 1) => `${v > 0 ? '+' : v < 0 ? '−' : '±'}${fmtNum(Math.abs(v), digits)}`

/**
 * Performance Lab: conclusão automática da sessão em relação à referência
 * (baseline) do jogo. Desfazer o pacote fica na página do jogo.
 */
export function SessionPackResult({ meta }: { meta: SessionMeta }) {
  const engine = useFixEngine()
  const perf = usePerformance()
  const navigate = useNavigate()
  const [e, setE] = useState<PackEvaluation | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [retry, setRetry] = useState(0)
  const stamp = engine.operations.map((o) => o.id + o.state).join() + JSON.stringify(perf.sessions.map((s) => [s.id, s.conditions, s.smartPack]))

  useEffect(() => {
    if (!engine.available || !meta.appId) return setE(null)
    let gone = false
    setE(null)
    setError(null)
    evaluateBenchmarkSession(meta.id).then(
      (r) => !gone && setE(r),
      (err) => !gone && setError(errorMessage(err)),
    )
    return () => void (gone = true)
  }, [engine.available, meta.id, meta.appId, stamp, retry])

  if (error) return <section role="alert" className="text-[13px] text-fg-2">
    <p>Não foi possível avaliar o pacote: {error}</p>
    <Button size="sm" variant="outline" onClick={() => setRetry((n) => n + 1)}>Tentar avaliação de novo</Button>
  </section>
  if (!e || e.sessionId !== meta.id || e.appId !== meta.appId) return null
  return (
    <section aria-labelledby="smart-pack-result" className="space-y-3">
      <div className="flex items-end justify-between">
        <h2 id="smart-pack-result" className="font-display text-[20px] font-semibold tracking-[-0.02em]">
          Pacote · resultado
        </h2>
        <Button size="sm" variant="ghost" onClick={() => navigate(`/biblioteca/${encodeURIComponent(e.appId)}`)}>
          Abrir o pacote do jogo
        </Button>
      </div>
      <PackEvaluationCard e={e} />
    </section>
  )
}

function PackEvaluationCard({ e }: { e: PackEvaluation }) {
  const compact = false
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
        <div className="mt-4 overflow-x-auto"><table className="w-full text-[12.5px]">
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
                  {signed(d.after - d.before)} ({d.percentageAvailable === false ? 'percentual indisponível' : `${signed(d.deltaPct)}%`})
                </td>
                <td className="py-1.5 text-right text-fg-3">{d.percentageAvailable === false ? '—' : `±${fmtNum(d.thresholdPct, 1)}%`}</td>
              </tr>
            ))}
          </tbody>
        </table></div>
      )}
      {e.conditions && <p className="mt-3 text-[12px] text-fg-3">
        Condições registradas: {[e.conditions.gpu, e.conditions.cpu, e.conditions.proton, e.conditions.driver, e.conditions.resolution, e.conditions.preset, e.conditions.fpsLimit && `Limite: ${e.conditions.fpsLimit}`, e.conditions.vsync != null && `VSync ${e.conditions.vsync ? 'ligado' : 'desligado'}`].filter(Boolean).join(' · ') || 'não informadas'}. Evidência local a esta configuração.
      </p>}
      <div className="mt-4 space-y-1 text-[12.5px]">
        {e.nextStep && (
          <p className="flex items-start gap-2 text-fg-2">
            <FlaskConical className="mt-0.5 size-3.5 shrink-0 text-accent" aria-hidden /> {e.nextStep}
          </p>
        )}
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
    </section>
  )
}
