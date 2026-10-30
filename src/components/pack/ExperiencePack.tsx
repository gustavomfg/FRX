import { useCallback, useEffect, useState } from 'react'
import { motion } from 'motion/react'
import { ExternalLink, PackageCheck, RotateCcw, ScanSearch, ShieldCheck, TriangleAlert } from 'lucide-react'
import { Button } from '@/components/ui/Button'
import { Pill } from '@/components/ui/Status'
import { Disclosure } from '@/components/ui/Disclosure'
import { type ApplyTarget } from '@/components/fix/ApplyFixDialog'
import { OperationHistory } from '@/components/fix/OperationHistory'
import { Mascot } from '@/mascot/Mascot'
import { getExperiencePack } from '@/services/smartPacks'
import { errorMessage } from '@/services/native'
import { openExternal } from '@/services/opener'
import { useFixEngine } from '@/state/fixEngine'
import { useFixReports } from '@/state/fixes'
import { useToast } from '@/state/store'
import { fmtRelative } from '@/lib/format'
import type { ProviderStatus } from '@/types/fixes'
import type { ExperiencePack as Pack, LeftOut, LeftOutKind, PackAction } from '@/types/smartPacks'
import { cn } from '@/lib/cn'

const LEFT_OUT: Record<LeftOutKind, string> = {
  negative: 'Relatos dizem que não resolveu ou piorou',
  incompatible: 'Não vale para este hardware, Proton ou instalação',
  conflict: 'Alternativa a uma ação já incluída',
  risky: 'Risco conhecido de perder desempenho',
  'already-included': 'Já resolvido neste jogo',
  'weak-evidence': 'Evidência insuficiente',
  unsupported: 'O FRX ainda não sabe aplicar com segurança',
  'no-action': 'Relatos sem ação técnica ou de outro jogo',
}

const PROVIDER_STATE: Record<ProviderStatus['state'], { label: string; color: string }> = {
  ok: { label: 'Consultado agora', color: 'var(--frx-good)' },
  cached: { label: 'Do cache', color: 'var(--frx-info)' },
  stale: { label: 'Cache desatualizado', color: 'var(--frx-caution)' },
  'no-data': { label: 'Nada encontrado', color: 'var(--frx-text-3)' },
  unavailable: { label: 'Indisponível', color: 'var(--frx-bad)' },
  'rate-limited': { label: 'Limite de consultas', color: 'var(--frx-caution)' },
  disabled: { label: 'Desativado', color: 'var(--frx-text-3)' },
}

function status(p: Pack): { label: string; color: string; soft: string } {
  if (p.state === 'verified' || p.state === 'applied') return { label: 'Aplicado', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)' }
  if (p.state === 'failed' || p.state === 'partially_recovered') return { label: 'Precisa de atenção', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)' }
  if (p.availability.state === 'available') return { label: 'Pronto para aplicar', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)' }
  return { label: 'Aguardando', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)' }
}

/**
 * Experiência principal da página do jogo: buscar melhorias → um pacote
 * recomendado → preparar (prévia) → aplicar → desfazer. A investigação fica
 * nos detalhes; a interface só envia IDs ao núcleo.
 */
