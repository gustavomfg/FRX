import { useMemo } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { ArrowRight, Gamepad2, RefreshCw, ScanSearch } from 'lucide-react'
import { Page } from '@/layouts/Page'
import { Mascot } from '@/mascot/Mascot'
import { Button } from '@/components/ui/Button'
import { CoverImage } from '@/components/game/CoverImage'
import { InstallBadge } from '@/components/game/InstallBadge'
import { SourceTag } from '@/components/game/LibraryNotices'
import { ActivityList, BigNumber, SummaryColumn } from './Home'
import { useLibrary } from '@/state/library'
import { useFixReports } from '@/state/fixes'
import { usePerformance } from '@/state/performance'
import { useAppState } from '@/state/store'
import { INSTALL_LABEL } from '@/lib/steamGame'
import { fmtNum, fmtRelative, greeting } from '@/lib/format'
import type { InstallationStatus } from '@/types/steam'
import { cn } from '@/lib/cn'

const INSTALL_ORDER: InstallationStatus[] = ['installed', 'incomplete', 'unavailable', 'unknown']
const INSTALL_COLOR: Record<InstallationStatus, string> = {
  installed: 'var(--frx-good)',
  incomplete: 'var(--frx-caution)',
  unavailable: 'var(--frx-bad)',
  unknown: 'var(--frx-text-3)',
}

