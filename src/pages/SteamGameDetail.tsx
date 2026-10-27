import { useMemo, useState } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { motion } from 'motion/react'
import { ArrowLeft, ExternalLink, Gauge, RefreshCw, ScanSearch, ShieldCheck, Wifi } from 'lucide-react'
import { Page } from '@/layouts/Page'
import { CoverImage } from '@/components/game/CoverImage'
import { InstallBadge } from '@/components/game/InstallBadge'
import { InfoBlock, InfoRow } from '@/components/game/InfoBlock'
import { SourceTag } from '@/components/game/LibraryNotices'
import { FindingCard, ProviderStatusList } from '@/components/fix/FindingCard'
import { ApplyFixDialog, type ApplyTarget } from '@/components/fix/ApplyFixDialog'
import { OperationHistory } from '@/components/fix/OperationHistory'
import { useRecipes } from '@/components/fix/RecipesPanel'
import { useAssessments } from '@/components/fix/useAssessments'
import { Button } from '@/components/ui/Button'
import { Segmented } from '@/components/ui/Controls'
import { Mascot } from '@/mascot/Mascot'
import { useLibrary } from '@/state/library'
import { useFixReports } from '@/state/fixes'
import { useFixEngine } from '@/state/fixEngine'
import { useToast } from '@/state/store'
import { openExternal } from '@/services/opener'
import { TOOL_SOURCE_LABEL, toolLabel } from '@/lib/steamGame'
import { fmtDateTime, fmtNum, fmtRelative } from '@/lib/format'
import type { Game } from '@/types'
import type { FixFinding } from '@/types/fixes'
import { cn } from '@/lib/cn'
import { useSmartPacks } from '@/components/smart/SmartPacksPanel'
import { isRelevant } from '@/lib/findings'
import { ResolutionCenter } from '@/components/resolution/ResolutionCenter'

type FindingFilter = 'relevantes' | 'todas'

const ORDER: Record<FixFinding['confidence'], number> = { verified: 0, curated: 1, 'community-reported': 2, unverified: 3 }

