import { useCallback, useEffect, useRef, useState } from 'react'
import { Link } from 'react-router-dom'
import { ExternalLink, FileText, RefreshCw, RotateCcw, ScanSearch, Search } from 'lucide-react'
import { ApplyFixDialog, type ApplyTarget } from '@/components/fix/ApplyFixDialog'
import { HardwareContext } from '@/components/fix/HardwareContext'
import { Button } from '@/components/ui/Button'
import { Mascot } from '@/mascot/Mascot'
import { useFixEngine } from '@/state/fixEngine'
import { getFixOperation } from '@/services/fixEngine'
import { errorMessage } from '@/services/native'
import { isSafeExternalUrl, openExternal } from '@/services/opener'
import * as api from '@/services/resolution'
import { fmtDateTime } from '@/lib/format'
import type { ResolutionEvidence, ResolutionPlan, ResolutionReport } from '@/types/resolution'

const SYMPTOMS = [
  ['startup', 'Não inicia / crash'], ['low-performance', 'Desempenho baixo'], ['stuttering', 'Stuttering'],
  ['physx', 'Problema de física'], ['dlss', 'DLSS indisponível'], ['frame-generation', 'Frame generation'],
  ['ray-tracing', 'Ray tracing'], ['fsr', 'FSR'], ['graphics', 'Artefatos gráficos'],
  ['audio', 'Áudio'], ['video', 'Vídeos / cutscenes'], ['controls', 'Controles'],
] as const

const sourceDate = (value: string) => Number.isFinite(new Date(value).getTime()) ? fmtDateTime(value) : 'Data não confirmada pela fonte'

const STATES: Record<string, string> = {
  unknown: 'Desconhecido', supported: 'Suportado', unsupported: 'Não suportado',
  detected: 'Indício estático', observed: 'Observado em log', configured: 'Solicitado',
  available: 'Disponível', unavailable: 'Indisponível', conflicting: 'Evidências conflitantes',
  incompatible: 'Incompatível', 'static-candidate': 'Indício estático', reported: 'Relatado pela fonte',
  present: 'Componente presente', failed: 'Falha observada em log', candidate: 'Configuração candidata',
  'component-observed': 'Componente observado em log',
}
const PLAN_STATES: Record<string, string> = {
  applicable: 'Prévia disponível', review: 'Requer revisão',
  'applied-unverified': 'Aplicada — funcionamento ainda não verificado',
  'verified-configuration': 'Configuração conferida — funcionamento ainda não verificado',
  'rolled-back': 'Alteração desfeita', error: 'Verificação requer atenção',
}
const EVIDENCE_KINDS: Record<string, string> = {
  'official-documentation': 'Documentação oficial', 'community-report': 'Relato comunitário',
  'static-inspection': 'Inspeção estática', 'runtime-log': 'Log importado',
  'local-configuration': 'Configuração local', 'user-report': 'Sintoma informado',
  'capability-rule': 'Requisito documentado',
  'official-release': 'Release do projeto', 'source-unavailable': 'Fonte indisponível',
  'static-candidate': 'Indício estático', 'configuration-candidate': 'Configuração candidata',
  'runtime-component': 'Componente observado em log', 'runtime-failure': 'Falha observada em log',
  'runtime-capability': 'Capacidade relatada em log', 'hardware-policy': 'Requisito de hardware documentado',
}

/** A keyed scope prevents a request or application dialog following the user to another game. */
export function ResolutionCenter({ appId, available }: { appId: string; available: boolean }) {
  if (!available) return <section aria-label="Resolution Center" className="mt-10 border-t border-line pt-6"><h2 className="font-display text-[24px] font-semibold tracking-[-0.025em]">Resolution Center</h2><p className="mt-2 text-[13px] text-fg-2">O diagnóstico real está disponível no app desktop, com a biblioteca real. O modo demonstrativo não pesquisa nem aplica correções.</p></section>
  return <ScopedResolutionCenter key={appId} appId={appId} />
}

