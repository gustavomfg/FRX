import { useEffect, useMemo, useRef, useState } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { ArrowRight, ArrowUpRight, Gauge, RefreshCw, Undo2, Wrench, ScanSearch, CircleAlert, Activity as ActivityIcon } from 'lucide-react'
import { Page } from '@/layouts/Page'
import { Mascot } from '@/mascot/Mascot'
import { Button } from '@/components/ui/Button'
import { ConfidenceMeter, DemoTag, STATUS_META, StatusDot } from '@/components/ui/Status'
import { GameArt } from '@/components/game/GameArt'
import { games, gameById } from '@/data/games'
import { systemInfo } from '@/data/system'
import { compareRuns } from '@/data/benchmarks'
import { useAppState, useDispatch, useFixSummary, useToast } from '@/state/store'
import { useAttention } from '@/state/selectors'
import { fmtPct, fmtRelative, greeting } from '@/lib/format'
import type { Activity, CompatStatus } from '@/types'
import { cn } from '@/lib/cn'
import { useLibrary } from '@/state/library'
import { RealHome } from './RealHome'

const STATUS_ORDER: CompatStatus[] = ['perfeito', 'jogavel', 'ajustes', 'problematico', 'nao-testado']

export function HomePage() {
  const { mode } = useLibrary()
  return mode === 'real' ? <RealHome /> : <DemoHome />
}

