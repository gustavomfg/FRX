import { useEffect, useMemo, useRef, useState } from 'react'
import { useSearchParams } from 'react-router-dom'
import { AnimatePresence, motion } from 'motion/react'
import { Activity, Play, Square, Thermometer, Zap } from 'lucide-react'
import { Page } from '@/layouts/Page'
import { CompareHero } from '@/components/perf/CompareHero'
import { MetricChart, type ChartMetric } from '@/components/perf/MetricChart'
import { VERDICT } from '@/components/perf/verdict'
import { Button } from '@/components/ui/Button'
import { Meter, Segmented, Select } from '@/components/ui/Controls'
import { DemoTag } from '@/components/ui/Status'
import { EmptyState } from '@/components/ui/EmptyState'
import { Mascot } from '@/mascot/Mascot'
import { games, gameById } from '@/data/games'
import { compareRuns } from '@/data/benchmarks'
import { systemInfo } from '@/data/system'
import { runBenchmark, startLiveMonitor } from '@/services/demo'
import { useAppState, useDispatch, useSettings, useToast } from '@/state/store'
import { fmtDateTime, fmtNum, fmtPct } from '@/lib/format'
import type { BenchmarkRun, MetricSample } from '@/types'
import { cn } from '@/lib/cn'
import { useLibrary } from '@/state/library'
import { RealPerformanceLab } from './RealPerformance'

const EXAMPLES = [
  { game: 'ferrous-drift', verdict: 'melhora' as const },
  { game: 'ashen-choir', verdict: 'regressao' as const },
  { game: 'moss-mortar', verdict: 'estavel' as const },
  { game: 'glasswing', verdict: 'inconclusivo' as const },
]

export function PerformancePage() {
  const { mode } = useLibrary()
  return mode === 'real' ? <RealPerformanceLab /> : <DemoPerformance />
}