export function ExperiencePackSection({ appId, title, onAct }: { appId: string; title: string; onAct: (t: ApplyTarget) => void }) {
  const fixes = useFixReports()
  const engine = useFixEngine()
  const toast = useToast()
  const report = fixes.reports[appId]
  const searching = !!fixes.searching[appId]
  const [pack, setPack] = useState<Pack | null>(null)
  const [without, setWithout] = useState<string[]>([])
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const stamp = engine.operations.map((o) => o.id + o.state).join()

  const load = useCallback(() => {
    if (!engine.available) return
    setLoading(true)
    getExperiencePack(appId, without)
      .then((p) => {
        setPack(p)
        setError(null)
      })
      .catch((e) => setError(errorMessage(e)))
      .finally(() => setLoading(false))
  }, [appId, without, engine.available])

  useEffect(() => {
    if (report) load()
  }, [report, load, stamp])

  const search = (force: boolean) => fixes.search(appId, title, force)
  const undo = pack?.operationId ? engine.operations.find((o) => o.id === pack.operationId && o.state === 'applied') : undefined
  const prepare = () => pack?.applyRecipeId && onAct({ kind: 'apply', recipeId: pack.applyRecipeId, appId, title: `Complete Experience Pack · ${title}`, expectedFingerprint: pack.availability.planFingerprint ?? undefined })
  const toggle = (id: string) => setWithout((w) => (w.includes(id) ? w.filter((x) => x !== id) : [...w, id]))
  const open = (url: string) => openExternal(url).catch(() => toast({ tone: 'error', title: 'Não foi possível abrir o link' }))

  if (!report || (searching && !pack)) {
    return (
      <section aria-labelledby="pacote" aria-busy={searching} className="flex items-center gap-6 rounded-xl border border-dashed border-line-2 px-7 py-7">
        <Mascot pose={searching ? 'inspect' : 'idle'} scale={4} />
        <div className="flex-1">
          <h2 id="pacote" className="font-display text-[22px] font-semibold tracking-[-0.02em]">
            {searching ? 'Investigando melhorias…' : 'Buscar melhorias'}
          </h2>
          <p className="mt-1 max-w-xl text-[13.5px] text-fg-2">
            {searching
              ? 'Lendo GitHub, ValveSoftware/Proton, GE-Proton, UMU e a sua instalação. Só o que tem evidência e serve para o seu hardware entra no pacote.'
              : 'O FRX pesquisa o jogo, entende o seu hardware e o seu Proton e monta um pacote único com as melhorias compatíveis. Nada é alterado sem a sua confirmação.'}
          </p>
          {fixes.errors[appId] && <p className="mt-2 text-[13px] text-bad">A pesquisa falhou: {fixes.errors[appId]}</p>}
        </div>
        {!searching && (
          <Button variant="primary" size="lg" icon={<ScanSearch className="size-4" />} onClick={() => void search(false)}>
            Buscar melhorias
          </Button>
        )}
      </section>
    )
  }

  const selected = pack?.actions.filter((a) => a.selected) ?? []
  const s = pack && status(pack)
  const unsupported = pack?.leftOut.filter((l) => l.kind === 'unsupported').reduce((n, l) => n + l.count, 0) ?? 0
  const blocked = pack && pack.applyRecipeId && pack.availability.state !== 'available' && !undo

  return (
    <section aria-labelledby="pacote" className="space-y-5">
      <motion.div initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.25 }} className="rounded-xl border border-line bg-surface p-7">
        <div className="flex items-start gap-6">
          <Mascot pose={undo ? 'celebrate' : selected.length ? 'fix' : 'confused'} scale={4} className="shrink-0" />
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <h2 id="pacote" className="font-display text-[24px] font-semibold tracking-[-0.03em]">
                Complete Experience Pack
              </h2>
              {s && selected.length > 0 && (
                <Pill color={s.color} soft={s.soft}>
                  {s.label}
                </Pill>
              )}
            </div>
            {error ? (
              <p className="mt-2 text-[13.5px] text-bad" role="alert">Não foi possível montar o pacote: {error}</p>
            ) : !pack ? (
              <p className="mt-2 text-[13.5px] text-fg-2">Montando o pacote…</p>
            ) : selected.length ? (
              <>
                <p className="mt-2 max-w-2xl text-[14px] text-fg-2">
                  {selected.length} {selected.length === 1 ? 'melhoria compatível' : 'melhorias compatíveis'} para {pack.environment.join(' · ')}, sem conflitos entre si. Não há garantia de jogo perfeito: entra só o que tem evidência.
                </p>
                <ul className="mt-4 flex flex-wrap gap-2" aria-label="Áreas beneficiadas">
                  {pack.areas.map((a) => (
                    <li key={a} className="rounded-xs border border-accent-line px-2 py-1 text-[12px] text-accent">
                      {a}
                    </li>
                  ))}
                </ul>
              </>
            ) : (
              <p className="mt-2 max-w-2xl text-[14px] text-fg-2">
                Nenhuma melhoria tem evidência e suporte suficientes para entrar no pacote agora. O FRX não completa o pacote com ajustes sem comprovação; veja nos detalhes o que foi encontrado e por que ficou de fora.
              </p>
            )}
            {pack && pack.risks.length > 0 && selected.length > 0 && (
              <p className="mt-3 flex items-start gap-2 text-[12.5px] text-caution">
                <TriangleAlert className="mt-0.5 size-3.5 shrink-0" aria-hidden /> {pack.risks[0]}
                {pack.risks.length > 1 && ` (+${pack.risks.length - 1} nos detalhes)`}
              </p>
            )}
            {pack && unsupported > 0 && (
              <p className="mt-2 text-[12.5px] text-fg-3">
                Pacote parcial: {unsupported} {unsupported === 1 ? 'possível melhoria ficou' : 'possíveis melhorias ficaram'} de fora porque o FRX ainda não sabe aplicá-las com segurança.
              </p>
            )}
            {blocked && <p className="mt-2 text-[12.5px] text-caution">{pack.availability.reasons.map((r) => r.message).join(' ')}</p>}
            {pack && <p className="mt-3 text-[12px] text-fg-3">Pesquisado {fmtRelative(report.retrievedAt)}{report.fromCache ? ' · inclui cache' : ''}</p>}
          </div>
          <div className="flex shrink-0 flex-col items-end gap-2">
            {undo ? (
              <Button variant="outline" size="lg" icon={<RotateCcw className="size-4" />} onClick={() => onAct({ kind: 'rollback', operation: undo })}>
                Desfazer pacote
              </Button>
            ) : (
              selected.length > 0 && (
                <Button variant="primary" size="lg" icon={<PackageCheck className="size-4" />} disabled={!!blocked} onClick={prepare}>
                  Preparar pacote
                </Button>
              )
            )}
            <Button size="sm" variant="ghost" loading={searching || loading} onClick={() => void search(true)}>
              Buscar de novo
            </Button>
          </div>
        </div>
      </motion.div>

      {pack && (
        <Disclosure title="Detalhes técnicos" className="rounded-lg border border-line px-6 py-4" titleClassName="text-[14px] font-medium">
          <div className="mt-5 space-y-8">
            {pack.actions.length > 0 && (
              <div>
                <h3 className="text-[13px] font-semibold text-fg">O que o pacote altera</h3>
                <p className="mt-1 text-[12px] text-fg-3">Desmarque uma ação para tirá-la do pacote. Cada ação já foi validada sozinha e em conjunto.</p>
                <ul className="mt-3 space-y-2">
                  {pack.actions.map((a) => (
                    <ActionRow key={a.recipeId} a={a} onToggle={() => toggle(a.recipeId)} onOpen={open} />
                  ))}
                </ul>
                {pack.compatibility.length > 0 && (
                  <ul className="mt-3 space-y-1 text-[12px] text-fg-3">
                    {pack.compatibility.map((c) => (
                      <li key={c}>{c}</li>
                    ))}
                  </ul>
                )}
                {pack.risks.length > 0 && (
                  <ul className="mt-3 space-y-1 text-[12px] text-caution">
                    {pack.risks.map((r) => (
                      <li key={r}>{r}</li>
                    ))}
                  </ul>
                )}
              </div>
            )}
            {pack.leftOut.length > 0 && <LeftOutList items={pack.leftOut} />}
            <div>
              <h3 className="flex items-center gap-2 text-[13px] font-semibold text-fg">
                <ShieldCheck className="size-3.5 text-accent" aria-hidden /> Backup e desfazer
              </h3>
              <p className="mt-1 text-[12px] text-fg-3">O pacote é aplicado numa única operação: uma prévia, um backup conferido e uma gravação. Desfazer remove só o que o FRX adicionou, mesmo depois de reiniciar.</p>
              <div className="mt-3">
                <OperationHistory appId={appId} onAct={onAct} compact />
              </div>
            </div>
            <div>
              <h3 className="text-[13px] font-semibold text-fg">Fontes consultadas</h3>
              <ProviderStatusList providers={report.providers} />
            </div>
          </div>
        </Disclosure>
      )}
    </section>
  )
}

