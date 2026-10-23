import { useMemo, useRef, useState, useEffect } from 'react'
import { useSearchParams } from 'react-router-dom'
import { AnimatePresence, motion } from 'motion/react'
import { LayoutGrid, List, Search, X } from 'lucide-react'
import { Page } from '@/layouts/Page'
import { games } from '@/data/games'
import { GameCard, GameRow } from '@/components/game/GameCard'
import { Segmented, Select, Kbd } from '@/components/ui/Controls'
import { STATUS_META, StatusDot } from '@/components/ui/Status'
import { EmptyState } from '@/components/ui/EmptyState'
import { Button } from '@/components/ui/Button'
import { Mascot } from '@/mascot/Mascot'
import { DemoTag } from '@/components/ui/Status'
import { useAppState } from '@/state/store'
import type { CompatStatus } from '@/types'
import { cn } from '@/lib/cn'

type Sort = 'recentes' | 'nome' | 'status' | 'tempo'
type View = 'grid' | 'list'
type StatusFilter = CompatStatus | 'todos'
type FixFilter = 'todos' | 'com-correcoes'
type RunnerFilter = 'todos' | 'Proton' | 'Wine' | 'Nativo'

const STATUS_RANK: Record<CompatStatus, number> = { problematico: 0, ajustes: 1, 'nao-testado': 2, jogavel: 3, perfeito: 4 }
const VIEW_KEY = 'frx.library.view'

