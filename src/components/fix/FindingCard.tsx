import { useState } from 'react'
import { AnimatePresence, motion } from 'motion/react'
import { ChevronDown, ExternalLink, FlaskConical, Lock, Package, Rocket, SquareTerminal, KeyRound, FileCode2, Layers, CircleHelp, ThumbsDown, ThumbsUp } from 'lucide-react'
import type { FixAction, FixFinding, ProviderStatus } from '@/types/fixes'
import { SourceIcon } from './SourceIcon'
import { CATEGORY, COMPATIBILITY, CONFIDENCE, PROVIDER_STATE } from './findingMeta'
import { Button } from '@/components/ui/Button'
import { openExternal } from '@/services/opener'
import { useToast } from '@/state/store'
import { fmtDate, fmtRelative } from '@/lib/format'
import { cn } from '@/lib/cn'
import type { SourceKind } from '@/types'

const PROVIDER_ICON: Record<string, SourceKind> = {
  'umu-protonfixes': 'umu-protonfixes',
  'umu-database': 'umu-protonfixes',
  github: 'github',
  protondb: 'protondb',
}

const ACTION_ICON: Record<FixAction['kind'], typeof Package> = {
  winetricks: Package,
  env: SquareTerminal,
  'dll-override': Layers,
  'launch-option': Rocket,
  'config-file': FileCode2,
  registry: KeyRound,
  'proton-version': Layers,
  other: CircleHelp,
}

export function Pill({ color, soft, children, title }: { color: string; soft: string; children: React.ReactNode; title?: string }) {
  return (
    <span className="inline-flex h-6 items-center gap-1.5 rounded-xs px-2 text-[12px] font-medium whitespace-nowrap" style={{ background: soft, color }} title={title}>
      {children}
    </span>
  )
}

export function ConfidencePill({ finding }: { finding: FixFinding }) {
  const c = CONFIDENCE[finding.confidence]
  return (
    <Pill color={c.color} soft={c.soft} title={c.help}>
      <span className="size-1.5" style={{ background: c.color }} aria-hidden />
      {c.label}
    </Pill>
  )
}

export function CompatPill({ finding }: { finding: FixFinding }) {
  const c = COMPATIBILITY[finding.compatibility]
  return (
    <Pill color={c.color} soft={c.soft} title={finding.compatibilityNote}>
      {c.label}
    </Pill>
  )
}

function useOpen() {
  const toast = useToast()
  return (url: string) => openExternal(url).catch((e) => toast({ tone: 'error', title: 'Não foi possível abrir o link', body: String(e?.message ?? e) }))
}

/**
 * Uma descoberta de correção. Apenas leitura: a aplicação automática fica
 * desabilitada até existir o sistema de backup e validação.
 */
