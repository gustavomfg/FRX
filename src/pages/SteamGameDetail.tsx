import { useEffect, useState } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { motion } from 'motion/react'
import { ArrowLeft, ExternalLink, Gauge, ScanSearch } from 'lucide-react'
import { Page } from '@/layouts/Page'
import { CoverImage } from '@/components/game/CoverImage'
import { InstallBadge } from '@/components/game/InstallBadge'
import { InfoBlock, InfoRow } from '@/components/game/InfoBlock'
import { SourceTag } from '@/components/game/LibraryNotices'
import { ApplyFixDialog, type ApplyTarget } from '@/components/fix/ApplyFixDialog'
import { ExperiencePackSection } from '@/components/pack/ExperiencePack'
import { Button } from '@/components/ui/Button'
import { Disclosure } from '@/components/ui/Disclosure'
import { useLibrary } from '@/state/library'
import { useFixReports } from '@/state/fixes'
import { useFixEngine } from '@/state/fixEngine'
import { useToast } from '@/state/store'
import { openExternal } from '@/services/opener'
import { TOOL_SOURCE_LABEL, toolLabel } from '@/lib/steamGame'
import { fmtNum, fmtRelative } from '@/lib/format'
import { gameTechnologies } from '@/services/fixEngine'
import type { Game } from '@/types'

/** Jogo real detectado na Steam: buscar melhorias → um pacote → aplicar → desfazer. */
export function SteamGameDetail({ game }: { game: Game }) {
  const navigate = useNavigate()
  const s = game.steam!
  const c = s.compatibility
  const { system } = useLibrary()
  const fixes = useFixReports()
  const toast = useToast()
  const report = fixes.reports[game.id]
  const searching = !!fixes.searching[game.id]
  const engine = useFixEngine()
  const [target, setTarget] = useState<ApplyTarget | null>(null)

  const open = (url: string) => openExternal(url).catch(() => toast({ tone: 'error', title: 'Não foi possível abrir o link' }))
  const gpu = system?.primaryGpu?.name ?? system?.primaryGpu?.vendor.toUpperCase()

  return (
    <Page className="pt-6">
      <Link to="/biblioteca" className="inline-flex items-center gap-1.5 rounded-xs text-[13px] text-fg-3 transition-colors hover:text-fg">
        <ArrowLeft className="size-3.5" /> Biblioteca
      </Link>

      {/* Banner */}
      <section className="relative mt-4 overflow-hidden rounded-xl border border-line">
        <div className="relative h-[260px]">
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
                <SourceTag />
              </div>
            </div>
            <div className="flex shrink-0 items-center gap-2">
              <Button variant="secondary" icon={<Gauge className="size-4" />} onClick={() => navigate(`/performance?game=${game.id}`)}>
                Medir desempenho
              </Button>
              {!report && (
                <Button variant="primary" icon={<ScanSearch className="size-4" />} onClick={() => void fixes.search(game.id, game.title)} loading={searching}>
                  {searching ? 'Pesquisando…' : 'Buscar melhorias'}
                </Button>
              )}
            </div>
          </div>
        </div>
      </section>

      {/* Ambiente: o essencial para entender o pacote. */}
      <dl className="mt-6 grid grid-cols-3 divide-x divide-line border-y border-line">
        {[
          ['Proton', toolLabel(c.tool) ?? 'Não identificado'],
          ['GPU', gpu ?? 'Carregando…'],
          ['Driver', system?.primaryGpu ? [system.primaryGpu.driver, system.primaryGpu.driverVersion].filter(Boolean).join(' ') || '—' : '—'],
        ].map(([k, v]) => (
          <div key={k} className="min-w-0 px-5 py-4 first:pl-0">
            <dt className="text-[12px] text-fg-3">{k}</dt>
            <dd className="mt-1 truncate text-[15px] font-medium text-fg" title={v}>{v}</dd>
          </div>
        ))}
      </dl>
      <DetectedTechnologies appId={game.id} />

      <div className="mt-8">
        <ExperiencePackSection appId={game.id} title={game.title} onAct={setTarget} />
      </div>

      <Disclosure title="Jogo e sistema" className="mt-10 border-t border-line pt-6" titleClassName="font-display text-[16px] font-semibold tracking-[-0.02em] text-fg-2">
        <div className="mt-6 grid gap-10 md:grid-cols-2">
          <InfoBlock title="Instalação">
            <InfoRow k="Tempo de jogo" v={s.playtimeMinutes != null ? (s.playtimeMinutes < 60 ? `${s.playtimeMinutes} min` : `${fmtNum(s.playtimeMinutes / 60, 1)} h`) : '—'} />
            <InfoRow k="Última sessão" v={s.lastPlayed ? fmtRelative(s.lastPlayed) : 'Nunca'} />
            <InfoRow k="Tamanho" v={s.sizeOnDisk ? `${fmtNum(s.sizeOnDisk / 1e9, 1)} GB` : '—'} />
            <InfoRow k="Build" v={s.buildId ?? '—'} />
            <InfoRow k="Pasta do jogo" v={s.installPath ?? 'Desconhecida'} mono />
            <InfoRow k="Biblioteca" v={s.libraryPath} mono />
          </InfoBlock>
          <InfoBlock title="Proton e hardware">
            <InfoRow k="Origem do Proton" v={TOOL_SOURCE_LABEL[c.toolSource]} />
            {c.lastUsedTool && c.lastUsedTool.internalName !== c.tool?.internalName && <InfoRow k="Última execução" v={toolLabel(c.lastUsedTool)!} />}
            <InfoRow k="UMU-Protonfixes embutido" v={c.tool ? (c.tool.includesProtonfixes ? 'Sim' : 'Não') : '—'} />
            <InfoRow k="Prefixo" v={c.prefixPath ?? '—'} mono />
            {system && system.gpus.length > 1 && <InfoRow k="GPUs detectadas" v={system.gpus.map((g) => g.name ?? g.vendor).join(', ')} />}
            <InfoRow k="CPU" v={system?.cpu ?? '—'} />
            <InfoRow k="Sistema" v={system ? [system.distro, system.kernel && `kernel ${system.kernel}`].filter(Boolean).join(' · ') || '—' : '—'} />
            <div className="pt-3">
              <Button size="sm" variant="ghost" icon={<ExternalLink className="size-3.5" />} onClick={() => open(`https://www.protondb.com/app/${s.appId}`)}>
                ProtonDB
              </Button>
            </div>
          </InfoBlock>
        </div>
      </Disclosure>
      {target && <ApplyFixDialog target={target} onClose={() => { setTarget(null); void engine.refresh() }} />}
    </Page>
  )
}

