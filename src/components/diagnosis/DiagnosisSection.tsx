import { useEffect, useRef, useState } from 'react'
import { ExternalLink, Stethoscope, TriangleAlert } from 'lucide-react'
import { Button } from '@/components/ui/Button'
import { Select } from '@/components/ui/Controls'
import { Disclosure } from '@/components/ui/Disclosure'
import { Pill } from '@/components/ui/Status'
import { type ApplyTarget } from '@/components/fix/ApplyFixDialog'
import { diagnoseGame } from '@/services/diagnosis'
import { errorMessage } from '@/services/native'
import { openExternal } from '@/services/opener'
import { useSettings, useToast } from '@/state/store'
import type { Claim, DiagnosisResult, Excluded, Suggestion, SuggestionState, Symptom } from '@/types/diagnosis'
import type { EvidenceLevel } from '@/types/fixEngine'

const SYMPTOMS: { value: Symptom; label: string }[] = [
  { value: 'unspecified', label: 'Não informado' },
  { value: 'crash-or-wont-start', label: 'Crash ou não inicia' },
  { value: 'performance', label: 'Desempenho' },
  { value: 'graphics', label: 'Gráficos' },
  { value: 'other', label: 'Outro' },
]

const STATE: Record<SuggestionState, { label: string; color: string; soft: string }> = {
  applicable: { label: 'Pronta para revisar', color: 'var(--frx-accent)', soft: 'var(--frx-raised)' },
  temporary: { label: 'Aguardando', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)' },
  'already-applied': { label: 'Já aplicada', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)' },
  'already-included': { label: 'Já incluída no Proton', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)' },
  review: { label: 'Em revisão', color: 'var(--frx-text-2)', soft: 'var(--frx-raised)' },
}

const CONFIDENCE: Record<EvidenceLevel, string> = {
  documented: 'Confiança: documentada',
  corroborated: 'Confiança: vários relatos',
  single: 'Confiança: um relato',
  contested: 'Confiança: relatos divididos',
  none: 'Confiança: sem relato de resultado',
}

const CLAIM_KIND: Record<Claim['kind']['type'], string> = {
  'reported-result': 'Relato de resultado',
  documented: 'Documentação',
  'mention-only': 'Só cita a opção',
}

function Sources({ claims, open }: { claims: Claim[]; open: (url: string) => void }) {
  return (
    <ul className="mt-3 space-y-2">
      {claims.map((c) => (
        <li key={c.source.url} className="text-[12.5px] text-fg-2">
          <button type="button" onClick={() => open(c.source.url)} className="inline-flex items-center gap-1 text-fg underline-offset-2 hover:underline">
            <ExternalLink className="size-3" aria-hidden /> {c.source.label}
          </button>
          <span className="ml-2 text-fg-3">{CLAIM_KIND[c.kind.type]}</span>
          {c.quote && <q className="mt-0.5 block text-fg-3">{c.quote}</q>}
        </li>
      ))}
    </ul>
  )
}

function SuggestionCard({ s, apply, open }: { s: Suggestion; apply?: () => void; open: (url: string) => void }) {
  const state = STATE[s.state]
  return (
    <li className="rounded-sm border border-line p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <p className="text-[14px] font-medium text-fg">{s.title}</p>
          <code className="mt-0.5 block text-[12px] break-all text-fg-3">{s.key.id}</code>
        </div>
        {apply && (
          <Button variant="secondary" onClick={apply}>
            Ver prévia
          </Button>
        )}
      </div>
      <div className="mt-3 flex flex-wrap gap-2">
        <Pill color={state.color} soft={state.soft}>
          {state.label}
        </Pill>
        <Pill color="var(--frx-text-2)" soft="var(--frx-raised)">
          {CONFIDENCE[s.confidence]}
        </Pill>
        <Pill color="var(--frx-text-2)" soft="var(--frx-raised)">
          {s.compatibility === 'compatible' ? 'Compatível' : 'Compatibilidade desconhecida'}
        </Pill>
      </div>
      <p className="mt-3 text-[13px] text-fg-2">{s.reason}</p>
      <p className="mt-1 text-[12.5px] text-fg-3">{s.compatibilityDetail}</p>
      <Disclosure title={`Fontes (${s.claims.length})`} className="mt-3" titleClassName="text-[12.5px] text-fg-2">
        <Sources claims={s.claims} open={open} />
      </Disclosure>
    </li>
  )
}

function ExcludedRow({ e }: { e: Excluded }) {
  return (
    <li className="text-[13px] text-fg-2">
      <span className="text-fg">{e.title}</span>
      <code className="ml-2 text-[12px] break-all text-fg-3">{e.key.id}</code>
      <span className="block text-fg-3">{e.reason}</span>
    </li>
  )
}

/**
 * Central de Resolução: diagnóstico por regras, só ao clicar em Diagnosticar.
 * Conteúdo de fonte é mostrado como texto. Aplicar abre a prévia do Fix Engine.
 */