function ActionRow({ a, onToggle, onOpen }: { a: PackAction; onToggle: () => void; onOpen: (url: string) => void }) {
  return (
    <li className={cn('rounded-md border border-line bg-bg px-4 py-3', !a.selected && 'opacity-55')}>
      <div className="flex items-start gap-3">
        <input type="checkbox" checked={a.selected} onChange={onToggle} aria-label={`Incluir ${a.title}`} className="mt-1 size-4 accent-(--frx-accent)" />
        <div className="min-w-0 flex-1">
          <p className="text-[13.5px] font-medium text-fg">
            {a.title} <span className="font-normal text-fg-3">· {a.area}</span>
          </p>
          {a.effects[0] && <p className="mt-0.5 text-[12.5px] text-fg-2">{a.effects[0]}</p>}
          <p className="mt-1.5 flex flex-wrap gap-1.5">
            {a.changes.map((c) => (
              <code key={c} className="rounded-xs border border-line px-1.5 py-0.5 font-mono text-[11.5px] text-fg-2">
                {c}
              </code>
            ))}
          </p>
          <p className="mt-1.5 text-[12px] text-fg-3">
            {a.evidenceNote}
            {(a.positive > 0 || a.negative > 0) && ` · ${a.positive} relato(s) dizem que resolveu, ${a.negative} que não`}
          </p>
          {a.sources.length > 0 && (
            <ul className="mt-1.5 flex flex-wrap gap-x-3 gap-y-1 text-[12px]">
              {a.sources.slice(0, 6).map((s) => (
                <li key={s.url}>
                  <button type="button" onClick={() => onOpen(s.url)} className="inline-flex items-center gap-1 text-fg-2 underline-offset-2 hover:text-fg hover:underline">
                    {s.label} <ExternalLink className="size-3" aria-hidden />
                  </button>
                </li>
              ))}
              {a.sources.length > 6 && <li className="text-fg-3">+{a.sources.length - 6} fontes</li>}
            </ul>
          )}
        </div>
      </div>
    </li>
  )
}

