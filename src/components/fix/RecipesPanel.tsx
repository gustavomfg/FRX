import { useEffect, useMemo, useState } from 'react'
import { AnimatePresence, motion } from 'motion/react'
import { ExternalLink, RotateCcw, ShieldCheck, Wrench } from 'lucide-react'
import { Button } from '@/components/ui/Button'
import { Select } from '@/components/ui/Controls'
import { EmptyState } from '@/components/ui/EmptyState'
import { Pill } from './FindingCard'
import { ChecksList, type ApplyTarget } from './ApplyFixDialog'
import { PLAN_STATUS, PURPOSE, RISK, TRUST } from './engineMeta'
import { checkFixCompatibility, listFixRecipes } from '@/services/fixEngine'
import { errorMessage } from '@/services/native'
import { openExternal } from '@/services/opener'
import { useFixEngine } from '@/state/fixEngine'
import type { Game } from '@/types'
import type { FixPlan, RecipeAction, RecipeView } from '@/types/fixEngine'
import { cn } from '@/lib/cn'

export function describeAction(a: RecipeAction) {
  switch (a.type) {
    case 'set_launch_env':
      return { label: 'Variável nas opções de inicialização', value: `${a.key}=${a.value}` }
    case 'add_launch_args':
      return { label: 'Argumentos do jogo', value: a.args.join(' ') }
    case 'write_game_file':
      return { label: 'Arquivo de configuração do jogo', value: a.path }
  }
}

/** Receitas do catálogo local + compatibilidade calculada pelo núcleo para o jogo escolhido. */
export function useRecipes(appId: string | null, withChecks = true) {
  const engine = useFixEngine()
  const [recipes, setRecipes] = useState<RecipeView[]>([])
  const [checks, setChecks] = useState<Record<string, FixPlan | string>>({})
  const [loading, setLoading] = useState(false)
  // Recalcula quando o histórico muda (aplicação, rollback, verificação).
  const stamp = engine.operations.map((o) => o.id + o.state).join()

  useEffect(() => {
    if (!engine.available) return
    let gone = false
    setLoading(true)
    void (async () => {
      try {
        const list = await listFixRecipes(appId)
        if (gone) return
        setRecipes(list)
        if (!appId || !withChecks) return setChecks({})
        const results = await Promise.allSettled(list.map((r) => checkFixCompatibility(r.id, appId)))
        if (gone) return
        setChecks(Object.fromEntries(list.map((r, i) => [r.id, results[i].status === 'fulfilled' ? results[i].value : errorMessage(results[i].reason)])))
      } catch {
        if (!gone) setRecipes([])
      } finally {
        if (!gone) setLoading(false)
      }
    })()
    return () => void (gone = true)
  }, [engine.available, appId, stamp, withChecks])

  return { recipes, checks, loading }
}

