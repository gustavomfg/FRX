import { useMemo, useRef, useState, useEffect } from 'react'
import { Link, useNavigate, useParams } from 'react-router-dom'
import { AnimatePresence, motion } from 'motion/react'
import { ArrowLeft, Check, ChevronDown, Eye, Gauge, RotateCcw, ScanSearch, Wrench, X } from 'lucide-react'
import { Page } from '@/layouts/Page'
import { GameArt } from '@/components/game/GameArt'
import { Button } from '@/components/ui/Button'
import { ConfidenceMeter, DemoTag, RiskBadge, SeverityMark, StatusBadge } from '@/components/ui/Status'
import { EmptyState } from '@/components/ui/EmptyState'
import { SourceChip, SourceIcon } from '@/components/fix/SourceIcon'
import { useFixFlow } from '@/components/fix/useFixFlow'
import { Mascot } from '@/mascot/Mascot'
import { gameById } from '@/data/games'
import { issueById } from '@/data/issues'
import { systemInfo } from '@/data/system'
import { compareRuns } from '@/data/benchmarks'
import { fixPackById } from '@/data/fixpacks'
import { useAppState, useDispatch, useToast } from '@/state/store'
import { useGameFixes } from '@/state/selectors'
import { ANALYZE_STEPS, analyzeGame } from '@/services/demo'
import { fmtDateTime, fmtNum, fmtPct, fmtRelative } from '@/lib/format'
import type { Game, Issue, SourceKind } from '@/types'
import { cn } from '@/lib/cn'

export function GameDetailPage() {
  const { gameId } = useParams()
  const game = gameById(gameId)
  const navigate = useNavigate()
  if (!game)
    return (
      <Page>
        <EmptyState
          pose="confused"
          title="Jogo não encontrado"
          body="Talvez ele tenha sido desinstalado — ou nunca existiu nesta demonstração."
          action={<Button onClick={() => navigate('/biblioteca')}>Voltar à biblioteca</Button>}
          className="min-h-[70vh]"
        />
      </Page>
    )
  return <GameDetail key={game.id} game={game} />
}