export function FindingCard({ finding, defaultOpen = false }: { finding: FixFinding; defaultOpen?: boolean }) {
  const [open, setOpen] = useState(defaultOpen)
  const openUrl = useOpen()
  const id = `finding-${finding.id}`
  const dim = finding.compatibility === 'incompatible'
  return (
    <li className={cn('rounded-lg border bg-surface transition-colors', dim ? 'border-line opacity-70' : 'border-line hover:border-line-2')}>
      <button onClick={() => setOpen((o) => !o)} aria-expanded={open} aria-controls={id} className="group flex w-full items-start gap-4 px-5 py-4 text-left">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="text-[15px] font-medium text-fg">{finding.title}</h3>
            {finding.experimental && (
              <span className="inline-flex h-5 items-center gap-1 rounded-xs bg-caution-soft px-1.5 text-[11.5px] font-medium text-caution">
                <FlaskConical className="size-3" aria-hidden /> Experimental
              </span>
            )}
          </div>
          {!open && <p className="mt-1 line-clamp-2 text-[13px] text-fg-2">{finding.description}</p>}
          <div className="mt-3 flex flex-wrap items-center gap-2">
            <ConfidencePill finding={finding} />
            <CompatPill finding={finding} />
            <span className="text-[12px] text-fg-3">{CATEGORY[finding.category]}</span>
            <span className="flex items-center gap-1.5 text-[12px] text-fg-3">
              {[...new Set(finding.sources.map((s) => s.provider))].map((p) => (
                <SourceIcon key={p} kind={PROVIDER_ICON[p] ?? 'wiki'} />
              ))}
              {finding.sources.length} fonte{finding.sources.length === 1 ? '' : 's'}
            </span>
          </div>
        </div>
        <ChevronDown className={cn('mt-1 size-4 text-fg-3 transition-transform duration-(--frx-dur) ease-frx group-hover:text-fg', open && 'rotate-180')} aria-hidden />
      </button>

      <AnimatePresence initial={false}>
        {open && (
          <motion.div id={id} initial={{ height: 0, opacity: 0 }} animate={{ height: 'auto', opacity: 1 }} exit={{ height: 0, opacity: 0 }} transition={{ duration: 0.24, ease: [0.2, 0.8, 0.2, 1] }} className="overflow-hidden">
            <div className="space-y-5 border-t border-line px-5 pt-4 pb-5">
              <p className="text-[13.5px] text-fg-2">{finding.description}</p>

              {finding.compatibilityNote && <p className="text-[12.5px] text-fg-3">{finding.compatibilityNote}</p>}

              {finding.analysis !== 'parsed' && (
                <p className="rounded-md border border-caution/25 bg-caution-soft px-3.5 py-2.5 text-[12.5px] text-caution">
                  {finding.analysis === 'partial'
                    ? 'Parte do arquivo original não pôde ser interpretada com segurança. Confira a fonte antes de seguir qualquer passo.'
                    : 'A descrição técnica não foi verificada — veja o arquivo original.'}
                </p>
              )}

              {finding.actions.length > 0 && (
                <div>
                  <div className="eyebrow mb-2.5">O que a correção faz (análise estática)</div>
                  <ul className="divide-y divide-line overflow-hidden rounded-md border border-line bg-bg/50">
                    {finding.actions.map((a, i) => {
                      const I = ACTION_ICON[a.kind]
                      return (
                        <li key={i} className="flex items-center gap-2.5 px-3.5 py-2.5">
                          <I className="size-3.5 shrink-0 text-fg-3" aria-hidden />
                          <span className="min-w-0 flex-1 text-[13px] text-fg">{a.label}</span>
                          <code className="max-w-[45%] truncate font-mono text-[12px] text-fg-2" title={a.value}>
                            {a.value}
                          </code>
                          {a.conditional && <span className="shrink-0 text-[11px] text-fg-3">condicional</span>}
                        </li>
                      )
                    })}
                  </ul>
                </div>
              )}

              {(finding.requirements.gpuVendor || finding.requirements.protonVersions.length > 0) && (
                <dl className="flex flex-wrap gap-x-6 gap-y-1 text-[12.5px]">
                  {finding.requirements.gpuVendor && (
                    <div className="flex gap-2">
                      <dt className="text-fg-3">GPU citada</dt>
                      <dd className="text-fg uppercase">{finding.requirements.gpuVendor}</dd>
                    </div>
                  )}
                  {finding.requirements.protonVersions.length > 0 && (
                    <div className="flex gap-2">
                      <dt className="text-fg-3">Versões citadas</dt>
                      <dd className="text-fg">{finding.requirements.protonVersions.join(', ')}</dd>
                    </div>
                  )}
                </dl>
              )}

              {(finding.evidence.positive > 0 || finding.evidence.negative > 0) && (
                <p className="flex items-center gap-4 text-[12.5px] text-fg-2">
                  <span className="flex items-center gap-1.5">
                    <ThumbsUp className="size-3.5 text-good" aria-hidden /> {finding.evidence.positive} menç{finding.evidence.positive === 1 ? 'ão favorável' : 'ões favoráveis'}
                  </span>
                  {finding.evidence.negative > 0 && (
                    <span className="flex items-center gap-1.5">
                      <ThumbsDown className="size-3.5 text-bad" aria-hidden /> {finding.evidence.negative} dizendo que não resolveu
                    </span>
                  )}
                </p>
              )}

              <div>
                <div className="eyebrow mb-2.5">Fontes</div>
                <ul className="space-y-2">
                  {finding.sources.map((s, i) => (
                    <li key={i} className="rounded-md border border-line bg-bg/40 px-3.5 py-2.5">
                      <div className="flex items-center gap-2.5">
                        <SourceIcon kind={PROVIDER_ICON[s.provider] ?? 'wiki'} className="text-fg-3" />
                        <span className="min-w-0 flex-1 truncate text-[13px] text-fg">{s.label}</span>
                        {s.publishedAt && <span className="shrink-0 text-[11.5px] text-fg-3">{fmtDate(s.publishedAt)}</span>}
                        <button onClick={() => openUrl(s.url)} className="inline-flex shrink-0 items-center gap-1 rounded-xs text-[12px] font-medium text-fg-2 hover:text-accent" aria-label={`Abrir fonte: ${s.label}`}>
                          Abrir <ExternalLink className="size-3" />
                        </button>
                      </div>
                      {s.snippet && <p className="mt-1.5 line-clamp-3 border-l-2 border-line-2 pl-2.5 font-mono text-[11.5px] break-words text-fg-3">{s.snippet}</p>}
                    </li>
                  ))}
                </ul>
              </div>

              <div className="flex items-center justify-between gap-4 border-t border-line pt-4">
                <span className="text-[12px] text-fg-3">Consultado {fmtRelative(finding.retrievedAt)}</span>
                <span title="A aplicação automática chega em uma versão futura, com backup e reversão.">
                  <Button size="sm" variant="secondary" disabled icon={<Lock className="size-3.5" />}>
                    Aplicar · em breve
                  </Button>
                </span>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </li>
  )
}

export function ProviderStatusList({ providers }: { providers: ProviderStatus[] }) {
  return (
    <ul className="flex flex-wrap gap-2" aria-label="Situação das fontes">
      {providers.map((p) => {
        const m = PROVIDER_STATE[p.state]
        return (
          <li key={p.id} className="inline-flex h-7 items-center gap-2 rounded-sm border border-line px-2.5 text-[12px]" title={p.message ?? undefined}>
            <span className="size-1.5" style={{ background: m.color }} aria-hidden />
            <span className="text-fg">{p.name}</span>
            <span className="text-fg-3">
              {m.label}
              {p.state === 'rate-limited' && p.resetAt ? ` até ${new Date(p.resetAt).toLocaleTimeString('pt-BR', { hour: '2-digit', minute: '2-digit' })}` : ''}
            </span>
          </li>
        )
      })}
    </ul>
  )
}