function DemoHome() {
  const navigate = useNavigate()
  const attention = useAttention()
  const { available, applied } = useFixSummary()
  const { benchmarks, activity } = useAppState()
  const featured = gameById('ferrous-drift')!

  const today = new Intl.DateTimeFormat('pt-BR', { weekday: 'long', day: 'numeric', month: 'long' }).format(new Date())

  const lastComparison = useMemo(() => {
    const fd = benchmarks.filter((b) => b.gameId === 'ferrous-drift')
    return fd.length >= 2 ? compareRuns(fd[0], fd[fd.length - 1]) : null
  }, [benchmarks])

  return (
    <Page>
      {/* Cabeçalho editorial */}
      <header className="flex items-end justify-between gap-6">
        <div>
          <p className="eyebrow mb-3 first-letter:uppercase">{today}</p>
          <h1 className="font-display text-[44px] leading-[1.02] font-bold tracking-[-0.045em]">
            {greeting()}.
            <span className="text-fg-3"> Vamos jogar?</span>
          </h1>
          <p className="mt-3 max-w-xl text-[15px] text-fg-2">
            {games.length} jogos na estante.{' '}
            {attention.length > 0 ? (
              <>
                <span className="text-fg">{attention.length} pedem atenção</span> e já têm correções esperando por você.
              </>
            ) : (
              'Nenhum pedindo atenção agora — bom jogo.'
            )}
          </p>
        </div>
        <ScanButton />
      </header>

      {/* Destaque — pequena cena viva */}
      <section aria-labelledby="destaque" className="relative mt-9 overflow-hidden rounded-xl border border-line bg-surface">
        <div className="grid min-h-[268px] grid-cols-[1.05fr_1fr]">
          <div className="relative z-10 flex flex-col justify-between p-8">
            <div>
              <div className="flex items-center gap-2">
                <span className="eyebrow text-accent!">Destaque</span>
                <DemoTag />
              </div>
              <h2 id="destaque" className="mt-4 max-w-md font-display text-[28px] leading-[1.1] font-semibold tracking-[-0.03em]">
                {featured.title} sem engasgos nas primeiras voltas.
              </h2>
              <p className="mt-3 max-w-md text-[14px] text-fg-2">
                Em um teste demonstrativo, o cache de shaders levou o FPS médio de <span className="tnum text-fg">50</span> para{' '}
                <span className="tnum text-fg">112</span>. Os resultados variam conforme hardware e não são garantidos.
              </p>
            </div>
            <div className="mt-6 flex items-center gap-2">
              <Button variant="primary" icon={<Wrench className="size-4" />} onClick={() => navigate('/correcoes?pack=fp-ferrous-shader')}>
                Ver Fix Pack
              </Button>
              <Button variant="ghost" iconRight={<ArrowRight className="size-4" />} onClick={() => navigate('/performance?game=ferrous-drift')}>
                Ver comparação
              </Button>
            </div>
          </div>

          <div className="relative">
            <div className="absolute inset-0">
              <GameArt art={featured.cover} variant="banner" />
            </div>
            <div className="absolute inset-0 bg-gradient-to-r from-surface via-surface/40 to-transparent" aria-hidden />
            <div className="absolute inset-x-0 bottom-0 h-16 bg-gradient-to-t from-surface/80 to-transparent" aria-hidden />
            {lastComparison && (
              <div className="absolute top-6 right-6 rounded-md border border-line-2 bg-bg/80 px-3 py-2 backdrop-blur-sm">
                <div className="eyebrow mb-1">FPS médio · demo</div>
                <div className="tnum flex items-baseline gap-2 font-mono">
                  <span className="text-[13px] text-fg-3 line-through decoration-fg-3/50">{Math.round(lastComparison.before.avgFps)}</span>
                  <span className="text-[20px] font-semibold text-fg">{Math.round(lastComparison.after.avgFps)}</span>
                  <span className="text-[12px] text-good">{fmtPct(lastComparison.deltaPct)}</span>
                </div>
              </div>
            )}
            <div className="absolute bottom-[10px] left-[18%]">
              <Mascot pose="wave" scale={5} />
            </div>
          </div>
        </div>
        {/* chão pixelado */}
        <div className="absolute inset-x-0 bottom-0 h-[10px] border-t-2 border-dashed border-line-2 bg-bg/60" aria-hidden />
      </section>

      {/* Três leituras rápidas, sem cartões aninhados */}
      <section className="mt-10 grid grid-cols-3 divide-x divide-line border-y border-line" aria-label="Resumo">
        <SummaryColumn eyebrow="Biblioteca" to="/biblioteca" cta="Abrir biblioteca">
          <BigNumber value={games.length} label="jogos encontrados" />
          <StatusDistribution />
        </SummaryColumn>

        <SummaryColumn eyebrow="Fix Packs" to="/correcoes" cta="Explorar correções">
          <BigNumber value={available.length} label="correções disponíveis" aside={`${applied.length} aplicadas`} />
          <ul className="mt-5 space-y-2.5">
            {[...available]
              .sort((a, b) => b.confidence - a.confidence)
              .slice(0, 3)
              .map((f) => (
                <li key={f.id}>
                  <Link to={`/correcoes?pack=${f.id}`} className="group flex items-center justify-between gap-3 rounded-sm text-[13px]">
                    <span className="min-w-0">
                      <span className="block truncate text-fg group-hover:text-accent">{f.name}</span>
                      <span className="block truncate text-[12px] text-fg-3">{gameById(f.gameId)?.title}</span>
                    </span>
                    <ConfidenceMeter value={f.confidence} showLabel={false} />
                  </Link>
                </li>
              ))}
          </ul>
        </SummaryColumn>

        <SummaryColumn eyebrow="Performance Lab" to="/performance" cta="Abrir Performance Lab">
          {lastComparison ? (
            <>
              <div className="flex items-baseline gap-3">
                <span className="tnum font-display text-[40px] leading-none font-bold tracking-[-0.04em] text-fg">{fmtPct(lastComparison.deltaPct)}</span>
                <span className="text-[13px] text-fg-2">FPS médio</span>
              </div>
              <p className="mt-1.5 text-[12.5px] text-fg-3">Último benchmark · {featured.title} · demo</p>
              <Sparkline before={lastComparison.before.samples.map((s) => s.fps)} after={lastComparison.after.samples.map((s) => s.fps)} />
            </>
          ) : (
            <p className="text-fg-2">Nenhum benchmark ainda.</p>
          )}
        </SummaryColumn>
      </section>

      {/* Atividade + sistema */}
      <div className="mt-10 grid grid-cols-[1.35fr_1fr] gap-12">
        <section aria-labelledby="atividade">
          <div className="mb-4 flex items-end justify-between">
            <h2 id="atividade" className="font-display text-[19px] font-semibold tracking-[-0.02em]">
              Atividade recente
            </h2>
            <span className="text-[12px] text-fg-3">{activity.length} eventos</span>
          </div>
          <ActivityList items={activity.slice(0, 6)} />
        </section>

        <section aria-labelledby="sistema">
          <div className="mb-4 flex items-end justify-between">
            <h2 id="sistema" className="font-display text-[19px] font-semibold tracking-[-0.02em]">
              Seu sistema
            </h2>
            <DemoTag>Perfil demo</DemoTag>
          </div>
          <dl className="divide-y divide-line border-y border-line text-[13px]">
            {[
              ['Distribuição', systemInfo.distro.replace(' (demo)', '')],
              ['Kernel', systemInfo.kernel],
              ['Sessão', systemInfo.desktop],
              ['CPU', systemInfo.cpu],
              ['GPU', systemInfo.gpu],
              ['Driver', systemInfo.driver],
              ['Proton padrão', systemInfo.defaultProton],
            ].map(([k, v]) => (
              <div key={k} className="flex items-center justify-between gap-4 py-2.5">
                <dt className="text-fg-3">{k}</dt>
                <dd className="truncate text-right text-fg">{v}</dd>
              </div>
            ))}
          </dl>
          <div className="mt-3 flex items-center gap-2 text-[12px] text-fg-3">
            <span className="anim-pulse-dot size-1.5 bg-good" aria-hidden />
            Tudo certo com os drivers (simulado)
          </div>
        </section>
      </div>
    </Page>
  )
}

