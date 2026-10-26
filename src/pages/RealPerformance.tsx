import { useEffect, useMemo, useState } from 'react'
import { useSearchParams } from 'react-router-dom'
import { AnimatePresence, motion } from 'motion/react'
import { AlertTriangle, Check, FileUp, Play, RefreshCw, X } from 'lucide-react'
import { Page } from '@/layouts/Page'
import { Button } from '@/components/ui/Button'
import { Select } from '@/components/ui/Controls'
import { EmptyState } from '@/components/ui/EmptyState'
import { SourceTag } from '@/components/game/LibraryNotices'
import { CaptureFlow, CaptureStrip } from '@/components/performance/CaptureFlow'
import { ComparisonView } from '@/components/performance/ComparisonView'
import { DeleteSessionDialog, HistoryTable } from '@/components/performance/HistoryTable'
import { EditSessionDialog, ImportDialog, toConditions } from '@/components/performance/SessionForm'
import { ResourcesGrid, SessionCharts, SessionHero, StabilityPanel } from '@/components/performance/SessionView'
import { Mascot } from '@/mascot/Mascot'
import { SessionPackResult } from '@/components/smart/SessionPackResult'
import { useLibrary } from '@/state/library'
import { usePerformance } from '@/state/performance'
import { useToast } from '@/state/store'
import { compareSessions, exportSession, pickLogFile } from '@/services/performance'
import { errorMessage } from '@/services/native'
import type { ComparisonResult, MangoHudStatus, Session, SessionMeta } from '@/types/performance'
import { cn } from '@/lib/cn'
import { fmtNum } from '@/lib/format'

/**
 * Performance Lab com dados reais do MangoHud. Mesmo layout da versão
 * demonstrativa: cabeçalho, herói, gráficos, recursos e histórico.
 */
