import type { FixFinding } from '@/types/fixes'
import type { Assessment } from '@/types/fixEngine'
import { FindingCard } from './FindingCard'
import { Button } from '@/components/ui/Button'

/** Only a complete planner assessment can promote a discovery to the main list. */
export function ActionableFindings({ appId, findings, assessments, loading, error, refresh, onApply, technicalOnly = false }: {
  appId: string
  findings: FixFinding[]
  assessments: Record<string, Assessment>
  loading: boolean
  error: string | null
  refresh: () => void
  onApply: (a: Assessment, finding: FixFinding) => void
  technicalOnly?: boolean
}) {
  const scoped = findings.filter((f) => f.gameAppId === appId)
  const assessment = (f: FixFinding) => assessments[f.id]?.appId === appId ? assessments[f.id] : undefined
  const state = (f: FixFinding) => {
    const a = assessment(f)
    return a?.availability?.state === 'available' && (!a.recipeId || !a.availability.planFingerprint) ? 'investigation' : a?.availability?.state
  }
  const ready = scoped.filter((f) => state(f) === 'available')
  const temporary = scoped.filter((f) => state(f) === 'temporary')
  const applied = scoped.filter((f) => state(f) === 'applied')
  const research = scoped.filter((f) => !['available', 'temporary', 'applied'].includes(state(f) ?? ''))
  const cards = (list: FixFinding[]) => <ul className="mt-3 space-y-3">{list.map((f) => <FindingCard key={f.id} finding={f} assessment={assessment(f)} analyzing={loading} onRefresh={refresh} onApply={(a) => onApply(a, f)} />)}</ul>
  return <div className="space-y-6" aria-busy={loading}>
    {error && <p role="alert" className="text-[13px] text-bad">Não foi possível verificar os planos: {error}</p>}
    <Button size="sm" variant="ghost" loading={loading} disabled={loading} onClick={refresh}>Revalidar correções</Button>
    {!technicalOnly && <>
      <section aria-label="Correções disponíveis">
        <h3 className="text-[15px] font-medium">Correções disponíveis</h3>
        {ready.length ? cards(ready) : <p className="mt-2 text-[13px] text-fg-2">{loading ? 'Verificando planos no ambiente atual…' : 'Nenhuma correção automática disponível para este jogo no momento.'}</p>}
      </section>
      {!!temporary.length && <section aria-label="Aguardando condição local"><h3 className="text-[15px] font-medium">Aguardando condição local</h3><p className="mt-1 text-[12.5px] text-fg-3">Estas operações são suportadas. Resolva a condição indicada e revalide para continuar.</p>{cards(temporary)}</section>}
      {!!applied.length && <section aria-label="Correções aplicadas"><h3 className="text-[15px] font-medium">Correções aplicadas</h3>{cards(applied)}</section>}
    </>}
    <details open={technicalOnly} className="border-t border-line pt-4">
      <summary className="cursor-pointer text-[14px] font-medium">Descobertas em investigação ({research.length})</summary>
      <p className="mt-2 text-[12.5px] text-fg-3">Procedimentos incompletos, incompatibilidades e ações sem aplicação segura permanecem aqui, com sua origem e o impedimento específico.</p>
      {research.length ? cards(research) : <p className="mt-3 text-[13px] text-fg-2">Nenhuma descoberta adicional em investigação.</p>}
    </details>
  </div>
}