function ScanButton() {
  const dispatch = useDispatch()
  const toast = useToast()
  const [progress, setProgress] = useState<number | null>(null)
  const timer = useRef<ReturnType<typeof setInterval> | null>(null)
  useEffect(() => () => {
    if (timer.current) clearInterval(timer.current)
  }, [])

  const scan = () => {
    setProgress(0)
    let n = 0
    timer.current = setInterval(() => {
      n++
      setProgress(n)
      if (n >= games.length) {
        clearInterval(timer.current!)
        setTimeout(() => {
          setProgress(null)
          dispatch({ type: 'activity', kind: 'scan', text: `Biblioteca verificada — ${games.length} jogos, nenhum novo` })
          toast({ tone: 'ok', title: 'Biblioteca verificada', body: `${games.length} jogos encontrados. Nenhum jogo novo (simulado).` })
        }, 250)
      }
    }, 140)
  }

  return (
    <Button variant="outline" onClick={scan} loading={progress !== null} icon={<RefreshCw className="size-4" />} aria-live="polite">
      {progress !== null ? (
        <span className="tnum">
          Verificando {progress}/{games.length}
        </span>
      ) : (
        'Verificar biblioteca'
      )}
    </Button>
  )
}

export function SummaryColumn({ eyebrow, to, cta, children }: { eyebrow: string; to: string; cta: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col px-7 py-6 first:pl-0 last:pr-0">
      <div className="eyebrow mb-4">{eyebrow}</div>
      <div className="flex-1">{children}</div>
      <Link to={to} className="group mt-6 inline-flex w-fit items-center gap-1.5 rounded-xs text-[13px] font-medium text-fg-2 transition-colors hover:text-accent">
        {cta}
        <ArrowUpRight className="size-3.5 transition-transform duration-(--frx-dur) ease-frx group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
      </Link>
    </div>
  )
}

export function BigNumber({ value, label, aside }: { value: number; label: string; aside?: string }) {
  return (
    <div className="flex items-baseline gap-3">
      <span className="tnum font-display text-[40px] leading-none font-bold tracking-[-0.04em]">{value}</span>
      <span className="text-[13px] text-fg-2">{label}</span>
      {aside && <span className="ml-auto text-[12px] text-fg-3">{aside}</span>}
    </div>
  )
}