/** Início com dados reais da Steam. Mesma composição editorial da versão demo. */
export function RealHome() {
  const navigate = useNavigate()
  const { games, scan, phase, rescan, system } = useLibrary()
  const { reports } = useFixReports()
  const { activity } = useAppState()
  const { sessions } = usePerformance()
  const busy = phase === 'scanning' || phase === 'loading'
  const today = new Intl.DateTimeFormat('pt-BR', { weekday: 'long', day: 'numeric', month: 'long' }).format(new Date())

  const recent = useMemo(() => [...games].filter((g) => g.lastPlayed).sort((a, b) => +new Date(b.lastPlayed!) - +new Date(a.lastPlayed!))[0] ?? games[0], [games])
  // Um jogo pesquisado tem um pacote; descobertas não são contadas como melhorias.
  const researched = useMemo(() => Object.values(reports).map((r) => ({ appId: r.appId, name: r.gameName })), [reports])
  const latest = sessions[0]
  const realActivity = activity.filter((a) => !a.demo)

  return (
    <Page>
      <header className="flex items-end justify-between gap-6">
        <div>
          <p className="eyebrow mb-3 first-letter:uppercase">{today}</p>
          <h1 className="font-display text-[44px] leading-[1.02] font-bold tracking-[-0.045em]">
            {greeting()}.<span className="text-fg-3"> Vamos jogar?</span>
          </h1>
          <p className="mt-3 max-w-xl text-[15px] text-fg-2">
            {scan?.status === 'no-steam' ? (
              'Não encontramos a Steam neste usuário.'
            ) : (
              <>
                {games.length} jogo{games.length === 1 ? '' : 's'} da Steam na estante.{' '}
                {researched.length > 0 ? (
                  <>
                    <span className="text-fg">{researched.length} com pacote de melhorias</span> pronto para revisar.
                  </>
                ) : (
                  'Abra um jogo e busque melhorias.'
                )}
              </>
            )}
          </p>
        </div>
        <Button variant="outline" onClick={() => void rescan()} loading={busy} icon={<RefreshCw className="size-4" />}>
          {busy ? 'Verificando…' : 'Verificar biblioteca'}
        </Button>
      </header>

      {/* Destaque: o último jogo aberto */}
      <section aria-labelledby="destaque" className="relative mt-9 overflow-hidden rounded-xl border border-line bg-surface">
        <div className="grid min-h-[268px] grid-cols-[1.05fr_1fr]">
          <div className="relative z-10 flex flex-col justify-between p-8">
            {recent ? (
              <>
                <div>
                  <div className="flex items-center gap-2">
                    <span className="eyebrow text-accent!">{recent.lastPlayed ? 'Continue jogando' : 'Na sua estante'}</span>
                    <SourceTag />
                  </div>
                  <h2 id="destaque" className="mt-4 max-w-md font-display text-[28px] leading-[1.1] font-semibold tracking-[-0.03em]">
                    {recent.title}
                  </h2>
                  <p className="mt-3 max-w-md text-[14px] text-fg-2">
                    {recent.runner === 'Proton' ? `Roda com ${recent.protonVersion}` : 'Ferramenta de compatibilidade não identificada'}
                    {recent.lastPlayed && ` · jogado ${fmtRelative(recent.lastPlayed)}`}
                    {recent.playtimeH != null && recent.playtimeH > 0 && ` · ${recent.playtimeH < 1 ? `${Math.round(recent.playtimeH * 60)} min` : `${fmtNum(recent.playtimeH, 1)} h`} no total`}.
                  </p>
                </div>
                <div className="mt-6 flex items-center gap-2">
                  <Button variant="primary" icon={<ScanSearch className="size-4" />} onClick={() => navigate(`/biblioteca/${recent.id}`)}>
                    {reports[recent.id] ? 'Ver correções' : 'Pesquisar correções'}
                  </Button>
                  <Button variant="ghost" iconRight={<ArrowRight className="size-4" />} onClick={() => navigate('/biblioteca')}>
                    Abrir biblioteca
                  </Button>
                </div>
              </>
            ) : (
              <div>
                <span className="eyebrow">Destaque</span>
                <h2 id="destaque" className="mt-4 font-display text-[28px] leading-[1.1] font-semibold tracking-[-0.03em]">
                  {busy ? 'Procurando seus jogos…' : 'Nenhum jogo detectado ainda'}
                </h2>
                <p className="mt-3 max-w-md text-[14px] text-fg-2">Instale um jogo pela Steam e clique em “Verificar biblioteca”.</p>
              </div>
            )}
          </div>
          <div className="relative">
            {recent && (
              <div className="absolute inset-0">
                <CoverImage game={recent} kind="hero" variant="banner" />
              </div>
            )}
            <div className="absolute inset-0 bg-gradient-to-r from-surface via-surface/40 to-transparent" aria-hidden />
            <div className="absolute inset-x-0 bottom-0 h-16 bg-gradient-to-t from-surface/80 to-transparent" aria-hidden />
            <div className="absolute bottom-[10px] left-[18%]">
              <Mascot pose={recent ? 'wave' : 'confused'} scale={5} />
            </div>
          </div>
        </div>
        <div className="absolute inset-x-0 bottom-0 h-[10px] border-t-2 border-dashed border-line-2 bg-bg/60" aria-hidden />
      </section>

      <section className="mt-10 grid grid-cols-3 divide-x divide-line border-y border-line" aria-label="Resumo">
        <SummaryColumn eyebrow="Biblioteca" to="/biblioteca" cta="Abrir biblioteca">
          <BigNumber value={games.length} label="jogos encontrados" aside={scan ? `${scan.libraries.length} biblioteca${scan.libraries.length === 1 ? '' : 's'}` : undefined} />
          <InstallDistribution />
        </SummaryColumn>

        <SummaryColumn eyebrow="Melhorias" to="/biblioteca" cta="Ver jogos">
          <BigNumber value={researched.length} label={researched.length === 1 ? 'jogo pesquisado' : 'jogos pesquisados'} />
          {researched.length > 0 ? (
            <ul className="mt-5 space-y-2.5">
              {researched.map(({ appId, name }) => (
                <li key={appId}>
                  <Link to={`/biblioteca/${encodeURIComponent(appId)}`} className="group flex items-center justify-between gap-3 rounded-sm text-[13px]">
                    <span className="min-w-0 truncate text-fg group-hover:text-accent">{name}</span>
                    <span className="shrink-0 text-fg-3">Ver pacote</span>
                  </Link>
                </li>
              ))}
            </ul>
          ) : (
            <p className="mt-5 text-[13px] text-fg-3">Nenhuma pesquisa ainda. Abra um jogo e clique em “Buscar melhorias”.</p>
          )}
        </SummaryColumn>

        <SummaryColumn eyebrow="Performance Lab" to="/performance" cta="Abrir Performance Lab">
          {latest ? (
            <>
              <div className="flex items-baseline gap-3">
                <span className="tnum font-display text-[40px] leading-none font-bold tracking-[-0.04em] text-fg">{latest.stats.fps.avg == null ? '—' : fmtNum(latest.stats.fps.avg, 1)}</span>
                <span className="text-[13px] text-fg-2">FPS médio</span>
              </div>
              <p className="mt-1.5 text-[12.5px] text-fg-3">{latest.gameName ?? latest.label} · {sessions.length} sessão{sessions.length === 1 ? '' : 's'} registrada{sessions.length === 1 ? '' : 's'}</p>
            </>
          ) : <p className="mt-5 text-[13px] text-fg-3">Nenhum benchmark ainda. Importe um log ou prepare uma captura com o MangoHud.</p>}
        </SummaryColumn>
      </section>

      <div className="mt-10 grid grid-cols-[1.35fr_1fr] gap-12">
        <section aria-labelledby="atividade">
          <div className="mb-4 flex items-end justify-between">
            <h2 id="atividade" className="font-display text-[19px] font-semibold tracking-[-0.02em]">
              Atividade recente
            </h2>
            <span className="text-[12px] text-fg-3">nesta sessão</span>
          </div>
          <ActivityList items={realActivity.slice(0, 6)} />
        </section>

        <section aria-labelledby="sistema">
          <div className="mb-4 flex items-end justify-between">
            <h2 id="sistema" className="font-display text-[19px] font-semibold tracking-[-0.02em]">
              Seu sistema
            </h2>
            <SourceTag>Detectado</SourceTag>
          </div>
          <dl className="divide-y divide-line border-y border-line text-[13px]">
            {(
              [
                ['Distribuição', system?.distro],
                ['Kernel', system?.kernel],
                ['Sessão', [system?.desktop, system?.sessionType].filter(Boolean).join(' · ')],
                ['CPU', system?.cpu],
                ['GPU', system?.primaryGpu?.name ?? system?.primaryGpu?.vendor.toUpperCase()],
                ['Driver', system?.primaryGpu ? [system.primaryGpu.driver, system.primaryGpu.driverVersion].filter(Boolean).join(' ') : null],
                ['Proton padrão', scan?.globalDefaultTool?.displayName ?? 'Não definido'],
              ] as [string, string | null | undefined][]
            ).map(([k, v]) => (
              <div key={k} className="flex items-center justify-between gap-4 py-2.5">
                <dt className="text-fg-3">{k}</dt>
                <dd className="truncate text-right text-fg" title={v ?? undefined}>
                  {v || '—'}
                </dd>
              </div>
            ))}
          </dl>
          <p className="mt-3 flex items-center gap-2 text-[12px] text-fg-3">
            <Gamepad2 className="size-3.5" aria-hidden /> Lido de /proc, /sys e /etc/os-release, sem privilégios.
          </p>
        </section>
      </div>
    </Page>
  )
}

function InstallDistribution() {
  const { games } = useLibrary()
  const counts = INSTALL_ORDER.map((s) => ({ s, n: games.filter((g) => g.steam?.installationStatus === s).length })).filter((c) => c.n > 0)
  if (!counts.length) return null
  return (
    <div className="mt-5">
      <div className="flex h-2 gap-[2px] overflow-hidden rounded-[3px]" role="img" aria-label={counts.map((c) => `${c.n} ${INSTALL_LABEL[c.s]}`).join(', ')}>
        {counts.map((c) => (
          <div key={c.s} style={{ flex: c.n, background: INSTALL_COLOR[c.s] }} />
        ))}
      </div>
      <ul className="mt-3 grid grid-cols-2 gap-x-4 gap-y-1.5 text-[12.5px]">
        {counts.map((c) => (
          <li key={c.s} className={cn('flex items-center gap-2 text-fg-2')}>
            <InstallBadge status={c.s} className="px-0! bg-transparent!" />
            <span className="ml-auto tnum font-mono text-fg">{c.n}</span>
          </li>
        ))}
      </ul>
    </div>
  )
}