const TECHNOLOGY_LABEL: Record<string, string> = {
  physx: 'PhysX (física)',
  dlss: 'DLSS',
  'ray-reconstruction': 'Ray reconstruction',
  'ray-tracing': 'Ray tracing',
  'frame-generation': 'Frame generation',
  xess: 'XeSS',
  fsr: 'FSR',
  'fsr-frame-generation': 'FSR frame generation',
}

/** Tecnologias que os arquivos do jogo trazem: só elas geram problemas e soluções nesta página. */
function DetectedTechnologies({ appId }: { appId: string }) {
  const [ids, setIds] = useState<string[] | null>(null)
  useEffect(() => {
    let alive = true
    void gameTechnologies(appId).then((list) => alive && setIds(list ?? [])).catch(() => alive && setIds([]))
    return () => {
      alive = false
    }
  }, [appId])
  return (
    <div className="mt-6">
      <p className="text-[12.5px] text-fg-3">Tecnologias detectadas nos arquivos do jogo</p>
      {ids === null ? (
        <p className="mt-2 text-[13px] text-fg-2">Verificando…</p>
      ) : ids.length ? (
        <ul className="mt-2 flex flex-wrap gap-2" aria-label="Tecnologias detectadas">
          {ids.map((id) => (
            <li key={id} className="rounded-xs border border-line px-2 py-1 text-[12px] text-fg-2">
              {TECHNOLOGY_LABEL[id] ?? id}
            </li>
          ))}
        </ul>
      ) : (
        <p className="mt-2 text-[13px] text-fg-2">Nenhuma tecnologia gráfica conhecida nos arquivos do jogo.</p>
      )}
    </div>
  )
}
