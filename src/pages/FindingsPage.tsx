import { useEffect, useRef, useState } from 'react'
import { useSearchParams } from 'react-router-dom'
import { RefreshCw, ScanSearch, TriangleAlert } from 'lucide-react'
import { Page } from '@/layouts/Page'
import { SourceTag } from '@/components/game/LibraryNotices'
import { ProviderStatusList } from '@/components/fix/FindingCard'
import { TechnicalFindingsBrowser } from '@/components/fix/TechnicalFindingsBrowser'
import { ActionableFindings } from '@/components/fix/ActionableFindings'
import { ApplyFixDialog, type ApplyTarget } from '@/components/fix/ApplyFixDialog'
import { RecipesPanel } from '@/components/fix/RecipesPanel'
import { OperationHistory } from '@/components/fix/OperationHistory'
import { SmartPacksPanel } from '@/components/smart/SmartPacksPanel'
import { useAssessments } from '@/components/fix/useAssessments'
import { Button } from '@/components/ui/Button'
import { Segmented, Select } from '@/components/ui/Controls'
import { EmptyState } from '@/components/ui/EmptyState'
import { Mascot } from '@/mascot/Mascot'
import { useLibrary } from '@/state/library'
import { useFixReports } from '@/state/fixes'
import { useFixEngine } from '@/state/fixEngine'

type View = 'disponiveis' | 'descobertas' | 'smart' | 'receitas' | 'historico'

