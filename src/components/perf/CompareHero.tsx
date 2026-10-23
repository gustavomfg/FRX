import { motion } from 'motion/react'
import { ArrowDown, ArrowUp, Equal, CircleHelp } from 'lucide-react'
import type { Comparison } from '@/data/benchmarks'
import { AnimatedNumber } from '@/components/ui/AnimatedNumber'
import { DemoTag } from '@/components/ui/Status'
import { Mascot } from '@/mascot/Mascot'
import { useMotion } from '@/state/motion'
import { fmtNum } from '@/lib/format'
import { VERDICT } from './verdict'
import { cn } from '@/lib/cn'

const ICON = { melhora: ArrowUp, regressao: ArrowDown, estavel: Equal, inconclusivo: CircleHelp }

/**
 * A comparação antes/depois. O número grande sobe, as barras crescem e o Pim
 * reage ao veredito — que nunca é apresentado como garantia.
 */
export function CompareHero({ c, gameTitle }: { c: Comparison; gameTitle: string }) {
  const v = VERDICT[c.verdict]
  const Icon = ICON[c.verdict]
  const motionLevel = useMotion()
  const max = Math.max(c.before.avgFps, c.after.avgFps) * 1.08
  const sign = c.deltaPct > 0 ? '+' : c.deltaPct < 0 ? '−' : '±'
  const noise = (Math.sqrt(c.before.stdev ** 2 + c.after.stdev ** 2) / c.before.avgFps) * 100

  return (
    <section aria-labelledby="comp-title" className="relative overflow-hidden rounded-xl border border-line bg-surface">
      <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1.25fr)]">
        {/* Veredito */}
        <div className="flex flex-col justify-between border-r border-line p-7">
          <div>
            <div className="flex items-center gap-2">
              <span className="inline-flex h-6 items-center gap-1.5 rounded-xs px-2 text-[12px] font-semibold" style={{ background: v.soft, color: v.color }}>
                <Icon className="size-3.5" strokeWidth={2.4} aria-hidden />
                {v.label}
              </span>
              <DemoTag>Resultado demo</DemoTag>
            </div>
            <h2 id="comp-title" className="sr-only">
              Comparação de desempenho de {gameTitle}
            </h2>
            <div className="mt-5 flex items-end gap-4">
              <p
                className={cn('tnum font-display text-[clamp(56px,6.2vw,84px)] leading-[0.85] font-bold tracking-[-0.06em]', c.verdict === 'inconclusivo' && 'text-fg-2')}
                style={{ color: c.verdict === 'melhora' ? 'var(--frx-text)' : c.verdict === 'regressao' ? 'var(--frx-bad)' : undefined }}
              >
                <AnimatedNumber value={Math.abs(c.deltaPct)} prefix={sign} suffix="%" duration={1.3} />
              </p>
              <Mascot pose={v.pose} scale={4} className="-mb-2" />
            </div>
            <p className="mt-3 text-[13.5px] text-fg-2">
              FPS médio em <span className="text-fg">{gameTitle}</span>
              {c.verdict === 'inconclusivo' && <span className="text-caution"> · ruído ±{fmtNum(noise, 0)}%</span>}
            </p>
            <dl className="mt-6 space-y-1.5 text-[12.5px]">
              {[
                ['Passadas', `${c.after.durationS} s · ${c.after.preset}`],
                ['Variação entre passadas', `±${fmtNum(c.before.stdev, 1)} → ±${fmtNum(c.after.stdev, 1)} FPS`],
                ['Correções no “depois”', c.after.fixIds.length ? `${c.after.fixIds.length} ativa${c.after.fixIds.length > 1 ? 's' : ''}` : 'nenhuma'],
              ].map(([k, val]) => (
                <div key={k} className="flex justify-between gap-4 border-b border-line pb-1.5">
                  <dt className="text-fg-3">{k}</dt>
                  <dd className="tnum text-fg-2">{val}</dd>
                </div>
              ))}
            </dl>
          </div>
          <p className="mt-6 max-w-sm text-[12.5px] leading-relaxed text-fg-3">
            {v.text} Dados simulados; resultados reais dependem do hardware e nunca são garantidos.
          </p>
        </div>

        {/* Antes × depois */}
        <div className="p-7">
          <div className="space-y-6">
            <BarPair label="FPS médio" unit="FPS" before={c.before.avgFps} after={c.after.avgFps} max={max} better="higher" verdictColor={v.color} animate={motionLevel !== 'off'} />
            <BarPair label="1% low" unit="FPS" before={c.before.low1} after={c.after.low1} max={max} better="higher" verdictColor={v.color} animate={motionLevel !== 'off'} delay={0.12} />
            <BarPair
              label="Frametime médio"
              unit="ms"
              digits={1}
              before={c.before.frametimeAvg}
              after={c.after.frametimeAvg}
              max={Math.max(c.before.frametimeAvg, c.after.frametimeAvg) * 1.08}
              better="lower"
              verdictColor={v.color}
              animate={motionLevel !== 'off'}
              delay={0.24}
            />
          </div>
          <div className="mt-6 flex items-center gap-5 border-t border-line pt-4 text-[12px] text-fg-3">
            <span className="flex items-center gap-2">
              <span className="h-2 w-4 rounded-[2px] bg-[repeating-linear-gradient(135deg,var(--frx-chart-base)_0_2px,transparent_2px_4px)]" aria-hidden />
              Antes · {c.before.label}
            </span>
            <span className="flex items-center gap-2">
              <span className="h-2 w-4 rounded-[2px] bg-accent" aria-hidden />
              Depois · {c.after.label}
            </span>
          </div>
        </div>
      </div>
    </section>
  )
}

