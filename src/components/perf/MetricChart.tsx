import { CartesianGrid, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis, ReferenceLine } from 'recharts'
import type { MetricSample } from '@/types'
import { useCssVars } from '@/lib/useCssVar'
import { useMotion } from '@/state/motion'
import { fmtNum } from '@/lib/format'

export type ChartMetric = 'fps' | 'frametime'

interface Series {
  key: string
  label: string
  samples: MetricSample[]
  role: 'base' | 'main'
}

const UNIT: Record<ChartMetric, string> = { fps: 'FPS', frametime: 'ms' }

/**
 * Linhas sobrepostas em um único eixo. "Antes" é tracejado e neutro, "Depois"
 * usa o acento — a identidade nunca depende só da cor.
 */
export function MetricChart({ series, metric, height = 260, live }: { series: Series[]; metric: ChartMetric; height?: number; live?: boolean }) {
  const c = useCssVars(['--frx-accent', '--frx-chart-base', '--frx-text-3', '--frx-line-2'])
  const motion = useMotion()
  const len = Math.max(...series.map((s) => s.samples.length), 0)
  const data = Array.from({ length: len }, (_, i) => {
    const row: Record<string, number> = { t: series[0]?.samples[i]?.t ?? i }
    for (const s of series) {
      const v = s.samples[i]?.[metric]
      if (v !== undefined) row[s.key] = v
    }
    return row
  })
  const target = metric === 'frametime' ? 16.7 : 60

  return (
    <div style={{ height }} className="w-full">
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={data} margin={{ top: 8, right: 12, bottom: 0, left: -8 }}>
          <CartesianGrid vertical={false} stroke={c['--frx-line-2']} strokeOpacity={0.5} />
          <XAxis
            dataKey="t"
            tickLine={false}
            axisLine={false}
            tick={{ fill: c['--frx-text-3'], fontSize: 11 }}
            tickFormatter={(t) => `${t}s`}
            interval="preserveStartEnd"
            minTickGap={40}
          />
          <YAxis tickLine={false} axisLine={false} tick={{ fill: c['--frx-text-3'], fontSize: 11 }} width={44} domain={[0, 'auto']} />
          <ReferenceLine
            y={target}
            stroke={c['--frx-text-3']}
            strokeDasharray="2 4"
            label={{ value: metric === 'fps' ? '60 FPS' : '16,7 ms', position: 'insideTopRight', fill: c['--frx-text-3'], fontSize: 10 }}
          />
          <Tooltip
            cursor={{ stroke: c['--frx-text-3'], strokeWidth: 1 }}
            isAnimationActive={false}
            content={({ active, payload, label }) =>
              active && payload?.length ? (
                <div className="rounded-md border border-line-2 bg-raised px-3 py-2 text-[12px] shadow-[0_12px_30px_-12px_rgb(0_0_0/0.8)]">
                  <p className="mb-1 text-fg-3">{label}s</p>
                  {payload.map((p) => {
                    const s = series.find((x) => x.key === p.dataKey)
                    return (
                      <p key={String(p.dataKey)} className="flex items-center gap-2">
                        <span
                          className="inline-block w-3"
                          style={{ borderTop: `2px ${s?.role === 'base' ? 'dashed' : 'solid'} ${s?.role === 'base' ? c['--frx-chart-base'] : c['--frx-accent']}` }}
                        />
                        <span className="text-fg-2">{s?.label}</span>
                        <span className="tnum ml-auto pl-3 font-mono text-fg">
                          {fmtNum(Number(p.value), metric === 'frametime' ? 1 : 0)} {UNIT[metric]}
                        </span>
                      </p>
                    )
                  })}
                </div>
              ) : null
            }
          />
          {series.map((s) => (
            <Line
              key={s.key}
              dataKey={s.key}
              type="monotone"
              stroke={s.role === 'base' ? c['--frx-chart-base'] : c['--frx-accent']}
              strokeWidth={s.role === 'base' ? 1.5 : 2}
              strokeDasharray={s.role === 'base' ? '4 3' : undefined}
              dot={false}
              activeDot={{ r: 4, strokeWidth: 2, stroke: 'var(--frx-surface)' }}
              isAnimationActive={!live && motion !== 'off'}
              animationDuration={motion === 'calm' ? 500 : 900}
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  )
}
