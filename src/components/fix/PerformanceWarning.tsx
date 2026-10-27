import { TriangleAlert } from 'lucide-react'

/** Análise preventiva do núcleo, independente de benchmarks e da segurança da gravação. */
export function PerformanceWarning({ warnings, compact = false }: { warnings?: string[]; compact?: boolean }) {
  if (!warnings?.length) return null
  if (compact) return <span className="inline-flex items-center rounded-xs bg-caution-soft px-2 py-1 text-[12px] font-medium text-caution" title={warnings.join(' ')}>Risco técnico geral</span>
  return (
    <section aria-label="Risco de perda de desempenho" className="space-y-2 rounded-md border border-caution/25 bg-caution-soft px-3.5 py-3 text-[13px] text-caution">
      <p className="flex items-center gap-2 font-medium"><TriangleAlert className="size-4 shrink-0" aria-hidden /> Risco potencial de desempenho</p>
      <ul className="space-y-1">{warnings.map((reason) => <li key={reason}>{reason}</li>)}</ul>
      <p className="text-[12px]">Aviso técnico geral; não é uma regressão medida neste jogo. Relatos confiáveis deste AppID podem justificar um candidato para teste. O ranking depende das medições comparáveis deste jogo e deste computador.</p>
    </section>
  )
}
