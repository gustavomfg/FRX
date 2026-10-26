import { useEffect, useMemo, useRef, useState } from 'react'
import { Link, useSearchParams } from 'react-router-dom'
import { AnimatePresence, motion } from 'motion/react'
import { ArrowUpRight, RefreshCw, ScanSearch, TriangleAlert } from 'lucide-react'
import { Page } from '@/layouts/Page'
import { CoverImage } from '@/components/game/CoverImage'
import { SourceTag } from '@/components/game/LibraryNotices'
import { CompatPill, ConfidencePill, FindingCard, ProviderStatusList } from '@/components/fix/FindingCard'
import { ApplyFixDialog, type ApplyTarget } from '@/components/fix/ApplyFixDialog'
import { RecipesPanel, useRecipes } from '@/components/fix/RecipesPanel'
import { OperationHistory } from '@/components/fix/OperationHistory'
import { SmartPacksPanel } from '@/components/smart/SmartPacksPanel'
import { useAssessments } from '@/components/fix/useAssessments'
import { DYN_STATUS } from '@/components/fix/engineMeta'
import { CATEGORY } from '@/components/fix/findingMeta'
import { Button } from '@/components/ui/Button'
import { Segmented, Select } from '@/components/ui/Controls'
import { EmptyState } from '@/components/ui/EmptyState'
import { Mascot } from '@/mascot/Mascot'
import { useLibrary } from '@/state/library'
import { useFixReports } from '@/state/fixes'
import { useFixEngine } from '@/state/fixEngine'
import { isRelevant } from '@/lib/findings'
import { fmtRelative } from '@/lib/format'
import type { FixFinding } from '@/types/fixes'
import { cn } from '@/lib/cn'

type Filter = 'relevantes' | 'todas'
type View = 'descobertas' | 'smart' | 'receitas' | 'historico'

/**
 * Fix Packs com dados reais. Três visões: descobertas (informativas, de
 * fontes públicas), receitas do catálogo local (as únicas aplicáveis) e o
 * histórico do Fix Engine. Mesmo layout de lista + detalhe da versão demo.
 */
