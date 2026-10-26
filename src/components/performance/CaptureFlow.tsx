import { useEffect, useState } from 'react'
import { Check, ClipboardCopy, Gamepad2, Play, Square, X } from 'lucide-react'
import type { CaptureState, CaptureStatus, StartMode } from '@/types/performance'
import type { Game } from '@/types'
import { Dialog } from '@/components/ui/Dialog'
import { Button } from '@/components/ui/Button'
import { Segmented, Select, Toggle } from '@/components/ui/Controls'
import { Mascot } from '@/mascot/Mascot'
import type { Pose } from '@/mascot/sprites'
import { usePerformance } from '@/state/performance'
import { ApplyFixDialog, type ApplyTarget } from '@/components/fix/ApplyFixDialog'
import { useToast } from '@/state/store'
import { openExternal } from '@/services/opener'
import { isTauri } from '@/services/platform'
import { openUrl } from '@tauri-apps/plugin-opener'
import { Field, SessionFields, emptyForm, toConditions, type FormValues } from './SessionForm'
import { duration } from './format'
import { fmtNum } from '@/lib/format'
import { cn } from '@/lib/cn'

export const CAPTURE_POSE: Record<CaptureState, Pose> = {
  idle: 'idle',
  checking_dependencies: 'prepare',
  ready: 'prepare',
  waiting_for_game: 'idle',
  recording: 'record',
  processing: 'analyze',
  completed: 'wave',
  failed: 'worried',
  cancelled: 'idle',
}

export const CAPTURE_LABEL: Record<CaptureState, string> = {
  idle: 'Pronto para preparar',
  checking_dependencies: 'Verificando o MangoHud',
  ready: 'Sessão preparada',
  waiting_for_game: 'Aguardando o jogo',
  recording: 'Gravando',
  processing: 'Analisando',
  completed: 'Concluída',
  failed: 'Falhou',
  cancelled: 'Cancelada',
}

async function copy(text: string) {
  try {
    await navigator.clipboard.writeText(text)
    return true
  } catch {
    const ta = document.createElement('textarea')
    ta.value = text
    document.body.appendChild(ta)
    ta.select()
    const ok = document.execCommand('copy')
    ta.remove()
    return ok
  }
}

function CodeLine({ text, label }: { text: string; label: string }) {
  const toast = useToast()
  const [done, setDone] = useState(false)
  return (
    <div className="flex items-stretch gap-2">
      <code className="min-w-0 flex-1 rounded-sm border border-line bg-bg px-3 py-2 font-mono text-[12px] break-all text-fg select-all">{text}</code>
      <Button
        size="sm"
        variant="secondary"
        className="h-auto self-stretch"
        icon={done ? <Check className="size-3.5" /> : <ClipboardCopy className="size-3.5" />}
        onClick={async () => {
          const ok = await copy(text)
          setDone(ok)
          if (!ok) toast({ tone: 'warn', title: 'Não foi possível copiar', body: 'Selecione o texto e copie manualmente.' })
          setTimeout(() => setDone(false), 1800)
        }}
        aria-label={`Copiar ${label}`}
      >
        {done ? 'Copiado' : 'Copiar'}
      </Button>
    </div>
  )
}

const DURATIONS = [
  { value: '30', label: '30 s' },
  { value: '60', label: '60 s' },
  { value: '120', label: '2 min' },
  { value: '300', label: '5 min' },
]