function DemoPerformance() {
  const [params, setParams] = useSearchParams()
  const { benchmarks, fixStates } = useAppState()
  const dispatch = useDispatch()
  const toast = useToast()
  const gameId = params.get('game') ?? 'ferrous-drift'
  const game = gameById(gameId) ?? games[0]
  const runs = useMemo(() => benchmarks.filter((b) => b.gameId === game.id), [benchmarks, game.id])

  const [beforeId, setBeforeId] = useState<string | null>(null)
  const [afterId, setAfterId] = useState<string | null>(null)
  const [metric, setMetric] = useState<ChartMetric>('fps')

  // Benchmark em execução / monitor ao vivo
  const [running, setRunning] = useState(false)
  const [progress, setProgress] = useState(0)
  const [liveSamples, setLiveSamples] = useState<MetricSample[]>([])
  const [monitor, setMonitor] = useState(false)
  const abort = useRef<AbortController | null>(null)

  // Padrão: primeira medição sem correções × última medição.
  useEffect(() => {
    const base = runs.find((r) => r.fixIds.length === 0) ?? runs[0]
    const last = runs[runs.length - 1]
    setBeforeId(base?.id ?? null)
    setAfterId(last && last.id !== base?.id ? last.id : null)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [game.id, runs.length])

  useEffect(() => () => abort.current?.abort(), [])

  // Monitor ao vivo: uma amostra a cada 500 ms, janela de 60 pontos.
  useEffect(() => {
    if (!monitor) return
    setLiveSamples([])
    const base = runs[runs.length - 1]?.avgFps ?? 90
    return startLiveMonitor(base, (s) => setLiveSamples((prev) => [...prev.slice(-59), s]))
  }, [monitor, runs])

  const before = runs.find((r) => r.id === beforeId)
  const after = runs.find((r) => r.id === afterId)
  const comparison = before && after ? compareRuns(before, after) : null

  const appliedForGame = game.fixPackIds.filter((id) => fixStates[id] === 'aplicado')

  const startBenchmark = async () => {
    setMonitor(false)
    setRunning(true)
    setProgress(0)
    setLiveSamples([])
    abort.current = new AbortController()
    try {
      const run = await runBenchmark(
        game,
        appliedForGame,
        (s, p) => {
          setLiveSamples((prev) => [...prev, s])
          setProgress(p)
        },
        abort.current.signal,
      )
      const base = runs.find((r) => r.fixIds.length === 0) ?? runs[runs.length - 1]
      const c = base ? compareRuns(base, run) : null
      dispatch({
        type: 'benchmark',
        run,
        text: c ? `Benchmark em ${game.title}: ${fmtPct(c.deltaPct)} FPS médio · ${VERDICT[c.verdict].label.toLowerCase()} (demo)` : `Primeiro benchmark em ${game.title} (demo)`,
      })
      if (base) setBeforeId(base.id)
      setAfterId(run.id)
      toast({ tone: c?.verdict === 'regressao' ? 'warn' : 'ok', title: 'Benchmark concluído', body: c ? `${VERDICT[c.verdict].label}: ${fmtPct(c.deltaPct)} FPS médio (simulado)` : 'Primeira medição registrada.' })
    } catch {
      /* cancelado */
    } finally {
      setRunning(false)
      setLiveSamples([])
    }
  }

  // Chegada pelo fluxo "Medir desempenho" (run=1) dispara o benchmark.
  // O timeout também sobrevive à montagem dupla do StrictMode.
  useEffect(() => {
    if (params.get('run') !== '1') return
    const t = setTimeout(() => {
      const next = new URLSearchParams(params)
      next.delete('run')
      setParams(next, { replace: true })
      startBenchmark()
    }, 350)
    return () => clearTimeout(t)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const selectGame = (id: string) => {
    abort.current?.abort()
    setMonitor(false)
    setParams({ game: id }, { replace: true })
  }

  const liveMode = running || monitor
  const chartSeries = liveMode
    ? [{ key: 'live', label: running ? 'Benchmark em curso' : 'Ao vivo', samples: liveSamples, role: 'main' as const }]
    : [
        ...(before ? [{ key: 'before', label: `Antes · ${before.label}`, samples: before.samples, role: 'base' as const }] : []),
        ...(after ? [{ key: 'after', label: `Depois · ${after.label}`, samples: after.samples, role: 'main' as const }] : []),
      ]

  const resourceSource: MetricSample[] = liveMode ? liveSamples : (after ?? before)?.samples ?? []
  const resourceBase: MetricSample[] | undefined = liveMode ? undefined : after && before ? before.samples : undefined

  return (
    <Page>
      <header className="flex items-end justify-between gap-6">
        <div>
          <div className="flex items-center gap-2">
            <p className="eyebrow">Performance Lab</p>
            <DemoTag>Dados simulados</DemoTag>
          </div>
          <h1 className="mt-3 font-display text-[36px] leading-none font-bold tracking-[-0.04em]">Desempenho</h1>
          <p className="mt-2 text-[14px] text-fg-2">Meça, compare e decida se uma correção vale a pena. Nenhum valor aqui é uma medição real.</p>
        </div>
        <div className="flex items-center gap-2">
          <Select label="Jogo" hideLabel value={game.id} onChange={selectGame} options={games.map((g) => ({ value: g.id, label: g.title }))} />
          <Button
            variant={monitor ? 'secondary' : 'outline'}
            icon={monitor ? <Square className="size-3.5" /> : <Activity className="size-4" />}
            onClick={() => setMonitor((m) => !m)}
            disabled={running}
            aria-pressed={monitor}
          >
            {monitor ? 'Parar monitor' : 'Monitor ao vivo'}
          </Button>
          {running ? (
            <Button variant="danger" icon={<Square className="size-3.5" />} onClick={() => abort.current?.abort()}>
              Cancelar
            </Button>
          ) : (
            <Button variant="primary" icon={<Play className="size-4" />} onClick={startBenchmark}>
              Executar benchmark
            </Button>
          )}
        </div>
      </header>

      {/* Exemplos de veredito */}
      <div className="mt-6 flex items-center gap-2 text-[12.5px]">
        <span className="text-fg-3">Exemplos:</span>
        {EXAMPLES.map((e) => {
          const v = VERDICT[e.verdict]
          const active = game.id === e.game
          return (
            <button
              key={e.game}
              onClick={() => selectGame(e.game)}
              aria-pressed={active}
              className={cn('inline-flex h-7 items-center gap-1.5 rounded-sm border px-2.5 font-medium transition-colors', active ? 'border-line-3 bg-raised text-fg' : 'border-line text-fg-2 hover:border-line-2 hover:text-fg')}
            >
              <span className="size-1.5" style={{ background: v.color }} aria-hidden />
              {v.label}
              <span className="text-fg-3">· {gameById(e.game)?.title}</span>
            </button>
          )
        })}
      </div>

      {/* Herói de comparação / progresso */}
      <div className="mt-5">
        <AnimatePresence mode="wait" initial={false}>
          {running ? (
            <motion.section key="running" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} className="flex items-center gap-8 rounded-xl border border-line bg-surface px-8 py-7" aria-live="polite">
              <Mascot pose="inspect" scale={4} />
              <div className="flex-1">
                <p className="font-display text-[22px] font-semibold tracking-[-0.02em]">Medindo {game.title}…</p>
                <p className="mt-1 text-[13px] text-fg-2">
                  Passada de 60 s simulada · {appliedForGame.length ? `${appliedForGame.length} correção(ões) ativa(s)` : 'configuração padrão'}
                </p>
                <div className="mt-4 h-1.5 overflow-hidden rounded-full bg-line-2">
                  <div className="h-full bg-accent transition-[width] duration-100" style={{ width: `${progress * 100}%` }} />
                </div>
              </div>
              <div className="w-28 text-right">
                <p className="tnum font-display text-[40px] leading-none font-bold tracking-[-0.04em]">{Math.round(liveSamples[liveSamples.length - 1]?.fps ?? 0)}</p>
                <p className="text-[12px] text-fg-3">FPS agora</p>
              </div>
            </motion.section>
          ) : comparison ? (
            <motion.div key={`${before!.id}-${after!.id}`} initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }}>
              <CompareHero c={comparison} gameTitle={game.title} />
            </motion.div>
          ) : (
            <motion.div key="empty" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} className="rounded-xl border border-dashed border-line-2">
              <EmptyState
                pose={runs.length ? 'idle' : 'confused'}
                title={runs.length ? 'Falta uma medição para comparar' : 'Nenhum benchmark ainda'}
                body={
                  runs.length
                    ? 'Aplique uma correção e rode outro benchmark — ou escolha duas medições no histórico.'
                    : `Rode um benchmark de ${game.title} para criar a linha de base.`
                }
                action={
                  <Button variant="primary" icon={<Play className="size-4" />} onClick={startBenchmark}>
                    Executar benchmark
                  </Button>
                }
                className="py-10"
              />
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* Gráfico */}
      <section className="mt-10" aria-labelledby="grafico">
        <div className="mb-4 flex items-end justify-between gap-4">
          <div>
            <h2 id="grafico" className="font-display text-[20px] font-semibold tracking-[-0.02em]">
              {metric === 'fps' ? 'Quadros por segundo' : 'Tempo de quadro'}
            </h2>
            <p className="mt-1 text-[12.5px] text-fg-3">
              {liveMode ? (running ? 'Benchmark em andamento' : 'Monitor ao vivo · atualiza a cada 0,5 s') : metric === 'fps' ? 'Maior é melhor' : 'Menor e mais estável é melhor — picos indicam engasgos'}
              {liveMode && <span className="anim-pulse-dot ml-2 inline-block size-1.5 bg-bad align-middle" aria-hidden />}
            </p>
          </div>
          <Segmented<ChartMetric>
            label="Métrica do gráfico"
            size="sm"
            value={metric}
            onChange={setMetric}
            options={[
              { value: 'fps', label: 'FPS' },
              { value: 'frametime', label: 'Frametime' },
            ]}
          />
        </div>
        <div className="relative rounded-xl border border-line bg-surface px-4 pt-5 pb-3">
          {/* O Pim espia o gráfico pela borda */}
          <div className="pointer-events-none absolute -top-[45px] right-[24%]">
            <Mascot pose="observe" scale={3} rows={15} />
          </div>
          {chartSeries.length === 0 || (liveMode && liveSamples.length < 2) ? (
            <div className="grid h-[260px] place-items-center text-[13px] text-fg-3">{liveMode ? 'Coletando amostras…' : 'Sem dados para exibir.'}</div>
          ) : (
            <MetricChart series={chartSeries} metric={metric} live={liveMode} />
          )}
          {!liveMode && chartSeries.length > 1 && (
            <div className="mt-2 flex gap-5 px-2 text-[12px] text-fg-3">
              {chartSeries.map((s) => (
                <span key={s.key} className="flex items-center gap-2">
                  <span className={cn('w-4', s.role === 'base' ? 'border-t-[1.5px] border-dashed border-(--frx-chart-base)' : 'border-t-2 border-accent')} aria-hidden />
                  {s.label}
                </span>
              ))}
            </div>
          )}
        </div>
      </section>

      {/* Recursos */}
      <section className="mt-10" aria-labelledby="recursos">
        <div className="mb-4 flex items-end justify-between">
          <h2 id="recursos" className="font-display text-[20px] font-semibold tracking-[-0.02em]">
            Recursos do sistema
          </h2>
          <span className="text-[12.5px] text-fg-3">{liveMode ? 'Valores instantâneos (simulados)' : after ? 'Média da medição "depois" · Δ em relação a "antes"' : 'Média da medição'}</span>
        </div>
        <Resources samples={resourceSource} base={resourceBase} live={liveMode} />
      </section>

      {/* Histórico */}
      <section className="mt-10" aria-labelledby="historico-bench">
        <div className="mb-4 flex items-end justify-between">
          <h2 id="historico-bench" className="font-display text-[20px] font-semibold tracking-[-0.02em]">
            Histórico de medições
          </h2>
          <span className="text-[12.5px] text-fg-3">Escolha quais medições comparar</span>
        </div>
        <RunHistory runs={runs} beforeId={beforeId} afterId={afterId} onBefore={setBeforeId} onAfter={setAfterId} />
      </section>
    </Page>
  )
}

