import { useState } from 'react'
import { Link } from 'react-router-dom'
import { AnimatePresence, motion } from 'motion/react'
import { ArrowUpRight, RefreshCw } from 'lucide-react'
import { CoverImage } from '@/components/game/CoverImage'
import { CompatPill, ConfidencePill, FindingCard, ProviderStatusList } from './FindingCard'
import type { ApplyTarget } from './ApplyFixDialog'
import { useAssessments } from './useAssessments'
import { CATEGORY } from './findingMeta'
import { Button } from '@/components/ui/Button'
import { Segmented } from '@/components/ui/Controls'
import { EmptyState } from '@/components/ui/EmptyState'
import { useLibrary } from '@/state/library'
import { useFixReports } from '@/state/fixes'
import { isRelevant } from '@/lib/findings'
import { fmtRelative } from '@/lib/format'
import { cn } from '@/lib/cn'

/** Existing list/detail browser remains available for transparent technical research. */
export function TechnicalFindingsBrowser({ gameFilter, selectedId, update, onAct }: { gameFilter: string; selectedId: string | null; update: (patch: Record<string, string | null>) => void; onAct: (target: ApplyTarget) => void }) {
  const { games } = useLibrary()
  const fixes = useFixReports()
  const [filter, setFilter] = useState<'relevantes' | 'todas'>('relevantes')
  const all = Object.values(fixes.reports).flatMap((r) => r.findings.filter((f) => f.gameAppId === r.appId).map((f) => ({ f, game: r.gameName })))
  const list = all.filter(({ f }) => (filter === 'todas' || isRelevant(f) || f.id === selectedId) && (gameFilter === 'todos' || f.gameAppId === gameFilter))
  const selected = list.find(({ f }) => f.id === selectedId) ?? list[0]
  const report = selected ? fixes.reports[selected.f.gameAppId] : undefined
  const selectedGame = selected ? games.find((g) => g.id === selected.f.gameAppId) : undefined
  const analysis = useAssessments(selected?.f.gameAppId ?? null)
  return <div className="mt-5">
    <p className="mb-3 text-[13px] text-fg-2">Descobertas em investigação e fontes técnicas. Uma descoberta não confirma uma instalação automática; a análise do plano informa o que falta.</p>
    <Segmented label="Filtrar descobertas" size="sm" value={filter} onChange={setFilter} options={[{ value: 'relevantes', label: 'Relevantes' }, { value: 'todas', label: 'Todas' }]} />
    {analysis.error && <p role="alert" className="mt-3 text-[13px] text-bad">{analysis.error}</p>}
    {!list.length ? <EmptyState pose="idle" title="Nenhuma descoberta com estes filtros" body="Use Todas para consultar também relatos gerais e incompatibilidades. As pesquisas salvas permanecem disponíveis sem conexão." /> : (
        <div className="mt-5 grid grid-cols-1 xl:grid-cols-[minmax(280px,360px)_minmax(0,1fr)] gap-8">
          <ul className="space-y-1" aria-label="Descobertas">
            {list.map(({ f, game }) => {
              const active = selected?.f.id === f.id
              return (
                <li key={f.id}>
                  <button
                    onClick={() => update({ finding: f.id })}
                    aria-current={active ? 'true' : undefined}
                    className={cn('group relative flex w-full items-start gap-3 rounded-md px-3 py-3 text-left transition-colors duration-(--frx-dur-fast)', active ? 'bg-surface' : 'hover:bg-surface/60')}
                  >
                    <span className={cn('absolute top-3 bottom-3 left-0 w-[3px] transition-opacity', active ? 'bg-accent opacity-100' : 'opacity-0')} aria-hidden />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-[13.5px] font-medium text-fg">{f.title}</span>
                      <span className="block truncate text-[12px] text-fg-3">
                        {game} · {CATEGORY[f.category]}
                        {analysis.byFinding[f.id]?.availability?.state === 'available' && <span className="text-good"> · plano validado</span>}
                      </span>
                    </span>
                    <ConfidencePill finding={f} />
                  </button>
                </li>
              )
            })}
          </ul>

          <AnimatePresence mode="wait" initial={false}>
            {selected && (
              <motion.div key={selected.f.id} initial={{ opacity: 0, x: 8 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -4 }} transition={{ duration: 0.18 }} className="min-w-0 space-y-4">
                <div className="flex items-center gap-4 rounded-xl border border-line bg-surface p-4">
                  {selectedGame && (
                    <span className="block h-14 w-[120px] shrink-0 overflow-hidden rounded-md border border-line">
                      <CoverImage game={selectedGame} kind="header" variant="banner" />
                    </span>
                  )}
                  <div className="min-w-0 flex-1">
                    <Link to={`/biblioteca/${selected.f.gameAppId}`} className="inline-flex items-center gap-1 text-[14px] font-medium text-fg hover:text-accent">
                      {selected.game} <ArrowUpRight className="size-3.5" />
                    </Link>
                    <p className="text-[12px] text-fg-3">
                      AppID {selected.f.gameAppId} · consultado {fmtRelative(report?.retrievedAt ?? selected.f.retrievedAt)}
                      {report?.stale && <span className="text-caution"> · desatualizado</span>}
                    </p>
                  </div>
                  <div className="flex shrink-0 items-center gap-2">
                    <CompatPill finding={selected.f} />
                    <Button
                      size="sm"
                      variant="outline"
                      icon={<RefreshCw className={cn('size-3.5', fixes.searching[selected.f.gameAppId] && 'animate-spin')} />}
                      disabled={!!fixes.searching[selected.f.gameAppId]}
                      onClick={() => void fixes.search(selected.f.gameAppId, selected.game, true)}
                    >
                      Atualizar
                    </Button>
                  </div>
                </div>
                {report && <ProviderStatusList providers={report.providers} />}
                <ul>
                  <FindingCard
                    key={selected.f.id}
                    finding={selected.f}
                    defaultOpen

                    onRefresh={analysis.refresh}
                    assessment={analysis.byFinding[selected.f.id] ?? null}
                    analyzing={analysis.loading}
                    onApply={(a) => onAct({ kind: 'apply', expectedFingerprint: a.availability?.planFingerprint ?? undefined, recipeId: a.recipeId!, appId: a.appId, title: a.recipe?.title ?? selected.f.title })}
                  />
                </ul>
              </motion.div>
            )}
          </AnimatePresence>
        </div>
    )}
  </div>
}