export function CaptureFlow({
  open,
  onClose,
  games,
  defaultAppId,
  onViewSession,
  preset,
  intro,
  expectedPackId,
}: {
  open: boolean
  onClose: () => void
  games: Game[]
  defaultAppId: string
  onViewSession: (id: string) => void
  /** Valores iniciais do formulário (ex.: teste de Smart Pack com as condições da referência). */
  preset?: Partial<FormValues>
  /** Texto extra no topo do formulário. */
  intro?: string
  expectedPackId?: string
}) {
  const perf = usePerformance()
  const toast = useToast()
  const status = perf.capture
  const state: CaptureState = status?.state ?? 'idle'
  const [form, setForm] = useState<FormValues>(emptyForm(defaultAppId))
  const [dur, setDur] = useState('60')
  const [start, setStart] = useState<StartMode>(intro ? 'manual' : 'auto')
  const [delay, setDelay] = useState('20')
  const [hud, setHud] = useState(true)
  const [busy, setBusy] = useState(false)
  const [setup, setSetup] = useState<ApplyTarget | null>(null)

  useEffect(() => {
    if (open && (state === 'idle' || state === 'cancelled' || state === 'failed' || state === 'completed')) setForm((f) => (preset ? { ...emptyForm(defaultAppId), ...preset } : { ...f, appId: f.appId || defaultAppId }))
    // O preset vale no momento em que o diálogo abre.
  }, [open, state, defaultAppId])

  const game = games.find((g) => g.id === (status?.plan?.appId ?? form.appId))
  const launchGame = async () => {
    if (!game) return
    try {
      if (isTauri()) await openUrl(`steam://rungameid/${game.id}`)
      else await openExternal(`https://store.steampowered.com/app/${game.id}`)
    } catch {
      toast({ tone: 'error', title: 'Não foi possível abrir a Steam', body: 'Abra o jogo pela própria Steam.' })
    }
  }

  const prepare = async () => {
    if (!form.appId) {
      toast({ tone: 'warn', title: 'Escolha um jogo' })
      return
    }
    setBusy(true)
    await perf.prepare({
      appId: form.appId,
      gameName: games.find((g) => g.id === form.appId)?.title ?? form.appId,
      durationS: Number(dur),
      start,
      startDelayS: Math.max(1, Math.min(600, Number(delay) || 20)),
      showHud: hud,
      expectedPackId: expectedPackId ?? null,
      input: { label: form.label || null, notes: form.notes || null, conditions: toConditions(form) },
    })
    setBusy(false)
  }

  const pose = CAPTURE_POSE[state]
  const fresh = state === 'idle' || state === 'cancelled'
  const plan = status?.plan
  const recordedPct = plan && status?.recordedS != null ? Math.min(100, (status.recordedS / plan.durationS) * 100) : 0

  const footer = (() => {
    if (fresh)
      return (
        <>
          <Button variant="ghost" onClick={onClose}>
            Fechar
          </Button>
          <Button variant="primary" onClick={prepare} loading={busy} disabled={!games.length}>
            Preparar sessão
          </Button>
        </>
      )
    switch (state) {
      case 'checking_dependencies':
        return <span className="text-[12.5px] text-fg-3">Verificando o MangoHud…</span>
      case 'ready':
        return (
          <>
            <Button variant="ghost" onClick={() => void perf.cancel()}>
              Cancelar
            </Button>
            <Button variant="primary" icon={<Play className="size-4" />} onClick={() => void perf.start()}>
              {plan?.launchOptionsReady ? 'Aguardar o jogo' : 'Já colei — aguardar o jogo'}
            </Button>
          </>
        )
      case 'waiting_for_game':
        return (
          <>
            <Button variant="ghost" onClick={() => void perf.cancel()}>
              Cancelar
            </Button>
            {status?.controllable && plan?.start === 'manual' && (
              <Button variant="secondary" icon={<Play className="size-4" />} onClick={() => void perf.logging(true)}>
                Iniciar gravação agora
              </Button>
            )}
            <Button variant="primary" icon={<Gamepad2 className="size-4" />} onClick={launchGame}>
              Abrir o jogo pela Steam
            </Button>
          </>
        )
      case 'recording':
        return (
          <>
            <Button variant="ghost" onClick={() => void perf.cancel()}>
              Descartar
            </Button>
            <Button variant="secondary" icon={<Square className="size-3.5" />} onClick={() => void perf.finish()}>
              Parar e analisar
            </Button>
          </>
        )
      case 'processing':
        return <span className="text-[12.5px] text-fg-3">Calculando estatísticas…</span>
      case 'completed':
        return (
          <>
            <Button variant="ghost" onClick={() => void perf.reset()}>
              Nova sessão
            </Button>
            <Button
              variant="primary"
              onClick={() => {
                if (status?.sessionId) onViewSession(status.sessionId)
                void perf.reset()
                onClose()
              }}
            >
              Ver resultado
            </Button>
          </>
        )
      default:
        return (
          <>
            <Button variant="ghost" onClick={onClose}>
              Fechar
            </Button>
            <Button variant="primary" onClick={() => void perf.reset()}>
              Tentar de novo
            </Button>
          </>
        )
    }
  })()

  return (
    <>
    <Dialog
      open={open && !setup}
      onClose={onClose}
      width={680}
      title={fresh ? 'Nova sessão de benchmark' : CAPTURE_LABEL[state]}
      description={fresh ? 'O FRX prepara o MangoHud só para este jogo e analisa o log quando a captura terminar.' : (status?.message ?? undefined)}
      footer={footer}
      aside={
        <div className="-mt-1 -ml-2">
          <Mascot pose={fresh ? 'prepare' : pose} scale={3} />
        </div>
      }
    >
      {fresh && (
        <div className="space-y-5">
          {intro && <p className="rounded-md border border-accent-line bg-accent-soft px-3.5 py-2.5 text-[13px] text-fg">{intro}</p>}
          <p className="text-[13px] text-fg-2">Prepare com o jogo fechado para carregar a configuração desta sessão. Se mudar a duração ou o HUD, abra o jogo novamente.</p>
          <div className="grid grid-cols-2 gap-4">
            <Field label="Jogo" className="col-span-2">
              <Select
                label="Jogo"
                hideLabel
                className="[&_select]:h-9 [&_select]:w-full [&>div]:w-full"
                value={form.appId || (games[0]?.id ?? '')}
                disabled={!!expectedPackId}
                onChange={(v) => setForm({ ...form, appId: v })}
                options={games.map((g) => ({ value: g.id, label: g.title }))}
              />
            </Field>
            <Field label="Duração da gravação">
              <Segmented label="Duração" size="sm" value={dur} onChange={setDur} options={DURATIONS} />
            </Field>
            <Field label="Início" hint={start === 'auto' ? 'Começa sozinho depois desse tempo, a partir da abertura do jogo.' : 'Pelo FRX (quando conectado) ou com Shift esquerdo + F2 no jogo.'}>
              <div className="flex items-center gap-2">
                <Segmented<StartMode>
                  label="Início da gravação"
                  size="sm"
                  value={start}
                  onChange={setStart}
                  options={[
                    { value: 'auto', label: 'Automático' },
                    { value: 'manual', label: 'Manual' },
                  ]}
                />
                {start === 'auto' && (
                  <span className="flex items-center gap-1.5 text-[12.5px] text-fg-2">
                    após
                    <input
                      aria-label="Segundos até começar"
                      className="h-8 w-14 rounded-sm border border-line-2 bg-surface px-2 text-center text-[13px] text-fg focus:border-accent-line focus:outline-none"
                      inputMode="numeric"
                      value={delay}
                      onChange={(e) => setDelay(e.target.value.replace(/\D/g, '').slice(0, 3))}
                    />
                    s
                  </span>
                )}
              </div>
            </Field>
          </div>
          <Toggle
            label="Mostrar o HUD do MangoHud durante a captura"
            description={
              hud
                ? start === 'manual' ? 'HUD visível. Inicie a gravação ao chegar ao trecho jogável.' : 'O próprio MangoHud inicia a gravação no tempo escolhido.'
                : start === 'auto'
                  ? 'Com o HUD oculto, o MangoHud não inicia sozinho: o FRX inicia a gravação pelo socket de controle quando o jogo conectar.'
                  : 'O overlay fica oculto; só os dados são gravados.'
            }
            checked={hud}
            onChange={setHud}
          />
          <div className="border-t border-line pt-5">
            <p className="eyebrow mb-3">Condições do teste (ajudam a comparar depois)</p>
            <SessionFields value={form} onChange={setForm} games={games} showGame={false} />
          </div>
        </div>
      )}

      {state === 'checking_dependencies' && <p className="py-6 text-[13.5px] text-fg-2">Procurando o MangoHud, a camada Vulkan e as bibliotecas de 32/64 bits…</p>}

      {state === 'ready' && plan?.launchOptionsReady && <p className="text-[13.5px] text-fg-2">MangoHud já configurado para este jogo. Clique em “Aguardar o jogo” e depois abra o jogo pela Steam. Repita o mesmo trecho e as mesmas configurações da referência.</p>}
      {state === 'ready' && plan && !plan.launchOptionsReady && (
        <div className="space-y-4">
        <div className="space-y-2 text-[13px] text-fg-2">
          <p>Para jogos Vulkan, o FRX pode preparar essas opções com backup. Feche o jogo e a Steam você mesmo e revise a prévia antes de confirmar.</p>
          <Button variant="primary" onClick={() => setSetup({ kind: 'apply', recipeId: `frx.capture.${plan.appId}`, appId: plan.appId, title: 'Preparar MangoHud' })}>Configurar pelo FRX (Vulkan)</Button>
        </div>
        <details><summary className="cursor-pointer text-[13px] text-fg-3">Alternativa manual e Steam Flatpak</summary>
        <ol className="space-y-5 text-[13.5px]">
          <li>
            <p className="font-medium">1. Na Steam, abra as Propriedades de {plan.gameName} → Geral → Opções de inicialização.</p>
            <p className="mt-1 text-[12.5px] text-fg-3">O FRX não altera a Steam: você decide o que colar e pode remover depois.</p>
          </li>
          <li>
            <p className="mb-2 font-medium">2. Cole esta linha{plan.existingLaunchOptions ? ', que já mantém as suas opções atuais' : ''}:</p>
            <CodeLine text={plan.launchOptions} label="opções de inicialização" />
            {plan.existingLaunchOptions && (
              <p className="mt-2 text-[12.5px] text-fg-3">
                Opções atuais lidas da Steam: <code className="font-mono text-fg-2">{plan.existingLaunchOptions}</code>
              </p>
            )}
            {plan.flatpakOverride && (
              <div className="mt-3 space-y-2 rounded-md border border-line bg-bg/40 p-3">
                <p className="text-[12.5px] text-fg-2">Steam Flatpak: use esta variante e libere a pasta do FRX para o sandbox (uma vez):</p>
                <CodeLine text={plan.launchOptionsFlatpak} label="opções para Flatpak" />
                <CodeLine text={plan.flatpakOverride} label="comando do Flatpak" />
              </div>
            )}
          </li>
          <li>
            <p className="font-medium">3. Clique em “Já colei — aguardar o jogo” e abra o jogo.</p>
            <p className="mt-1 text-[12.5px] text-fg-3">
              Vá até o trecho que quer medir. {plan.start === 'auto' ? `A gravação começa sozinha ~${plan.startDelayS} s após a abertura` : 'Inicie a gravação pelo FRX ou com Shift esquerdo + F2'} e dura {duration(plan.durationS)}.
            </p>
          </li>
          <li className="text-[12px] text-fg-3">
            Configuração do MangoHud só deste jogo: <code className="font-mono">{plan.configPath}</code>. Logs em <code className="font-mono">{plan.outputDir}</code>.
          </li>
        </ol></details></div>
      )}

      {(state === 'waiting_for_game' || state === 'recording' || state === 'processing') && (
        <div className="space-y-4" aria-live="polite">
          <ul className="space-y-2 text-[13.5px]">
            <Step done={!!status?.hudAttached} active={!status?.hudAttached} label={status?.hudAttached ? `MangoHud conectado ao FRX${status.device ? ` · ${status.device}` : ''}` : 'Aguardando o MangoHud do jogo responder'} />
            <Step done={state !== 'waiting_for_game'} active={state === 'waiting_for_game' && !!status?.hudAttached} label={state === 'waiting_for_game' ? 'Gravação ainda não começou' : `Gravando · ${fmtNum(status?.frames ?? 0)} quadros`} />
            <Step done={false} active={state === 'processing'} label="Análise do log" />
          </ul>
          {state === 'recording' && plan && (
            <div>
              <div className="mb-1.5 flex justify-between text-[12px] text-fg-3">
                <span>{status?.recordedS != null ? duration(status.recordedS) : '—'}</span>
                <span>{duration(plan.durationS)}</span>
              </div>
              <div className="h-1.5 overflow-hidden rounded-full bg-line-2">
                <div className="h-full bg-accent transition-[width] duration-500" style={{ width: `${recordedPct}%` }} />
              </div>
            </div>
          )}
          {state === 'waiting_for_game' && !status?.hudAttached && (
            <p className="text-[12.5px] text-fg-3">
              Se o jogo abrir e nada mudar aqui, confira se a linha foi colada nas opções de inicialização. Em contêineres sem rede compartilhada o FRX não vê o socket, mas detecta o log quando ele aparecer.
            </p>
          )}
        </div>
      )}

      {state === 'completed' && (
        <div className="space-y-3 text-[13.5px] text-fg-2">
          <p className="flex items-center gap-2 rounded-md border border-good/25 bg-good-soft px-3.5 py-3 text-good">
            <Check className="size-4" /> {status?.message}
            {status?.interrupted && <span className="text-caution"> · captura interrompida</span>}
          </p>
          <p className="text-[12.5px] text-fg-3">
            A configuração deste jogo voltou ao repouso e não grava sozinha. Se preparou o MangoHud pelo FRX, pode desfazer essa preparação pelo Histórico após concluir os testes.
          </p>
        </div>
      )}

      {(state === 'failed' || state === 'cancelled') && status?.message && (
        <p className={cn('flex items-start gap-2 rounded-md border px-3.5 py-3 text-[13.5px]', state === 'failed' ? 'border-bad/25 bg-bad-soft text-bad' : 'border-line bg-raised text-fg-2')}>
          <X className="mt-0.5 size-4 shrink-0" /> {status.message}
        </p>
      )}
    </Dialog>
    {setup && <ApplyFixDialog target={setup} onClose={() => { setSetup(null); void prepare() }} />}
    </>
  )
}

