import { BookOpen, ExternalLink, TriangleAlert, Wrench } from 'lucide-react'
import { Button } from '@/components/ui/Button'
import { Disclosure } from '@/components/ui/Disclosure'
import { Pill } from '@/components/ui/Status'
import { PLAN_STATUS } from '@/components/fix/engineMeta'
import type { Fit, Support, SystemAction, WikiAction, WikiSuggestion } from '@/types/hardware'

const NEUTRAL = { color: 'var(--frx-text-2)', soft: 'var(--frx-raised)' }

/** Selo de compatibilidade: a página é do modelo detectado ou é geral. */
export function fitBadge(fit: Fit, laptop: string | null) {
  if (fit === 'general') return { label: 'Geral', ...NEUTRAL }
  return { label: laptop ? `Para o seu ${laptop}` : 'Para o seu modelo', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)' }
}

export const SUPPORT: Record<Support, { label: string; color: string; soft: string }> = {
  works: { label: 'Funciona segundo a wiki', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)' },
  partial: { label: 'Funciona com ajustes', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)' },
  broken: { label: 'Não funciona segundo a wiki', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)' },
  unknown: { label: 'Suporte não informado', ...NEUTRAL },
}

const ACTION_KIND: Record<SystemAction['type'], string> = {
  add_modprobe_option: 'Opção de módulo',
  add_systemd_service: 'Serviço do systemd',
  add_kernel_parameters: 'Parâmetro do kernel',
}

/**
 * Cartão de sugestão, no mesmo desenho do cartão da Central de Resolução.
 * O trecho é texto da Arch Wiki para leitura — comandos nele nunca são executados.
 * “Aplicar correção” abre a prévia do Fix Engine; nada é aplicado sem ela.
 */
export function SuggestionCard({
  s,
  laptop,
  open,
  apply,
}: {
  s: WikiSuggestion
  laptop: string | null
  open: (url: string) => void
  /** Abre a prévia do Fix Engine para uma instrução reconhecida. */
  apply?: (action: WikiAction) => void
}) {
  const badge = fitBadge(s.fit, laptop)
  return (
    <li className="rounded-sm border border-line p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <p className="text-[14px] font-medium text-fg">{s.title}</p>
            <Pill color={badge.color} soft={badge.soft}>
              {badge.label}
            </Pill>
          </div>
          <p className="mt-0.5 flex items-center gap-1.5 text-[12px] text-fg-3">
            <BookOpen className="size-3" aria-hidden />
            Arch Wiki · {s.section ? `${s.page} › ${s.section}` : s.page}
          </p>
        </div>
        <Button variant="secondary" size="sm" icon={<ExternalLink className="size-3.5" />} onClick={() => open(s.url)}>
          Abrir na wiki
        </Button>
      </div>
      {s.excerpt &&
        (s.fit === 'general' ? (
          <p className="mt-3 text-[13px] text-fg-2">{s.excerpt}</p>
        ) : (
          <Disclosure title="Trecho da página" className="mt-3" titleClassName="text-[12.5px] text-fg-2">
            <blockquote className="mt-2 border-l-2 border-line-2 pl-3 font-mono text-[12.5px] whitespace-pre-wrap text-fg-2">{s.excerpt}</blockquote>
            <p className="mt-2 text-[11.5px] text-fg-3">Texto da Arch Wiki (GFDL 1.3+), só para leitura. Confira a página antes de rodar qualquer comando.</p>
          </Disclosure>
        ))}
      {s.actions.length > 0 && (
        <ul className="mt-3 space-y-2" aria-label="Correções aplicáveis">
          {s.actions.map((a) => (
            <li key={a.id} className="flex flex-wrap items-center gap-3 rounded-sm border border-line-2 bg-bg/40 px-3 py-2.5">
              <div className="min-w-0 flex-1">
                <p className="text-[11.5px] text-fg-3">{ACTION_KIND[a.action.type]}</p>
                <code className="block font-mono text-[12.5px] break-all text-fg">{a.snippet}</code>
                {a.conditional && (
                  <p className="mt-1 flex items-center gap-1.5 text-[12px] text-caution">
                    <TriangleAlert className="size-3" aria-hidden /> A fonte impõe condições: leia antes de aplicar.
                  </p>
                )}
                {a.check && <QuickCheckLine check={a.check} />}
                {a.action.type === 'add_kernel_parameters' && a.check?.applicable !== false && (
                  <p className="mt-1 text-[12px] text-fg-3">
                    Risco alto: altera /etc/default/grub. O comando só sobrescreve o arquivo se ele não tiver mudado desde a prévia.
                  </p>
                )}
              </div>
              {apply && (
                <Button
                  size="sm"
                  variant="primary"
                  icon={<Wrench className="size-3.5" />}
                  disabled={a.check?.applicable === false}
                  title={a.check?.applicable === false ? a.check.detail : undefined}
                  onClick={() => apply(a)}
                >
                  Aplicar correção
                </Button>
              )}
            </li>
          ))}
        </ul>
      )}
    </li>
  )
}

/** Resultado da checagem leve da busca: o status no card, antes da prévia. */
function QuickCheckLine({ check }: { check: { applicable: boolean; detail: string } }) {
  if (check.applicable) {
    const meta = PLAN_STATUS['requires-manual-step']
    return (
      <p className="mt-1.5">
        <Pill color={meta.color} soft={meta.soft} title={check.detail}>
          {meta.label}
        </Pill>
      </p>
    )
  }
  const meta = PLAN_STATUS.incompatible
  return (
    <div className="mt-1.5">
      <Pill color={meta.color} soft={meta.soft}>
        {meta.label} · Este sistema não usa GRUB
      </Pill>
      <p className="mt-1 text-[12px] text-fg-3">{check.detail}</p>
    </div>
  )
}