function GameDetail({ game }: { game: Game }) {
  const navigate = useNavigate()
  const { analyzedAt, fixHistory, fixStates, benchmarks } = useAppState()
  const { packs, available, applied } = useGameFixes(game.id)
  const issues = game.issueIds.map(issueById).filter(Boolean) as Issue[]
  const flow = useFixFlow()
  const analysis = useAnalysis(game)

  const history = fixHistory.filter((h) => h.gameId === game.id)
  const runs = benchmarks.filter((b) => b.gameId === game.id)
  const comparison = runs.length >= 2 ? compareRuns(runs[0], runs[runs.length - 1]) : null

  const sourceCounts = useMemo(() => {
    const m = new Map<SourceKind, { label: string; n: number }>()
    for (const s of [...issues.flatMap((i) => i.sources), ...packs.flatMap((p) => p.sources)]) {
      const cur = m.get(s.kind) ?? { label: s.label, n: 0 }
      cur.n++
      m.set(s.kind, cur)
    }
    return [...m.entries()]
  }, [issues, packs])

  const resolved = (issue: Issue) => packs.some((p) => p.solves.includes(issue.id) && fixStates[p.id] === 'aplicado')

  return (
    <Page className="pt-6">
      <Link to="/biblioteca" className="inline-flex items-center gap-1.5 rounded-xs text-[13px] text-fg-3 transition-colors hover:text-fg">
        <ArrowLeft className="size-3.5" /> Biblioteca
      </Link>

      {/* Banner */}
      <section className="relative mt-4 overflow-hidden rounded-xl border border-line">
        <div className="relative h-[300px]">
          <motion.div className="absolute inset-0" initial={{ scale: 1.06 }} animate={{ scale: 1 }} transition={{ duration: 1.2, ease: [0.16, 1, 0.3, 1] }}>
            <GameArt art={game.cover} variant="banner" title={game.title} />
          </motion.div>
          <div className="absolute inset-0 bg-gradient-to-t from-bg via-bg/55 to-transparent" aria-hidden />
          <div className="absolute inset-0 bg-gradient-to-r from-bg/70 via-transparent to-transparent" aria-hidden />
          <div className="absolute inset-x-8 bottom-7 flex items-end justify-between gap-8">
            <div className="min-w-0">
              <p className="text-[13px] text-fg-2">
                {game.studio} · {game.year} · {game.genre}
              </p>
              <h1 className="mt-1.5 font-display text-[52px] leading-[0.95] font-bold tracking-[-0.05em] text-fg">{game.title}</h1>
              <div className="mt-4 flex flex-wrap items-center gap-2">
                <StatusBadge status={game.status} />
                <span className="inline-flex h-6 items-center rounded-xs bg-bg/60 px-2 text-[12px] text-fg-2 backdrop-blur-sm">{game.protonVersion}</span>
                <span className="inline-flex h-6 items-center rounded-xs bg-bg/60 px-2 text-[12px] text-fg-2 backdrop-blur-sm">{game.store}</span>
                <DemoTag />
              </div>
            </div>
            <div className="flex shrink-0 items-center gap-2">
              <Button variant="secondary" icon={<ScanSearch className="size-4" />} onClick={analysis.start} loading={analysis.running}>
                {analysis.running ? 'Analisando…' : 'Analisar'}
              </Button>
              <Button variant="secondary" icon={<Gauge className="size-4" />} onClick={() => navigate(`/performance?game=${game.id}`)}>
                Comparar desempenho
              </Button>
              <Button
                variant="primary"
                icon={<Eye className="size-4" />}
                disabled={!packs.length}
                onClick={() => navigate(`/correcoes?game=${game.id}${packs[0] ? `&pack=${(available[0] ?? packs[0]).id}` : ''}`)}
                title={!packs.length ? 'Nenhuma correção para este jogo' : undefined}
              >
                Ver correção
              </Button>
            </div>
          </div>
        </div>
      </section>

      {/* Fatos rápidos */}
      <dl className="mt-6 grid grid-cols-5 divide-x divide-line border-y border-line">
        {[
          ['Tempo de jogo', `${game.playtimeH} h`],
          ['Última sessão', fmtRelative(game.lastPlayed)],
          ['Tamanho', `${fmtNum(game.sizeGb, 1)} GB`],
          ['Relatos (demo)', fmtNum(game.communityReports)],
          ['Correções', packs.length ? `${applied.length} de ${packs.length} aplicadas` : 'Nenhuma necessária'],
        ].map(([k, v]) => (
          <div key={k} className="px-5 py-4 first:pl-0">
            <dt className="text-[12px] text-fg-3">{k}</dt>
            <dd className="tnum mt-1 text-[15px] font-medium text-fg">{v}</dd>
          </div>
        ))}
      </dl>

      {/* Painel de análise */}
      <AnimatePresence initial={false}>
        {(analysis.running || analysis.justFinished) && (
          <motion.section
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.3, ease: [0.2, 0.8, 0.2, 1] }}
            className="overflow-hidden"
            aria-live="polite"
          >
            <div className="mt-6 flex items-center gap-6 rounded-lg border border-line bg-surface px-6 py-5">
              <Mascot pose={analysis.running ? 'inspect' : 'celebrate'} scale={4} />
              <div className="min-w-0 flex-1">
                <p className="font-display text-[17px] font-semibold tracking-[-0.02em]">
                  {analysis.running ? 'Investigando…' : `Análise concluída: ${issues.length} problema${issues.length === 1 ? '' : 's'} conhecido${issues.length === 1 ? '' : 's'}`}
                </p>
                <ol className="mt-3 grid grid-cols-5 gap-2">
                  {ANALYZE_STEPS.map((s, i) => {
                    const done = i < analysis.step
                    const active = i === analysis.step && analysis.running
                    return (
                      <li key={s} className="min-w-0">
                        <div className={cn('h-1 rounded-full transition-colors duration-(--frx-dur)', done ? 'bg-accent' : active ? 'bg-accent/40' : 'bg-line-2')} />
                        <p className={cn('mt-2 truncate text-[11.5px]', done || active ? 'text-fg-2' : 'text-fg-3')} title={s}>
                          {s}
                        </p>
                      </li>
                    )
                  })}
                </ol>
              </div>
              {!analysis.running && (
                <button onClick={analysis.dismiss} className="grid size-8 place-items-center rounded-sm text-fg-3 hover:bg-raised hover:text-fg" aria-label="Fechar resumo da análise">
                  <X className="size-4" />
                </button>
              )}
            </div>
          </motion.section>
        )}
      </AnimatePresence>

      <div className="mt-10 grid grid-cols-[minmax(0,1.65fr)_minmax(0,1fr)] gap-12">
        <div className="min-w-0 space-y-12">
          {/* Problemas */}
          <section aria-labelledby="problemas">
            <div className="mb-4 flex items-end justify-between">
              <div>
                <h2 id="problemas" className="font-display text-[20px] font-semibold tracking-[-0.02em]">
                  Problemas identificados
                </h2>
                <p className="mt-1 text-[12.5px] text-fg-3">
                  {analyzedAt[game.id] ? `Última análise ${fmtRelative(analyzedAt[game.id])}` : 'Ainda não analisado nesta sessão'} · fontes demonstrativas
                </p>
              </div>
            </div>
            {issues.length === 0 ? (
              <div className="flex items-center gap-5 rounded-lg border border-dashed border-line-2 px-6 py-6">
                <Mascot pose="idle" scale={3} />
                <div>
                  <p className="font-medium">Nenhum problema conhecido</p>
                  <p className="text-[13px] text-fg-2">Este jogo roda bem na configuração atual. Bom jogo!</p>
                </div>
              </div>
            ) : (
              <ul className="divide-y divide-line border-y border-line">
                {issues.map((issue) => (
                  <IssueRow key={issue.id} issue={issue} resolved={resolved(issue)} />
                ))}
              </ul>
            )}
          </section>

          {/* Correções */}
          <section aria-labelledby="correcoes">
            <div className="mb-4 flex items-end justify-between">
              <h2 id="correcoes" className="font-display text-[20px] font-semibold tracking-[-0.02em]">
                Correções recomendadas
              </h2>
              {packs.length > 0 && <span className="text-[12.5px] text-fg-3">{available.length} disponíveis</span>}
            </div>
            {packs.length === 0 ? (
              <p className="text-[13.5px] text-fg-2">Nenhuma correção necessária para este jogo.</p>
            ) : (
              <ul className="space-y-3">
                {packs.map((p) => {
                  const isApplied = fixStates[p.id] === 'aplicado'
                  return (
                    <li key={p.id} className={cn('rounded-lg border bg-surface px-5 py-4 transition-colors', isApplied ? 'border-good/20' : 'border-line hover:border-line-2')}>
                      <div className="flex items-start justify-between gap-6">
                        <div className="min-w-0">
                          <div className="flex items-center gap-2">
                            <h3 className="text-[15px] font-medium text-fg">{p.name}</h3>
                            {isApplied && (
                              <span className="inline-flex h-5 items-center gap-1 rounded-xs bg-good-soft px-1.5 text-[11.5px] font-medium text-good">
                                <Check className="size-3" strokeWidth={2.6} /> Aplicada
                              </span>
                            )}
                          </div>
                          <p className="mt-1 text-[13px] text-fg-2">{p.summary}</p>
                          <div className="mt-3 flex flex-wrap items-center gap-x-5 gap-y-2">
                            <ConfidenceMeter value={p.confidence} />
                            <RiskBadge risk={p.risk} />
                            <span className="flex items-center gap-1.5 text-[12px] text-fg-3">
                              {p.sources.map((s) => (
                                <SourceIcon key={s.kind} kind={s.kind} />
                              ))}
                              {p.sources.map((s) => s.label).join(' · ')}
                            </span>
                          </div>
                        </div>
                        <div className="flex shrink-0 flex-col items-end gap-2">
                          {isApplied ? (
                            <Button size="sm" variant="outline" icon={<RotateCcw className="size-3.5" />} onClick={() => flow.start(p, 'revert')}>
                              Reverter
                            </Button>
                          ) : (
                            <Button size="sm" variant="primary" icon={<Wrench className="size-3.5" />} onClick={() => flow.start(p, 'apply')}>
                              Aplicar
                            </Button>
                          )}
                          <Button size="sm" variant="ghost" onClick={() => navigate(`/correcoes?game=${game.id}&pack=${p.id}`)}>
                            Detalhes
                          </Button>
                        </div>
                      </div>
                    </li>
                  )
                })}
              </ul>
            )}
          </section>

          {/* Histórico */}
          <section aria-labelledby="historico">
            <h2 id="historico" className="mb-4 font-display text-[20px] font-semibold tracking-[-0.02em]">
              Histórico de correções
            </h2>
            {history.length === 0 ? (
              <p className="text-[13.5px] text-fg-3">Nenhuma alteração registrada ainda.</p>
            ) : (
              <ol className="space-y-0 border-l border-line-2 pl-5">
                {history.map((h) => (
                  <li key={h.id} className="relative py-2">
                    <span
                      className={cn('absolute top-[15px] -left-[24.5px] size-2', h.action === 'aplicado' ? 'bg-good' : h.action === 'falhou' ? 'bg-bad' : 'bg-fg-3')}
                      aria-hidden
                    />
                    <p className="text-[13.5px]">
                      <span className="text-fg">{fixPackById(h.fixId)?.name}</span>{' '}
                      <span className={cn(h.action === 'aplicado' ? 'text-good' : h.action === 'falhou' ? 'text-bad' : 'text-fg-2')}>
                        {h.action === 'aplicado' ? 'aplicada' : h.action === 'falhou' ? 'falhou' : 'revertida'}
                      </span>
                    </p>
                    <p className="text-[12px] text-fg-3">
                      {fmtDateTime(h.at)}
                      {h.note && ` · ${h.note}`}
                    </p>
                  </li>
                ))}
              </ol>
            )}
          </section>
        </div>

        {/* Coluna lateral */}
        <aside className="min-w-0 space-y-9">
          <InfoBlock title="Proton">
            <InfoRow k="Versão" v={game.protonVersion} />
            <InfoRow k="Runtime" v={systemInfo.steamRuntime} />
            <InfoRow k="Camada" v={game.runner} />
            <div className="pt-3">
              <p className="mb-1.5 text-[12px] text-fg-3">Opções de inicialização</p>
              <code className="block truncate rounded-sm border border-line bg-bg px-3 py-2 font-mono text-[12px] text-fg-2" title={game.launchOptions ?? '—'}>
                {game.launchOptions ?? '— (não se aplica)'}
              </code>
            </div>
          </InfoBlock>

          <InfoBlock title="Hardware utilizado" tag>
            <InfoRow k="GPU" v={systemInfo.gpu} />
            <InfoRow k="Driver" v={systemInfo.driver} />
            <InfoRow k="CPU" v={systemInfo.cpu} />
            <InfoRow k="Memória" v={`${systemInfo.ramGb} GB RAM · ${systemInfo.vramGb} GB VRAM`} />
          </InfoBlock>

          {sourceCounts.length > 0 && (
            <InfoBlock title="Origem das recomendações">
              {sourceCounts.map(([kind, { label, n }]) => (
                <div key={kind} className="flex items-center justify-between gap-3 py-2 text-[13px]">
                  <span className="flex items-center gap-2 text-fg">
                    <SourceIcon kind={kind} className="text-fg-3" />
                    {label}
                  </span>
                  <span className="text-fg-3">
                    {n} referência{n > 1 ? 's' : ''}
                  </span>
                </div>
              ))}
            </InfoBlock>
          )}

          <section aria-labelledby="bench" className="rounded-lg border border-line bg-surface p-5">
            <div className="flex items-center justify-between">
              <h3 id="bench" className="text-[14px] font-medium">
                Benchmark
              </h3>
              <DemoTag />
            </div>
            {comparison ? (
              <div className="mt-3">
                <div className="flex items-baseline gap-2">
                  <span className={cn('tnum font-display text-[30px] font-bold tracking-[-0.04em]', comparison.verdict === 'regressao' && 'text-bad')}>
                    {fmtPct(comparison.deltaPct)}
                  </span>
                  <span className="text-[12.5px] text-fg-2">FPS médio</span>
                </div>
                <p className="text-[12.5px] text-fg-3">
                  {Math.round(comparison.before.avgFps)} → {Math.round(comparison.after.avgFps)} FPS ·{' '}
                  {comparison.verdict === 'inconclusivo' ? 'inconclusivo' : comparison.verdict === 'estavel' ? 'estável' : comparison.verdict === 'regressao' ? 'regressão' : 'melhora'}
                </p>
              </div>
            ) : (
              <p className="mt-2 text-[13px] text-fg-2">Nenhuma comparação ainda para este jogo.</p>
            )}
            <Button size="sm" variant="secondary" className="mt-4 w-full" icon={<Gauge className="size-3.5" />} onClick={() => navigate(`/performance?game=${game.id}`)}>
              {comparison ? 'Abrir no Performance Lab' : 'Rodar benchmark'}
            </Button>
          </section>
        </aside>
      </div>
      {flow.dialog}
    </Page>
  )
}

