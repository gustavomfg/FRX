import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Button } from '@/components/ui/Button'
import { ApplyFixDialog, type ApplyTarget } from '@/components/fix/ApplyFixDialog'
import { evaluateBenchmarkSession } from '@/services/smartPacks'
import { errorMessage } from '@/services/native'
import { useFixEngine } from '@/state/fixEngine'
import { usePerformance } from '@/state/performance'
import type { SessionMeta } from '@/types/performance'
import type { PackEvaluation } from '@/types/smartPacks'
import { PackEvaluationCard } from './PackTest'
import { SwitchDialog } from './SmartPacksPanel'

/**
 * Performance Lab: conclusão automática da sessão em relação à referência
 * (baseline) do jogo, com a opção de restaurar a configuração anterior
 * quando a sessão indica regressão do pacote ativo.
 */
export function SessionPackResult({ meta }: { meta: SessionMeta }) {
  const engine = useFixEngine()
  const perf = usePerformance()
  const navigate = useNavigate()
  const [e, setE] = useState<PackEvaluation | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [retry, setRetry] = useState(0)
  const [restore, setRestore] = useState(false)
  const [target, setTarget] = useState<ApplyTarget | null>(null)
  const stamp = engine.operations.map((o) => o.id + o.state).join() + JSON.stringify(perf.sessions.map((s) => [s.id, s.conditions, s.smartPack]))

  useEffect(() => {
    if (!engine.available || !meta.appId) return setE(null)
    let gone = false
    setRestore(false)
    setTarget(null)
    setE(null)
    setError(null)
    evaluateBenchmarkSession(meta.id).then(
      (r) => !gone && setE(r),
      (err) => !gone && setError(errorMessage(err)),
    )
    return () => void (gone = true)
  }, [engine.available, meta.id, meta.appId, stamp, retry])

  if (error) return <section role="alert" className="text-[13px] text-fg-2">
    <p>Não foi possível avaliar o Smart Pack: {error}</p>
    <Button size="sm" variant="outline" onClick={() => setRetry((n) => n + 1)}>Tentar avaliação de novo</Button>
  </section>
  if (!e || e.sessionId !== meta.id || e.appId !== meta.appId) return null
  const canRestore = e.verdict === 'regression' && e.currentPackId === e.packId && e.packId !== e.baselineId
  return (
    <section aria-labelledby="smart-pack-result" className="space-y-3">
      <div className="flex items-end justify-between">
        <h2 id="smart-pack-result" className="font-display text-[20px] font-semibold tracking-[-0.02em]">
          Smart Pack · resultado
        </h2>
        <Button size="sm" variant="ghost" onClick={() => navigate(`/biblioteca/${encodeURIComponent(e.appId)}?secao=smart`)}>
          Abrir Smart Packs
        </Button>
      </div>
      <PackEvaluationCard e={e} onRestore={canRestore ? () => setRestore(true) : undefined} />
      {restore && (
        <SwitchDialog
          appId={e.appId}
          pack={{ id: e.baselineId, name: 'Baseline — sem correções do FRX', kind: 'baseline', applyRecipeId: null }}
          onClose={() => setRestore(false)}
          onStep={(t) => {
            setRestore(false)
            setTarget(t)
          }}
        />
      )}
      {target && <ApplyFixDialog target={target} onClose={() => setTarget(null)} />}
    </section>
  )
}
