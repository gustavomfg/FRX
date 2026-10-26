import { useCallback, useEffect, useMemo, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { AnimatePresence, motion } from 'motion/react'
import { ArrowRightLeft, ExternalLink, FlaskConical, Gauge, Info, Layers, RefreshCw, RotateCcw, Trophy, Wrench } from 'lucide-react'
import { Button } from '@/components/ui/Button'
import { Dialog } from '@/components/ui/Dialog'
import { Segmented, Select } from '@/components/ui/Controls'
import { EmptyState } from '@/components/ui/EmptyState'
import { Pill } from '@/components/fix/FindingCard'
import { ApplyFixDialog, type ApplyTarget } from '@/components/fix/ApplyFixDialog'
import { RISK } from '@/components/fix/engineMeta'
import { Mascot } from '@/mascot/Mascot'
import { compareSmartPacks, declareBenchmarkPack, getSmartPacks, planSmartPackSwitch } from '@/services/smartPacks'
import { errorMessage } from '@/services/native'
import { openExternal } from '@/services/opener'
import { useFixEngine } from '@/state/fixEngine'
import { usePerformance } from '@/state/performance'
import { useToast } from '@/state/store'
import { fmtDateTime, fmtNum, fmtPct } from '@/lib/format'
import { cn } from '@/lib/cn'
import type { Measurements, MetricDelta, PackComparison, Profile, RankEntry, SmartPack, SmartPackReport, SwitchPlan, Tier } from '@/types/smartPacks'
import { EVIDENCE_LABEL, GOAL, OUTCOME, PROFILE, STATE, TIER } from './meta'

type View = 'todas' | Tier

/** Relatório de Smart Packs do núcleo, recalculado quando o histórico do Fix Engine ou as sessões mudam. */
export function useSmartPacks(appId: string | null, profile: Profile) {
  const engine = useFixEngine()
  const perf = usePerformance()
  const [report, setReport] = useState<SmartPackReport | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const stamp = engine.operations.map((o) => o.id + o.state).join() + '|' + perf.sessions.map((s) => s.id + (s.smartPack?.packId ?? '')).join()
  const load = useCallback(async () => {
    if (!engine.available || !appId) return setReport(null)
    setLoading(true)
    try {
      setReport(await getSmartPacks(appId, profile))
      setError(null)
    } catch (e) {
      setError(errorMessage(e))
    } finally {
      setLoading(false)
    }
  }, [engine.available, appId, profile])
  useEffect(() => {
    void load()
  }, [load, stamp])
  return { report, loading, error, reload: load }
}

const fps = (v: number | null | undefined) => (v == null ? '—' : fmtNum(v, 1))

function delta(d: MetricDelta | undefined) {
  if (!d) return null
  return (
    <span style={{ color: OUTCOME[d.outcome].color }} title={`Limiar ±${fmtNum(d.thresholdPct, 1)}%${d.limited ? ' · indicativo' : ''}`}>
      {fmtPct(d.deltaPct, 1)}
      {d.limited && <span className="text-fg-3"> · indicativo</span>}
    </span>
  )
}

function mascotFor(r: SmartPackReport | null, loading: boolean) {
  if (loading && !r) return 'analyze' as const
  if (!r) return 'idle' as const
  if (r.ranking.winner) return 'celebrate' as const
  if (r.ranking.entries.some((e) => e.tier === 'not-recommended' && e.outcome === 'regression')) return 'worried' as const
  if (r.ranking.entries.some((e) => e.outcome === 'inconclusive' || e.outcome === 'mixed' || (e.tier === 'measured' && e.outcome === 'no-difference'))) return 'confused' as const
  return 'inspect' as const
}

export function SmartPacksPanel({ appId, onAct }: { appId: string | null; onAct?: (t: ApplyTarget) => void }) {
  const [profile, setProfile] = useState<Profile>('balanced')
  const [view, setView] = useState<View>('todas')
  const { report, loading, error, reload } = useSmartPacks(appId, profile)
  const [detail, setDetail] = useState<SmartPack | null>(null)
  const [target, setTarget] = useState<ApplyTarget | null>(null)
  const [switchTo, setSwitchTo] = useState<SmartPack | null>(null)
  const navigate = useNavigate()
  const act = onAct ?? setTarget

  if (!appId) return <EmptyState pose="idle" title="Escolha um jogo" body="Os Smart Packs são compostos por jogo, a partir das descobertas e receitas aplicáveis a ele." />
  if (error && !report)
    return (
      <EmptyState
        pose="worried"
        title="Não foi possível montar os Smart Packs"
        body={error}
        action={
          <Button variant="outline" icon={<RefreshCw className="size-3.5" />} onClick={() => void reload()}>
            Tentar de novo
          </Button>
        }
      />
    )
  if (!report) return <div className="mt-5 h-48 rounded-xl skeleton" aria-label="Analisando combinações" />

  const byId = (id: string) => report.packs.find((p) => p.id === id)
  const meas = (id: string) => report.measurements.find((m) => m.packId === id)
  const entries = report.ranking.entries.filter((e) => view === 'todas' || e.tier === view)
  const tiers: Tier[] = ['measured', 'candidate', 'not-recommended']
  const benchmark = () => navigate(`/performance?game=${report.appId}`)
  const baseline = byId(report.baselineId)!

  return (
    <div className="mt-5 space-y-6">
      <div className="flex flex-wrap items-center gap-3">
        <Segmented<Profile>
          label="Perfil de otimização"
          size="sm"
          value={profile}
          onChange={setProfile}
          options={(['performance', 'balanced', 'compatibility'] as Profile[]).map((p) => ({ value: p, label: PROFILE[p].label }))}
        />
        <Segmented<View>
          label="Mostrar"
          size="sm"
          value={view}
          onChange={setView}
          options={[
            { value: 'todas', label: 'Todos' },
            { value: 'measured', label: 'Ranking medido' },
            { value: 'candidate', label: 'Candidatos' },
            { value: 'not-recommended', label: 'Não recomendados' },
          ]}
        />
        <Button size="sm" variant="ghost" className="ml-auto" icon={<RefreshCw className={cn('size-3.5', loading && 'animate-spin')} />} onClick={() => void reload()} disabled={loading}>
          Recalcular
        </Button>
      </div>

      <section className="flex items-start gap-5 rounded-xl border border-line bg-surface p-5" aria-live="polite">
        <Mascot pose={mascotFor(report, loading)} scale={3} className="shrink-0" />
        <div className="min-w-0 flex-1 space-y-2">
          <p className="text-[15px] font-medium text-fg">{report.ranking.summary}</p>
          <p className="text-[12.5px] text-fg-2">{PROFILE[profile].help} {report.currentNote}</p>
          {report.ranking.nextTest && (
            <p className="flex items-start gap-2 text-[12.5px] text-fg-2">
              <FlaskConical className="mt-0.5 size-3.5 shrink-0 text-accent" aria-hidden /> <span>Próximo teste: {report.ranking.nextTest.reason}</span>
            </p>
          )}
          <div className="flex flex-wrap gap-2 pt-1">
            <Button size="sm" variant="primary" icon={<Gauge className="size-3.5" />} onClick={benchmark}>
              Iniciar benchmark
            </Button>
            {report.currentPackId !== report.baselineId && (
              <Button size="sm" variant="outline" icon={<RotateCcw className="size-3.5" />} onClick={() => setSwitchTo(baseline)}>
                Restaurar estado anterior
              </Button>
            )}
          </div>
        </div>
      </section>

      {tiers
        .filter((t) => view === 'todas' || view === t)
        .map((t) => {
          const list = entries.filter((e) => e.tier === t)
          return (
            <section key={t} aria-label={TIER[t]}>
              <h3 className="eyebrow mb-3">
                {TIER[t]} · {list.length}
              </h3>
              {t === 'candidate' && list.length > 0 && <p className="-mt-1 mb-3 text-[12px] text-fg-3">Ordem sugerida de teste — não é ordem de desempenho.</p>}
              {list.length === 0 ? (
                <p className="text-[12.5px] text-fg-3">
                  {t === 'measured' ? 'Nenhum pacote tem medições suficientes (≥ 2 sessões comparáveis com o pacote e com o baseline).' : 'Nada aqui.'}
                </p>
              ) : (
                <ul className="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-3">
                  {list.map((e) => (
                    <PackCard
                      key={e.packId}
                      entry={e}
                      pack={byId(e.packId)}
                      m={meas(e.packId)}
                      winner={report.ranking.winner === e.packId}
                      current={report.currentPackId === e.packId}
                      onDetail={setDetail}
                      onApply={(p) => (report.currentPackId === report.baselineId ? act({ kind: 'apply', recipeId: p.applyRecipeId!, appId: report.appId, title: p.name }) : setSwitchTo(p))}
                      onBenchmark={benchmark}
                    />
                  ))}
                </ul>
              )}
            </section>
          )
        })}

      <ComparePanel report={report} />
      <Unassigned report={report} onDone={() => void reload()} />

      <section className="grid gap-4 md:grid-cols-2">
        <div className="rounded-lg border border-line bg-surface p-4">
          <h3 className="eyebrow mb-2">Critérios do perfil</h3>
          <ul className="space-y-1 text-[12.5px] text-fg-2">
            {report.ranking.criteria.map((c) => (
              <li key={c}>{c}</li>
            ))}
          </ul>
          {report.notes.length > 0 && (
            <ul className="mt-3 space-y-1 border-t border-line pt-3 text-[12px] text-fg-3">
              {report.notes.map((n) => (
                <li key={n}>{n}</li>
              ))}
            </ul>
          )}
        </div>
        <div className="space-y-4">
          <div className="rounded-lg border border-line bg-surface p-4">
            <h3 className="eyebrow mb-2">Referência Windows</h3>
            <p className="text-[12.5px] text-fg-2">
              {report.windows ? `${report.windows.origin} · ${report.windows.conditions}` : 'Referência Windows indisponível. O FRX não usa relatos como “roda melhor no Linux” como benchmark.'}
            </p>
          </div>
          <div className="rounded-lg border border-line bg-surface p-4">
            <h3 className="eyebrow mb-2">Histórico do ranking</h3>
            <ul className="max-h-40 space-y-1 overflow-y-auto text-[12px] text-fg-2">
              {[...report.history].reverse().map((h, i) => (
                <li key={i} className="flex gap-3">
                  <span className="tnum w-36 shrink-0 text-fg-3">{fmtDateTime(h.at)}</span>
                  <span className="w-24 shrink-0 text-fg-3">{PROFILE[h.profile].short}</span>
                  <span className="min-w-0 flex-1">{h.reason}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>
      </section>

      {detail && <PackDetail pack={detail} report={report} onClose={() => setDetail(null)} />}
      {switchTo && (
        <SwitchDialog
          appId={report.appId}
          pack={switchTo}
          onClose={() => setSwitchTo(null)}
          onStep={(t) => {
            setSwitchTo(null)
            act(t)
          }}
        />
      )}
      {target && <ApplyFixDialog target={target} onClose={() => setTarget(null)} />}
    </div>
  )
}

function PackCard({
  entry,
  pack,
  m,
  winner,
  current,
  onDetail,
  onApply,
  onBenchmark,
}: {
  entry: RankEntry
  pack?: SmartPack
  m?: Measurements
  winner: boolean
  current: boolean
  onDetail: (p: SmartPack) => void
  onApply: (p: SmartPack) => void
  onBenchmark: () => void
}) {
  const vs = entry.vsBaseline
  const used = m?.sessions.filter((s) => !s.excluded).length ?? 0
  const canApply = pack && pack.applyRecipeId && (pack.state === 'ready' || pack.state === 'rolled_back' || pack.state === 'failed' || pack.state === 'conflict') && !current
  return (
    <motion.li
      layout
      initial={{ opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      className={cn('flex flex-col rounded-lg border bg-surface p-4', winner ? 'border-accent-line' : 'border-line')}
      aria-label={entry.title}
    >
      <div className="flex items-start gap-3">
        {entry.position != null && <span className="tnum font-display text-[22px] leading-none font-bold text-fg">#{entry.position}</span>}
        <div className="min-w-0 flex-1">
          {winner && (
            <p className="mb-1 inline-flex items-center gap-1 text-[11.5px] font-medium text-accent">
              <Trophy className="size-3" aria-hidden /> #1 — {entry.headline === 'Melhor que o baseline nas medições' ? 'Melhor resultado medido' : entry.headline}
            </p>
          )}
          <h4 className="truncate text-[14px] font-medium text-fg" title={entry.title}>
            {entry.title}
          </h4>
          <p className="text-[12px] text-fg-3">
            {pack ? `${GOAL[pack.goal]} · ${pack.members.length === 0 ? 'nenhuma correção' : `${pack.members.length} correç${pack.members.length === 1 ? 'ão' : 'ões'}`}` : 'Descoberta'}
            {current && <span className="text-good"> · configuração atual</span>}
          </p>
        </div>
      </div>
      <div className="mt-3 flex flex-wrap gap-1.5">
        {pack && (
          <Pill color={STATE[pack.state].color} soft={STATE[pack.state].soft} title={pack.stateDetail}>
            {pack.kind === 'baseline' ? (pack.state === 'verified' ? 'Ativo' : 'Inativo') : STATE[pack.state].label}
          </Pill>
        )}
        <Pill color={OUTCOME[entry.outcome].color} soft={OUTCOME[entry.outcome].soft}>
          {entry.headline}
        </Pill>
      </div>
      {pack && pack.kind !== 'baseline' && (
        <p className="mt-2 text-[12px] text-fg-3">
          Evidência: {EVIDENCE_LABEL[pack.evidence]} · <span style={{ color: RISK[pack.risk].color }}>{RISK[pack.risk].label}</span>
        </p>
      )}
      {!pack && <p className="mt-2 line-clamp-3 text-[12px] text-fg-3">{entry.reasons[0]}</p>}
      {pack && (
        <dl className="mt-3 grid grid-cols-3 gap-2 text-[12px]">
          <div>
            <dt className="text-fg-3">Benchmarks</dt>
            <dd className="tnum text-fg">{used}</dd>
          </div>
          <div>
            <dt className="text-fg-3">FPS médio</dt>
            <dd className="tnum text-fg">{fps(m?.avg?.median)}</dd>
            <dd className="tnum">{delta(vs?.deltas.find((d) => d.metric === 'fpsAvg'))}</dd>
          </div>
          <div>
            <dt className="text-fg-3">1% low</dt>
            <dd className="tnum text-fg">{fps(m?.low1?.median)}</dd>
            <dd className="tnum">{delta(vs?.deltas.find((d) => d.metric === 'fpsLow1'))}</dd>
          </div>
        </dl>
      )}
      <div className="mt-auto flex flex-wrap gap-2 pt-4">
        {pack && (
          <Button size="sm" variant="ghost" icon={<Info className="size-3.5" />} onClick={() => onDetail(pack)}>
            Detalhes
          </Button>
        )}
        {canApply && (
          <Button size="sm" variant="secondary" icon={current ? undefined : <Wrench className="size-3.5" />} onClick={() => onApply(pack!)}>
            {pack!.state === 'conflict' ? 'Trocar para este' : 'Aplicar'}
          </Button>
        )}
        {pack?.kind === 'baseline' && !current && (
          <Button size="sm" variant="secondary" icon={<RotateCcw className="size-3.5" />} onClick={() => onApply(pack)}>
            Restaurar
          </Button>
        )}
        {current && (
          <Button size="sm" variant="outline" icon={<Gauge className="size-3.5" />} onClick={onBenchmark}>
            {used ? 'Repetir benchmark' : 'Testar agora'}
          </Button>
        )}
      </div>
    </motion.li>
  )
}

function PackDetail({ pack, report, onClose }: { pack: SmartPack; report: SmartPackReport; onClose: () => void }) {
  const entry = report.ranking.entries.find((e) => e.packId === pack.id)
  const m = report.measurements.find((x) => x.packId === pack.id)
  const open = (u: string) => void openExternal(u).catch(() => {})
  const Block = ({ title, children }: { title: string; children: React.ReactNode }) => (
    <div>
      <div className="eyebrow mb-2">{title}</div>
      {children}
    </div>
  )
  return (
    <Dialog open onClose={onClose} title={pack.name} description={`${GOAL[pack.goal]} · ${STATE[pack.state].label}`} width={720} footer={<Button variant="ghost" onClick={onClose}>Fechar</Button>}>
      <div className="space-y-5 text-[12.5px] text-fg-2">
        <Block title="Objetivo">
          <p>{pack.goalText}</p>
        </Block>
        {pack.members.length > 0 && (
          <Block title="Correções incluídas">
            <ul className="divide-y divide-line overflow-hidden rounded-md border border-line bg-bg/50">
              {pack.members.map((x) => (
                <li key={x.recipeId} className="px-3.5 py-2.5">
                  <div className="flex items-center gap-2">
                    <span className="min-w-0 flex-1 text-fg">{x.title}</span>
                    <span className="text-fg-3">{x.trust === 'reviewed' ? 'revisada' : 'gerada'} · v{x.version}</span>
                  </div>
                  <code className="font-mono text-[12px] text-fg-2">{x.env.map((e) => `${e.key}=${e.value}`).join(' ')}</code>
                  <p className="text-[11.5px] text-fg-3">
                    Evidência: {EVIDENCE_LABEL[x.evidence]}
                    {x.discoveredAt && ` · fonte de ${fmtDateTime(x.discoveredAt)}`}
                    {x.protonVersions.length > 0 && ` · relatada com ${x.protonVersions.join(', ')}`}
                  </p>
                </li>
              ))}
            </ul>
          </Block>
        )}
        {pack.effects.length + pack.sideEffects.length > 0 && (
          <Block title="O que muda e riscos">
            <ul className="space-y-1">
              {pack.effects.map((e) => (
                <li key={e}>
                  <span className="text-fg-3">Efeito · </span>
                  {e}
                </li>
              ))}
              {pack.sideEffects.map((e) => (
                <li key={e}>
                  <span className="text-caution">Risco · </span>
                  {e}
                </li>
              ))}
            </ul>
          </Block>
        )}
        {(pack.requirements.length > 0 || pack.compatibility.length > 0) && (
          <Block title="Requisitos e compatibilidade">
            <ul className="space-y-1">
              {[...pack.requirements, ...pack.compatibility].map((r) => (
                <li key={r}>{r}</li>
              ))}
            </ul>
          </Block>
        )}
        {pack.conflictsAvoided.length > 0 && (
          <Block title="Conflitos evitados">
            <ul className="space-y-1">
              {pack.conflictsAvoided.map((e, i) => (
                <li key={i}>
                  <code className="font-mono text-[11.5px] text-fg-3">
                    {e.a} × {e.b}
                  </code>{' '}
                  — {e.reason}
                </li>
              ))}
            </ul>
          </Block>
        )}
        <Block title="Por que esta classificação">
          <ul className="space-y-1">
            {(entry?.reasons ?? []).map((r) => (
              <li key={r}>{r}</li>
            ))}
            {pack.kind !== 'baseline' && <li className="text-fg-3">{pack.evidenceNote}</li>}
          </ul>
        </Block>
        <Block title={`Benchmarks · ${m?.sessions.length ?? 0}`}>
          {m?.sessions.length ? (
            <ul className="space-y-1">
              {m.sessions.map((s) => (
                <li key={s.id} className="flex gap-3">
                  <span className="min-w-0 flex-1 truncate text-fg">{s.label}</span>
                  <span className="tnum">{fps(s.avgFps)} FPS · 1% {fps(s.low1)}</span>
                  <span className={cn('w-48 shrink-0 text-right', s.excluded ? 'text-caution' : 'text-fg-3')} title={[s.excluded, ...s.notes].filter(Boolean).join(' ')}>
                    {s.excluded ? 'fora do ranking' : s.source === 'declared' ? 'declarada' : 'registrada pelo FRX'}
                  </span>
                </li>
              ))}
            </ul>
          ) : (
            <p className="text-fg-3">Nenhuma sessão com este pacote.</p>
          )}
        </Block>
        <Block title="Backup e rollback">
          <p>
            {pack.kind === 'baseline'
              ? 'O baseline não grava nada: voltar a ele é desfazer as correções do FRX ativas.'
              : 'Aplicado pelo Fix Engine numa única operação: prévia, backup verificado (SHA-256), gravação atômica e conferência no disco. O rollback remove só o que o pacote adicionou e continua disponível depois de reiniciar o FRX.'}
          </p>
        </Block>
        {pack.sources.length > 0 && (
          <Block title="Fontes">
            <ul className="space-y-1">
              {pack.sources.map((s) => (
                <li key={s.url} className="flex items-center gap-2">
                  <span className="min-w-0 flex-1 truncate">{s.label}</span>
                  <button className="inline-flex items-center gap-1 text-fg-2 hover:text-accent" onClick={() => open(s.url)} aria-label={`Abrir fonte: ${s.label}`}>
                    Abrir <ExternalLink className="size-3" />
                  </button>
                </li>
              ))}
            </ul>
          </Block>
        )}
        <p className="font-mono text-[11px] text-fg-3">
          {pack.id} · v{pack.version} · {pack.fingerprint.slice(0, 12)}
        </p>
      </div>
    </Dialog>
  )
}

function SwitchDialog({ appId, pack, onClose, onStep }: { appId: string; pack: SmartPack; onClose: () => void; onStep: (t: ApplyTarget) => void }) {
  const engine = useFixEngine()
  const [plan, setPlan] = useState<SwitchPlan | null>(null)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    planSmartPackSwitch(appId, pack.id).then(setPlan, (e) => setError(errorMessage(e)))
  }, [appId, pack.id])
  const next = plan?.steps[0]
  const go = () => {
    if (!next) return
    if (next.kind === 'rollback') {
      const op = engine.operations.find((o) => o.id === next.operationId)
      if (op) onStep({ kind: 'rollback', operation: op })
    } else if (next.recipeId) onStep({ kind: 'apply', recipeId: next.recipeId, appId, title: pack.name })
  }
  const env = (l: { key: string; value: string }[]) => (l.length ? l.map((p) => `${p.key}=${p.value}`).join(' ') : '—')
  return (
    <Dialog
      open
      onClose={onClose}
      title={pack.kind === 'baseline' ? 'Restaurar estado anterior' : `Trocar para ${pack.name}`}
      description="Um passo por vez, cada um com a própria prévia e confirmação."
      width={600}
      aside={
        <div className="-mt-1 -ml-2 hidden sm:block">
          <Mascot pose="inspect" scale={3} />
        </div>
      }
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancelar
          </Button>
          {plan?.possible && next && (
            <Button variant="primary" icon={<ArrowRightLeft className="size-4" />} onClick={go}>
              {next.title}
            </Button>
          )}
        </>
      }
    >
      {error && <p className="rounded-md border border-bad/25 bg-bad-soft px-3.5 py-2.5 text-[13px] text-bad">{error}</p>}
      {!plan && !error && <div className="h-32 rounded-md skeleton" />}
      {plan && (
        <div className="space-y-4 text-[12.5px]">
          <dl className="space-y-1 font-mono text-[12px]">
            <div className="flex gap-3">
              <dt className="w-20 text-fg-3">mantém</dt>
              <dd className="text-fg-2">{env(plan.keep)}</dd>
            </div>
            <div className="flex gap-3">
              <dt className="w-20 text-bad">remove</dt>
              <dd className="text-fg-2">{env(plan.remove)}</dd>
            </div>
            <div className="flex gap-3">
              <dt className="w-20 text-good">adiciona</dt>
              <dd className="text-fg-2">{env(plan.add)}</dd>
            </div>
          </dl>
          <ol className="list-decimal space-y-1 pl-5 text-fg-2">
            {plan.steps.map((s, i) => (
              <li key={i}>
                {s.title} — <span className="text-fg-3">{s.detail}</span>
              </li>
            ))}
          </ol>
          <ul className="space-y-1 text-fg-3">
            {plan.notes.map((n) => (
              <li key={n}>{n}</li>
            ))}
          </ul>
        </div>
      )}
    </Dialog>
  )
}

function ComparePanel({ report }: { report: SmartPackReport }) {
  const measured = report.packs.filter((p) => report.measurements.some((m) => m.packId === p.id && m.sessions.some((s) => !s.excluded)))
  const [a, setA] = useState(report.baselineId)
  const [b, setB] = useState(measured.find((p) => p.id !== report.baselineId)?.id ?? '')
  const [res, setRes] = useState<PackComparison | null>(null)
  const [error, setError] = useState<string | null>(null)
  const navigate = useNavigate()
  const latest = (id: string) => report.measurements.find((m) => m.packId === id)?.sessions.filter((s) => !s.excluded).sort((x, y) => (y.capturedAt ?? '').localeCompare(x.capturedAt ?? ''))[0]
  const opts = useMemo(() => measured.map((p) => ({ value: p.id, label: p.name })), [measured])
  if (measured.length < 2) return null
  const run = () => compareSmartPacks(report.appId, a, b).then(setRes, (e) => setError(errorMessage(e)))
  return (
    <section className="rounded-lg border border-line bg-surface p-4" aria-label="Comparar pacotes">
      <h3 className="eyebrow mb-3 flex items-center gap-2">
        <Layers className="size-3.5" aria-hidden /> Comparar pacotes
      </h3>
      <div className="flex flex-wrap items-end gap-3">
        <Select label="A" value={a} onChange={setA} options={opts} />
        <Select label="B" value={b} onChange={setB} options={opts} />
        <Button size="sm" variant="outline" disabled={!a || !b || a === b} onClick={() => void run()}>
          Comparar
        </Button>
        {latest(a) && latest(b) && (
          <Button size="sm" variant="ghost" onClick={() => navigate(`/performance?antes=${latest(a)!.id}&depois=${latest(b)!.id}`)}>
            Ver sessões no Performance Lab
          </Button>
        )}
      </div>
      {error && <p className="mt-3 text-[12.5px] text-bad">{error}</p>}
      <AnimatePresence>
        {res && (
          <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="mt-4 space-y-2 text-[12.5px]">
            <p className="text-fg">{res.summary}</p>
            <ul className="grid grid-cols-2 gap-x-6 gap-y-1 md:grid-cols-3">
              {res.deltas.map((d) => (
                <li key={d.metric} className="flex justify-between gap-3">
                  <span className="text-fg-3">{d.label}</span>
                  {delta(d)}
                </li>
              ))}
            </ul>
            {res.notes.map((n) => (
              <p key={n} className="text-fg-3">
                {n}
              </p>
            ))}
          </motion.div>
        )}
      </AnimatePresence>
    </section>
  )
}

function Unassigned({ report, onDone }: { report: SmartPackReport; onDone: () => void }) {
  const toast = useToast()
  const perf = usePerformance()
  const [choice, setChoice] = useState<Record<string, string>>({})
  if (!report.unassignedSessions.length) return null
  const opts = report.packs.map((p) => ({ value: p.id, label: p.name }))
  const declare = async (id: string) => {
    try {
      await declareBenchmarkPack(id, choice[id] ?? report.baselineId)
      await perf.reload()
      onDone()
      toast({ tone: 'ok', title: 'Configuração declarada', body: 'A sessão entra no ranking marcada como “declarada por você”.' })
    } catch (e) {
      toast({ tone: 'error', title: 'Não foi possível declarar', body: errorMessage(e) })
    }
  }
  return (
    <section className="rounded-lg border border-line bg-surface p-4" aria-label="Sessões sem configuração registrada">
      <h3 className="eyebrow mb-1">Sessões sem configuração registrada · {report.unassignedSessions.length}</h3>
      <p className="mb-3 text-[12px] text-fg-3">Importadas ou anteriores à v0.4: o FRX não sabe quais correções estavam ativas. Se você sabe, declare — a sessão entra no ranking marcada como declarada.</p>
      <ul className="space-y-2">
        {report.unassignedSessions.map((s) => (
          <li key={s.id} className="flex flex-wrap items-end gap-3 text-[12.5px]">
            <span className="min-w-0 flex-1 truncate text-fg">
              {s.label} <span className="text-fg-3">· {fps(s.avgFps)} FPS</span>
            </span>
            <Select label="Configuração" hideLabel value={choice[s.id] ?? report.baselineId} onChange={(v) => setChoice((c) => ({ ...c, [s.id]: v }))} options={opts} />
            <Button size="sm" variant="outline" onClick={() => void declare(s.id)}>
              Declarar
            </Button>
          </li>
        ))}
      </ul>
    </section>
  )
}
