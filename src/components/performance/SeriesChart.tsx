import { useMemo, useState } from 'react'
import { CartesianGrid, Line, LineChart, ReferenceArea, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import { RotateCcw } from 'lucide-react'
import { useCssVars } from '@/lib/useCssVar'
import { useMotion } from '@/state/motion'
import { fmtNum } from '@/lib/format'

export interface ChartLine {
  key: string
  label: string
  points: [number, number][]
  /** Linha de referência ("antes"): tracejada e neutra. A identidade nunca depende só da cor. */
  role: 'main' | 'base' | 'aux'
  /** Como agregar pontos que caem na mesma coluna do gráfico (o pior frametime usa `max`). */
  agg?: 'mean' | 'max'
}

/**
 * Coloca todas as linhas numa grade de tempo comum (necessário para o tooltip
 * mostrar "antes" e "depois" no mesmo instante). Colunas sem dados ficam
 * vazias: o gráfico mostra a lacuna em vez de inventar valores.
 */
function toGrid(lines: ChartLine[], minT: number, maxT: number) {
  const n = Math.max(1, Math.min(1200, Math.max(...lines.map((l) => l.points.length), 1)))
  const width = (maxT - minT) / n || 1
  const rows: Record<string, number | null>[] = Array.from({ length: n + 1 }, (_, i) => ({ t: minT + i * width }))
  for (const l of lines) {
    const acc = new Map<number, { sum: number; n: number; max: number }>()
    for (const [t, v] of l.points) {
      const i = Math.min(n, Math.max(0, Math.round((t - minT) / width)))
      const a = acc.get(i) ?? { sum: 0, n: 0, max: -Infinity }
      a.sum += v
      a.n += 1
      a.max = Math.max(a.max, v)
      acc.set(i, a)
    }
    for (const [i, a] of acc) rows[i][l.key] = l.agg === 'max' ? a.max : a.sum / a.n
  }
  // Mantém só linhas com algum valor; lacunas reais (> 3 colunas) viram quebra explícita.
  const filled = rows.filter((r) => lines.some((l) => r[l.key] != null))
  const out: Record<string, number | null>[] = []
  for (let k = 0; k < filled.length; k++) {
    if (k > 0 && (filled[k].t as number) - (filled[k - 1].t as number) > width * 3.5) {
      out.push({ t: ((filled[k].t as number) + (filled[k - 1].t as number)) / 2, ...Object.fromEntries(lines.map((l) => [l.key, null])) })
    }
    out.push(filled[k])
  }
  return out
}

interface Props {
  lines: ChartLine[]
  unit: string
  digits?: number
  height?: number
  /** Trechos instáveis destacados ao fundo. */
  highlights?: { start: number; end: number }[]
  /** Começa em zero (FPS, uso) ou no mínimo dos dados (temperaturas). */
  zeroBased?: boolean
}

/**
 * Série temporal real com zoom por arraste. Os pontos já chegam reduzidos pelo
 * Rust (picos preservados); nada é interpolado ou inventado nas lacunas.
 */
export function SeriesChart({ lines, unit, digits = 0, height = 260, highlights = [], zeroBased = true }: Props) {
  const c = useCssVars(['--frx-accent', '--frx-chart-base', '--frx-text-3', '--frx-line-2', '--frx-info', '--frx-bad'])
  const motion = useMotion()
  const [drag, setDrag] = useState<{ a: number; b: number } | null>(null)
  const [zoom, setZoom] = useState<[number, number] | null>(null)

  const [minT, maxT] = useMemo(() => {
    const ts = lines.flatMap((l) => l.points.map((p) => p[0]))
    return ts.length ? [Math.min(...ts), Math.max(...ts)] : [0, 1]
  }, [lines])
  const domain: [number, number] = zoom ?? [minT, maxT]
  const grid = useMemo(() => toGrid(lines, minT, maxT), [lines, minT, maxT])
  const rows = useMemo(() => grid.filter((r) => (r.t as number) >= domain[0] && (r.t as number) <= domain[1]), [grid, domain])
  const color = (role: ChartLine['role']) => (role === 'base' ? c['--frx-chart-base'] : role === 'aux' ? c['--frx-info'] : c['--frx-accent'])

  return (
    <div className="relative w-full select-none" style={{ height }}>
      <span className="pointer-events-none absolute -top-3 left-1 text-[10.5px] text-fg-3" aria-hidden>
        {unit}
      </span>
      {zoom && (
        <button
          onClick={() => setZoom(null)}
          className="absolute top-0 right-2 z-10 inline-flex h-7 items-center gap-1.5 rounded-sm border border-line-2 bg-raised px-2.5 text-[12px] text-fg-2 hover:text-fg"
        >
          <RotateCcw className="size-3" /> Redefinir zoom
        </button>
      )}
      <ResponsiveContainer width="100%" height="100%">
        <LineChart
          data={rows}
          margin={{ top: 8, right: 12, bottom: 0, left: -4 }}
          onMouseDown={(e) => e?.activeLabel != null && setDrag({ a: Number(e.activeLabel), b: Number(e.activeLabel) })}
          onMouseMove={(e) => drag && e?.activeLabel != null && setDrag({ ...drag, b: Number(e.activeLabel) })}
          onMouseUp={() => {
            if (drag && Math.abs(drag.b - drag.a) > (maxT - minT) / 200) setZoom([Math.min(drag.a, drag.b), Math.max(drag.a, drag.b)])
            setDrag(null)
          }}
        >
          <CartesianGrid vertical={false} stroke={c['--frx-line-2']} strokeOpacity={0.5} />
          <XAxis
            dataKey="t"
            type="number"
            domain={domain}
            allowDataOverflow
            tickLine={false}
            axisLine={false}
            tick={{ fill: c['--frx-text-3'], fontSize: 11 }}
            tickFormatter={(t) => `${fmtNum(t, t < 10 && domain[1] - domain[0] < 20 ? 1 : 0)} s`}
            minTickGap={40}
          />
          <YAxis
            tickLine={false}
            axisLine={false}
            tick={{ fill: c['--frx-text-3'], fontSize: 11 }}
            width={52}
            domain={zeroBased ? [0, 'auto'] : ['auto', 'auto']}
            tickFormatter={(v) => `${fmtNum(v, digits)}`}
          />
          {highlights.map((h, i) => (
            <ReferenceArea key={i} x1={h.start} x2={h.end} fill={c['--frx-bad']} fillOpacity={0.08} stroke="none" ifOverflow="hidden" />
          ))}
          {drag && <ReferenceArea x1={drag.a} x2={drag.b} fill={c['--frx-accent']} fillOpacity={0.1} stroke="none" />}
          <Tooltip
            cursor={{ stroke: c['--frx-text-3'], strokeWidth: 1 }}
            isAnimationActive={false}
            content={({ active, payload, label }) =>
              active && payload?.length ? (
                <div className="rounded-md border border-line-2 bg-raised px-3 py-2 text-[12px] shadow-[0_12px_30px_-12px_rgb(0_0_0/0.8)]">
                  <p className="mb-1 text-fg-3">{fmtNum(Number(label), 1)} s</p>
                  {payload.filter((p) => p.value != null).map((p, i) => {
                    const line = lines.find((l) => l.key === p.dataKey)
                    return (
                      <p key={i} className="flex items-center gap-2">
                        <span className="inline-block w-3" style={{ borderTop: `2px ${line?.role === 'base' ? 'dashed' : 'solid'} ${color(line?.role ?? 'main')}` }} />
                        <span className="text-fg-2">{line?.label}</span>
                        <span className="tnum ml-auto pl-3 font-mono text-fg">
                          {fmtNum(Number(p.value), digits)} {unit}
                        </span>
                      </p>
                    )
                  })}
                </div>
              ) : null
            }
          />
          {lines.map((l) => (
            <Line
              key={l.key}
              name={l.label}
              dataKey={l.key}
              type="linear"
              stroke={color(l.role)}
              strokeWidth={l.role === 'main' ? 2 : 1.5}
              strokeDasharray={l.role === 'base' ? '4 3' : undefined}
              dot={false}
              activeDot={{ r: 4, strokeWidth: 2, stroke: 'var(--frx-surface)' }}
              isAnimationActive={motion !== 'off' && !zoom}
              animationDuration={motion === 'calm' ? 400 : 700}
              connectNulls={false}
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  )
}

export function ChartLegend({ lines, highlights }: { lines: ChartLine[]; highlights?: boolean }) {
  return (
    <div className="mt-2 flex flex-wrap gap-x-5 gap-y-1 px-2 text-[12px] text-fg-3">
      {lines.map((l) => (
        <span key={l.key} className="flex items-center gap-2">
          <span
            className={
              l.role === 'base' ? 'w-4 border-t-[1.5px] border-dashed border-(--frx-chart-base)' : l.role === 'aux' ? 'w-4 border-t-[1.5px] border-info' : 'w-4 border-t-2 border-accent'
            }
            aria-hidden
          />
          {l.label}
        </span>
      ))}
      {highlights && (
        <span className="flex items-center gap-2">
          <span className="h-2.5 w-4 rounded-[2px] bg-bad/15" aria-hidden /> Trecho instável
        </span>
      )}
      <span className="ml-auto">Arraste no gráfico para ampliar um trecho</span>
    </div>
  )
}
