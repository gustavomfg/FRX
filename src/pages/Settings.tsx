import type { ReactNode } from 'react'
import { Check, Lock, RotateCcw, Trash2 } from 'lucide-react'
import { Page } from '@/layouts/Page'
import { Segmented, Select, Toggle } from '@/components/ui/Controls'
import { Button } from '@/components/ui/Button'
import { DemoTag } from '@/components/ui/Status'
import { Logo } from '@/components/brand/Logo'
import { Mascot } from '@/mascot/Mascot'
import type { Pose } from '@/mascot/sprites'
import { useDispatch, useSettings, useToast } from '@/state/store'
import { useMotion, useSystemReducedMotion } from '@/state/motion'
import { platform } from '@/services/platform'
import { useAppInfo } from '@/services/useAppInfo'
import type { Settings } from '@/types'
import { cn } from '@/lib/cn'

const ACCENTS: { id: Settings['accent']; label: string; color: string }[] = [
  { id: 'manteiga', label: 'Manteiga', color: '#F1C66A' },
  { id: 'menta', label: 'Menta', color: '#9ED6B4' },
  { id: 'coral', label: 'Coral', color: '#F0957E' },
  { id: 'ceu', label: 'Céu', color: '#92C4EA' },
]

const THEMES = [
  { id: 'frx-dark', label: 'FRX Escuro', available: true, swatch: ['#101011', '#1B1B1D', '#F5F5F2'] },
  { id: 'papel', label: 'Papel', available: false, swatch: ['#F3F1EA', '#E6E3D8', '#1B1B1D'] },
  { id: 'contraste', label: 'Alto contraste', available: false, swatch: ['#000000', '#141414', '#FFFFFF'] },
]

const POSES: { pose: Pose; label: string }[] = [
  { pose: 'wave', label: 'Início' },
  { pose: 'rest', label: 'Biblioteca' },
  { pose: 'inspect', label: 'Diagnóstico' },
  { pose: 'fix', label: 'Correções' },
  { pose: 'observe', label: 'Performance' },
  { pose: 'celebrate', label: 'Melhora' },
  { pose: 'worried', label: 'Regressão' },
]