function ScopedResolutionCenter({ appId }: { appId: string }) {
  const engine = useFixEngine()
  const [report, setReport] = useState<ResolutionReport | null>(null)
  const [symptoms, setSymptoms] = useState<string[]>([])
  const [busy, setBusy] = useState<string | null>('load')
  const [error, setError] = useState<string | null>(null)
  const [target, setTarget] = useState<ApplyTarget | null>(null)
  const alive = useRef(true)
  const sequence = useRef(0)

  const run = useCallback(async (label: string, task: () => Promise<ResolutionReport | null>) => {
    const request = ++sequence.current
    setBusy(label)
    setError(null)
    try {
      const result = await task()
      if (!alive.current || request !== sequence.current) return
      if (result && result.appId !== appId) throw new Error('O diagnóstico retornou outro AppID. Nada foi associado a este jogo; tente diagnosticar novamente.')
      setReport(result)
      if (result?.symptoms) setSymptoms(result.symptoms)
    } catch (e) {
      if (alive.current && request === sequence.current) setError(errorMessage(e))
    } finally {
      if (alive.current && request === sequence.current) setBusy(null)
    }
  }, [appId])

  useEffect(() => {
    alive.current = true
    void run('load', () => api.getResolutionReport(appId))
    return () => { alive.current = false; sequence.current++ }
  }, [appId, run])

  const diagnose = () => run('diagnose', () => api.diagnoseGame(appId, symptoms))
  const research = () => run('research', () => api.researchGameResolutions(appId, symptoms))
  const importLog = () => run('import', async () => {
    const path = await api.chooseResolutionLog()
    // Closing the file picker preserves the current report and imports nothing.
    return path ? api.importResolutionLog(appId, path) : report
  })
  const open = (url: string) => void openExternal(url).catch((e) => { if (alive.current) setError(errorMessage(e)) })
  const rollback = async (plan: ResolutionPlan) => {
    if (!plan.operationId) return
    setBusy('rollback')
    setError(null)
    try {
      const op = await getFixOperation(plan.operationId)
      if (!alive.current) return
      if (op.appId !== appId) throw new Error('A operação pertence a outro jogo. Abra o histórico correto para revisá-la.')
      setTarget({ kind: 'rollback', operation: op })
    } catch (e) { if (alive.current) setError(errorMessage(e)) }
    finally { if (alive.current) setBusy(null) }
  }
  const closeDialog = () => {
    setTarget(null)
    void engine.refresh()
    void diagnose()
  }
  const safePlans = report?.plans.filter((p) => p.appId === appId) ?? []

  return <section aria-labelledby="resolution-center" className="mt-10 border-t border-line pt-7" aria-busy={!!busy}>
    <div className="flex flex-wrap items-start justify-between gap-5">
      <div className="max-w-[65ch]">
        <h2 id="resolution-center" className="font-display text-[26px] font-semibold tracking-[-0.025em]">Resolution Center</h2>
        <p className="mt-1.5 text-[13px] leading-relaxed text-fg-2">Diagnóstico, fontes e soluções para este jogo e ambiente. Aplicar uma correção e verificar seu funcionamento são etapas distintas.</p>
        {report && <p className="mt-2 text-[12px] text-fg-3">AppID {report.appId} · Diagnóstico de {sourceDate(report.generatedAt)}</p>}
      </div>
      <Mascot pose={busy && busy !== 'load' ? 'inspect' : error ? 'confused' : 'prepare'} scale={2} />
    </div>

    <fieldset disabled={!!busy} className="mt-5">
      <legend className="mb-2 text-[13px] font-medium text-fg">O que você quer investigar? <span className="font-normal text-fg-3">Opcional</span></legend>
      <div className="flex flex-wrap gap-x-4 gap-y-2">
        {SYMPTOMS.map(([id, label]) => <label key={id} className="inline-flex cursor-pointer items-center gap-2 text-[12.5px] text-fg-2"><input type="checkbox" className="size-3.5 accent-accent" checked={symptoms.includes(id)} onChange={(e) => setSymptoms((old) => e.target.checked ? [...old, id] : old.filter((s) => s !== id))} />{label}</label>)}
      </div>
    </fieldset>
    <div className="mt-4 flex flex-wrap gap-2">
      <Button variant="primary" icon={<ScanSearch className="size-4" />} disabled={!!busy} loading={busy === 'diagnose'} onClick={() => void diagnose()}>Diagnosticar</Button>
      <Button icon={<Search className="size-4" />} disabled={!!busy} loading={busy === 'research'} onClick={() => void research()}>Pesquisar soluções</Button>
      <Button variant="ghost" icon={<FileText className="size-4" />} disabled={!!busy} loading={busy === 'import'} onClick={() => void importLog()}>Importar log deste jogo</Button>
    </div>
    <p className="mt-2 text-[12px] leading-relaxed text-fg-3">O diagnóstico é somente leitura. Pesquisar consulta fontes públicas; importar analisa apenas o log escolhido, localmente. Nenhum jogo será iniciado.</p>
    {error && <div role="alert" className="mt-4 rounded-md border border-bad/25 bg-bad-soft p-3 text-[13px] text-bad">{error} <span className="block mt-1">Você pode tentar diagnosticar novamente. Nenhuma ação será aplicada sem prévia e confirmação.</span></div>}
    {busy && <p role="status" className="mt-4 text-[13px] text-fg-2">{busy === 'load' ? 'Carregando diagnóstico salvo…' : busy === 'research' ? 'Consultando fontes e verificando o contexto…' : busy === 'import' ? 'Aguardando e analisando o log selecionado…' : busy === 'verify' ? 'Conferindo a alteração registrada…' : 'Analisando o ambiente do jogo…'}</p>}
    {!report && !busy && <p className="mt-6 text-[13px] text-fg-2">Ainda não há diagnóstico salvo. Use Diagnosticar para inspecionar o ambiente ou Pesquisar soluções para consultar fontes.</p>}

    {report && <div className="mt-7 space-y-7">
      <HardwareContext environment={report.environment} />
      {!!report.technologies.length && <div>
        <h3 className="font-medium text-[15px]">Tecnologias e compatibilidade</h3>
        <p className="mt-1 text-[12px] text-fg-3">Um arquivo presente é um indício. Suporte do hardware, implementação do jogo e funcionamento em execução são avaliados separadamente.</p>
        <div className="mt-3 overflow-x-auto rounded-md border border-line">
          <table className="w-full min-w-[650px] text-left text-[12px]">
            <thead className="bg-raised text-fg-3"><tr>{['Tecnologia', 'Hardware', 'Jogo', 'Proton / driver', 'Configuração', 'Em execução'].map((label) => <th key={label} className="px-3 py-2.5 font-medium">{label}</th>)}</tr></thead>
            <tbody>{report.technologies.map((tech) => <tr key={tech.id} className="border-t border-line align-top">
              <td className="px-3 py-3 font-medium text-fg"><details><summary className="cursor-pointer">{tech.name}</summary><div className="mt-3 min-w-48 space-y-2 font-normal"><TextList items={tech.limitations} /><EvidenceList evidence={tech.evidence} onOpen={open} /></div></details></td>
              {[tech.hardware, tech.gameSupport, tech.environment, tech.configured, tech.runtime].map((state, i) => <td key={i} className={`px-3 py-3 ${['unsupported', 'incompatible', 'failed', 'conflicting'].includes(state) ? 'text-caution' : state === 'unknown' ? 'text-fg-3' : 'text-fg-2'}`}>{STATES[state] ?? 'Desconhecido'}</td>)}
            </tr>)}</tbody>
          </table>
        </div>
      </div>}

      <div>
        <h3 className="font-medium text-[15px]">Problemas e soluções</h3>
        {!report.problems.length && <p className="mt-2 text-[13px] text-fg-2">Nenhum problema confirmado com os dados disponíveis. Isso não garante que todos os recursos estejam funcionando.</p>}
        <div className="mt-3 divide-y divide-line border-y border-line">
          {report.problems.map((problem) => <details key={problem.id} className="py-4" open={report.problems.length === 1}>
            <summary className="cursor-pointer text-[14px] font-medium text-fg">{problem.symptom}<span className="ml-3 text-[12px] font-normal text-fg-3">{problem.certainty === 'observed' ? 'Evidência observada' : ['reported', 'user-reported'].includes(problem.certainty) ? 'Problema relatado' : 'Hipótese a verificar'}</span></summary>
            <div className="mt-4 grid gap-5 lg:grid-cols-2">
              <div className="min-w-0 space-y-4">
                <Detail title="O que encontramos"><EvidenceList evidence={problem.evidence} onOpen={open} /></Detail>
                <Detail title="Por que pode acontecer"><TextList items={problem.hypotheses} /></Detail>
                {!!problem.ruledOut.length && <Detail title="Causas descartadas"><TextList items={problem.ruledOut} /></Detail>}
                {!!problem.limitations.length && <Detail title="Limites do diagnóstico"><TextList items={problem.limitations} /></Detail>}
                <Detail title="Próxima ação"><p>{problem.nextAction}</p></Detail>
              </div>
              <div className="min-w-0 space-y-5">
                {safePlans.filter((p) => p.problemId === problem.id || problem.planIds.includes(p.id)).map((plan) => <PlanDetails key={plan.id} plan={plan} busy={!!busy} onOpen={open} onPreview={() => { if (plan.recipeId) setTarget({ kind: 'apply', recipeId: plan.recipeId, appId, title: plan.title }) }} onVerify={() => void run('verify', () => api.verifyResolutionPlan(appId, plan.id))} onRollback={() => void rollback(plan)} />)}
                {!safePlans.some((p) => p.problemId === problem.id || problem.planIds.includes(p.id)) && <p className="text-[13px] text-fg-2">Problema identificado — nenhuma solução aplicável confirmada.</p>}
              </div>
            </div>
          </details>)}
        </div>
      </div>
      {!!report.packs.filter((pack) => pack.appId === appId).length && <div>
        <h3 className="font-medium text-[15px]">Smart Resolution Packs</h3>
        <ul className="mt-2 divide-y divide-line">{report.packs.filter((pack) => pack.appId === appId).map((pack) => <li key={pack.id} className="flex flex-wrap items-center justify-between gap-3 py-3"><div className="min-w-0"><p className="text-[13px] font-medium">{pack.title}</p><p className="mt-1 max-w-[65ch] text-[12px] text-fg-2">{pack.explanation}</p></div>{pack.smartPackId && <Link className="text-[13px] text-accent underline underline-offset-4" to={`/correcoes?view=smart&game=${encodeURIComponent(appId)}&pack=${encodeURIComponent(pack.smartPackId)}`}>Ver pacote</Link>}</li>)}</ul>
      </div>}
      {!!report.knowledge.length && <details className="border-t border-line pt-4"><summary className="cursor-pointer text-[13px] font-medium">Fontes consultadas ({report.knowledge.length})</summary><div className="mt-3"><EvidenceList evidence={report.knowledge} onOpen={open} /></div></details>}
      {!!report.limitations.length && <Detail title="O que ainda não sabemos"><TextList items={report.limitations} /></Detail>}
    </div>}
    <ApplyFixDialog target={target} onClose={closeDialog} />
  </section>
}