export function FindingsPage() {
  const [params, setParams] = useSearchParams()
  const { games } = useLibrary()
  const fixes = useFixReports()
  const [filter, setFilter] = useState<Filter>('relevantes')
  const [bulk, setBulk] = useState<{ done: number; total: number } | null>(null)
  const cancel = useRef(false)
  const gameFilter = params.get('game') ?? 'todos'
  const view = (params.get('view') as View | null) ?? 'descobertas'
  const engine = useFixEngine()
  const [target, setTarget] = useState<ApplyTarget | null>(null)
  const steamGames = games.filter((g) => g.origin === 'steam')

  const all = useMemo(() => {
    const list: { f: FixFinding; game: string }[] = []
    for (const r of Object.values(fixes.reports)) for (const f of r.findings) list.push({ f, game: r.gameName })
    return list
  }, [fixes.reports])

  const list = useMemo(
    () =>
      all
        .filter(({ f }) => (filter === 'todas' ? true : isRelevant(f)))
        .filter(({ f }) => gameFilter === 'todos' || f.gameAppId === gameFilter),
    [all, filter, gameFilter],
  )

  const selectedId = params.get('finding')
  const selected = list.find(({ f }) => f.id === selectedId) ?? list[0]
  const pending = games.filter((g) => !fixes.reports[g.id])
  const searched = Object.keys(fixes.reports).length
  const report = selected ? fixes.reports[selected.f.gameAppId] : undefined
  const selectedGame = selected ? games.find((g) => g.id === selected.f.gameAppId) : undefined
  const { recipes } = useRecipes(view === 'descobertas' && selected ? selected.f.gameAppId : null, false)
  const recipeFor = (id: string) => recipes.find((r) => r.relatedFindings.includes(id)) ?? null
  const analysis = useAssessments(view === 'descobertas' && selected ? selected.f.gameAppId : null)
  const attention = engine.operations.filter((o) => o.state === 'recovery-required' || o.state === 'requires-manual-step')

  const update = (patch: Record<string, string | null>) => {
    const next = new URLSearchParams(params)
    for (const [k, v] of Object.entries(patch)) {
      if (v === null || v === 'todos') next.delete(k)
      else next.set(k, v)
    }
    setParams(next, { replace: true })
  }

  useEffect(() => () => void (cancel.current = true), [])

  const searchPending = async () => {
    cancel.current = false
    const queue = [...pending]
    setBulk({ done: 0, total: queue.length })
    for (let i = 0; i < queue.length && !cancel.current; i++) {
      await fixes.search(queue[i].id, queue[i].title)
      setBulk({ done: i + 1, total: queue.length })
    }
    setBulk(null)
  }

  return (
    <Page>
      <header className="flex items-end justify-between gap-6">
        <div>
          <div className="flex items-center gap-2">
            <p className="eyebrow">Fix Packs</p>
            <SourceTag>{view === 'receitas' ? 'Catálogo local' : view === 'historico' ? 'Fix Engine' : view === 'smart' ? 'Smart Packs' : 'Descobertas'}</SourceTag>
          </div>
          <h1 className="mt-3 font-display text-[36px] leading-none font-bold tracking-[-0.04em]">Correções</h1>
          <p className="mt-2 max-w-xl text-[14px] text-fg-2">
            O FRX analisa cada descoberta e gera a correção quando reconhece uma ação segura e reversível. Nada é aplicado sem prévia, confirmação, backup e rollback.
          </p>
        </div>
        <div className="flex items-end gap-4">
          <div className="text-right text-[13px]">
            <p>
              <span className="tnum font-mono text-fg">{all.filter(({ f }) => isRelevant(f) && f.kind !== 'report').length}</span> <span className="text-fg-2">correções relevantes</span>
            </p>
            <p>
              <span className="tnum font-mono text-fg">{searched}</span> <span className="text-fg-2">
                de {games.length} jogo{games.length === 1 ? '' : 's'} pesquisado{games.length === 1 ? '' : 's'}
              </span>
            </p>
          </div>
          <Mascot pose={bulk ? 'inspect' : 'fix'} scale={3} className="-mb-1" />
        </div>
      </header>

      {attention.length > 0 && (
        <button
          onClick={() => update({ view: 'historico' })}
          className="mt-6 flex w-full items-center gap-3 rounded-md border border-caution/25 bg-caution-soft px-4 py-2.5 text-left text-[13px] text-caution"
        >
          <TriangleAlert className="size-4 shrink-0" aria-hidden />
          {attention.length} operaç{attention.length === 1 ? 'ão precisa' : 'ões precisam'} da sua atenção (passo manual ou recuperação). Nada disso conta como aplicado até o FRX conferir no disco.
        </button>
      )}

      <div className="mt-7 flex items-center gap-3 border-b border-line pb-4">
        <Segmented<View>
          label="Visão"
          size="sm"
          value={view}
          onChange={(v) => update({ view: v === 'descobertas' ? null : v, finding: null })}
          options={[
            { value: 'descobertas', label: 'Descobertas' },
            { value: 'smart', label: 'Smart Packs' },
            { value: 'receitas', label: 'Receitas do FRX' },
            { value: 'historico', label: `Histórico${engine.operations.length ? ` · ${engine.operations.length}` : ''}` },
          ]}
        />
        {view === 'descobertas' && (
        <>
        <Segmented<Filter>
          label="Filtrar descobertas"
          size="sm"
          value={filter}
          onChange={setFilter}
          options={[
            { value: 'relevantes', label: 'Relevantes' },
            { value: 'todas', label: 'Todas' },
          ]}
        />
        <Select
          label="Jogo"
          value={gameFilter}
          onChange={(v) => update({ game: v, finding: null })}
          options={[{ value: 'todos', label: 'Todos os jogos' }, ...Object.values(fixes.reports).map((r) => ({ value: r.appId, label: r.gameName }))]}
        />
        <div className="ml-auto flex items-center gap-2">
          {bulk ? (
            <Button size="sm" variant="outline" onClick={() => (cancel.current = true)} icon={<RefreshCw className="size-3.5 animate-spin" />}>
              Pesquisando {bulk.done}/{bulk.total} · parar
            </Button>
          ) : (
            pending.length > 0 && (
              <Button size="sm" variant="outline" icon={<ScanSearch className="size-3.5" />} onClick={() => void searchPending()}>
                Pesquisar {pending.length} jogo{pending.length === 1 ? '' : 's'} restante{pending.length === 1 ? '' : 's'}
              </Button>
            )
          )}
        </div>
        </>
        )}
      </div>

      {view === 'smart' ? (
        <>
          <div className="mt-5 flex items-center gap-3">
            <Select
              label="Jogo"
              value={gameFilter !== 'todos' ? gameFilter : steamGames[0]?.id ?? 'todos'}
              onChange={(v) => update({ game: v })}
              options={steamGames.length ? steamGames.map((g) => ({ value: g.id, label: g.title })) : [{ value: 'todos', label: 'Nenhum jogo da Steam' }]}
            />
          </div>
          <SmartPacksPanel appId={gameFilter !== 'todos' ? gameFilter : steamGames[0]?.id ?? null} onAct={setTarget} />
        </>
      ) : view === 'receitas' ? (
        <RecipesPanel
          games={games}
          appId={gameFilter !== 'todos' ? gameFilter : steamGames.length === 1 ? steamGames[0].id : null}
          recipeId={params.get('recipe')}
          onSelect={(p) => update({ ...(p.game !== undefined ? { game: p.game || null, recipe: null } : {}), ...(p.recipe ? { recipe: p.recipe } : {}) })}
          onAct={setTarget}
          onShowFinding={(id, appId) => update({ view: null, game: appId, finding: id, recipe: null })}
        />
      ) : view === 'historico' ? (
        <OperationHistory onAct={setTarget} />
      ) : searched === 0 ? (
        <EmptyState
          pose={bulk ? 'inspect' : 'idle'}
          title={bulk ? 'Pesquisando correções…' : 'Nenhuma pesquisa ainda'}
          body="O FRX consulta UMU-Protonfixes e GitHub pelo AppID de cada jogo. As respostas ficam em cache para funcionar sem conexão."
          action={
            !bulk && games.length > 0 ? (
              <Button variant="primary" icon={<ScanSearch className="size-4" />} onClick={() => void searchPending()}>
                Pesquisar {games.length} jogo{games.length === 1 ? '' : 's'}
              </Button>
            ) : undefined
          }
        />
      ) : list.length === 0 ? (
        <EmptyState
          pose="sleep"
          title={all.length ? 'Nada relevante com estes filtros' : 'Nenhuma correção conhecida'}
          body={all.length ? 'Use “Todas” para ver relatos gerais e descobertas incompatíveis com seu hardware.' : 'As fontes não têm correções específicas para os jogos pesquisados. O Pim aproveitou para tirar um cochilo.'}
        />
      ) : (
        <div className="mt-5 grid grid-cols-[minmax(280px,360px)_minmax(0,1fr)] gap-8">
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
                        {analysis.byFinding[f.id]?.class === 'applicable' && <span className="text-good"> · {DYN_STATUS[analysis.byFinding[f.id].status].label.toLowerCase()}</span>}
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
                    recipe={recipeFor(selected.f.id)}
                    onOpenRecipe={(r) => update({ view: 'receitas', game: selected.f.gameAppId, recipe: r.id, finding: null })}
                    assessment={analysis.byFinding[selected.f.id] ?? null}
                    analyzing={analysis.loading}
                    onApply={(a) => setTarget({ kind: 'apply', recipeId: a.recipeId!, appId: a.appId, title: a.recipe?.title ?? selected.f.title })}
                  />
                </ul>
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      )}
      {target && <ApplyFixDialog target={target} onClose={() => setTarget(null)} />}
    </Page>
  )
}