export function DiagnosisSection({ appId, title, onAct }: { appId: string; title: string; onAct: (t: ApplyTarget) => void }) {
  const { fixCacheHours } = useSettings()
  const toast = useToast()
  const [symptom, setSymptom] = useState<Symptom>('unspecified')
  const [result, setResult] = useState<DiagnosisResult | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [running, setRunning] = useState(false)
  const [stale, setStale] = useState(false)
  const current = useRef(appId)

  useEffect(() => {
    current.current = appId
    setResult(null)
    setError(null)
    setStale(false)
    setRunning(false)
  }, [appId])

  const open = (url: string) => void openExternal(url).catch(() => toast({ tone: 'error', title: 'Não foi possível abrir o link' }))

  const run = async () => {
    setRunning(true)
    setError(null)
    try {
      const r = await diagnoseGame(appId, title, symptom, fixCacheHours)
      if (current.current === appId) {
        setResult(r)
        setStale(false)
      }
    } catch (e) {
      if (current.current === appId) setError(errorMessage(e))
    } finally {
      if (current.current === appId) setRunning(false)
    }
  }

  const applyOf = (s: Suggestion) =>
    s.state === 'applicable' && s.recipeId
      ? () => {
          setStale(true)
          onAct({ kind: 'apply', recipeId: s.recipeId!, appId, title: `${s.title} · ${title}`, expectedFingerprint: s.expectedFingerprint ?? undefined })
        }
      : undefined

  const fixes = result?.suggestions.filter((s) => !s.optionalImprovement) ?? []
  const improvements = result?.suggestions.filter((s) => s.optionalImprovement) ?? []

  return (
    <section aria-labelledby="central" className="border-t border-line pt-6">
      <h2 id="central" className="font-display text-[22px] font-semibold tracking-[-0.02em]">
        Central de Resolução
      </h2>
      <p className="mt-1 text-[13px] text-fg-2">Diagnóstico por regras: consulta as fontes e as receitas do FRX. Não altera nada no jogo ou na Steam.</p>
      <div className="mt-4 flex flex-wrap items-center gap-3">
        <Select label="Sintoma" value={symptom} onChange={setSymptom} options={SYMPTOMS} disabled={running} />
        <Button variant="primary" icon={<Stethoscope className="size-4" />} onClick={() => void run()} loading={running}>
          {running ? 'Diagnosticando…' : result ? 'Diagnosticar novamente' : 'Diagnosticar'}
        </Button>
      </div>

      {error && (
        <p role="alert" className="mt-4 flex items-center gap-2 text-[13px] text-bad">
          <TriangleAlert className="size-4 shrink-0" aria-hidden /> {error}
        </p>
      )}

      {result && (
        <div className="mt-5">
          <p className="text-[14px] text-fg" data-verdict={result.verdict}>
            {result.message}
          </p>
          {stale && <p className="mt-2 text-[13px] text-caution">Uma prévia foi aberta depois deste resultado. Diagnostique novamente para atualizar os estados.</p>}
          <p className="mt-2 text-[12.5px] text-fg-3">
            GPU: {result.context.gpuName ?? result.context.gpuVendor?.toUpperCase() ?? 'não identificada'} · Driver: {result.context.driver ?? 'desconhecido'} · Ferramenta: {result.context.protonTool ?? 'desconhecida'}
            {result.context.gameFiles.length > 0 && ` · Arquivos: ${result.context.gameFiles.join(', ')}`}
          </p>

          {fixes.length > 0 && (
            <ul className="mt-4 space-y-3" aria-label="Sugestões">
              {fixes.map((s) => (
                <SuggestionCard key={s.key.id} s={s} apply={applyOf(s)} open={open} />
              ))}
            </ul>
          )}

          {improvements.length > 0 && (
            <div className="mt-5">
              <h3 className="text-[14px] font-medium text-fg">Melhorias opcionais</h3>
              <p className="mt-0.5 text-[12.5px] text-fg-3">Melhoram a imagem. Não corrigem falhas.</p>
              <ul className="mt-3 space-y-3" aria-label="Melhorias opcionais">
                {improvements.map((s) => (
                  <SuggestionCard key={s.key.id} s={s} apply={applyOf(s)} open={open} />
                ))}
              </ul>
            </div>
          )}

          {result.inReview.length > 0 && (
            <Disclosure title={`Em revisão (${result.inReview.length})`} className="mt-5" titleClassName="text-[14px] font-medium text-fg">
              <ul className="mt-3 space-y-3" aria-label="Em revisão">
                {result.inReview.map((s) => (
                  <SuggestionCard key={s.key.id} s={s} open={open} />
                ))}
              </ul>
            </Disclosure>
          )}

          {result.excluded.length > 0 && (
            <Disclosure title={`Não sugeridas (${result.excluded.length})`} className="mt-4" titleClassName="text-[14px] font-medium text-fg">
              <ul className="mt-3 space-y-2">
                {result.excluded.map((e) => (
                  <ExcludedRow key={e.key.id} e={e} />
                ))}
              </ul>
            </Disclosure>
          )}

          <Disclosure title="Limitações e origem" className="mt-4" titleClassName="text-[14px] font-medium text-fg">
            <ul className="mt-3 list-disc space-y-1 pl-5 text-[13px] text-fg-2">
              {result.limitations.map((l) => (
                <li key={l}>{l}</li>
              ))}
            </ul>
            <p className="mt-3 text-[12.5px] text-fg-3">
              Método: {result.origin.method}. Componentes aprendidos: {result.origin.learnedComponents.length ? result.origin.learnedComponents.join(', ') : 'nenhum'}.
            </p>
          </Disclosure>
        </div>
      )}
    </section>
  )
}
