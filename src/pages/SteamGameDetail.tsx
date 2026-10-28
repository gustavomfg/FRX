import { useCallback, useState } from 'react'
import { Link, useNavigate, useSearchParams } from 'react-router-dom'
import { motion } from 'motion/react'
import { ArrowLeft, ExternalLink, Gauge, RefreshCw, ScanSearch, Wifi } from 'lucide-react'
import { Page } from '@/layouts/Page'
import { CoverImage } from '@/components/game/CoverImage'
import { InstallBadge } from '@/components/game/InstallBadge'
import { InfoBlock, InfoRow } from '@/components/game/InfoBlock'
import { SourceTag } from '@/components/game/LibraryNotices'
import { ProviderStatusList } from '@/components/fix/FindingCard'
import { ApplyFixDialog, type ApplyTarget } from '@/components/fix/ApplyFixDialog'
import { OperationHistory } from '@/components/fix/OperationHistory'
import { RecipesPanel } from '@/components/fix/RecipesPanel'
import { TechnicalFindingsBrowser } from '@/components/fix/TechnicalFindingsBrowser'
import { useAssessments } from '@/components/fix/useAssessments'
import { Button } from '@/components/ui/Button'
import { ActionableFindings } from '@/components/fix/ActionableFindings'
import { Mascot } from '@/mascot/Mascot'
import { useLibrary } from '@/state/library'
import { useFixReports } from '@/state/fixes'
import { useFixEngine } from '@/state/fixEngine'
import { useToast } from '@/state/store'
import { openExternal } from '@/services/opener'
import { TOOL_SOURCE_LABEL, toolLabel } from '@/lib/steamGame'
import { fmtNum, fmtRelative } from '@/lib/format'
import type { Game } from '@/types'
import { cn } from '@/lib/cn'
import { SmartPacksPanel } from '@/components/smart/SmartPacksPanel'
import { ResolutionCenter } from '@/components/resolution/ResolutionCenter'

/** Detalhes de um jogo real detectado na Steam. Mesmo desenho da página demonstrativa. */
export function SteamGameDetail({ game }: { game: Game }) {
  const navigate = useNavigate()
  // Sugestões deste jogo ficam nesta página: a seção aberta vai na própria URL.
  const [params, setParams] = useSearchParams()
  const secao = params.get('secao')
  const update = useCallback((patch: Record<string, string | null>) => setParams((prev) => {
    const next = new URLSearchParams(prev)
    for (const [key, value] of Object.entries(patch)) {
      if (value) next.set(key, value)
      else next.delete(key)
    }
    return next
  }, { replace: true }), [setParams])
  const showSection = (name: string | null) => update({ secao: name, pack: null, recipe: null, finding: null })
  const s = game.steam!
  const c = s.compatibility
  const { system } = useLibrary()
  const fixes = useFixReports()
  const toast = useToast()
  const report = fixes.reports[game.id]
  const searching = !!fixes.searching[game.id]
  const error = fixes.errors[game.id]
  const engine = useFixEngine()
  const [target, setTarget] = useState<ApplyTarget | null>(null)
  const analysis = useAssessments(game.id)

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

      {secao ? (
        <div className="mt-10 space-y-6">
          <button type="button" onClick={() => showSection(null)} className="inline-flex items-center gap-1.5 rounded-xs text-[13px] text-fg-3 transition-colors hover:text-fg">
            <ArrowLeft className="size-3.5" /> Visão do jogo
          </button>
          {secao === 'smart' && <SmartPacksPanel appId={game.id} selectedPackId={params.get('pack')} onAct={setTarget} />}
          {secao === 'receitas' && (
            <RecipesPanel games={[game]} gameSelector={false} appId={game.id} recipeId={params.get('recipe')} onSelect={(p) => update({ recipe: p.recipe ?? null })} onAct={setTarget} onShowFinding={(findingId) => update({ secao: 'descobertas', finding: findingId })} />
          )}
          {secao === 'descobertas' && <TechnicalFindingsBrowser gameFilter={game.id} selectedId={params.get('finding')} update={update} onAct={setTarget} />}
        </div>
      ) : (
        <>
                    {/* Correções descobertas */}
          <section aria-labelledby="descobertas" aria-busy={searching}>
            <div className="mb-4 flex items-end justify-between gap-4">
              <div>
                <h2 id="descobertas" className="font-display text-[20px] font-semibold tracking-[-0.02em]">
                  Soluções para este jogo
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
              <div className="flex flex-wrap items-center gap-2">
                {report && (
                  <Button size="sm" variant="outline" icon={<RefreshCw className={cn('size-3.5', searching && 'animate-spin')} />} onClick={() => void search(true)} disabled={searching}>
                    Pesquisar de novo
                  </Button>
                )}
                <Button size="sm" variant="ghost" onClick={() => showSection('smart')}>Smart Packs</Button>
                <Button size="sm" variant="ghost" onClick={() => showSection('receitas')}>Catálogo técnico</Button>
                {report && <Button size="sm" variant="ghost" onClick={() => showSection('descobertas')}>Pesquisa técnica</Button>}
              </div>
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
                <ActionableFindings appId={game.id} findings={report.findings} assessments={analysis.byFinding} loading={analysis.loading} error={analysis.error} refresh={analysis.refresh} onApply={(a, f) => setTarget({ kind: 'apply', expectedFingerprint: a.availability?.planFingerprint ?? undefined, recipeId: a.recipeId!, appId: game.id, title: a.recipe?.title ?? f.title })} />
              </>
            )}
            <details className="mt-8 border-t border-line pt-5">
              <summary className="cursor-pointer text-[14px] font-medium">Histórico de aplicações</summary>
              <div className="mt-4">
                <OperationHistory appId={game.id} onAct={setTarget} compact />
              </div>
            </details>
          </section>

          <details className="mt-10 border-t border-line pt-6">
            <summary className="cursor-pointer font-display text-[18px] font-semibold tracking-[-0.02em]">Diagnóstico e detalhes técnicos</summary>
            <div className="mt-8 space-y-10">
              <ResolutionCenter appId={game.id} available={engine.available} />
              <div className="grid gap-10 md:grid-cols-2">
                <div className="min-w-0 space-y-9">
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
                <div className="min-w-0 space-y-9">
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

                </div>
              </div>
            </div>
          </details>
        </>
      )}
      {target && <ApplyFixDialog target={target} onClose={() => { setTarget(null); void engine.refresh(); analysis.refresh() }} />}
    </Page>
  )
}