function BarPair({
  label,
  unit,
  before,
  after,
  max,
  better,
  digits = 0,
  animate,
  delay = 0,
}: {
  label: string
  unit: string
  before: number
  after: number
  max: number
  better: 'higher' | 'lower'
  digits?: number
  verdictColor: string
  animate: boolean
  delay?: number
}) {
  const delta = ((after - before) / before) * 100
  const good = better === 'higher' ? delta > 0 : delta < 0
  const flat = Math.abs(delta) < 3
  return (
    <div>
      <div className="mb-2 flex items-baseline justify-between">
        <span className="text-[13px] font-medium text-fg">{label}</span>
        <span className={cn('tnum font-mono text-[12px]', flat ? 'text-fg-3' : good ? 'text-good' : 'text-bad')}>
          {delta > 0 ? '+' : delta < 0 ? '−' : '±'}
          {fmtNum(Math.abs(delta), 0)}%{better === 'lower' && !flat && <span className="text-fg-3"> {good ? '(menor é melhor)' : ''}</span>}
        </span>
      </div>
      <div className="space-y-1.5">
        <Bar value={before} max={max} unit={unit} digits={digits} hatched animate={animate} delay={delay} />
        <Bar value={after} max={max} unit={unit} digits={digits} animate={animate} delay={delay + 0.15} />
      </div>
    </div>
  )
}

function Bar({ value, max, unit, digits, hatched, animate, delay }: { value: number; max: number; unit: string; digits: number; hatched?: boolean; animate: boolean; delay: number }) {
  const pct = (value / max) * 100
  return (
    <div className="flex items-center gap-3">
      <div className="relative h-[18px] flex-1">
        <motion.div
          className={cn('absolute inset-y-0 left-0 rounded-r-[4px]', hatched ? 'bg-[repeating-linear-gradient(135deg,var(--frx-chart-base)_0_2px,transparent_2px_5px)] opacity-80' : 'bg-accent')}
          initial={animate ? { width: 0 } : false}
          animate={{ width: `${pct}%` }}
          transition={{ duration: 0.9, delay, ease: [0.16, 1, 0.3, 1] }}
        />
      </div>
      <span className={cn('tnum w-[78px] text-right font-mono text-[13px]', hatched ? 'text-fg-2' : 'font-semibold text-fg')}>
        <AnimatedNumber value={value} digits={digits} delay={delay} /> <span className="text-[11px] font-normal text-fg-3">{unit}</span>
      </span>
    </div>
  )
}
