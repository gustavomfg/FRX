import { useCallback, useEffect, useRef, useState } from 'react'
import { useSearchParams } from 'react-router-dom'
import { FlaskConical, RefreshCw, RotateCcw, TerminalSquare } from 'lucide-react'
import { Page } from '@/layouts/Page'
import { Mascot } from '@/mascot/Mascot'
import { Button } from '@/components/ui/Button'
import { EmptyState } from '@/components/ui/EmptyState'
import { Pill } from '@/components/ui/Status'
import { SuggestionCard, SUPPORT } from '@/components/hardware/SuggestionCard'
import { isSymptom, SYMPTOMS } from '@/components/hardware/symptoms'
import { SystemFixDialog, type SystemFixTarget } from '@/components/hardware/SystemFixDialog'
import { OP_STATE } from '@/components/fix/engineMeta'
import { searchHardwareFixes, setDevLaptop } from '@/services/hardware'
import { listFixOperations } from '@/services/fixEngine'
import { isTauri } from '@/services/platform'
import { errorMessage } from '@/services/native'
import { openExternal } from '@/services/opener'
import { laptopName, useHardware } from '@/state/hardware'
import { useSettings, useToast } from '@/state/store'
import { greeting } from '@/lib/format'
import { cn } from '@/lib/cn'
import type { HardwareReport, HardwareSymptom } from '@/types/hardware'
import type { Operation } from '@/types/fixEngine'

/** Início: o laptop detectado e os sintomas. O sintoma escolhido fica na URL (`?sintoma=`). */
export function DashboardPage() {
  const { identity, system, loading, error, reload } = useHardware()
  const [dialog, setDialog] = useState<SystemFixTarget | null>(null)
  const [opsVersion, setOpsVersion] = useState(0)
  const [params, setParams] = useSearchParams()
  const raw = params.get('sintoma')
  const symptom = isSymptom(raw) ? raw : null

  const laptop = laptopName(identity)
  const gpu = system?.primaryGpu
  const gpuName = gpu ? (gpu.name ?? gpu.vendor.toUpperCase()) : null
  const summary = [laptop ?? 'Modelo não identificado', gpuName ?? 'GPU não identificada', system?.kernel ? `Linux ${system.kernel}` : 'Kernel desconhecido']

  return (
    <Page>
      <header className="flex items-end justify-between gap-6">
        <div className="min-w-0">
          <p className="eyebrow mb-3">Hardware Fixer</p>
          <h1 className="font-display text-[44px] leading-[1.02] font-bold tracking-[-0.045em]">
            {greeting()}.<span className="text-fg-3"> O que não está funcionando?</span>
          </h1>
          <div className="mt-4 text-[15px] text-fg-2" aria-live="polite">
            {loading ? (
              <span className="inline-block h-5 w-[420px] max-w-full rounded-sm skeleton" aria-label="Lendo o sistema" />
            ) : error ? (
              <span className="flex flex-wrap items-center gap-3">
                <span className="text-bad">Não foi possível ler o sistema: {error}</span>
                <Button size="sm" variant="outline" icon={<RefreshCw className="size-3.5" />} onClick={reload}>
                  Tentar de novo
                </Button>
              </span>
            ) : (
              <p>
                <span className="text-fg-3">Seu sistema: </span>
                {summary.map((part, i) => (
                  <span key={i}>
                    {i > 0 && <span className="text-fg-3"> · </span>}
                    <span className={i === 0 ? 'font-medium text-fg' : undefined}>{part}</span>
                  </span>
                ))}
              </p>
            )}
            {identity?.simulated && (
              <p className="mt-2">
                <Pill color="var(--frx-caution)" soft="var(--frx-caution-soft)">
                  Laptop de teste (dev) — o DMI real não está sendo usado
                </Pill>
              </p>
            )}
            {identity?.vendor && !loading && !error && (
              <p className="mt-1 text-[12.5px] text-fg-3">
                {identity.vendor}
                {identity.boardName && identity.boardName !== laptop ? ` · placa ${identity.boardName}` : ''}
                {identity.model && identity.model !== laptop ? ` · ${identity.model}` : ''}
              </p>
            )}
          </div>
        </div>
        <Mascot pose={symptom ? 'inspect' : 'wave'} scale={5} className="hidden shrink-0 md:block" />
      </header>

      <section aria-labelledby="sintomas" className="mt-10">
        <h2 id="sintomas" className="eyebrow mb-3">
          Sintomas
        </h2>
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5" role="radiogroup" aria-label="Sintoma">
          {SYMPTOMS.map((s) => {
            const active = s.id === symptom
            return (
              <button
                key={s.id}
                role="radio"
                aria-checked={active}
                onClick={() => setParams(active ? {} : { sintoma: s.id })}
                className={cn(
                  'group flex flex-col items-start gap-3 rounded-lg border p-4 text-left transition-colors duration-(--frx-dur-fast)',
                  active ? 'border-accent-line bg-surface' : 'border-line hover:border-line-3 hover:bg-surface/60',
                )}
              >
                <s.icon className={cn('size-5', active ? 'text-accent' : 'text-fg-3 group-hover:text-fg-2')} strokeWidth={1.9} aria-hidden />
                <span>
                  <span className="block text-[14px] font-medium text-fg">{s.label}</span>
                  <span className="mt-0.5 block text-[12.5px] text-fg-3">{s.hint}</span>
                </span>
              </button>
            )
          })}
        </div>
      </section>

      {import.meta.env.DEV && isTauri() && <DevLaptopSwitch simulated={!!identity?.simulated} onChange={reload} />}

      <SystemFixes version={opsVersion} onOpen={setDialog} />

      {symptom && <Results key={`${symptom}:${laptop}`} symptom={symptom} laptop={laptop} onApply={isTauri() ? setDialog : undefined} />}

      <SystemFixDialog target={dialog} onClose={() => setDialog(null)} onChanged={() => setOpsVersion((v) => v + 1)} />
    </Page>
  )
}