export function RecipesPanel({
  games,
  appId,
  recipeId,
  onSelect,
  onAct,
  onShowFinding,
}: {
  games: Game[]
  appId: string | null
  recipeId: string | null
  onSelect: (patch: { game?: string; recipe?: string }) => void
  onAct: (t: ApplyTarget) => void
  onShowFinding: (findingId: string, appId: string) => void
}) {
  const engine = useFixEngine()
  const { recipes, checks, loading } = useRecipes(appId)
  const steamGames = games.filter((g) => g.origin === 'steam')
  // Ordem estável (revisadas primeiro): a lista não pula quando o estado muda.
  const ordered = useMemo(() => [...recipes].sort((a, b) => Number(b.executable) - Number(a.executable)), [recipes])
  const selected = ordered.find((r) => r.id === recipeId) ?? ordered[0]
  const game = steamGames.find((g) => g.id === appId)

  return (
    <div className="mt-5">
      <div className="mb-4 flex items-center gap-3">
        <Select
          label="Jogo"
          value={appId ?? ''}
          onChange={(v) => onSelect({ game: v, recipe: undefined })}
          options={[{ value: '', label: 'Escolha um jogo' }, ...steamGames.map((g) => ({ value: g.id, label: g.title }))]}
        />
        <p className="text-[12.5px] text-fg-3">
          Catálogo local {engine.status?.catalogVersion ?? '…'} · {engine.status?.reviewed ?? 0} receita{engine.status?.reviewed === 1 ? '' : 's'} revisada{engine.status?.reviewed === 1 ? '' : 's'} de {engine.status?.recipes ?? 0}
        </p>
      </div>
      {engine.status && engine.status.issues.length > 0 && (
        <p className="mb-4 rounded-md border border-bad/25 bg-bad-soft px-3.5 py-2.5 text-[12.5px] text-bad">Receitas recusadas pela verificação de integridade: {engine.status.issues.join(' · ')}</p>
      )}
      {!appId ? (
        <EmptyState pose="idle" title="Escolha um jogo" body="O FRX confere cada receita contra o estado real do jogo e da Steam: Proton em uso, opções de inicialização atuais, Steam aberta ou fechada." />
      ) : (
        <div className="grid grid-cols-[minmax(280px,360px)_minmax(0,1fr)] gap-8">
          <ul className="space-y-1" aria-label="Receitas" aria-busy={loading}>
            {ordered.map((r) => {
              const c = checks[r.id]
              const st = typeof c === 'object' ? c.status : null
              const meta = st ? PLAN_STATUS[st] : null
              const active = selected?.id === r.id
              return (
                <li key={r.id}>
                  <button
                    onClick={() => onSelect({ recipe: r.id })}
                    aria-current={active ? 'true' : undefined}
                    className={cn('group relative flex w-full items-start gap-3 rounded-md px-3 py-3 text-left transition-colors duration-(--frx-dur-fast)', active ? 'bg-surface' : 'hover:bg-surface/60')}
                  >
                    <span className={cn('absolute top-3 bottom-3 left-0 w-[3px] transition-opacity', active ? 'bg-accent opacity-100' : 'opacity-0')} aria-hidden />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-[13.5px] font-medium text-fg">{r.title}</span>
                      <span className="block truncate text-[12px] text-fg-3">
                        {TRUST[r.trust].label} · {PURPOSE[r.purpose]}
                        {r.relatedFindings.length > 0 && ` · citada em ${r.relatedFindings.length} descoberta${r.relatedFindings.length === 1 ? '' : 's'}`}
                      </span>
                    </span>
                    {meta ? (
                      <Pill color={meta.color} soft={meta.soft}>
                        {st === 'not-executable' ? TRUST[r.trust].label : meta.label}
                      </Pill>
                    ) : (
                      <span className="h-6 w-16 rounded-xs skeleton" aria-hidden />
                    )}
                  </button>
                </li>
              )
            })}
          </ul>
          <AnimatePresence mode="wait" initial={false}>
            {selected && game && (
              <motion.div key={selected.id + appId} initial={{ opacity: 0, x: 8 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -4 }} transition={{ duration: 0.18 }} className="min-w-0">
                <RecipeDetail recipe={selected} check={checks[selected.id]} game={game} onAct={onAct} onShowFinding={onShowFinding} />
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      )}
    </div>
  )
}

function RecipeDetail({ recipe, check, game, onAct, onShowFinding }: { recipe: RecipeView; check: FixPlan | string | undefined; game: Game; onAct: (t: ApplyTarget) => void; onShowFinding: (id: string, appId: string) => void }) {
  const engine = useFixEngine()
  const plan = typeof check === 'object' ? check : null
  const applied = engine.applied.find((a) => a.appId === game.id && a.recipeId === recipe.id)
  const pending = engine.operations.find((o) => o.appId === game.id && o.recipeId === recipe.id && (o.state === 'requires-manual-step' || o.state === 'recovery-required'))
  const canApply = recipe.executable && (plan?.status === 'ready' || plan?.status === 'requires-manual-step') && !pending
  return (
    <article className="space-y-5 rounded-xl border border-line bg-surface p-6">
      <header>
        <div className="flex flex-wrap items-center gap-2">
          <Pill color={TRUST[recipe.trust].color} soft={TRUST[recipe.trust].soft} title={TRUST[recipe.trust].help}>
            {recipe.trust === 'reviewed' && <ShieldCheck className="size-3" aria-hidden />}
            {TRUST[recipe.trust].label}
          </Pill>
          {plan && (
            <Pill color={PLAN_STATUS[plan.status].color} soft={PLAN_STATUS[plan.status].soft}>
              {PLAN_STATUS[plan.status].label}
            </Pill>
          )}
          <Pill color={RISK[recipe.risk].color} soft={RISK[recipe.risk].soft}>
            {RISK[recipe.risk].label}
          </Pill>
          <span className="text-[12px] text-fg-3">
            {PURPOSE[recipe.purpose]} · v{recipe.version}
          </span>
        </div>
        <h2 className="mt-3 font-display text-[22px] font-semibold tracking-[-0.02em]">{recipe.title}</h2>
        <p className="mt-1.5 text-[13.5px] text-fg-2">{recipe.summary}</p>
      </header>

      <p className="text-[13px] text-fg-2">{recipe.description}</p>

      <div>
        <div className="eyebrow mb-2.5">O que a receita altera</div>
        <ul className="divide-y divide-line overflow-hidden rounded-md border border-line bg-bg/50">
          {recipe.actions.map((a, i) => {
            const d = describeAction(a)
            return (
              <li key={i} className="flex items-center gap-2.5 px-3.5 py-2.5">
                <Wrench className="size-3.5 shrink-0 text-fg-3" aria-hidden />
                <span className="min-w-0 flex-1 text-[13px] text-fg">{d.label}</span>
                <code className="max-w-[55%] truncate font-mono text-[12px] text-fg-2">{d.value}</code>
              </li>
            )
          })}
        </ul>
        <p className="mt-2 text-[12px] text-fg-3">Só as opções de inicialização deste jogo mudam. Prefixo, arquivos do jogo e outras configurações ficam intactos.</p>
      </div>

      {(recipe.effects.length > 0 || recipe.sideEffects.length > 0) && (
        <dl className="grid grid-cols-2 gap-5 text-[12.5px]">
          <div>
            <dt className="eyebrow mb-1.5">Efeito esperado</dt>
            {recipe.effects.map((e) => (
              <dd key={e} className="text-fg-2">
                {e}
              </dd>
            ))}
          </div>
          <div>
            <dt className="eyebrow mb-1.5">Efeitos colaterais</dt>
            {recipe.sideEffects.map((e) => (
              <dd key={e} className="text-fg-2">
                {e}
              </dd>
            ))}
          </div>
        </dl>
      )}

      {plan ? <ChecksList checks={plan.checks} /> : typeof check === 'string' ? <p className="text-[12.5px] text-bad">{check}</p> : <div className="h-24 rounded-md skeleton" />}

      {recipe.relatedFindings.length > 0 && (
        <p className="text-[12.5px] text-fg-2">
          Também aparece em{' '}
          {recipe.relatedFindings.map((id, i) => (
            <span key={id}>
              {i > 0 && ', '}
              <button className="text-fg underline decoration-line-3 underline-offset-2 hover:text-accent" onClick={() => onShowFinding(id, game.id)}>
                descoberta {i + 1}
              </button>
            </span>
          ))}{' '}
          da pesquisa — o FRX não executa a descoberta, só esta receita revisada.
        </p>
      )}

      <div>
        <div className="eyebrow mb-2">Fontes e revisão</div>
        <ul className="space-y-1.5 text-[12.5px]">
          {recipe.sources.map((s) => (
            <li key={s.url} className="flex items-center gap-2">
              <span className="min-w-0 flex-1 truncate text-fg-2">{s.label}</span>
              <button onClick={() => void openExternal(s.url)} className="inline-flex shrink-0 items-center gap-1 text-[12px] font-medium text-fg-2 hover:text-accent" aria-label={`Abrir fonte: ${s.label}`}>
                Abrir <ExternalLink className="size-3" />
              </button>
            </li>
          ))}
          {recipe.review ? (
            <li className="text-fg-3">
              Revisada por {recipe.review.by} em {recipe.review.at}. {recipe.review.notes}
            </li>
          ) : (
            <li className="text-caution">Sem revisão registrada — não pode ser aplicada.</li>
          )}
        </ul>
      </div>

      <div className="flex items-center justify-between gap-4 border-t border-line pt-4">
        <span className="text-[12px] text-fg-3">{applied ? `Aplicada ${applied.mode === 'manual' ? 'manualmente' : 'pelo FRX'} · ${applied.effective === 'active' ? 'conferida agora no disco' : applied.effective === 'drifted' ? 'alterada fora do FRX' : 'estado desconhecido'}` : pending ? 'Há uma operação aguardando conferência no histórico.' : (plan?.reasons[0] ?? '')}</span>
        <div className="flex shrink-0 gap-2">
          {applied && (
            <Button
              size="sm"
              variant="outline"
              icon={<RotateCcw className="size-3.5" />}
              onClick={() => {
                const op = engine.operations.find((o) => o.id === applied.operationId)
                if (op) onAct({ kind: 'rollback', operation: op })
              }}
            >
              Desfazer
            </Button>
          )}
          {pending && (
            <Button size="sm" variant="outline" onClick={() => onAct({ kind: 'rollback', operation: pending })}>
              Abrir operação
            </Button>
          )}
          <Button size="sm" variant="primary" disabled={!canApply} onClick={() => onAct({ kind: 'apply', recipeId: recipe.id, appId: game.id, title: recipe.title })}>
            Revisar e aplicar
          </Button>
        </div>
      </div>
    </article>
  )
}