function useAnalysis(game: Game) {
  const dispatch = useDispatch()
  const toast = useToast()
  const [step, setStep] = useState(0)
  const [running, setRunning] = useState(false)
  const [justFinished, setJustFinished] = useState(false)
  const abort = useRef<AbortController | null>(null)
  useEffect(() => () => abort.current?.abort(), [])

  const start = async () => {
    setJustFinished(false)
    setRunning(true)
    setStep(0)
    abort.current = new AbortController()
    try {
      await analyzeGame(game, (p) => setStep(p.step), abort.current.signal)
      setRunning(false)
      setJustFinished(true)
      dispatch({ type: 'analyzed', gameId: game.id, text: `${game.title} analisado: ${game.issueIds.length} problema(s) conhecido(s)` })
      toast({ tone: 'ok', title: 'Análise concluída', body: `${game.title} · dados demonstrativos` })
    } catch {
      setRunning(false)
    }
  }
  return { start, step, running, justFinished, dismiss: () => setJustFinished(false) }
}

function IssueRow({ issue, resolved }: { issue: Issue; resolved: boolean }) {
  const [open, setOpen] = useState(false)
  const id = `issue-${issue.id}`
  return (
    <li>
      <button onClick={() => setOpen((o) => !o)} aria-expanded={open} aria-controls={id} className="group flex w-full items-center gap-4 py-4 text-left">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2.5">
            <span className={cn('text-[15px] font-medium', resolved ? 'text-fg-2 line-through decoration-fg-3/50' : 'text-fg')}>{issue.title}</span>
            {resolved && <span className="rounded-xs bg-good-soft px-1.5 text-[11.5px] leading-5 font-medium text-good">Resolvido por correção</span>}
          </div>
          <div className="mt-1.5 flex items-center gap-4">
            <SeverityMark severity={issue.severity} />
            <span className="text-[12px] text-fg-3 capitalize">{issue.symptom === 'inicializacao' ? 'Inicialização' : issue.symptom}</span>
            <span className="text-[12px] text-fg-3">
              {issue.sources.length} fonte{issue.sources.length > 1 ? 's' : ''}
            </span>
          </div>
        </div>
        <ChevronDown className={cn('size-4 text-fg-3 transition-transform duration-(--frx-dur) ease-frx group-hover:text-fg', open && 'rotate-180')} aria-hidden />
      </button>
      <AnimatePresence initial={false}>
        {open && (
          <motion.div
            id={id}
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.24, ease: [0.2, 0.8, 0.2, 1] }}
            className="overflow-hidden"
          >
            <div className="pb-5">
              <p className="text-[13.5px] text-fg-2">{issue.description}</p>
              <div className="mt-3 flex flex-wrap gap-2">
                {issue.sources.map((s, i) => (
                  <SourceChip key={i} source={s} />
                ))}
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </li>
  )
}

function InfoBlock({ title, children, tag }: { title: string; children: React.ReactNode; tag?: boolean }) {
  return (
    <section>
      <div className="mb-2 flex items-center justify-between">
        <h3 className="eyebrow">{title}</h3>
        {tag && <DemoTag />}
      </div>
      <div className="divide-y divide-line border-t border-line">{children}</div>
    </section>
  )
}

function InfoRow({ k, v }: { k: string; v: string }) {
  return (
    <div className="flex items-center justify-between gap-4 py-2 text-[13px]">
      <span className="shrink-0 text-fg-3">{k}</span>
      <span className="truncate text-right text-fg" title={v}>
        {v}
      </span>
    </div>
  )
}
