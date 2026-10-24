import { useEffect, useMemo, useRef } from 'react'
import { Link, useSearchParams } from 'react-router-dom'
import { AnimatePresence, motion } from 'motion/react'
import { ArchiveRestore, ArrowUpRight, Check, Cpu, Layers, RotateCcw, ShieldCheck, Wrench, X } from 'lucide-react'
import { Page } from '@/layouts/Page'
import { fixPacks, fixPackById } from '@/data/fixpacks'
import { games, gameById } from '@/data/games'
import { issueById } from '@/data/issues'
import { useAppState, useFixSummary } from '@/state/store'
import { useFixFlow } from '@/components/fix/useFixFlow'
import { ChangeList } from '@/components/fix/ChangeList'
import { SourceChip } from '@/components/fix/SourceIcon'
import { ConfidenceMeter, DemoTag, RiskBadge, SeverityMark } from '@/components/ui/Status'
import { Button } from '@/components/ui/Button'
import { Segmented, Select } from '@/components/ui/Controls'
import { EmptyState } from '@/components/ui/EmptyState'
import { GameArt } from '@/components/game/GameArt'
import { Mascot } from '@/mascot/Mascot'
import type { FixPack } from '@/types'
import { cn } from '@/lib/cn'
import { useLibrary } from '@/state/library'
import { FindingsPage } from './FindingsPage'

type StateFilter = 'todas' | 'disponivel' | 'aplicado'

export function FixPacksPage() {
  const { mode } = useLibrary()
  return mode === 'real' ? <FindingsPage /> : <DemoFixPacks />
}