export function RealPerformanceLab() {
  const [params, setParams] = useSearchParams()
  const { games } = useLibrary()
  const perf = usePerformance()
  const toast = useToast()
  const gameFilter = params.get('game') ?? 'todos'
  const [captureOpen, setCaptureOpen] = useState(false)
  const [importPath, setImportPath] = useState<string | null>(null)
  const [editing, setEditing] = useState<SessionMeta | null>(null)
  const [deleting, setDeleting] = useState<SessionMeta | null>(null)

  const set = (patch: Record<string, string | null>) => {
    const next = new URLSearchParams(params)
    for (const [k, v] of Object.entries(patch)) {
      if (v == null || v === 'todos') next.delete(k)
      else next.set(k, v)
    }
    setParams(next, { replace: true })
  }

  const list = useMemo(
    () => perf.sessions.filter((s) => (gameFilter === 'todos' ? true : gameFilter === 'sem-jogo' ? !s.appId : s.appId === gameFilter)),
    [perf.sessions, gameFilter],
  )
  const beforeId = params.get('antes')
  const afterId = params.get('depois')
  const viewId = params.get('sessao') ?? list[0]?.id ?? null
  const comparing = !!(beforeId && afterId)

  // Sessões completas (com séries) sob demanda.
  const [loaded, setLoaded] = useState<Record<string, Session>>({})
  const [loadError, setLoadError] = useState<string | null>(null)
  useEffect(() => {
    const ids = comparing ? [beforeId!, afterId!] : viewId ? [viewId] : []
    let alive = true
    setLoadError(null)
    for (const id of ids) {
      perf
        .getSession(id)
        .then((s) => alive && setLoaded((m) => ({ ...m, [id]: s })))
        .catch((e) => alive && setLoadError(errorMessage(e)))
    }
    return () => {
      alive = false
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [viewId, beforeId, afterId, comparing, perf.sessions])

  const [comparison, setComparison] = useState<ComparisonResult | null>(null)
  useEffect(() => {
    setComparison(null)
    if (!comparing) return
    let alive = true
    compareSessions(beforeId!, afterId!)
      .then((r) => alive && setComparison(r))
      .catch((e) => alive && toast({ tone: 'error', title: 'Não foi possível comparar', body: errorMessage(e) }))
    return () => {
      alive = false
    }
  }, [comparing, beforeId, afterId, toast, perf.sessions])

  const doImport = async () => {
    try {
      const p = await pickLogFile()
      if (p) setImportPath(p)
    } catch (e) {
      toast({ tone: 'error', title: 'Não foi possível abrir o seletor de arquivos', body: errorMessage(e) })
    }
  }

  const defaultGame = gameFilter !== 'todos' && gameFilter !== 'sem-jogo' ? gameFilter : (games[0]?.id ?? '')
  const view = viewId ? loaded[viewId] : undefined
  const cap = perf.capture
  const sameGame = comparing && perf.sessions.find((s) => s.id === beforeId)?.appId !== perf.sessions.find((s) => s.id === afterId)?.appId

  return (
    <Page>
      <header className="flex flex-wrap items-end justify-between gap-6">
        <div>
          <div className="flex items-center gap-2">
            <p className="eyebrow">Performance Lab</p>
            <SourceTag>MangoHud</SourceTag>
          </div>
          <h1 className="mt-3 font-display text-[36px] leading-none font-bold tracking-[-0.04em]">Desempenho</h1>
          <p className="mt-2 max-w-xl text-[14px] text-fg-2">Registre benchmarks reais com o MangoHud, compare sessões e veja se uma mudança melhorou ou piorou o jogo.</p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Select
            label="Jogo"
            hideLabel
            value={gameFilter}
            onChange={(v) => set({ game: v, sessao: null, antes: null, depois: null })}
            options={[{ value: 'todos', label: 'Todos os jogos' }, ...games.map((g) => ({ value: g.id, label: g.title })), { value: 'sem-jogo', label: 'Sem jogo associado' }]}
          />
          <Button variant="outline" icon={<FileUp className="size-4" />} onClick={doImport}>
            Importar benchmark
          </Button>
          <Button variant="primary" icon={<Play className="size-4" />} onClick={() => setCaptureOpen(true)} disabled={!games.length}>
            Nova sessão
          </Button>
        </div>
      </header>

      <MangoHudLine status={perf.mangohud} onRecheck={() => void perf.checkMangoHud()} />

      {cap && cap.state !== 'idle' && !captureOpen && (
        <div className="mt-5">
          <CaptureStrip status={cap} onOpen={() => setCaptureOpen(true)} />
        </div>
      )}

      {perf.error && (
        <p className="mt-5 flex items-center gap-2.5 rounded-md border border-bad/25 bg-bad-soft px-4 py-2.5 text-[13px] text-bad">
          <AlertTriangle className="size-4" /> Não foi possível ler o histórico: {perf.error}
        </p>
      )}

      <div className="mt-5">
        <AnimatePresence mode="wait" initial={false}>
          {perf.loading && !perf.sessions.length ? (
            <motion.div key="loading" className="skeleton h-[320px] rounded-xl" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} />
          ) : !list.length ? (
            <motion.div key="empty" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} className="rounded-xl border border-dashed border-line-2">
              <EmptyState
                pose="idle"
                title="Nenhum benchmark ainda"
                body="Importe um log do MangoHud que você já tenha, ou prepare uma sessão: o FRX configura o MangoHud só para o jogo e analisa o resultado."
                action={
                  <div className="flex gap-2">
                    <Button variant="outline" icon={<FileUp className="size-4" />} onClick={doImport}>
                      Importar benchmark
                    </Button>
                    <Button variant="primary" icon={<Play className="size-4" />} onClick={() => setCaptureOpen(true)} disabled={!games.length}>
                      Preparar sessão
                    </Button>
                  </div>
                }
                className="py-12"
              />
            </motion.div>
          ) : comparing ? (
            <motion.div key={`cmp-${beforeId}-${afterId}`} initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }}>
              {sameGame && (
                <p className="mb-4 rounded-md border border-caution/25 bg-caution-soft px-4 py-2.5 text-[13px] text-caution">
                  As sessões são de jogos diferentes (ou uma não tem jogo associado). A comparação aparece, mas não diz nada sobre uma mudança no jogo.
                </p>
              )}
              {comparison && loaded[beforeId!] && loaded[afterId!] ? (
                <ComparisonView result={comparison} before={loaded[beforeId!]} after={loaded[afterId!]} />
              ) : (
                <div className="flex h-[320px] items-center justify-center gap-4 rounded-xl border border-line bg-surface">
                  <Mascot pose="analyze" scale={4} />
                  <p className="text-[14px] text-fg-2">Comparando as sessões…</p>
                </div>
              )}
            </motion.div>
          ) : view ? (
            <motion.div key={`view-${view.meta.id}`} initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }} className="space-y-10">
              <SessionHero meta={view.meta} />
              <SessionPackResult meta={view.meta} />
              <SessionCharts session={view} />
              <section aria-labelledby="recursos-reais">
                <div className="mb-4 flex items-end justify-between">
                  <h2 id="recursos-reais" className="font-display text-[20px] font-semibold tracking-[-0.02em]">
                    Recursos do sistema
                  </h2>
                  <span className="text-[12.5px] text-fg-3">Média da sessão · campos sem dado ficam vazios, nunca zerados</span>
                </div>
                <ResourcesGrid meta={view.meta} />
              </section>
              <StabilityPanel meta={view.meta} />
            </motion.div>
          ) : (
            <motion.div key="loading-view" className="flex h-[320px] items-center justify-center gap-4 rounded-xl border border-line bg-surface" initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
              <Mascot pose="analyze" scale={4} />
              <p className="text-[14px] text-fg-2">{loadError ?? 'Carregando a sessão…'}</p>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {list.length > 0 && (
        <section className="mt-10" aria-labelledby="historico-real">
          <div className="mb-4 flex items-end justify-between gap-4">
            <div>
              <h2 id="historico-real" className="font-display text-[20px] font-semibold tracking-[-0.02em]">
                Histórico de medições
              </h2>
              <p className="mt-1 text-[12.5px] text-fg-3">Clique numa sessão para ver os detalhes. Marque “Antes” e “Depois” para comparar duas sessões.</p>
            </div>
            {comparing && (
              <Button size="sm" variant="ghost" icon={<X className="size-3.5" />} onClick={() => set({ antes: null, depois: null })}>
                Sair da comparação
              </Button>
            )}
          </div>
          <HistoryTable
            sessions={list}
            viewId={comparing ? null : viewId}
            beforeId={beforeId}
            afterId={afterId}
            onView={(id) => set({ sessao: id, antes: null, depois: null })}
            onBefore={(id) => set({ antes: id })}
            onAfter={(id) => set({ depois: id })}
            onEdit={setEditing}
            onDelete={setDeleting}
            onExport={async (m) => {
              try {
                const json = await exportSession(m, 'json')
                if (json) toast({ tone: 'ok', title: 'Sessão exportada', body: 'Arquivo salvo onde você escolheu.' })
              } catch (e) {
                toast({ tone: 'error', title: 'Não foi possível exportar', body: errorMessage(e) })
              }
            }}
          />
          {list.some((m) => m.stats.fps.lowsEstimated) && <p className="mt-2 text-[11.5px] text-fg-3">* 1% low estimado a partir de amostras periódicas.</p>}
        </section>
      )}

      <CaptureFlow open={captureOpen} onClose={() => setCaptureOpen(false)} games={games} defaultAppId={defaultGame} onViewSession={(id) => set({ sessao: id, antes: null, depois: null })} />

      <ImportDialog
        open={!!importPath}
        path={importPath}
        games={games}
        defaultAppId={gameFilter !== 'todos' && gameFilter !== 'sem-jogo' ? gameFilter : ''}
        stage={perf.importStage}
        onClose={() => setImportPath(null)}
        onSubmit={async (v) => {
          const game = games.find((g) => g.id === v.appId)
          const meta = await perf.importLog(importPath!, {
            appId: v.appId || null,
            gameName: game?.title ?? null,
            label: v.label || null,
            notes: v.notes || null,
            conditions: toConditions(v),
          })
          if (meta) {
            setImportPath(null)
            toast({ tone: 'ok', title: 'Benchmark importado', body: `${meta.label} · ${meta.stats.fps.avg != null ? `${fmtNum(meta.stats.fps.avg, 1)} FPS médio` : 'sem FPS'}` })
            set({ game: meta.appId ?? 'sem-jogo', sessao: meta.id, antes: null, depois: null })
          }
        }}
      />

      <EditSessionDialog
        meta={editing}
        games={games}
        onClose={() => setEditing(null)}
        onSave={async (v) => {
          if (!editing) return
          const game = games.find((g) => g.id === v.appId)
          const ok = await perf.update(editing.id, {
            label: v.label,
            notes: v.notes,
            appId: v.appId,
            gameName: game?.title ?? '',
            conditions: toConditions(v, editing.conditions),
          })
          if (ok) {
            setLoaded(({ [editing.id]: _, ...rest }) => rest)
            setEditing(null)
          }
        }}
      />

      <DeleteSessionDialog
        meta={deleting}
        onClose={() => setDeleting(null)}
        onConfirm={async () => {
          if (!deleting) return
          const id = deleting.id
          if (await perf.remove(id)) {
            setDeleting(null)
            set({ sessao: viewId === id ? null : viewId, antes: beforeId === id ? null : beforeId, depois: afterId === id ? null : afterId })
            toast({ tone: 'info', title: 'Sessão excluída' })
          }
        }}
      />
    </Page>
  )
}

