import { useState } from 'react'
import { ArrowDown, ArrowUp, CircleHelp, Equal, Scale } from 'lucide-react'
import type { ComparisonResult, Session, VerdictKind } from '@/types/performance'
import { AnimatedNumber } from '@/components/ui/AnimatedNumber'
import { Segmented } from '@/components/ui/Controls'
import { BarPair } from '@/components/perf/CompareHero'
import { Mascot } from '@/mascot/Mascot'
import type { Pose } from '@/mascot/sprites'
import { useMotion } from '@/state/motion'
import { SessionCharts, SessionHero } from './SessionView'
import { num } from './format'
import { fmtNum } from '@/lib/format'
import { cn } from '@/lib/cn'

export const REAL_VERDICT: Record<VerdictKind, { label: string; color: string; soft: string; pose: Pose; icon: typeof ArrowUp }> = {
  improvement: { label: 'Melhoria observada', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)', pose: 'celebrate', icon: ArrowUp },
  regression: { label: 'Regressão observada', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)', pose: 'worried', icon: ArrowDown },
  'small-difference': { label: 'Diferença pequena', color: 'var(--frx-text-2)', soft: 'var(--frx-raised)', pose: 'idle', icon: Equal },
  inconclusive: { label: 'Inconclusivo', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)', pose: 'confused', icon: CircleHelp },
}

/** Herói de comparação com números reais — mesmo desenho do Performance Lab. */
function RealCompareHero({ r, before, after }: { r: ComparisonResult; before: Session; after: Session }) {
  const v = REAL_VERDICT[r.verdict.kind]
  const motion = useMotion()
  const d = r.verdict.avgDeltaPct
  const b = before.meta.stats
  const a = after.meta.stats
  const maxFps = Math.max(b.fps.avg ?? 0, a.fps.avg ?? 0) * 1.08 || 1
  const sign = d == null ? '' : d > 0 ? '+' : d < 0 ? '−' : '±'
  return (
    <section aria-labelledby="comp-real" className="relative overflow-hidden rounded-xl border border-line bg-surface">
      <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1.25fr)]">
        <div className="flex flex-col justify-between border-r border-line p-7">
          <div>
            <span className="inline-flex h-6 items-center gap-1.5 rounded-xs px-2 text-[12px] font-semibold" style={{ background: v.soft, color: v.color }}>
              <v.icon className="size-3.5" strokeWidth={2.4} aria-hidden />
              {v.label}
            </span>
            <h2 id="comp-real" className="sr-only">
              Comparação entre {before.meta.label} e {after.meta.label}
            </h2>
            <div className="mt-5 flex items-end gap-4">
              <p
                className={cn('tnum font-display text-[clamp(56px,6.2vw,84px)] leading-[0.85] font-bold tracking-[-0.06em]', r.verdict.kind === 'inconclusive' || r.verdict.kind === 'small-difference' ? 'text-fg-2' : '')}
                style={{ color: r.verdict.kind === 'regression' ? 'var(--frx-bad)' : undefined }}
              >
                {d == null ? '—' : <AnimatedNumber value={Math.abs(d)} digits={Math.abs(d) < 10 ? 1 : 0} prefix={sign} suffix="%" duration={1.3} />}
              </p>
              <Mascot pose={v.pose} scale={4} className="-mb-2" />
            </div>
            <p className="mt-3 text-[13.5px] text-fg-2">
              FPS médio · {num(b.fps.avg, 1)} → {num(a.fps.avg, 1)}
              {r.verdict.noisePct != null && <span className="text-fg-3"> · variação natural ±{fmtNum(Math.max(r.verdict.noisePct, 3), 1)}%</span>}
            </p>
            <ul className="mt-5 space-y-1.5 text-[12.5px] text-fg-2">
              {r.verdict.reasons.map((x) => (
                <li key={x} className="flex gap-2">
                  <span className="mt-[7px] size-1 shrink-0 bg-fg-3" aria-hidden />
                  {x}
                </li>
              ))}
            </ul>
          </div>
          <p className="mt-6 max-w-sm text-[12px] leading-relaxed text-fg-3">
            Resultado medido nas duas sessões registradas. Ele não prova, sozinho, que uma mudança específica causou a diferença.
          </p>
        </div>
        <div className="p-7">
          <div className="space-y-6">
            {b.fps.avg != null && a.fps.avg != null && (
              <BarPair label="FPS médio" unit="FPS" before={b.fps.avg} after={a.fps.avg} max={maxFps} better="higher" digits={1} verdictColor={v.color} animate={motion !== 'off'} />
            )}
            {b.fps.low1 != null && a.fps.low1 != null && (
              <BarPair label={b.fps.lowsEstimated || a.fps.lowsEstimated ? '1% low (estimativa)' : '1% low'} unit="FPS" before={b.fps.low1} after={a.fps.low1} max={maxFps} better="higher" digits={1} verdictColor={v.color} animate={motion !== 'off'} delay={0.12} />
            )}
            {b.frametime.avg != null && a.frametime.avg != null && (
              <BarPair
                label="Frametime médio"
                unit="ms"
                digits={2}
                before={b.frametime.avg}
                after={a.frametime.avg}
                max={Math.max(b.frametime.avg, a.frametime.avg) * 1.08}
                better="lower"
                verdictColor={v.color}
                animate={motion !== 'off'}
                delay={0.24}
              />
            )}
          </div>
          <div className="mt-6 flex items-center gap-5 border-t border-line pt-4 text-[12px] text-fg-3">
            <span className="flex min-w-0 items-center gap-2">
              <span className="h-2 w-4 shrink-0 rounded-[2px] bg-[repeating-linear-gradient(135deg,var(--frx-chart-base)_0_2px,transparent_2px_4px)]" aria-hidden />
              <span className="truncate">Antes · {before.meta.label}</span>
            </span>
            <span className="flex min-w-0 items-center gap-2">
              <span className="h-2 w-4 shrink-0 rounded-[2px] bg-accent" aria-hidden />
              <span className="truncate">Depois · {after.meta.label}</span>
            </span>
          </div>
        </div>
      </div>
    </section>
  )
}