function PlanDetails({ plan, busy, onOpen, onPreview, onVerify, onRollback }: { plan: ResolutionPlan; busy: boolean; onOpen: (url: string) => void; onPreview: () => void; onVerify: () => void; onRollback: () => void }) {
  const applied = plan.state === 'applied-unverified' || plan.state === 'verified-configuration'
  return <article aria-label={plan.title} className="space-y-3">
    <div><h4 className="text-[14px] font-medium">{plan.title}</h4><p className={`mt-1 text-[12px] ${plan.state === 'review' || plan.state === 'error' ? 'text-caution' : 'text-fg-2'}`}>{PLAN_STATES[plan.state] ?? 'Estado não confirmado'}</p></div>
    <Detail title="Solução sugerida"><p>{plan.objective}</p><TextList items={plan.actions} /></Detail>
    {!!plan.dependencies.length && <Detail title="Compatibilidade e dependências"><TextList items={plan.dependencies} /></Detail>}
    {!!plan.risks.length && <Detail title="Riscos"><TextList items={plan.risks} /></Detail>}
    <Detail title="Verificação"><p>{plan.verificationMethod}</p><p className="mt-1">Critério: {plan.successCriterion}</p>{plan.verification && <p className="mt-2 font-medium text-fg">{plan.verification}</p>}</Detail>
    {!!plan.limitations.length && <TextList items={plan.limitations} />}
    {!!plan.sources.length && <Detail title="Fontes"><EvidenceList evidence={plan.sources} onOpen={onOpen} /></Detail>}
    <div className="flex flex-wrap gap-2">
      {(plan.state === 'applicable' || plan.state === 'rolled-back') && plan.recipeId && plan.reversible && <Button size="sm" disabled={busy} onClick={onPreview}>Visualizar prévia</Button>}
      {applied && plan.operationId && <Button size="sm" disabled={busy} icon={<RefreshCw className="size-3.5" />} onClick={onVerify}>Verificar alteração</Button>}
      {applied && plan.verificationMethod.includes('Performance Lab') && <Link className="inline-flex items-center text-[13px] text-accent underline underline-offset-4" to={`/performance?game=${encodeURIComponent(plan.appId)}`}>Abrir Performance Lab</Link>}
      {(applied || plan.state === 'error') && plan.operationId && plan.reversible && <Button size="sm" variant="ghost" disabled={busy} icon={<RotateCcw className="size-3.5" />} onClick={onRollback}>Desfazer</Button>}
    </div>
  </article>
}