/** Situação do MangoHud em uma linha — e instruções quando falta algo. */
function MangoHudLine({ status, onRecheck }: { status: MangoHudStatus | null; onRecheck: () => void }) {
  if (!status) return <div className="mt-6 h-7" aria-hidden />
  const ok = status.loggingSupported
  const chips = [
    { on: ok, label: ok ? `MangoHud ${status.version ?? ''}`.trim() : 'MangoHud não encontrado' },
    { on: status.has64bit, label: '64 bits' },
    { on: status.has32bit, label: '32 bits' },
    { on: status.layers.length > 0, label: 'Camada Vulkan' },
    ...(status.flatpakSteam ? [{ on: status.flatpakLayer.length > 0, label: 'Extensão Flatpak' }] : []),
  ]
  const problems = status.notes.filter((n) => !n.startsWith('O mangohudctl'))
  return (
    <div className="mt-6 space-y-3">
      <div className="flex flex-wrap items-center gap-2 text-[12.5px]">
        <span className="text-fg-3">MangoHud:</span>
        {chips.map((c) => (
          <span key={c.label} className={cn('inline-flex h-7 items-center gap-1.5 rounded-sm border px-2.5 font-medium', c.on ? 'border-line text-fg-2' : 'border-caution/30 text-caution')}>
            {c.on ? <Check className="size-3 text-good" strokeWidth={3} /> : <X className="size-3" strokeWidth={3} />}
            {c.label}
          </span>
        ))}
        <button onClick={onRecheck} className="ml-1 inline-flex items-center gap-1 text-fg-3 hover:text-fg" aria-label="Verificar o MangoHud de novo">
          <RefreshCw className="size-3" /> verificar
        </button>
      </div>
      {(problems.length > 0 || status.installHint) && (
        <div className="rounded-md border border-caution/25 bg-caution-soft px-4 py-3 text-[13px] text-caution">
          {problems.map((p) => (
            <p key={p}>{p}</p>
          ))}
          {status.installHint && (
            <div className="mt-2 text-fg-2">
              <p className="text-[12.5px]">Para {status.installHint.distro}, no terminal (o FRX não instala nada sozinho):</p>
              {status.installHint.commands.map((c) => (
                <code key={c} className="mt-1 block rounded-sm bg-bg px-2.5 py-1.5 font-mono text-[12px] text-fg select-all">
                  {c}
                </code>
              ))}
            </div>
          )}
          <p className="mt-2 text-[12px] text-fg-3">Importar logs existentes funciona mesmo sem o MangoHud instalado.</p>
        </div>
      )}
    </div>
  )
}