function CompareTable({ r }: { r: ComparisonResult }) {
  return (
    <div className="overflow-hidden rounded-xl border border-line">
      <table className="w-full text-[13px]">
        <thead>
          <tr className="border-b border-line bg-surface text-left text-[11.5px] text-fg-3">
            <th className="px-5 py-2.5 font-medium">Métrica</th>
            <th className="px-3 py-2.5 text-right font-medium">Antes</th>
            <th className="px-3 py-2.5 text-right font-medium">Depois</th>
            <th className="px-3 py-2.5 text-right font-medium">Diferença</th>
            <th className="px-5 py-2.5 text-right font-medium">Variação</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-line">
          {r.rows.map((row) => {
            const digits = row.unit === 'ms' || row.unit === 'GiB' ? 2 : row.unit === '/min' ? 1 : row.unit === 'FPS' ? 1 : 0
            const tone = row.direction === 'improved' ? 'text-good' : row.direction === 'worsened' ? 'text-bad' : 'text-fg-3'
            return (
              <tr key={row.key}>
                <td className="px-5 py-2.5 text-fg">
                  {row.label}
                  {row.estimated && <span className="ml-2 text-[11px] text-caution">estimativa</span>}
                  {row.better === 'neutral' && <span className="ml-2 text-[11px] text-fg-3">sem direção “melhor”</span>}
                </td>
                <td className="tnum px-3 py-2.5 text-right font-mono whitespace-nowrap text-fg-2">
                  {num(row.before, digits)} <span className="text-fg-3">{row.unit}</span>
                </td>
                <td className="tnum px-3 py-2.5 text-right font-mono whitespace-nowrap text-fg">
                  {num(row.after, digits)} <span className="text-fg-3">{row.unit}</span>
                </td>
                <td className={cn('tnum px-3 py-2.5 text-right font-mono whitespace-nowrap', tone)}>
                  {row.absDiff > 0 ? '+' : row.absDiff < 0 ? '−' : '±'}
                  {num(Math.abs(row.absDiff), digits)}
                </td>
                <td className={cn('tnum px-5 py-2.5 text-right font-mono whitespace-nowrap', tone)}>{row.pctDiff == null ? '—' : `${row.pctDiff > 0 ? '+' : row.pctDiff < 0 ? '−' : '±'}${fmtNum(Math.abs(row.pctDiff), 1)}%`}</td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}

function Comparability({ r }: { r: ComparisonResult }) {
  const c = r.comparability
  return (
    <section aria-labelledby="condicoes" className="rounded-xl border border-line bg-surface p-5">
      <div className="flex items-center justify-between">
        <h3 id="condicoes" className="flex items-center gap-2 text-[14px] font-medium">
          <Scale className="size-4 text-fg-3" aria-hidden /> Condições dos testes
        </h3>
        <span className={cn('text-[12px] font-medium', c.controlled ? 'text-good' : 'text-caution')}>{c.controlled ? 'Comparação controlada' : 'Comparação não controlada'}</span>
      </div>
      {c.differences.length > 0 ? (
        <ul className="mt-3 divide-y divide-line">
          {c.differences.map((d) => (
            <li key={d.field} className="grid grid-cols-[140px_1fr_1fr] gap-3 py-2 text-[12.5px]">
              <span className={d.major ? 'text-caution' : 'text-fg-3'}>{d.label}</span>
              <span className="truncate text-fg-2" title={d.before ?? ''}>
                {d.before ?? '—'}
              </span>
              <span className="truncate text-fg" title={d.after ?? ''}>
                {d.after ?? '—'}
              </span>
            </li>
          ))}
        </ul>
      ) : (
        <p className="mt-2 text-[12.5px] text-fg-2">Nenhuma diferença registrada entre as sessões.</p>
      )}
      <ul className="mt-3 space-y-1 text-[12px] text-fg-3">
        {c.notes.map((n) => (
          <li key={n}>{n}</li>
        ))}
      </ul>
    </section>
  )
}

type View = 'overlay' | 'before' | 'after'

export function ComparisonView({ result, before, after }: { result: ComparisonResult; before: Session; after: Session }) {
  const [view, setView] = useState<View>('overlay')
  return (
    <div className="space-y-10">
      <RealCompareHero r={result} before={before} after={after} />
      <div className="grid grid-cols-[minmax(0,1.6fr)_minmax(0,1fr)] gap-8">
        <CompareTable r={result} />
        <Comparability r={result} />
      </div>
      <div>
        <div className="mb-4 flex justify-end">
          <Segmented<View>
            label="Como ver as sessões"
            size="sm"
            value={view}
            onChange={setView}
            options={[
              { value: 'overlay', label: 'Sobrepostas' },
              { value: 'before', label: 'Só “antes”' },
              { value: 'after', label: 'Só “depois”' },
            ]}
          />
        </div>
        {view === 'overlay' && <SessionCharts session={after} compareWith={before} />}
        {view === 'before' && (
          <div className="space-y-8">
            <SessionHero meta={before.meta} />
            <SessionCharts session={before} />
          </div>
        )}
        {view === 'after' && (
          <div className="space-y-8">
            <SessionHero meta={after.meta} />
            <SessionCharts session={after} />
          </div>
        )}
      </div>
    </div>
  )
}