export function LibraryPage() {
  const [params, setParams] = useSearchParams()
  const { fixStates } = useAppState()
  const [q, setQ] = useState(params.get('q') ?? '')
  const [status, setStatus] = useState<StatusFilter>((params.get('status') as StatusFilter) ?? 'todos')
  const [fixFilter, setFixFilter] = useState<FixFilter>('todos')
  const [runner, setRunner] = useState<RunnerFilter>('todos')
  const [sort, setSort] = useState<Sort>('recentes')
  const [view, setView] = useState<View>(() => {
    try {
      return (localStorage.getItem(VIEW_KEY) as View) || 'grid'
    } catch {
      return 'grid'
    }
  })
  const search = useRef<HTMLInputElement>(null)

  useEffect(() => {
    try {
      localStorage.setItem(VIEW_KEY, view)
    } catch {
      /* ignora */
    }
  }, [view])

  // "/" foca a busca, como em muitas ferramentas de desktop.
  useEffect(() => {
    const on = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement
      if (e.key === '/' && !/input|textarea|select/i.test(t.tagName)) {
        e.preventDefault()
        search.current?.focus()
      }
    }
    window.addEventListener('keydown', on)
    return () => window.removeEventListener('keydown', on)
  }, [])

  useEffect(() => {
    const next = new URLSearchParams()
    if (q) next.set('q', q)
    if (status !== 'todos') next.set('status', status)
    setParams(next, { replace: true })
  }, [q, status, setParams])

  const result = useMemo(() => {
    const s = q.trim().toLowerCase()
    const list = games.filter((g) => {
      if (s && !`${g.title} ${g.studio} ${g.genre} ${g.protonVersion}`.toLowerCase().includes(s)) return false
      if (status !== 'todos' && g.status !== status) return false
      if (runner !== 'todos' && g.runner !== runner) return false
      if (fixFilter === 'com-correcoes' && !g.fixPackIds.some((id) => fixStates[id] === 'disponivel')) return false
      return true
    })
    return list.sort((a, b) => {
      if (sort === 'nome') return a.title.localeCompare(b.title, 'pt-BR')
      if (sort === 'status') return STATUS_RANK[a.status] - STATUS_RANK[b.status]
      if (sort === 'tempo') return b.playtimeH - a.playtimeH
      return +new Date(b.lastPlayed) - +new Date(a.lastPlayed)
    })
  }, [q, status, runner, fixFilter, sort, fixStates])

  const counts = useMemo(() => {
    const c: Record<string, number> = { todos: games.length }
    for (const g of games) c[g.status] = (c[g.status] ?? 0) + 1
    return c
  }, [])

  const filtersActive = q || status !== 'todos' || runner !== 'todos' || fixFilter !== 'todos'
  const clear = () => {
    setQ('')
    setStatus('todos')
    setRunner('todos')
    setFixFilter('todos')
  }

  const statusChips: StatusFilter[] = ['todos', 'perfeito', 'jogavel', 'ajustes', 'problematico', 'nao-testado']

  return (
    <Page>
      <header className="relative flex items-end justify-between gap-6">
        <div>
          <div className="flex items-center gap-2">
            <p className="eyebrow">Biblioteca</p>
            <DemoTag />
          </div>
          <h1 className="mt-3 font-display text-[36px] leading-none font-bold tracking-[-0.04em]">Seus jogos</h1>
          <p className="mt-2 text-[14px] text-fg-2">
            {games.length} jogos de Steam, GOG, Epic e instalações locais (dados fictícios).
          </p>
        </div>
        <div className="flex items-end gap-6">
          {/* O Pim cochila na prateleira, de olho nos jogos */}
          <Shelf />
          <Segmented<View>
          label="Modo de visualização"
          value={view}
          onChange={setView}
          options={[
            { value: 'grid', label: <span className="sr-only">Grade</span>, icon: <LayoutGrid className="size-4" />, title: 'Grade' },
            { value: 'list', label: <span className="sr-only">Lista</span>, icon: <List className="size-4" />, title: 'Lista' },
          ]}
          />
        </div>
      </header>

      {/* Barra de busca e filtros */}
      <div className="sticky top-0 z-20 -mx-3 mt-6 border-b border-line bg-bg/92 px-3 pt-3 pb-3 backdrop-blur-md">
        <div className="flex items-center gap-3">
          <div className="relative flex-1">
            <Search className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-fg-3" aria-hidden />
            <input
              ref={search}
              value={q}
              onChange={(e) => setQ(e.target.value)}
              type="search"
              placeholder="Buscar por nome, estúdio, gênero ou versão do Proton"
              aria-label="Buscar jogos"
              className="h-10 w-full rounded-md border border-line-2 bg-surface pr-16 pl-9 text-[14px] text-fg placeholder:text-fg-3 transition-colors hover:border-line-3 focus:border-accent-line focus:outline-none [&::-webkit-search-cancel-button]:hidden"
            />
            <span className="absolute top-1/2 right-3 -translate-y-1/2">
              {q ? (
                <button onClick={() => setQ('')} aria-label="Limpar busca" className="grid size-5 place-items-center rounded-xs text-fg-3 hover:text-fg">
                  <X className="size-3.5" />
                </button>
              ) : (
                <Kbd>/</Kbd>
              )}
            </span>
          </div>
          <Select<RunnerFilter>
            label="Camada"
            value={runner}
            onChange={setRunner}
            options={[
              { value: 'todos', label: 'Todas' },
              { value: 'Proton', label: 'Proton' },
              { value: 'Wine', label: 'Wine' },
              { value: 'Nativo', label: 'Nativo' },
            ]}
          />
          <Select<Sort>
            label="Ordenar"
            value={sort}
            onChange={setSort}
            options={[
              { value: 'recentes', label: 'Jogados recentemente' },
              { value: 'nome', label: 'Nome (A–Z)' },
              { value: 'status', label: 'Precisa de atenção' },
              { value: 'tempo', label: 'Tempo de jogo' },
            ]}
          />
        </div>
        <div className="mt-3 flex flex-wrap items-center gap-1.5" role="group" aria-label="Filtrar por compatibilidade">
          {statusChips.map((s) => {
            const active = status === s
            return (
              <button
                key={s}
                aria-pressed={active}
                onClick={() => setStatus(s)}
                className={cn(
                  'inline-flex h-7 items-center gap-1.5 rounded-sm border px-2.5 text-[12.5px] font-medium transition-colors duration-(--frx-dur-fast)',
                  active ? 'border-accent-line bg-accent-soft text-fg' : 'border-line text-fg-2 hover:border-line-2 hover:text-fg',
                )}
              >
                {s !== 'todos' && <StatusDot status={s} />}
                {s === 'todos' ? 'Todos' : STATUS_META[s].label}
                <span className="tnum font-mono text-[11px] text-fg-3">{counts[s] ?? 0}</span>
              </button>
            )
          })}
          <span className="mx-1.5 h-4 w-px bg-line-2" aria-hidden />
          <button
            aria-pressed={fixFilter === 'com-correcoes'}
            onClick={() => setFixFilter((f) => (f === 'todos' ? 'com-correcoes' : 'todos'))}
            className={cn(
              'inline-flex h-7 items-center gap-1.5 rounded-sm border px-2.5 text-[12.5px] font-medium transition-colors',
              fixFilter === 'com-correcoes' ? 'border-accent-line bg-accent-soft text-fg' : 'border-line text-fg-2 hover:border-line-2 hover:text-fg',
            )}
          >
            Com correções disponíveis
          </button>
          {filtersActive && (
            <button onClick={clear} className="ml-auto text-[12.5px] text-fg-3 underline-offset-4 hover:text-fg hover:underline">
              Limpar filtros
            </button>
          )}
        </div>
      </div>

      <p className="sr-only" aria-live="polite">
        {result.length} jogos encontrados
      </p>

      <div className="mt-6">
        {result.length === 0 ? (
          <EmptyState
            pose="confused"
            title="Nenhum jogo por aqui"
            body={q ? <>O Pim procurou “{q}” na estante inteira e voltou de mãos vazias.</> : 'Nenhum jogo combina com esses filtros.'}
            action={
              <Button variant="secondary" onClick={clear}>
                Limpar filtros
              </Button>
            }
          />
        ) : view === 'grid' ? (
          <motion.ul layout className="grid grid-cols-[repeat(auto-fill,minmax(176px,1fr))] gap-x-5 gap-y-7">
            <AnimatePresence initial={false} mode="popLayout">
              {result.map((g) => (
                <motion.li key={g.id} layout initial={{ opacity: 0, scale: 0.97 }} animate={{ opacity: 1, scale: 1 }} exit={{ opacity: 0, scale: 0.97 }} transition={{ duration: 0.18 }}>
                  <GameCard game={g} />
                </motion.li>
              ))}
            </AnimatePresence>
          </motion.ul>
        ) : (
          <div>
            <div className="grid grid-cols-[48px_minmax(0,2.2fr)_minmax(0,1.3fr)_minmax(0,1.1fr)_minmax(0,1fr)_90px] gap-5 border-b border-line px-3 pb-2.5 text-[11.5px] font-medium text-fg-3">
              <span />
              <span>Jogo</span>
              <span>Proton / Wine</span>
              <span>Compatibilidade</span>
              <span>Correções</span>
              <span className="text-right">Jogado</span>
            </div>
            <ul className="mt-1">
              {result.map((g) => (
                <li key={g.id}>
                  <GameRow game={g} />
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </Page>
  )
}

function Shelf() {
  return (
    <div className="pointer-events-none hidden flex-col items-center lg:flex" aria-hidden={false}>
      <Mascot pose="rest" scale={3} className="-mb-[3px]" />
      <svg width={84} height={8} viewBox="0 0 21 2" className="pixelated" aria-hidden>
        <rect x={0} y={0} width={21} height={1} style={{ fill: 'var(--frx-line-3)' }} />
        <rect x={3} y={1} width={1} height={1} style={{ fill: 'var(--frx-line-2)' }} />
        <rect x={17} y={1} width={1} height={1} style={{ fill: 'var(--frx-line-2)' }} />
      </svg>
    </div>
  )
}