/** Detalhes de um jogo real detectado na Steam. Mesmo desenho da página demonstrativa. */
export function SteamGameDetail({ game }: { game: Game }) {
  const navigate = useNavigate()
  const s = game.steam!
  const c = s.compatibility
  const { system } = useLibrary()
  const fixes = useFixReports()
  const toast = useToast()
  const report = fixes.reports[game.id]
  const searching = !!fixes.searching[game.id]
  const error = fixes.errors[game.id]
  const [filter, setFilter] = useState<FindingFilter>('relevantes')
  const engine = useFixEngine()
  const [target, setTarget] = useState<ApplyTarget | null>(null)
  const { recipes, checks } = useRecipes(game.id)
  const applicable = recipes.filter((r) => {
    const c = checks[r.id]
    return r.executable && typeof c === 'object' && (c.status === 'ready' || c.status === 'requires-manual-step')
  }).length
  const applied = engine.applied.filter((a) => a.appId === game.id)
  const analysis = useAssessments(game.id)
  const generated = new Set(Object.values(analysis.byFinding).filter((a) => a.status === 'generated').map((a) => a.recipeId)).size

  const findings = useMemo(() => {
    const list = [...(report?.findings ?? [])].sort((a, b) => {
      const inc = Number(a.compatibility === 'incompatible') - Number(b.compatibility === 'incompatible')
      return inc || ORDER[a.confidence] - ORDER[b.confidence] || b.sources.length - a.sources.length
    })
    return filter === 'todas' ? list : list.filter(isRelevant)
  }, [report, filter])
  const hidden = (report?.findings.length ?? 0) - findings.length

  const search = (force: boolean) =>
    fixes.search(game.id, game.title, force).then((r) => {
      if (r) toast({ tone: 'ok', title: 'Pesquisa concluída', body: `${r.findings.length} descoberta${r.findings.length === 1 ? '' : 's'} para ${game.title}${r.fromCache ? ' · parte do cache' : ''}` })
    })

  const open = (url: string) => openExternal(url).catch(() => toast({ tone: 'error', title: 'Não foi possível abrir o link' }))

  return (
    <Page className="pt-6">
      <Link to="/biblioteca" className="inline-flex items-center gap-1.5 rounded-xs text-[13px] text-fg-3 transition-colors hover:text-fg">
        <ArrowLeft className="size-3.5" /> Biblioteca
      </Link>

      {/* Banner */}
      <section className="relative mt-4 overflow-hidden rounded-xl border border-line">
        <div className="relative h-[300px]">
          <motion.div className="absolute inset-0" initial={{ scale: 1.06 }} animate={{ scale: 1 }} transition={{ duration: 1.2, ease: [0.16, 1, 0.3, 1] }}>
            <CoverImage game={game} kind="hero" variant="banner" />
          </motion.div>
          <div className="absolute inset-0 bg-gradient-to-t from-bg via-bg/55 to-transparent" aria-hidden />
          <div className="absolute inset-0 bg-gradient-to-r from-bg/70 via-transparent to-transparent" aria-hidden />
          <div className="absolute inset-x-8 bottom-7 flex items-end justify-between gap-8">
            <div className="min-w-0">
              <p className="text-[13px] text-fg-2">Steam · AppID {s.appId}</p>
              <h1 className="mt-1.5 font-display text-[52px] leading-[0.95] font-bold tracking-[-0.05em] text-fg">{game.title}</h1>
              <div className="mt-4 flex flex-wrap items-center gap-2">
                <InstallBadge status={s.installationStatus} solid />
                <span className="inline-flex h-6 items-center rounded-xs bg-bg/60 px-2 text-[12px] text-fg-2 backdrop-blur-sm">{game.protonVersion}</span>
                <SourceTag />
              </div>
            </div>
            <div className="flex shrink-0 items-center gap-2">
              <Button variant="secondary" icon={<Gauge className="size-4" />} onClick={() => navigate(`/performance?game=${game.id}`)}>
                Medir desempenho
              </Button>
              <Button variant="secondary" icon={<ExternalLink className="size-4" />} onClick={() => open(`https://www.protondb.com/app/${s.appId}`)}>
                ProtonDB
              </Button>
              <Button variant="primary" icon={<ScanSearch className="size-4" />} onClick={() => void search(false)} loading={searching}>
                {searching ? 'Pesquisando…' : report ? 'Ver correções' : 'Pesquisar correções'}
              </Button>
            </div>
          </div>
        </div>
      </section>

      {/* Fatos rápidos */}
      <dl className="mt-6 grid grid-cols-5 divide-x divide-line border-y border-line">
        {[
          ['Tempo de jogo', s.playtimeMinutes != null ? (s.playtimeMinutes < 60 ? `${s.playtimeMinutes} min` : `${fmtNum(s.playtimeMinutes / 60, 1)} h`) : '—'],
          ['Última sessão', s.lastPlayed ? fmtRelative(s.lastPlayed) : 'Nunca'],
          ['Tamanho', s.sizeOnDisk ? `${fmtNum(s.sizeOnDisk / 1e9, 1)} GB` : '—'],
          ['Build', s.buildId ?? '—'],
          ['Atualizado', s.lastUpdated ? fmtRelative(s.lastUpdated) : '—'],
        ].map(([k, v]) => (
          <div key={k} className="px-5 py-4 first:pl-0">
            <dt className="text-[12px] text-fg-3">{k}</dt>
            <dd className="tnum mt-1 truncate text-[15px] font-medium text-fg">{v}</dd>
          </div>
        ))}
      </dl>

      <ResolutionCenter appId={game.id} available={engine.available} />

      <div className="mt-10 grid grid-cols-[minmax(0,1.65fr)_minmax(0,1fr)] gap-12">
        <div className="min-w-0 space-y-12">
          {/* Correções descobertas */}
          <section aria-labelledby="descobertas" aria-busy={searching}>
            <div className="mb-4 flex items-end justify-between gap-4">
              <div>
                <h2 id="descobertas" className="font-display text-[20px] font-semibold tracking-[-0.02em]">
                  Correções para este jogo e hardware
                </h2>
                <p className="mt-1 text-[12.5px] text-fg-3">
                  {report ? (
                    <>
                      Consultado {fmtRelative(report.retrievedAt)}
                      {report.fromCache && ' · inclui dados do cache'}
                      {report.stale && <span className="text-caution"> · desatualizado</span>}
                    </>
                  ) : (
                    'Descoberta somente leitura em UMU-Protonfixes e GitHub. Descobertas nunca são executadas.'
                  )}
                </p>
              </div>
              {report && (
                <div className="flex items-center gap-2">
                  <Segmented<FindingFilter>
                    label="Filtrar descobertas"
                    size="sm"
                    value={filter}
                    onChange={setFilter}
                    options={[
                      { value: 'relevantes', label: 'Relevantes' },
                      { value: 'todas', label: 'Todas' },
                    ]}
                  />
                  <Button size="sm" variant="outline" icon={<RefreshCw className={cn('size-3.5', searching && 'animate-spin')} />} onClick={() => void search(true)} disabled={searching}>
                    Pesquisar de novo
                  </Button>
                </div>
              )}
            </div>

            {error && (
              <p className="mb-4 flex items-start gap-2.5 rounded-md border border-bad/25 bg-bad-soft px-4 py-2.5 text-[13px] text-bad">
                <Wifi className="mt-0.5 size-4 shrink-0" aria-hidden /> A pesquisa falhou: {error}
              </p>
            )}

            {searching && !report ? (
              <div className="flex items-center gap-5 rounded-lg border border-line bg-surface px-6 py-5" aria-live="polite">
                <Mascot pose="inspect" scale={4} />
                <div>
                  <p className="font-display text-[17px] font-semibold tracking-[-0.02em]">Investigando…</p>
                  <p className="mt-1 text-[13px] text-fg-2">Consultando UMU-Protonfixes, UMU Database e GitHub. Respostas ficam em cache local.</p>
                </div>
              </div>
            ) : !report ? (
              <div className="flex items-center gap-5 rounded-lg border border-dashed border-line-2 px-6 py-6">
                <Mascot pose="idle" scale={3} />
                <div className="flex-1">
                  <p className="font-medium">Ainda não pesquisado</p>
                  <p className="text-[13px] text-fg-2">O FRX procura correções conhecidas pelo AppID e mostra cada fonte original. Funciona sem conexão usando o cache.</p>
                </div>
                <Button variant="primary" icon={<ScanSearch className="size-4" />} onClick={() => void search(false)}>
                  Pesquisar
                </Button>
              </div>
            ) : (
              <>
                <div className="mb-4">
                  <ProviderStatusList providers={report.providers} />
                </div>
                {findings.length === 0 ? (
                  <div className="flex items-center gap-5 rounded-lg border border-dashed border-line-2 px-6 py-6">
                    <Mascot pose={report.providers.some((p) => p.state === 'unavailable' || p.state === 'rate-limited') ? 'worried' : 'idle'} scale={3} />
                    <div>
                      <p className="font-medium">{report.findings.length ? 'Nenhuma descoberta relevante para o seu sistema' : 'Nenhuma correção conhecida encontrada'}</p>
                      <p className="text-[13px] text-fg-2">
                        {report.findings.length
                          ? `${hidden} descoberta${hidden === 1 ? '' : 's'} oculta${hidden === 1 ? '' : 's'} (incompatíveis ou contestadas). Use “Todas” para ver.`
                          : 'As fontes consultadas não têm correções específicas para este AppID. Isso costuma significar que o jogo roda sem ajustes — mas não é garantia.'}
                      </p>
                    </div>
                  </div>
                ) : (
                  <ul className="space-y-3">
                    {findings.map((f, i) => (
                      <FindingCard
                        key={f.id}
                        finding={f}
                        defaultOpen={i === 0 && findings.length <= 2}
                        recipe={recipes.find((r) => r.relatedFindings.includes(f.id)) ?? null}
                        onOpenRecipe={(r) => navigate(`/correcoes?view=receitas&game=${game.id}&recipe=${r.id}`)}
                        assessment={analysis.byFinding[f.id] ?? null}
                        analyzing={analysis.loading}
                        onApply={(a) => setTarget({ kind: 'apply', recipeId: a.recipeId!, appId: game.id, title: a.recipe?.title ?? f.title })}
                      />
                    ))}
                  </ul>
                )}
                {hidden > 0 && findings.length > 0 && (
                  <p className="mt-3 text-[12.5px] text-fg-3">
                    {hidden} descoberta{hidden === 1 ? '' : 's'} oculta{hidden === 1 ? '' : 's'} — incompatíveis com seu hardware ou contestadas por outros relatos.
                  </p>
                )}
              </>
            )}
          </section>

          {/* Instalação */}
          <section aria-labelledby="instalacao">
            <h2 id="instalacao" className="mb-4 font-display text-[20px] font-semibold tracking-[-0.02em]">
              Instalação
            </h2>
            <div className="divide-y divide-line border-y border-line">
              <InfoRow k="Pasta do jogo" v={s.installPath ?? 'Desconhecida'} mono />
              <InfoRow k="Biblioteca" v={s.libraryPath} mono />
              <InfoRow k="Instalação da Steam" v={s.steamInstall} mono />
              <InfoRow k="Estado (StateFlags)" v={s.stateFlags.length ? s.stateFlags.join(', ') : '—'} />
            </div>
            <p className="mt-3 text-[12px] text-fg-3">Lido de appmanifest_{s.appId}.acf. O FRX nunca altera estes arquivos.</p>
          </section>
        </div>

        <aside className="min-w-0 space-y-9">
          <InfoBlock title="Proton">
            <InfoRow k="Ferramenta" v={toolLabel(c.tool) ?? 'Não identificada'} />
            <InfoRow k="Origem" v={TOOL_SOURCE_LABEL[c.toolSource]} />
            <InfoRow k="Versão" v={c.tool?.version ?? 'Desconhecida'} />
            {c.lastUsedTool && c.lastUsedTool.internalName !== c.tool?.internalName && <InfoRow k="Última execução" v={toolLabel(c.lastUsedTool)!} />}
            <InfoRow k="Versão do prefixo" v={c.prefixVersion ?? '—'} />
            <InfoRow k="UMU-Protonfixes embutido" v={c.tool ? (c.tool.includesProtonfixes ? 'Sim' : 'Não') : '—'} />
            <div className="pt-3">
              <p className="mb-1.5 text-[12px] text-fg-3">Prefixo</p>
              <code className="block truncate rounded-sm border border-line bg-bg px-3 py-2 font-mono text-[12px] text-fg-2" title={c.prefixPath ?? undefined}>
                {c.prefixPath ?? (c.usesProton === null ? 'Nenhum prefixo — jogo nativo ou nunca executado' : '—')}
              </code>
            </div>
          </InfoBlock>

          <InfoBlock title="Hardware detectado">
            {system ? (
              <>
                <InfoRow k="GPU provável" v={system.primaryGpu?.name ?? system.primaryGpu?.vendor.toUpperCase() ?? 'Desconhecida'} />
                <InfoRow k="Driver" v={system.primaryGpu ? [system.primaryGpu.driver, system.primaryGpu.driverVersion].filter(Boolean).join(' ') || '—' : '—'} />
                {system.gpus.length > 1 && <InfoRow k="GPUs detectadas" v={system.gpus.map((g) => `${g.name ?? g.vendor}${g.pciAddress ? ` (${g.pciAddress})` : ''}`).join(', ')} />}
                <InfoRow k="CPU" v={system.cpu ?? '—'} />
                <InfoRow k="Sistema" v={[system.distro, system.kernel && `kernel ${system.kernel}`].filter(Boolean).join(' · ') || '—'} />
                <p className="mt-2 text-[12px] text-fg-3">A detecção do sistema não confirma qual GPU executa o jogo. O Performance Lab registra o renderizador informado pela captura quando identificável.</p>
              </>
            ) : (
              <InfoRow k="Perfil" v="Carregando…" />
            )}
          </InfoBlock>

          {report?.identity.umuId && (
            <InfoBlock title="Identidade UMU">
              <InfoRow k="UMU ID" v={report.identity.umuId} mono />
              {report.identity.aliases.slice(0, 3).map((a) => (
                <InfoRow key={a} k="Também conhecido como" v={a} />
              ))}
            </InfoBlock>
          )}

          {engine.available && <SmartPacksSummary appId={game.id} onOpen={() => navigate(`/correcoes?view=smart&game=${game.id}`)} />}

          <section className="rounded-lg border border-line bg-surface p-5" aria-labelledby="fix-engine">
            <h3 id="fix-engine" className="flex items-center gap-2 text-[14px] font-medium">
              <ShieldCheck className="size-4 text-fg-3" aria-hidden /> Correções do FRX
            </h3>
            <p className="mt-2 text-[12.5px] text-fg-2">
              {engine.available
                ? `${applied.length ? `${applied.length} aplicada${applied.length === 1 ? '' : 's'}` : 'Nenhuma aplicada'} · ${applicable} receita${applicable === 1 ? '' : 's'} revisada${applicable === 1 ? '' : 's'} aplicáve${applicable === 1 ? 'l' : 'is'} agora${generated ? ` · ${generated} gerada${generated === 1 ? '' : 's'} a partir das descobertas` : ''}. Tudo passa por prévia, backup e pode ser desfeito.`
                : 'Disponível no app desktop.'}
            </p>
            <div className="mt-4">
              <OperationHistory appId={game.id} onAct={setTarget} compact />
            </div>
            <Button size="sm" variant="outline" className="mt-4" onClick={() => navigate(`/correcoes?view=receitas&game=${game.id}`)}>
              Ver receitas para este jogo
            </Button>
            {report && <p className="mt-3 text-[11.5px] text-fg-3">Relatório de descobertas gerado em {fmtDateTime(report.retrievedAt)}.</p>}
          </section>
        </aside>
      </div>
      {target && <ApplyFixDialog target={target} onClose={() => setTarget(null)} />}
    </Page>
  )
}