function StatusDistribution() {
  const counts = STATUS_ORDER.map((s) => ({ s, n: games.filter((g) => g.status === s).length })).filter((c) => c.n > 0)
  return (
    <div className="mt-5">
      <div className="flex h-2 gap-[2px] overflow-hidden rounded-[3px]" role="img" aria-label={counts.map((c) => `${c.n} ${STATUS_META[c.s].label}`).join(', ')}>
        {counts.map((c) => (
          <div key={c.s} style={{ flex: c.n, background: STATUS_META[c.s].color }} title={`${STATUS_META[c.s].label}: ${c.n}`} />
        ))}
      </div>
      <ul className="mt-3 grid grid-cols-2 gap-x-4 gap-y-1.5 text-[12.5px]">
        {counts.map((c) => (
          <li key={c.s} className="flex items-center gap-2 text-fg-2">
            <StatusDot status={c.s} />
            <span className="flex-1 truncate">{STATUS_META[c.s].label}</span>
            <span className="tnum font-mono text-fg">{c.n}</span>
          </li>
        ))}
      </ul>
    </div>
  )
}

export function Sparkline({ before, after }: { before: number[]; after: number[] }) {
  const W = 260
  const H = 54
  const max = Math.max(...before, ...after) * 1.05
  const path = (arr: number[]) => arr.map((v, i) => `${i ? 'L' : 'M'}${((i / (arr.length - 1)) * W).toFixed(1)} ${(H - (v / max) * H).toFixed(1)}`).join('')
  return (
    <div className="mt-4">
      <svg viewBox={`0 0 ${W} ${H}`} className="h-[54px] w-full overflow-visible" role="img" aria-label="FPS antes e depois da correção (demonstrativo)" preserveAspectRatio="none">
        <path d={path(before)} fill="none" stroke="var(--frx-chart-base)" strokeWidth={1.5} strokeDasharray="3 3" vectorEffect="non-scaling-stroke" />
        <path d={path(after)} fill="none" stroke="var(--frx-accent)" strokeWidth={2} vectorEffect="non-scaling-stroke" />
      </svg>
      <div className="mt-2 flex gap-4 text-[11.5px] text-fg-3">
        <span className="flex items-center gap-1.5">
          <span className="w-3 border-t border-dashed border-(--frx-chart-base)" /> Antes
        </span>
        <span className="flex items-center gap-1.5">
          <span className="w-3 border-t-2 border-accent" /> Depois
        </span>
      </div>
    </div>
  )
}

const ACT_ICON: Record<Activity['kind'], typeof Wrench> = {
  scan: RefreshCw,
  analise: ScanSearch,
  fix: Wrench,
  revert: Undo2,
  benchmark: Gauge,
  erro: CircleAlert,
}

export function ActivityList({ items }: { items: Activity[] }) {
  if (!items.length) return <p className="py-6 text-[13px] text-fg-3">Nada por aqui ainda.</p>
  return (
    <ol className="relative">
      {items.map((a, i) => {
        const Icon = ACT_ICON[a.kind] ?? ActivityIcon
        const g = a.gameId ? gameById(a.gameId) : undefined
        return (
          <li key={a.id} className="group relative flex gap-4 pb-4 last:pb-0">
            {i < items.length - 1 && <span className="absolute top-8 bottom-1 left-[13px] w-px bg-line-2" aria-hidden />}
            <span className={cn('relative grid size-7 shrink-0 place-items-center rounded-sm border border-line bg-surface', a.kind === 'erro' ? 'text-bad' : 'text-fg-2')}>
              <Icon className="size-3.5" aria-hidden />
            </span>
            <div className="min-w-0 flex-1 pt-1">
              <p className="text-[13.5px] text-fg">
                {g ? (
                  <Link to={`/biblioteca/${g.id}`} className="hover:text-accent">
                    {a.text}
                  </Link>
                ) : (
                  a.text
                )}
              </p>
              <p className="mt-0.5 text-[12px] text-fg-3">{fmtRelative(a.at)}</p>
            </div>
          </li>
        )
      })}
    </ol>
  )
}