function avg(arr: MetricSample[], k: keyof MetricSample) {
  if (!arr.length) return 0
  return arr.reduce((a, s) => a + (s[k] as number), 0) / arr.length
}

function Resources({ samples, base, live }: { samples: MetricSample[]; base?: MetricSample[]; live: boolean }) {
  const { tempUnit } = useSettings()
  const pick = (k: keyof MetricSample) => (live ? ((samples[samples.length - 1]?.[k] as number) ?? 0) : avg(samples, k))
  const temp = pick('tempGpu')
  const shownTemp = tempUnit === 'F' ? temp * 1.8 + 32 : temp
  const rows: { label: string; value: number; max: number; unit: string; digits?: number; k: keyof MetricSample }[] = [
    { label: 'CPU', value: pick('cpu'), max: 100, unit: '%', k: 'cpu' },
    { label: 'GPU', value: pick('gpu'), max: 100, unit: '%', k: 'gpu' },
    { label: 'RAM', value: pick('ram'), max: systemInfo.ramGb, unit: `/ ${systemInfo.ramGb} GB`, digits: 1, k: 'ram' },
    { label: 'VRAM', value: pick('vram'), max: systemInfo.vramGb, unit: `/ ${systemInfo.vramGb} GB`, digits: 1, k: 'vram' },
  ]
  return (
    <div className="grid grid-cols-[repeat(4,minmax(0,1fr))_minmax(0,1.15fr)] divide-x divide-line rounded-xl border border-line bg-surface">
      {rows.map((r) => {
        const b = base ? avg(base, r.k) : undefined
        const delta = b !== undefined ? r.value - b : undefined
        return (
          <div key={r.label} className="p-5">
            <div className="flex items-center justify-between text-[12px] text-fg-3">
              <span>{r.label}</span>
              {delta !== undefined && Math.abs(delta) >= 0.5 && (
                <span className="tnum font-mono">
                  {delta > 0 ? '+' : '−'}
                  {fmtNum(Math.abs(delta), r.digits ?? 0)}
                </span>
              )}
            </div>
            <p className="tnum mt-2 font-display text-[26px] leading-none font-semibold tracking-[-0.03em]">
              {fmtNum(r.value, r.digits ?? 0)}
              <span className="ml-1 font-sans text-[12px] font-normal tracking-normal text-fg-3">{r.unit}</span>
            </p>
            <Meter className="mt-3" value={r.value} max={r.max} label={`${r.label} ${fmtNum(r.value, r.digits ?? 0)}`} color={r.value / r.max > 0.9 ? 'var(--frx-caution)' : 'var(--frx-text-2)'} />
          </div>
        )
      })}
      <div className="flex flex-col justify-between gap-3 p-5">
        <div className="flex items-center justify-between text-[12.5px]">
          <span className="flex items-center gap-2 text-fg-2">
            <Thermometer className="size-3.5 text-fg-3" aria-hidden /> Temperatura GPU
          </span>
          <span className="tnum font-mono text-fg">
            {fmtNum(shownTemp)} °{tempUnit}
          </span>
        </div>
        <div className="flex items-center justify-between text-[12.5px]">
          <span className="flex items-center gap-2 text-fg-2">
            <Zap className="size-3.5 text-fg-3" aria-hidden /> Consumo GPU
          </span>
          <span className="tnum font-mono text-fg">{fmtNum(pick('powerW'))} W</span>
        </div>
        <p className="text-[11.5px] leading-snug text-fg-3">Sensores simulados. Leitura real dependerá do suporte do driver em versões futuras.</p>
      </div>
    </div>
  )
}