/** Resumo dos Smart Packs do jogo (perfil equilibrado), com atalho para a análise completa. */
function SmartPacksSummary({ appId, onOpen }: { appId: string; onOpen: () => void }) {
  const { report, loading } = useSmartPacks(appId, 'balanced')
  // Pacotes (sem o baseline) por grupo; em “não recomendados” entram também as descobertas fora dos pacotes.
  const count = (t: string) => report?.ranking.entries.filter((e) => e.tier === t && e.packId !== report.baselineId).length ?? 0
  return (
    <section className="rounded-lg border border-line bg-surface p-5" aria-labelledby="smart-packs">
      <h3 id="smart-packs" className="flex items-center gap-2 text-[14px] font-medium">
        <Mascot pose={loading && !report ? 'analyze' : report?.ranking.winner ? 'celebrate' : 'inspect'} scale={2} /> Smart Packs
      </h3>
      {report ? (
        <>
          <p className="mt-2 text-[12.5px] text-fg-2">{report.ranking.summary}</p>
          <p className="mt-1 text-[12px] text-fg-3">
            {count('measured')} no ranking medido · {count('candidate')} para testar · {count('performance-risk')} com risco de perda · {count('not-recommended')} não recomendado{count('not-recommended') === 1 ? '' : 's'}
          </p>
          {report.ranking.nextTest && <p className="mt-2 text-[12px] text-fg-2">Próximo teste: {report.ranking.nextTest.reason}</p>}
        </>
      ) : (
        <p className="mt-2 text-[12.5px] text-fg-3">{loading ? 'Analisando combinações…' : 'Pesquise correções para montar os pacotes.'}</p>
      )}
      <Button size="sm" variant="outline" className="mt-4" onClick={onOpen}>
        Abrir Smart Packs
      </Button>
    </section>
  )
}
