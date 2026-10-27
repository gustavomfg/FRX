import type { GamePerformanceEvidence } from '@/types/fixEngine'
import type { DiscoveryScope } from '@/types/fixes'

export function gameEvidenceText(e?: GamePerformanceEvidence) {
  if (!e || e.positive + e.negative === 0) return 'Impacto neste jogo desconhecido'
  if (e.positive > 0 && e.negative > 0) return 'Evidências conflitantes'
  if (!e.contextCompatible) return 'Relatos em contexto diferente ou não confirmado'
  if (e.negative > 0) return 'Relatos de perda de desempenho neste jogo'
  if (e.authoritativePositive || e.positive >= 2) return 'Possível melhoria específica para este jogo — requer validação local'
  return 'Relato positivo isolado para este jogo'
}

/** Public claims have their own scope; they never masquerade as local benchmarks. */
export function GameEvidence({ appId, scope, evidence, compact = false }: { appId: string; scope?: DiscoveryScope; evidence?: GamePerformanceEvidence; compact?: boolean }) {
  const scoped = scope?.kind === 'verified-game' && scope.appId === appId
  const relevant = evidence?.appId === appId && (!scope || scoped) ? evidence : undefined
  const scopeText = scoped ? `Específico deste jogo · AppID ${appId}` : scope?.kind === 'general-technical' ? 'Documentação técnica geral' : scope ? 'Atribuição ao jogo não confirmada' : `Contexto do jogo · AppID ${appId}`
  if (compact) return <span className="text-[12px] text-fg-3">{scopeText}</span>
  return (
    <section aria-label="Evidência específica do jogo" className="space-y-1 text-[12.5px] text-fg-2">
      <p className="font-medium text-fg">{scopeText}</p>
      {scope?.basis && <p>{scope.basis}</p>}
      <p>{gameEvidenceText(relevant)}</p>
      {!!relevant && relevant.positive + relevant.negative > 0 && <p>Relatos públicos: {relevant.positive} favorável(is), {relevant.negative} contrário(s). A compatibilidade relatada não comprova ganho de FPS neste computador.</p>}
      {relevant?.limitations.map((note) => <p key={note} className="text-fg-3">{note}</p>)}
      {scope && !scoped && <p className="text-caution">Esta origem não autoriza gerar uma correção específica para o jogo selecionado.</p>}
    </section>
  )
}