function Results({ symptom, laptop, onApply }: { symptom: HardwareSymptom; laptop: string | null; onApply?: (t: SystemFixTarget) => void }) {
  const { fixCacheHours } = useSettings()
  const toast = useToast()
  const [report, setReport] = useState<HardwareReport | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [running, setRunning] = useState(false)
  const alive = useRef(true)
  const label = SYMPTOMS.find((s) => s.id === symptom)!.label

  const run = (force: boolean) => {
    setRunning(true)
    setError(null)
    searchHardwareFixes({ symptom, force, maxAgeHours: fixCacheHours })
      .then((r) => alive.current && setReport(r))
      .catch((e) => alive.current && setError(errorMessage(e)))
      .finally(() => alive.current && setRunning(false))
  }

  useEffect(() => {
    alive.current = true
    run(false)
    return () => {
      alive.current = false
    }
  }, [])

  const open = (url: string) => void openExternal(url).catch(() => toast({ tone: 'error', title: 'Não foi possível abrir o link' }))
  const support = report?.support ? SUPPORT[report.support] : null

  return (
    <section aria-labelledby="resultados" aria-busy={running} className="mt-10">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 id="resultados" className="font-display text-[22px] font-semibold tracking-[-0.02em]">
            {label}
          </h2>
          <p className="mt-1 text-[13px] text-fg-2">
            {report?.modelPage ? (
              <>
                Página da Arch Wiki do seu modelo: <span className="text-fg">{report.modelPage}</span>
              </>
            ) : (
              'Sugestões da Arch Wiki para o seu laptop.'
            )}
          </p>
        </div>
        <div className="flex items-center gap-2">
          {support && (
            <Pill color={support.color} soft={support.soft} title="Coluna “Working?” da tabela de hardware da página do modelo">
              {support.label}
            </Pill>
          )}
          <Button size="sm" variant="ghost" icon={<RefreshCw className="size-3.5" />} loading={running} onClick={() => run(true)}>
            Pesquisar de novo
          </Button>
        </div>
      </div>

      {report?.status.message && <p className="mt-3 text-[12.5px] text-fg-3">{report.status.message}</p>}

      {running && !report ? (
        <div className="mt-5 space-y-3" aria-label="Pesquisando na Arch Wiki">
          {[0, 1].map((i) => (
            <div key={i} className="h-[92px] rounded-sm skeleton" />
          ))}
        </div>
      ) : error ? (
        <EmptyState pose="worried" title="A pesquisa falhou" body={error} action={<Button onClick={() => run(true)}>Tentar de novo</Button>} />
      ) : report && report.suggestions.length === 0 ? (
        <EmptyState title="Nada encontrado" body="A Arch Wiki não trouxe sugestões para este sintoma." />
      ) : report ? (
        <ul className="mt-5 space-y-3">
          {report.suggestions.map((s) => (
            <SuggestionCard key={s.id} s={s} laptop={laptop} open={open} apply={onApply && ((action) => onApply({ kind: 'apply', symptom, suggestion: s, action }))} />
          ))}
        </ul>
      ) : null}
    </section>
  )
}