function Step({ done, active, label }: { done: boolean; active: boolean; label: string }) {
  return (
    <li className={cn('flex h-9 items-center gap-3 rounded-md px-2', active && 'bg-raised')}>
      <span
        className={cn(
          'grid size-5 place-items-center rounded-[5px] border',
          done ? 'border-good/40 bg-good-soft text-good' : active ? 'border-accent-line text-accent' : 'border-line-2 text-fg-3',
        )}
        aria-hidden
      >
        {done ? <Check className="size-3" strokeWidth={3} /> : active ? <span className="anim-pulse-dot size-1.5 bg-accent" /> : null}
      </span>
      <span className={done || active ? 'text-fg' : 'text-fg-3'}>{label}</span>
    </li>
  )
}

/** Faixa de status da captura, visível na página mesmo com o diálogo fechado. */
export function CaptureStrip({ status, onOpen }: { status: CaptureStatus; onOpen: () => void }) {
  const perf = usePerformance()
  const plan = status.plan
  const s = status.state
  if (s === 'idle') return null
  return (
    <section className="flex items-center gap-6 rounded-xl border border-line bg-surface px-7 py-5" aria-live="polite">
      <Mascot pose={CAPTURE_POSE[s]} scale={4} />
      <div className="min-w-0 flex-1">
        <p className="font-display text-[20px] font-semibold tracking-[-0.02em]">
          {CAPTURE_LABEL[s]}
          {plan && <span className="text-fg-3"> · {plan.gameName}</span>}
        </p>
        <p className="mt-1 truncate text-[13px] text-fg-2">{status.message}</p>
        {s === 'recording' && plan && (
          <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-line-2">
            <div className="h-full bg-accent transition-[width] duration-500" style={{ width: `${status.recordedS != null ? Math.min(100, (status.recordedS / plan.durationS) * 100) : 0}%` }} />
          </div>
        )}
      </div>
      {s === 'recording' && (
        <div className="w-28 text-right">
          <p className="tnum font-display text-[32px] leading-none font-bold tracking-[-0.04em]">{fmtNum(status.frames)}</p>
          <p className="text-[12px] text-fg-3">quadros</p>
        </div>
      )}
      <div className="flex shrink-0 gap-2">
        {(s === 'completed' || s === 'failed' || s === 'cancelled') && (
          <Button variant="ghost" onClick={() => void perf.reset()}>
            Dispensar
          </Button>
        )}
        <Button variant="secondary" onClick={onOpen}>
          Detalhes
        </Button>
      </div>
    </section>
  )
}
