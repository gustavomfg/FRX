import { useMemo, useState } from 'react'
import { AlertTriangle, CircleAlert, Gauge, Info, Thermometer, Zap } from 'lucide-react'
import type { Metric, Session, SessionMeta } from '@/types/performance'
import { AnimatedNumber } from '@/components/ui/AnimatedNumber'
import { Meter, Segmented } from '@/components/ui/Controls'
import { SourceTag } from '@/components/game/LibraryNotices'
import { Pill } from '@/components/ui/Status'
import { Mascot } from '@/mascot/Mascot'
import type { Pose } from '@/mascot/sprites'
import { ChartLegend, SeriesChart, type ChartLine } from './SeriesChart'
import { DIGITS, METRIC_LABEL, QUALITY, duration, num } from './format'
import { useSettings } from '@/state/store'
import { fmtDateTime, fmtNum } from '@/lib/format'
import { cn } from '@/lib/cn'

/** Resumo de uma sessão real, no mesmo desenho do herói de comparação. */
export function SessionHero({ meta, pose = 'observe' }: { meta: SessionMeta; pose?: Pose }) {
  const s = meta.stats
  const q = QUALITY[meta.quality.level]
  const est = s.fps.lowsEstimated
  const kpis: { label: string; value: string; unit: string; note?: string }[] = [
    { label: '1% low', value: num(s.fps.low1, 1), unit: 'FPS', note: est ? 'estimativa' : s.fps.low1 == null ? 'poucos quadros' : undefined },
    { label: '0,1% low', value: num(s.fps.low01, 1), unit: 'FPS', note: est ? 'estimativa' : s.fps.low01 == null ? 'precisa de 1000+ quadros' : undefined },
    { label: 'Frametime médio', value: num(s.frametime.avg, 2), unit: 'ms' },
    { label: 'Frametime P99', value: num(s.frametime.p99, 2), unit: 'ms', note: est ? 'amostrado' : undefined },
    { label: 'Picos por minuto', value: num(s.stability.spikesPerMin, 1), unit: '/min', note: s.mode === 'periodic' ? 'baixa resolução' : undefined },
    { label: 'Duração', value: duration(s.durationS), unit: '', note: s.frames ? `${fmtNum(s.frames)} quadros` : `${fmtNum(s.validRows)} amostras` },
  ]
  return (
    <section aria-label={`Sessão ${meta.label}`} className="relative overflow-hidden rounded-xl border border-line bg-surface">
      <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1.25fr)]">
        <div className="flex flex-col justify-between border-r border-line p-7">
          <div>
            <div className="flex flex-wrap items-center gap-2">
              <Pill color={q.color} soft={q.soft} title={q.help}>
                <span className="size-1.5" style={{ background: q.color }} aria-hidden />
                {q.label}
              </Pill>
              <SourceTag>{meta.source === 'synthetic' ? 'Sintético · teste' : meta.source === 'assisted' ? 'Captura FRX' : 'Importado'}</SourceTag>
            </div>
            <div className="mt-5 flex items-end gap-4">
              <p className="tnum font-display text-[clamp(56px,6.2vw,84px)] leading-[0.85] font-bold tracking-[-0.06em]">
                {s.fps.avg == null ? '—' : <AnimatedNumber value={s.fps.avg} digits={1} duration={1.1} />}
              </p>
              <Mascot pose={pose} scale={4} className="-mb-2" />
            </div>
            <p className="mt-3 text-[13.5px] text-fg-2">
              FPS médio{meta.gameName && <> em <span className="text-fg">{meta.gameName}</span></>}
              {s.mode === 'periodic' && <span className="text-caution"> · amostras periódicas</span>}
            </p>
          </div>
          <dl className="mt-6 space-y-1.5 text-[12.5px]">
            {[
              ['Sessão', meta.label],
              ['Capturada', meta.capturedAt ? fmtDateTime(meta.capturedAt) : fmtDateTime(meta.createdAt)],
              ['Proton', meta.conditions.proton ?? '—'],
              ['GPU · driver', [meta.conditions.gpu, meta.conditions.driver].filter(Boolean).join(' · ') || '—'],
              ['Correções do FRX', meta.activeFixes == null ? 'Não registrado' : meta.activeFixes.length ? meta.activeFixes.map((f) => `${f.title} v${f.recipeVersion}`).join(', ') : 'Nenhuma'],
              [
                'Smart Pack',
                meta.smartPack == null
                  ? 'Não registrado'
                  : `${meta.smartPack.env.length ? meta.smartPack.env.map((e) => `${e.key}=${e.value}`).join(' ') : 'Baseline (nenhuma variável do FRX)'}${meta.smartPack.source === 'declared' ? ' · declarado por você' : meta.smartPack.verified ? ' · conferido' : ' · não conferido'}`,
              ],
            ].map(([k, v]) => (
              <div key={k} className="flex justify-between gap-4 border-b border-line pb-1.5">
                <dt className="shrink-0 text-fg-3">{k}</dt>
                <dd className="truncate text-right text-fg-2" title={v}>
                  {v}
                </dd>
              </div>
            ))}
          </dl>
        </div>
        <div className="grid grid-cols-2 content-start gap-x-6 gap-y-6 p-7">
          {kpis.map((k) => (
            <div key={k.label}>
              <p className="text-[12px] text-fg-3">{k.label}</p>
              <p className="tnum mt-1 font-display text-[28px] leading-none font-semibold tracking-[-0.03em]">
                {k.value}
                {k.unit && <span className="ml-1 font-sans text-[12px] font-normal tracking-normal text-fg-3">{k.unit}</span>}
              </p>
              {k.note && <p className={cn('mt-1 text-[11.5px]', k.note === 'estimativa' || k.note === 'amostrado' || k.note === 'baixa resolução' ? 'text-caution' : 'text-fg-3')}>{k.note}</p>}
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}

type Tab = 'fps' | 'frametime' | 'load' | 'memory' | 'temps' | 'power'

const TABS: { id: Tab; label: string; metrics: Metric[]; unit: string; zero: boolean }[] = [
  { id: 'fps', label: 'FPS', metrics: ['fps'], unit: 'FPS', zero: true },
  { id: 'frametime', label: 'Frametime', metrics: ['frametime'], unit: 'ms', zero: true },
  { id: 'load', label: 'CPU e GPU', metrics: ['cpuLoad', 'gpuLoad'], unit: '%', zero: true },
  { id: 'memory', label: 'Memória', metrics: ['ram', 'vram'], unit: 'GiB', zero: true },
  { id: 'temps', label: 'Temperaturas', metrics: ['cpuTemp', 'gpuTemp'], unit: '°C', zero: false },
  { id: 'power', label: 'Consumo', metrics: ['cpuPower', 'gpuPower'], unit: 'W', zero: true },
]

/** Linhas de uma sessão para uma aba (com tempo relativo ao início). */
export function linesFor(session: Session, tab: Tab, role: 'main' | 'base', labelPrefix = ''): ChartLine[] {
  const t0 = session.series.find((s) => s.key === 'fps')?.points[0]?.[0] ?? 0
  const pick = (key: string) => session.series.find((s) => s.key === key)
  const rel = (pts: [number, number][]) => pts.map(([t, v]) => [t - t0, v] as [number, number])
  const def = TABS.find((t) => t.id === tab)!
  if (tab === 'frametime') {
    const out: ChartLine[] = []
    const avg = pick('frametime')
    const worst = pick('frametimeMax')
    if (avg) out.push({ key: `${role}-ft`, label: `${labelPrefix}Frametime médio`, points: rel(avg.points), role })
    if (worst && role === 'main') out.push({ key: `${role}-ftmax`, label: `${labelPrefix}Pior quadro no intervalo`, points: rel(worst.points), role: 'aux', agg: 'max' })
    return out
  }
  return def.metrics
    .map((m, i) => {
      const s = pick(m)
      if (!s) return null
      const r: ChartLine['role'] = role === 'base' ? 'base' : i === 0 ? 'main' : 'aux'
      return { key: `${role}-${m}`, label: `${labelPrefix}${METRIC_LABEL[m]}`, points: rel(s.points), role: r }
    })
    .filter(Boolean) as ChartLine[]
}

export function availableTabs(sessions: Session[]) {
  return TABS.filter((t) => sessions.every((s) => t.metrics.some((m) => s.series.some((x) => x.key === m))))
}

export function SessionCharts({ session, compareWith }: { session: Session; compareWith?: Session }) {
  const sessions = compareWith ? [compareWith, session] : [session]
  const tabs = availableTabs(sessions)
  const [tab, setTab] = useState<Tab>('fps')
  const current = tabs.find((t) => t.id === tab) ?? tabs[0]
  const lines = useMemo(() => {
    if (!current) return []
    if (compareWith) return [...linesFor(compareWith, current.id, 'base', 'Antes · '), ...linesFor(session, current.id, 'main', 'Depois · ').filter((l) => l.role !== 'aux' || current.id !== 'frametime')]
    return linesFor(session, current.id, 'main')
  }, [session, compareWith, current])
  const t0 = session.series.find((s) => s.key === 'fps')?.points[0]?.[0] ?? 0
  const highlights = compareWith ? [] : session.meta.stats.stability.unstableIntervals.map((i) => ({ start: i.startS - t0, end: i.endS - t0 }))
  if (!current) return null
  return (
    <section aria-labelledby="graficos-reais">
      <div className="mb-4 flex items-end justify-between gap-4">
        <div>
          <h2 id="graficos-reais" className="font-display text-[20px] font-semibold tracking-[-0.02em]">
            {current.id === 'fps' ? 'Quadros por segundo' : current.label}
          </h2>
          <p className="mt-1 text-[12.5px] text-fg-3">
            {current.id === 'fps'
              ? 'FPS real por intervalo (quadros ÷ tempo). Maior é melhor.'
              : current.id === 'frametime'
                ? 'Menor e mais estável é melhor. A linha azul mostra o pior quadro de cada intervalo — os picos são preservados.'
                : 'Média por intervalo, a partir dos sensores registrados pelo MangoHud.'}
          </p>
        </div>
        <Segmented<Tab> label="Métrica do gráfico" size="sm" value={current.id} onChange={setTab} options={tabs.map((t) => ({ value: t.id, label: t.label }))} />
      </div>
      <div className="relative rounded-xl border border-line bg-surface px-4 pt-5 pb-3">
        <div className="pointer-events-none absolute -top-[45px] left-[44%]">
          <Mascot pose="observe" scale={3} rows={15} />
        </div>
        <SeriesChart lines={lines} unit={current.unit} digits={DIGITS[current.metrics[0]] ?? 0} zeroBased={current.zero} highlights={highlights} />
        <ChartLegend lines={lines} highlights={highlights.length > 0} />
      </div>
    </section>
  )
}

const RES: { m: Metric; max?: (meta: SessionMeta) => number | undefined }[] = [{ m: 'cpuLoad' }, { m: 'gpuLoad' }, { m: 'ram' }, { m: 'vram' }]

export function ResourcesGrid({ meta }: { meta: SessionMeta }) {
  const { tempUnit } = useSettings()
  const r = meta.stats.resources
  const temp = (v: number) => (tempUnit === 'F' ? v * 1.8 + 32 : v)
  const extras: { label: string; icon: typeof Zap; m: Metric; fmt: (v: number) => string }[] = [
    { label: 'Temperatura GPU', icon: Thermometer, m: 'gpuTemp', fmt: (v) => `${num(temp(v))} °${tempUnit}` },
    { label: 'Temperatura CPU', icon: Thermometer, m: 'cpuTemp', fmt: (v) => `${num(temp(v))} °${tempUnit}` },
    { label: 'Consumo GPU', icon: Zap, m: 'gpuPower', fmt: (v) => `${num(v)} W` },
    { label: 'Consumo CPU', icon: Zap, m: 'cpuPower', fmt: (v) => `${num(v, 1)} W` },
    { label: 'Clock GPU', icon: Gauge, m: 'gpuCoreClock', fmt: (v) => `${num(v)} MHz` },
  ]
  return (
    <div className="grid grid-cols-[repeat(4,minmax(0,1fr))_minmax(0,1.15fr)] divide-x divide-line rounded-xl border border-line bg-surface">
      {RES.map(({ m }) => {
        const v = r[m]
        const pctScale = m === 'cpuLoad' || m === 'gpuLoad' ? 100 : v ? Math.max(v.max * 1.25, 1) : 1
        return (
          <div key={m} className="p-5">
            <div className="flex items-center justify-between text-[12px] text-fg-3">
              <span>{METRIC_LABEL[m]}</span>
              {v && <span className="tnum font-mono">pico {num(v.max, DIGITS[m] ?? 0)}</span>}
            </div>
            <p className="tnum mt-2 font-display text-[26px] leading-none font-semibold tracking-[-0.03em]">
              {v ? num(v.avg, DIGITS[m] ?? 0) : '—'}
              <span className="ml-1 font-sans text-[12px] font-normal tracking-normal text-fg-3">{v ? v.unit : 'sem dados'}</span>
            </p>
            {v ? (
              <Meter className="mt-3" value={v.avg} max={pctScale} label={`${METRIC_LABEL[m]} ${num(v.avg)}`} color={m.endsWith('Load') && v.avg > 95 ? 'var(--frx-caution)' : 'var(--frx-text-2)'} />
            ) : (
              <p className="mt-3 text-[11.5px] text-fg-3">Não registrado no log</p>
            )}
          </div>
        )
      })}
      <div className="flex flex-col justify-center gap-2.5 p-5">
        {extras.map((e) =>
          r[e.m] ? (
            <div key={e.m} className="flex items-center justify-between text-[12.5px]">
              <span className="flex items-center gap-2 text-fg-2">
                <e.icon className="size-3.5 text-fg-3" aria-hidden /> {e.label}
              </span>
              <span className="tnum font-mono text-fg" title={`máx. ${e.fmt(r[e.m]!.max)}`}>
                {e.fmt(r[e.m]!.avg)}
              </span>
            </div>
          ) : null,
        )}
        {extras.every((e) => !r[e.m]) && <p className="text-[12px] text-fg-3">Temperaturas e consumo não foram registrados neste log.</p>}
      </div>
    </div>
  )
}

/** Análise descritiva: nada de diagnóstico definitivo de gargalo. */
export function StabilityPanel({ meta }: { meta: SessionMeta }) {
  const st = meta.stats.stability
  const hist = meta.stats.frametime.histogram
  const colors = ['var(--frx-good)', 'var(--frx-text-2)', 'var(--frx-caution)', 'var(--frx-bad)', 'var(--frx-bad)']
  const lines: string[] = []
  lines.push(
    st.spikes === 0
      ? 'Nenhum pico de frametime acima do critério (2× a mediana local e +8 ms).'
      : `${st.spikes} pico${st.spikes === 1 ? '' : 's'} de frametime${st.spikesPerMin != null ? ` (${fmtNum(st.spikesPerMin, 1)} por minuto)` : ''}.`,
  )
  if (st.cv != null) lines.push(`Variação do frametime ao longo da sessão: ${fmtNum(st.cv * 100, 0)}% (coeficiente de variação).`)
  if (st.trendPct != null && Math.abs(st.trendPct) >= 3)
    lines.push(`O FPS médio do último terço foi ${fmtNum(Math.abs(st.trendPct), 0)}% ${st.trendPct > 0 ? 'maior' : 'menor'} que o do primeiro terço${st.trendPct < 0 ? ' — pode indicar aquecimento, cenas mais pesadas ou acúmulo de dados' : ''}.`)
  return (
    <div className="grid grid-cols-2 gap-8">
      <section aria-labelledby="estabilidade">
        <h2 id="estabilidade" className="font-display text-[20px] font-semibold tracking-[-0.02em]">
          Estabilidade
        </h2>
        <ul className="mt-3 space-y-1.5 text-[13.5px] text-fg-2">
          {lines.map((l) => (
            <li key={l}>{l}</li>
          ))}
          {st.resolutionNote && <li className="text-caution">{st.resolutionNote}</li>}
        </ul>
        {hist.length > 0 && (
          <div className="mt-5">
            <p className="mb-2 text-[12px] text-fg-3">Tempo gasto em cada faixa de quadros</p>
            <div className="flex h-2.5 gap-[2px] overflow-hidden rounded-[3px]" role="img" aria-label={hist.map((h) => `${h.label}: ${fmtNum(h.timeShare * 100, 1)}%`).join(', ')}>
              {hist.map((h, i) => (h.timeShare > 0 ? <div key={h.label} style={{ flex: h.timeShare, background: colors[i] }} title={`${h.label}: ${fmtNum(h.timeShare * 100, 1)}%`} /> : null))}
            </div>
            <ul className="mt-2.5 grid grid-cols-3 gap-x-4 gap-y-1 text-[12px] text-fg-2">
              {hist.map((h, i) => (
                <li key={h.label} className="flex items-center gap-2">
                  <span className="size-2 shrink-0" style={{ background: colors[i] }} aria-hidden />
                  <span className="flex-1 truncate">{h.label}</span>
                  <span className="tnum font-mono text-fg">{fmtNum(h.timeShare * 100, 1)}%</span>
                </li>
              ))}
            </ul>
          </div>
        )}
        {st.unstableIntervals.length > 0 && (
          <div className="mt-5">
            <p className="mb-2 text-[12px] text-fg-3">Trechos de maior instabilidade (destacados no gráfico)</p>
            <ul className="divide-y divide-line rounded-md border border-line">
              {st.unstableIntervals.map((iv) => (
                <li key={iv.startS} className="flex items-center justify-between gap-4 px-3.5 py-2 text-[12.5px]">
                  <span className="tnum font-mono text-fg">
                    {fmtNum(iv.startS, 0)}–{fmtNum(iv.endS, 0)} s
                  </span>
                  <span className="text-fg-2">
                    {iv.spikes} pico{iv.spikes === 1 ? '' : 's'} · pior quadro {fmtNum(iv.worstMs, 1)} ms
                  </span>
                </li>
              ))}
            </ul>
          </div>
        )}
        <p className="mt-4 text-[11.5px] leading-relaxed text-fg-3">
          Picos isolados são comuns (carregamentos, compilação de shaders, autosave). Padrões repetidos merecem atenção. Uso de CPU ou GPU sozinho não prova onde está o gargalo.
        </p>
      </section>
      <QualityPanel meta={meta} />
    </div>
  )
}

export function QualityPanel({ meta }: { meta: SessionMeta }) {
  const q = meta.quality
  const meta2 = QUALITY[q.level]
  const icon = { info: Info, warning: AlertTriangle, error: CircleAlert }
  const color = { info: 'var(--frx-text-3)', warning: 'var(--frx-caution)', error: 'var(--frx-bad)' }
  return (
    <section aria-labelledby="qualidade">
      <div className="flex items-center justify-between">
        <h2 id="qualidade" className="font-display text-[20px] font-semibold tracking-[-0.02em]">
          Qualidade dos dados
        </h2>
        <Pill color={meta2.color} soft={meta2.soft}>
          {meta2.label}
        </Pill>
      </div>
      <p className="mt-2 text-[12.5px] text-fg-3">
        {fmtNum(q.totalRows)} linhas no log · {fmtNum(q.excludedRows)} fora das estatísticas · formato {meta.format === 'generic-frametime' ? 'CSV de frametime' : 'MangoHud'}
        {meta.mangohudVersion && ` ${meta.mangohudVersion}`} · {meta.stats.mode === 'per-frame' ? 'um registro por quadro' : 'amostras periódicas'}
      </p>
      {q.issues.length === 0 ? (
        <p className="mt-4 text-[13.5px] text-fg-2">Nenhum problema encontrado nos dados.</p>
      ) : (
        <ul className="mt-4 space-y-2">
          {q.issues.map((i, k) => {
            const I = icon[i.severity]
            return (
              <li key={k} className="flex items-start gap-2.5 text-[13px] text-fg-2">
                <I className="mt-0.5 size-3.5 shrink-0" style={{ color: color[i.severity] }} aria-hidden />
                <span>{i.message}</span>
              </li>
            )
          })}
        </ul>
      )}
      {meta.notes && (
        <div className="mt-5 rounded-md border border-line bg-bg/40 px-3.5 py-2.5">
          <p className="eyebrow mb-1.5">Notas</p>
          <p className="text-[13px] whitespace-pre-wrap text-fg-2">{meta.notes}</p>
        </div>
      )}
    </section>
  )
}