function Detail({ title, children }: { title: string; children: React.ReactNode }) {
  return <div className="text-[12.5px] leading-relaxed text-fg-2"><h4 className="mb-1 font-medium text-fg">{title}</h4>{children}</div>
}

function TextList({ items }: { items: string[] }) {
  if (!items.length) return null
  return <ul className="space-y-1 text-[12.5px] leading-relaxed text-fg-2">{items.map((item, i) => <li key={`${i}:${item}`} className="break-words">{item}</li>)}</ul>
}

function EvidenceList({ evidence, onOpen }: { evidence: ResolutionEvidence[]; onOpen: (url: string) => void }) {
  if (!evidence.length) return <p className="text-[12.5px] text-fg-3">Nenhuma evidência suficiente disponível.</p>
  return <ul className="space-y-3 text-[12.5px] leading-relaxed text-fg-2">{evidence.map((e) => <li key={e.id} className="break-words">
    <p>{e.summary}</p>
    <p className="mt-1 text-[11.5px] text-fg-3">{EVIDENCE_KINDS[e.kind] ?? 'Evidência contextual'}{e.publisher && ` · ${e.publisher}`}{e.version && ` · ${e.version}`}{e.appId ? ` · AppID ${e.appId}` : ' · Contexto técnico geral'}{e.stale && ' · Fonte antiga; revalidar'}</p>
    {e.publishedAt && <p className="text-[11.5px] text-fg-3">Publicada: {sourceDate(e.publishedAt)}</p>}
    {e.updatedAt && <p className="text-[11.5px] text-fg-3">Atualizada: {sourceDate(e.updatedAt)}</p>}
    {!!e.limitations.length && <TextList items={e.limitations} />}
    {e.sourceUrl && (isSafeExternalUrl(e.sourceUrl) ? <button type="button" onClick={() => onOpen(e.sourceUrl!)} className="mt-1 inline-flex items-center gap-1 text-accent hover:underline underline-offset-4" aria-label={`Abrir fonte: ${e.publisher ?? e.summary}`}>Ver fonte <ExternalLink className="size-3" /></button> : <p className="mt-1 text-[11.5px] text-caution">Endereço fora das fontes permitidas; abertura bloqueada.</p>)}
  </li>)}</ul>
}