/** Correções de sistema em andamento ou aplicadas, com conferência e desfazer. */
function SystemFixes({ version, onOpen }: { version: number; onOpen: (t: SystemFixTarget) => void }) {
  const [ops, setOps] = useState<Operation[]>([])
  const load = useCallback(() => {
    if (!isTauri()) return
    listFixOperations('system')
      .then((all) => setOps(all.filter((o) => ['requires-manual-step', 'applied', 'recovery-required', 'conflict'].includes(o.state))))
      .catch(() => setOps([]))
  }, [])
  useEffect(load, [load, version])
  if (!ops.length) return null
  return (
    <section aria-labelledby="correcoes-sistema" className="mt-10">
      <h2 id="correcoes-sistema" className="eyebrow mb-3">
        Correções no sistema
      </h2>
      <ul className="divide-y divide-line rounded-lg border border-line">
        {ops.map((o) => {
          const state = OP_STATE[o.state]
          const pending = o.state === 'requires-manual-step' || o.state === 'recovery-required'
          return (
            <li key={o.id} className="flex flex-wrap items-center gap-3 px-4 py-3">
              <div className="min-w-0 flex-1">
                <p className="truncate font-mono text-[12.5px] text-fg">{o.recipeTitle}</p>
                <p className="mt-0.5 text-[12px] text-fg-3">
                  {o.gameName}
                  {o.message ? ` · ${o.message}` : ''}
                </p>
              </div>
              <Pill color={state.color} soft={state.soft}>
                {o.pending?.kind === 'rollback' ? 'Aguardando desfazer' : state.label}
              </Pill>
              {pending && (
                <Button size="sm" variant="secondary" icon={<TerminalSquare className="size-3.5" />} onClick={() => onOpen({ kind: 'pending', operation: o })}>
                  Ver comando e conferir
                </Button>
              )}
              {(o.state === 'applied' || (o.state === 'requires-manual-step' && o.pending?.kind === 'apply') || o.state === 'recovery-required') && (
                <Button size="sm" variant="ghost" icon={<RotateCcw className="size-3.5" />} onClick={() => onOpen({ kind: 'rollback', operation: o })}>
                  {o.state === 'requires-manual-step' ? 'Cancelar' : 'Desfazer'}
                </Button>
              )}
            </li>
          )
        })}
      </ul>
    </section>
  )
}

const DEV_LAPTOPS = [
  { id: 'thinkpad-x1-carbon-gen9', label: 'ThinkPad X1 Carbon Gen 9' },
  { id: 'framework-13', label: 'Framework Laptop 13 (tem correções aplicáveis)' },
]

/** Só em desenvolvimento: simula a identidade de um laptop. A busca e o Fix Engine são reais. */
function DevLaptopSwitch({ simulated, onChange }: { simulated: boolean; onChange: () => void }) {
  const toast = useToast()
  const set = (id: string | null) =>
    setDevLaptop(id)
      .then(onChange)
      .catch((e) => toast({ tone: 'error', title: 'Fixture indisponível', body: errorMessage(e) }))
  return (
    <section aria-label="Fixture de desenvolvimento" className="mt-6 flex flex-wrap items-center gap-2 rounded-md border border-dashed border-line-2 px-3 py-2.5 text-[12.5px] text-fg-2">
      <FlaskConical className="size-3.5 text-fg-3" aria-hidden />
      <span className="mr-1">Injetar laptop de teste (dev):</span>
      {DEV_LAPTOPS.map((l) => (
        <Button key={l.id} size="sm" variant="outline" onClick={() => set(l.id)}>
          {l.label}
        </Button>
      ))}
      {simulated && (
        <Button size="sm" variant="ghost" onClick={() => set(null)}>
          Voltar ao DMI real
        </Button>
      )}
    </section>
  )
}