function RunHistory({ runs, beforeId, afterId, onBefore, onAfter }: { runs: BenchmarkRun[]; beforeId: string | null; afterId: string | null; onBefore: (id: string) => void; onAfter: (id: string) => void }) {
  if (!runs.length) return <p className="text-[13.5px] text-fg-3">Nenhuma medição para este jogo.</p>
  return (
    <div className="overflow-hidden rounded-xl border border-line">
      <table className="w-full text-[13px]">
        <thead>
          <tr className="border-b border-line bg-surface text-left text-[11.5px] font-medium text-fg-3">
            <th className="px-5 py-2.5 font-medium">Quando</th>
            <th className="px-3 py-2.5 font-medium">Configuração</th>
            <th className="px-3 py-2.5 font-medium">Preset</th>
            <th className="px-3 py-2.5 text-right font-medium">FPS médio</th>
            <th className="px-3 py-2.5 text-right font-medium">1% low</th>
            <th className="px-3 py-2.5 text-right font-medium">Frametime</th>
            <th className="px-3 py-2.5 text-right font-medium">Variação</th>
            <th className="px-5 py-2.5 text-right font-medium">Comparar como</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-line">
          {[...runs].reverse().map((r) => {
            const isB = r.id === beforeId
            const isA = r.id === afterId
            return (
              <tr key={r.id} className={cn('transition-colors', isA || isB ? 'bg-surface/70' : 'hover:bg-surface/40')}>
                <td className="px-5 py-3 whitespace-nowrap text-fg-2">{fmtDateTime(r.at)}</td>
                <td className="px-3 py-3 text-fg">{r.label}</td>
                <td className="px-3 py-3 text-fg-2">{r.preset}</td>
                <td className="tnum px-3 py-3 text-right font-mono text-fg">{fmtNum(r.avgFps)}</td>
                <td className="tnum px-3 py-3 text-right font-mono text-fg-2">{fmtNum(r.low1)}</td>
                <td className="tnum px-3 py-3 text-right font-mono text-fg-2">{fmtNum(r.frametimeAvg, 1)} ms</td>
                <td className="tnum px-3 py-3 text-right font-mono text-fg-3">±{fmtNum(r.stdev, 1)}</td>
                <td className="px-5 py-2">
                  <div className="flex justify-end gap-1">
                    <PickButton active={isB} onClick={() => onBefore(r.id)} disabled={isA}>
                      Antes
                    </PickButton>
                    <PickButton active={isA} onClick={() => onAfter(r.id)} disabled={isB}>
                      Depois
                    </PickButton>
                  </div>
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}

function PickButton({ active, children, ...rest }: React.ButtonHTMLAttributes<HTMLButtonElement> & { active: boolean }) {
  return (
    <button
      aria-pressed={active}
      className={cn(
        'h-7 rounded-sm border px-2.5 text-[12px] font-medium transition-colors disabled:opacity-35',
        active ? 'border-accent-line bg-accent-soft text-accent' : 'border-line text-fg-2 hover:border-line-2 hover:text-fg',
      )}
      {...rest}
    >
      {children}
    </button>
  )
}