function DemoFixPacks() {
  const [params, setParams] = useSearchParams()
  const { fixStates } = useAppState()
  const { available, applied } = useFixSummary()
  const flow = useFixFlow()

  const stateFilter = (params.get('estado') as StateFilter) ?? 'todas'
  const gameFilter = params.get('game') ?? 'todos'
  const update = (patch: Record<string, string | null>) => {
    const next = new URLSearchParams(params)
    for (const [k, v] of Object.entries(patch)) {
      if (v === null || v === 'todos' || v === 'todas') next.delete(k)
      else next.set(k, v)
    }
    setParams(next, { replace: true })
  }

  const list = useMemo(
    () =>
      fixPacks
        .filter((f) => (stateFilter === 'todas' ? true : fixStates[f.id] === stateFilter))
        .filter((f) => (gameFilter === 'todos' ? true : f.gameId === gameFilter))
        .sort((a, b) => Number(fixStates[a.id] === 'aplicado') - Number(fixStates[b.id] === 'aplicado') || b.confidence - a.confidence),
    [stateFilter, gameFilter, fixStates],
  )

  const selected = fixPackById(params.get('pack') ?? undefined) ?? list[0]
  const gamesWithFixes = games.filter((g) => g.fixPackIds.length)

  return (
    <Page>
      <header className="flex items-end justify-between gap-6">
        <div>
          <div className="flex items-center gap-2">
            <p className="eyebrow">Fix Packs</p>
            <DemoTag />
          </div>
          <h1 className="mt-3 font-display text-[36px] leading-none font-bold tracking-[-0.04em]">Correções</h1>
          <p className="mt-2 max-w-xl text-[14px] text-fg-2">
            Pacotes montados a partir de relatos da comunidade. Cada um mostra o que muda, de onde veio e como desfazer.
          </p>
        </div>
        <div className="flex items-end gap-4">
          <div className="text-right text-[13px]">
            <p>
              <span className="tnum font-mono text-fg">{available.length}</span> <span className="text-fg-2">disponíveis</span>
            </p>
            <p>
              <span className="tnum font-mono text-fg">{applied.length}</span> <span className="text-fg-2">aplicadas</span>
            </p>
          </div>
          <Mascot pose="fix" scale={3} className="-mb-1" />
        </div>
      </header>

      <div className="mt-7 flex items-center gap-3 border-b border-line pb-4">
        <Segmented<StateFilter>
          label="Filtrar por estado"
          size="sm"
          value={stateFilter}
          onChange={(v) => update({ estado: v, pack: null })}
          options={[
            { value: 'todas', label: 'Todas' },
            { value: 'disponivel', label: 'Disponíveis' },
            { value: 'aplicado', label: 'Aplicadas' },
          ]}
        />
        <Select
          label="Jogo"
          value={gameFilter}
          onChange={(v) => update({ game: v, pack: null })}
          options={[{ value: 'todos', label: 'Todos os jogos' }, ...gamesWithFixes.map((g) => ({ value: g.id, label: g.title }))]}
        />
        <span className="ml-auto text-[12.5px] text-fg-3">
          {list.length} pacote{list.length === 1 ? '' : 's'}
        </span>
      </div>

      {list.length === 0 ? (
        <EmptyState
          pose="sleep"
          title="Nada para consertar aqui"
          body="Nenhum pacote combina com estes filtros. O Pim aproveitou para tirar um cochilo."
          action={<Button onClick={() => update({ estado: null, game: null, pack: null })}>Ver todas</Button>}
        />
      ) : (
        <div className="mt-5 grid grid-cols-[minmax(280px,360px)_minmax(0,1fr)] gap-8">
          <ul className="space-y-1" aria-label="Pacotes de correção">
            {list.map((f) => (
              <PackListItem key={f.id} fix={f} active={selected?.id === f.id} applied={fixStates[f.id] === 'aplicado'} onSelect={() => update({ pack: f.id })} />
            ))}
          </ul>

          <AnimatePresence mode="wait" initial={false}>
            {selected && (
              <motion.div key={selected.id} initial={{ opacity: 0, x: 8 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -4 }} transition={{ duration: 0.18 }} className="min-w-0">
                <PackDetail fix={selected} applied={fixStates[selected.id] === 'aplicado'} onApply={() => flow.start(selected, 'apply')} onRevert={() => flow.start(selected, 'revert')} />
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      )}
      {flow.dialog}
    </Page>
  )
}

function PackListItem({ fix, active, applied, onSelect }: { fix: FixPack; active: boolean; applied: boolean; onSelect: () => void }) {
  const game = gameById(fix.gameId)
  const ref = useRef<HTMLLIElement>(null)
  useEffect(() => {
    if (active) ref.current?.scrollIntoView({ block: 'nearest' })
  }, [active])
  return (
    <li ref={ref}>
      <button
        onClick={onSelect}
        aria-current={active ? 'true' : undefined}
        className={cn(
          'group relative flex w-full items-center gap-3 rounded-md px-3 py-3 text-left transition-colors duration-(--frx-dur-fast)',
          active ? 'bg-surface' : 'hover:bg-surface/60',
        )}
      >
        <span className={cn('absolute top-3 bottom-3 left-0 w-[3px] transition-opacity', active ? 'bg-accent opacity-100' : 'opacity-0')} aria-hidden />
        <span className="block size-9 shrink-0 overflow-hidden rounded-[6px] border border-line">{game && <GameArt art={game.cover} variant="thumb" />}</span>
        <span className="min-w-0 flex-1">
          <span className={cn('block truncate text-[13.5px] font-medium', active ? 'text-fg' : 'text-fg group-hover:text-fg')}>{fix.name}</span>
          <span className="block truncate text-[12px] text-fg-3">{game?.title}</span>
        </span>
        {applied ? (
          <span className="inline-flex items-center gap-1 text-[11.5px] font-medium text-good">
            <Check className="size-3" strokeWidth={2.6} /> Aplicada
          </span>
        ) : (
          <ConfidenceMeter value={fix.confidence} showLabel={false} />
        )}
      </button>
    </li>
  )
}

function PackDetail({ fix, applied, onApply, onRevert }: { fix: FixPack; applied: boolean; onApply: () => void; onRevert: () => void }) {
  const game = gameById(fix.gameId)
  const solves = fix.solves.map(issueById).filter(Boolean)
  return (
    <article className="rounded-xl border border-line bg-surface" aria-labelledby="pack-title">
      <div className="flex items-start justify-between gap-6 border-b border-line px-7 py-6">
        <div className="min-w-0">
          <Link to={`/biblioteca/${fix.gameId}`} className="inline-flex items-center gap-1 text-[12.5px] text-fg-3 hover:text-accent">
            {game?.title} <ArrowUpRight className="size-3" />
          </Link>
          <h2 id="pack-title" className="mt-1 font-display text-[26px] leading-tight font-semibold tracking-[-0.03em]">
            {fix.name}
          </h2>
          <p className="mt-2 max-w-xl text-[14px] text-fg-2">{fix.summary}</p>
        </div>
        <div className="flex shrink-0 flex-col items-end gap-2">
          {applied ? (
            <>
              <span className="inline-flex h-6 items-center gap-1 rounded-xs bg-good-soft px-2 text-[12px] font-medium text-good">
                <Check className="size-3" strokeWidth={2.6} /> Aplicada (simulado)
              </span>
              <Button variant="outline" icon={<RotateCcw className="size-4" />} onClick={onRevert} disabled={!fix.reversible}>
                Reverter
              </Button>
            </>
          ) : (
            <Button variant="primary" size="lg" icon={<Wrench className="size-4" />} onClick={onApply}>
              Aplicar correção
            </Button>
          )}
        </div>
      </div>

      {/* Fatos essenciais */}
      <dl className="grid grid-cols-4 divide-x divide-line border-b border-line">
        <Fact label="Confiança">
          <ConfidenceMeter value={fix.confidence} />
        </Fact>
        <Fact label="Risco">
          <RiskBadge risk={fix.risk} />
        </Fact>
        <Fact label="Backup">
          <YesNo ok={fix.backup} yes="Automático" no="Não disponível" icon={<ArchiveRestore className="size-3.5" />} />
        </Fact>
        <Fact label="Reversão">
          <YesNo ok={fix.reversible} yes="Um clique" no="Manual" icon={<RotateCcw className="size-3.5" />} />
        </Fact>
      </dl>

      <div className="space-y-7 px-7 py-6">
        <section>
          <h3 className="eyebrow mb-3">Problema que resolve</h3>
          <ul className="space-y-2.5">
            {solves.map((i) => (
              <li key={i!.id} className="flex items-start justify-between gap-4">
                <div>
                  <p className="text-[14px] text-fg">{i!.title}</p>
                  <p className="text-[12.5px] text-fg-3">{i!.description}</p>
                </div>
                <SeverityMark severity={i!.severity} />
              </li>
            ))}
          </ul>
        </section>

        <section className="grid grid-cols-2 gap-6">
          <div>
            <h3 className="eyebrow mb-3 flex items-center gap-1.5">
              <Cpu className="size-3" /> Hardware compatível
            </h3>
            <div className="flex flex-wrap gap-1.5">
              {fix.hardware.map((h) => (
                <span key={h} className="rounded-xs border border-line bg-bg/40 px-2 py-0.5 text-[12.5px] text-fg-2">
                  {h}
                </span>
              ))}
            </div>
          </div>
          <div>
            <h3 className="eyebrow mb-3 flex items-center gap-1.5">
              <Layers className="size-3" /> Proton compatível
            </h3>
            <div className="flex flex-wrap gap-1.5">
              {fix.proton.map((p) => (
                <span key={p} className="rounded-xs border border-line bg-bg/40 px-2 py-0.5 text-[12.5px] text-fg-2">
                  {p}
                </span>
              ))}
            </div>
          </div>
        </section>

        <section>
          <h3 className="eyebrow mb-3 flex items-center gap-1.5">
            <ShieldCheck className="size-3" /> Origem da informação
          </h3>
          <div className="flex flex-wrap gap-2">
            {fix.sources.map((s, i) => (
              <SourceChip key={i} source={s} />
            ))}
          </div>
        </section>

        <section>
          <div className="mb-3 flex items-center justify-between">
            <h3 className="eyebrow">Arquivos e configurações alterados</h3>
            <span className="text-[12px] text-fg-3">Pré-visualização · nada é escrito em disco</span>
          </div>
          <ChangeList changes={fix.changes} />
        </section>

        {fix.risk === 'alto' && (
          <p className="flex items-start gap-2.5 rounded-md border border-bad/25 bg-bad-soft px-4 py-3 text-[13px] text-bad">
            <X className="mt-0.5 size-4 shrink-0" />
            Relatos divergentes. Aplique somente se precisar do recurso e compare o desempenho antes e depois.
          </p>
        )}
      </div>
    </article>
  )
}

function Fact({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="px-5 py-4 first:pl-7">
      <dt className="mb-2 text-[12px] text-fg-3">{label}</dt>
      <dd>{children}</dd>
    </div>
  )
}

function YesNo({ ok, yes, no, icon }: { ok: boolean; yes: string; no: string; icon: React.ReactNode }) {
  return <span className={cn('inline-flex items-center gap-1.5 text-[13px] font-medium', ok ? 'text-fg' : 'text-caution')}>{ok ? icon : '!'} {ok ? yes : no}</span>
}