/** O que ficou fora, agrupado pelo motivo: dez relatos iguais são uma linha. */
function LeftOutList({ items }: { items: LeftOut[] }) {
  const groups = Object.entries(LEFT_OUT)
    .map(([kind, label]) => ({ kind, label, rows: items.filter((i) => i.kind === kind) }))
    .filter((g) => g.rows.length)
  const total = items.reduce((n, i) => n + i.count, 0)
  return (
    <div>
      <h3 className="text-[13px] font-semibold text-fg">Fora do pacote · {total} {total === 1 ? 'descoberta' : 'descobertas'}</h3>
      <div className="mt-3 space-y-4">
        {groups.map((g) => (
          <div key={g.kind}>
            <p className="text-[12.5px] text-fg-2">{g.label}</p>
            <ul className="mt-1.5 space-y-1 text-[12px] text-fg-3">
              {g.rows.map((r) => (
                <li key={`${r.title}|${r.changes.join()}|${r.reason}`}>
                  <span className="text-fg-2">{r.changes.length ? r.changes.join(' ') : r.title}</span>
                  {r.count > 1 && ` ×${r.count}`} — {r.reason}
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </div>
  )
}

function ProviderStatusList({ providers }: { providers: ProviderStatus[] }) {
  return (
    <ul className="mt-2 flex flex-wrap gap-2" aria-label="Situação das fontes">
      {providers.map((p) => {
        const m = PROVIDER_STATE[p.state]
        return (
          <li key={p.id} className="inline-flex h-7 items-center gap-2 rounded-sm border border-line px-2.5 text-[12px]" title={p.message ?? undefined}>
            <span className="size-1.5" style={{ background: m.color }} aria-hidden />
            <span className="text-fg">{p.name}</span>
            <span className="text-fg-3">{m.label}</span>
          </li>
        )
      })}
    </ul>
  )
}