export function SettingsPage() {
  const s = useSettings()
  const dispatch = useDispatch()
  const toast = useToast()
  const systemReduce = useSystemReducedMotion()
  const effective = useMotion()
  const set = (patch: Partial<Settings>) => dispatch({ type: 'settings', patch })
  const info = useAppInfo()

  return (
    <Page narrow>
      <header>
        <p className="eyebrow">Configurações</p>
        <h1 className="mt-3 font-display text-[36px] leading-none font-bold tracking-[-0.04em]">Preferências</h1>
        <p className="mt-2 text-[14px] text-fg-2">Salvas apenas neste dispositivo.</p>
      </header>

      <Section title="Aparência" description="O FRX nasce escuro. A estrutura de temas já está pronta para novas paletas.">
        <div className="grid grid-cols-3 gap-3" role="radiogroup" aria-label="Tema">
          {THEMES.map((t) => {
            const active = s.theme === t.id
            return (
              <button
                key={t.id}
                role="radio"
                aria-checked={active}
                disabled={!t.available}
                onClick={() => t.available && set({ theme: t.id as Settings['theme'] })}
                className={cn(
                  'group relative overflow-hidden rounded-lg border p-3 text-left transition-colors',
                  active ? 'border-accent-line bg-surface' : 'border-line',
                  t.available ? 'hover:border-line-3' : 'cursor-not-allowed opacity-55',
                )}
              >
                <div className="flex h-16 overflow-hidden rounded-md border border-line" aria-hidden>
                  <div className="w-1/4" style={{ background: t.swatch[1] }} />
                  <div className="flex flex-1 flex-col justify-center gap-1.5 px-3" style={{ background: t.swatch[0] }}>
                    <div className="h-1.5 w-2/3 rounded-full" style={{ background: t.swatch[2] }} />
                    <div className="h-1.5 w-1/3 rounded-full opacity-50" style={{ background: t.swatch[2] }} />
                  </div>
                </div>
                <div className="mt-2.5 flex items-center justify-between">
                  <span className="text-[13px] font-medium">{t.label}</span>
                  {active ? <Check className="size-3.5 text-accent" /> : !t.available ? <span className="flex items-center gap-1 text-[11px] text-fg-3"><Lock className="size-3" /> Em breve</span> : null}
                </div>
              </button>
            )
          })}
        </div>

        <Row label="Cor de destaque" description="Usada com parcimônia: ações principais, seleção e o gorro do Pim.">
          <div className="flex gap-2" role="radiogroup" aria-label="Cor de destaque">
            {ACCENTS.map((a) => {
              const active = s.accent === a.id
              return (
                <button
                  key={a.id}
                  role="radio"
                  aria-checked={active}
                  aria-label={a.label}
                  title={a.label}
                  onClick={() => set({ accent: a.id })}
                  className={cn('grid size-8 place-items-center rounded-md border-2 transition-transform duration-(--frx-dur-fast) hover:scale-105', active ? 'border-fg' : 'border-transparent')}
                >
                  <span className="grid size-6 place-items-center rounded-[5px]" style={{ background: a.color }}>
                    {active && <Check className="size-3.5 text-black/70" strokeWidth={3} />}
                  </span>
                </button>
              )
            })}
          </div>
        </Row>
      </Section>

      <Section title="Mascotes" description="O Pim mora dentro do FRX e reage ao que acontece em cada tela. Ele nunca bloqueia a navegação.">
        <Toggle label="Mostrar o Pim" description="Desativado, a interface fica completamente sóbria." checked={s.mascots} onChange={(v) => set({ mascots: v })} />
        <div className={cn('mt-5 grid grid-cols-7 gap-2 rounded-lg border border-line bg-surface p-4 transition-opacity', !s.mascots && 'opacity-35')} aria-hidden={!s.mascots}>
          {POSES.map((p) => (
            <div key={p.pose} className="flex flex-col items-center gap-1.5">
              <Mascot pose={p.pose} scale={3} force />
              <span className="text-[11.5px] text-fg-3">{p.label}</span>
            </div>
          ))}
        </div>
      </Section>

      <Section title="Animações" description="Movimento leve e intencional. Animações decorativas param quando a janela fica oculta.">
        <Row label="Intensidade" description="Suave reduz durações e a velocidade do Pim pela metade.">
          <Segmented<Settings['motion']>
            label="Intensidade das animações"
            value={s.motion}
            onChange={(v) => set({ motion: v })}
            options={[
              { value: 'full', label: 'Completa' },
              { value: 'calm', label: 'Suave' },
              { value: 'off', label: 'Desligada' },
            ]}
          />
        </Row>
        <Row
          label="Redução de movimento"
          description={
            <>
              O sistema {systemReduce ? <strong className="font-medium text-fg">pede</strong> : 'não pede'} redução de movimento. Efetivo agora:{' '}
              <span className="text-fg">{effective === 'off' ? 'sem movimento' : effective === 'calm' ? 'suave' : 'completo'}</span>.
            </>
          }
        >
          <Segmented<Settings['reduceMotion']>
            label="Redução de movimento"
            value={s.reduceMotion}
            onChange={(v) => set({ reduceMotion: v })}
            options={[
              { value: 'sistema', label: 'Seguir o sistema' },
              { value: 'sempre', label: 'Sempre reduzir' },
            ]}
          />
        </Row>
      </Section>

      <Section title="Geral">
        <Toggle label="Verificar a biblioteca ao abrir" description="Procura jogos novos ou removidos ao iniciar (simulado)." checked={s.scanOnStartup} onChange={(v) => set({ scanOnStartup: v })} />
        <Divider />
        <Toggle
          label="Confirmar antes de aplicar correções"
          description="Mostra um passo extra de confirmação com o resumo do backup."
          checked={s.confirmBeforeApply}
          onChange={(v) => set({ confirmBeforeApply: v })}
        />
        <Divider />
        <Toggle label="Barra lateral compacta" description="Mostra apenas ícones. Também é ativada automaticamente em janelas estreitas." checked={s.compactSidebar} onChange={(v) => set({ compactSidebar: v })} />
        <Divider />
        <Row label="Manter backups por">
          <Select
            label="Retenção de backups"
            hideLabel
            value={String(s.backupRetentionDays) as '7' | '30' | '90'}
            onChange={(v) => set({ backupRetentionDays: Number(v) as Settings['backupRetentionDays'] })}
            options={[
              { value: '7', label: '7 dias' },
              { value: '30', label: '30 dias' },
              { value: '90', label: '90 dias' },
            ]}
          />
        </Row>
        <Row label="Unidade de temperatura">
          <Segmented<Settings['tempUnit']>
            label="Unidade de temperatura"
            size="sm"
            value={s.tempUnit}
            onChange={(v) => set({ tempUnit: v })}
            options={[
              { value: 'C', label: '°C' },
              { value: 'F', label: '°F' },
            ]}
          />
        </Row>
      </Section>

      <Section title="Sobre">
        <div className="flex items-start gap-5">
          <div className="grid size-16 shrink-0 place-items-center rounded-lg border border-line bg-surface">
            <Logo compact />
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <span className="font-display text-[22px] font-bold tracking-[-0.04em]">FRX</span>
              <span className="font-mono text-[12px] text-fg-3">{platform.version}</span>
              <DemoTag>Modo demonstrativo</DemoTag>
            </div>
            <p className="mt-0.5 font-pixel text-[10px] tracking-[0.08em] text-accent uppercase">Play. Fix. Repeat.</p>
            <p className="mt-3 text-[13.5px] text-fg-2">
              Gerenciador de compatibilidade e otimização de jogos para Linux. Esta versão é um protótipo de interface: jogos, sistema, problemas, correções e medições são fictícios.
            </p>
            {info && (
              <p className="mt-3 text-[12.5px] text-fg-3">
                Execução:{' '}
                <span className="text-fg-2">
                  {info.runtime === 'tauri'
                    ? [`App desktop · Tauri ${info.tauriVersion}`, info.distro, info.kernel && `kernel ${info.kernel}`, info.desktop && `${info.desktop}${info.session ? ` (${info.session})` : ''}`].filter(Boolean).join(' · ')
                    : 'Navegador · modo de desenvolvimento web'}
                </span>
              </p>
            )}
            <ul className="mt-4 grid grid-cols-2 gap-x-6 gap-y-1.5 text-[12.5px] text-fg-3">
              {['Nenhum arquivo é lido ou alterado', 'Nenhum download é feito', 'Nenhum comando é executado', 'Benchmarks são gerados por simulação', 'Fontes (ProtonDB, GitHub…) são ilustrativas', 'Apenas preferências ficam salvas'].map((t) => (
                <li key={t} className="flex items-center gap-2">
                  <span className="size-1 bg-fg-3" aria-hidden />
                  {t}
                </li>
              ))}
            </ul>
            <div className="mt-6 flex gap-2">
              <Button
                variant="outline"
                size="sm"
                icon={<RotateCcw className="size-3.5" />}
                onClick={() => {
                  dispatch({ type: 'resetSettings' })
                  toast({ tone: 'info', title: 'Preferências restauradas' })
                }}
              >
                Restaurar preferências
              </Button>
              <Button
                variant="danger"
                size="sm"
                icon={<Trash2 className="size-3.5" />}
                onClick={() => {
                  dispatch({ type: 'resetDemo' })
                  toast({ tone: 'info', title: 'Dados da demonstração reiniciados', body: 'Correções, histórico e benchmarks voltaram ao estado inicial.' })
                }}
              >
                Reiniciar dados da demo
              </Button>
            </div>
          </div>
        </div>
      </Section>
    </Page>
  )
}

function Section({ title, description, children }: { title: string; description?: string; children: ReactNode }) {
  return (
    <section className="mt-12 grid grid-cols-[220px_minmax(0,1fr)] gap-10 border-t border-line pt-8">
      <div>
        <h2 className="font-display text-[17px] font-semibold tracking-[-0.02em]">{title}</h2>
        {description && <p className="mt-1.5 text-[12.5px] leading-relaxed text-fg-3">{description}</p>}
      </div>
      <div className="min-w-0 space-y-5">{children}</div>
    </section>
  )
}

function Row({ label, description, children }: { label: string; description?: ReactNode; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-6">
      <div className="min-w-0">
        <p className="text-[14px] font-medium">{label}</p>
        {description && <p className="mt-0.5 text-[13px] text-fg-2">{description}</p>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  )
}

function Divider() {
  return <div className="h-px bg-line" aria-hidden />
}
