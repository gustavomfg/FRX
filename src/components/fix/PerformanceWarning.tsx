import { TriangleAlert } from 'lucide-react'

/** Análise preventiva do núcleo, independente de benchmarks e da segurança da gravação. */
export function PerformanceWarning({ warnings, compact = false }: { warnings?: string[]; compact?: boolean }) {
  if (!warnings?.length) return null
  if (compact) return <span className="inline-flex items-center rounded-xs bg-caution-soft px-2 py-1 text-[12px] font-medium text-caution" title={warnings.join(' ')}>Risco de perda de desempenho</span>
  return (
    <section aria-label="Risco de perda de desempenho" className="space-y-2 rounded-md border border-caution/25 bg-caution-soft px-3.5 py-3 text-[13px] text-caution">
      <p className="flex items-center gap-2 font-medium"><TriangleAlert className="size-4 shrink-0" aria-hidden /> Risco de perda de desempenho</p>
      <ul className="space-y-1">{warnings.map((reason) => <li key={reason}>{reason}</li>)}</ul>
      <p className="text-[12px]">Análise preventiva do FRX. Sem evidência local favorável, esta alternativa fica reservada à compatibilidade e fora das sugestões automáticas de otimização; você não precisa gastar um teste para o FRX reconhecer esse risco.</p>
    </section>
  )
}