/** Availability comes from fresh native planning for one selected game. */
export function FindingsPage() {
  const [params, setParams] = useSearchParams()
  const { games } = useLibrary()
  const fixes = useFixReports()
  const engine = useFixEngine()
  const [bulk, setBulk] = useState<{ done: number; total: number } | null>(null)
  const cancel = useRef(false)
  const [target, setTarget] = useState<ApplyTarget | null>(null)
  const [validationEpoch, setValidationEpoch] = useState(0)
  const steamGames = games.filter((g) => g.origin === 'steam')
  const requestedGame = params.get('game')
  const game = steamGames.find((g) => g.id === requestedGame) ?? steamGames[0]
  const appId = game?.id ?? null
  const requestedView = params.get('view') ?? (params.has('finding') ? 'descobertas' : null)
  const view: View = ['descobertas', 'smart', 'receitas', 'historico'].includes(requestedView ?? '') ? requestedView as View : 'disponiveis'
  const report = appId ? fixes.reports[appId] : undefined
  const analysis = useAssessments(view === 'disponiveis' ? appId : null)
  const attention = engine.operations.filter((o) => o.state === 'recovery-required' || o.state === 'requires-manual-step')
  const pending = steamGames.filter((g) => !fixes.reports[g.id])
  const update = (patch: Record<string, string | null>) => {
    const next = new URLSearchParams(params)
    for (const [key, value] of Object.entries(patch)) value ? next.set(key, value) : next.delete(key)
    setParams(next, { replace: true })
  }
  useEffect(() => () => void (cancel.current = true), [])
  const searchPending = async () => {
    cancel.current = false
    const queue = [...pending]
    setBulk({ done: 0, total: queue.length })
    try {
      for (let i = 0; i < queue.length && !cancel.current; i++) {
        await fixes.search(queue[i].id, queue[i].title)
        setBulk({ done: i + 1, total: queue.length })
      }
    } finally { setBulk(null) }
  }
  return <Page>
    <header className="flex items-end justify-between gap-6">
      <div>
        <div className="flex items-center gap-2"><p className="eyebrow">Fix Packs</p><SourceTag>{view === 'historico' ? 'Fix Engine' : view === 'smart' ? 'Smart Packs' : view === 'descobertas' ? 'Pesquisa técnica' : 'Planos verificados'}</SourceTag></div>
        <h1 className="mt-3 font-display text-[36px] leading-none font-bold tracking-[-0.04em]">Correções</h1>
        <p className="mt-2 max-w-xl text-[14px] text-fg-2">Soluções com plano completo para o jogo escolhido. Prévia, confirmação, backup e rollback fazem parte da aplicação.</p>
      </div>
      <Mascot pose={bulk ? 'inspect' : 'fix'} scale={3} className="-mb-1" />
    </header>
    {!!attention.length && <button onClick={() => update({ view: 'historico' })} className="mt-6 flex w-full items-center gap-3 rounded-md border border-caution/25 bg-caution-soft px-4 py-2.5 text-left text-[13px] text-caution"><TriangleAlert className="size-4 shrink-0" />{attention.length} operação(ões) precisa(m) da sua atenção. Abra o histórico para conferir recuperação ou pendências.</button>}
    <div className="mt-7 flex flex-wrap items-center gap-3 border-b border-line pb-4">
      <Segmented<View> label="Visão" size="sm" value={view} onChange={(v) => update({ view: v === 'disponiveis' ? null : v, finding: null })} options={[
        { value: 'disponiveis', label: 'Correções disponíveis' }, { value: 'smart', label: 'Smart Packs' },
        { value: 'descobertas', label: 'Pesquisa técnica' }, { value: 'receitas', label: 'Catálogo técnico' },
        { value: 'historico', label: 'Histórico' },
      ]} />
      {view !== 'historico' && view !== 'receitas' && <Select label="Jogo" value={view === 'descobertas' ? requestedGame ?? 'todos' : appId ?? ''} onChange={(id) => update({ game: id, finding: null, recipe: null, pack: null })} options={view === 'descobertas' ? [{ value: 'todos', label: 'Todos os jogos' }, ...Object.values(fixes.reports).map((r) => ({ value: r.appId, label: r.gameName }))] : steamGames.length ? steamGames.map((g) => ({ value: g.id, label: g.title })) : [{ value: '', label: 'Nenhum jogo da Steam' }]} />}
    </div>
    {view === 'historico' ? <OperationHistory onAct={setTarget} />
      : view === 'smart' ? <SmartPacksPanel key={validationEpoch} appId={appId} selectedPackId={params.get('pack')} onAct={setTarget} />
      : view === 'receitas' ? <RecipesPanel key={validationEpoch} games={games} appId={appId} recipeId={params.get('recipe')} onSelect={(p) => update({ ...(p.game !== undefined ? { game: p.game || null, recipe: null } : {}), ...(p.recipe ? { recipe: p.recipe } : {}) })} onAct={setTarget} onShowFinding={(finding, id) => update({ view: 'descobertas', game: id, finding, recipe: null })} />
      : view === 'descobertas' ? <TechnicalFindingsBrowser key={validationEpoch} gameFilter={requestedGame ?? 'todos'} selectedId={params.get('finding')} update={update} onAct={setTarget} />
      : !game ? <EmptyState pose="idle" title="Escolha um jogo instalado" body="A aplicabilidade depende do jogo, hardware e Proton identificados na biblioteca real." />
      : <div className="mt-5 space-y-5">
        <div className="flex flex-wrap gap-2">
          <Button size="sm" icon={<ScanSearch className="size-4" />} loading={!!fixes.searching[game.id]} disabled={!!fixes.searching[game.id]} onClick={() => void fixes.search(game.id, game.title, true)}>Pesquisar soluções para este jogo</Button>
          {bulk ? <Button size="sm" variant="outline" onClick={() => { cancel.current = true }} icon={<RefreshCw className="size-3.5 animate-spin" />}>Pesquisando {bulk.done}/{bulk.total} · parar</Button>
            : pending.length > 0 && <Button size="sm" variant="ghost" onClick={() => void searchPending()}>Pesquisar {pending.length} jogo(s) restante(s)</Button>}
        </div>
        {fixes.errors[game.id] && <p role="alert" className="text-[13px] text-bad">A pesquisa falhou: {fixes.errors[game.id]}</p>}
        {report && <ProviderStatusList providers={report.providers} />}
        <ActionableFindings key={game.id} appId={game.id} findings={report?.findings ?? []} assessments={analysis.byFinding} loading={analysis.loading} error={analysis.error} refresh={analysis.refresh} onApply={(a, f) => setTarget({ kind: 'apply', expectedFingerprint: a.availability?.planFingerprint ?? undefined, recipeId: a.recipeId!, appId: game.id, title: a.recipe?.title ?? f.title })} />
      </div>}
    <ApplyFixDialog target={target} onClose={() => { setTarget(null); setValidationEpoch((n) => n + 1); void engine.refresh(); analysis.refresh() }} />
  </Page>
}
