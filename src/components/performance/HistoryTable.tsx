import { useState } from 'react'
import { Download, Pencil, Trash2 } from 'lucide-react'
import type { SessionMeta } from '@/types/performance'
import { Dialog } from '@/components/ui/Dialog'
import { Button, IconButton } from '@/components/ui/Button'
import { QUALITY, duration, num } from './format'
import { fmtDateTime } from '@/lib/format'
import { cn } from '@/lib/cn'

function PickButton({ active, children, ...rest }: React.ButtonHTMLAttributes<HTMLButtonElement> & { active: boolean }) {
  return (
    <button
      aria-pressed={active}
      className={cn(
        'h-7 rounded-sm border px-2.5 text-[12px] font-medium transition-colors disabled:opacity-35',
        active ? 'border-accent-line bg-accent-soft text-accent' : 'border-line text-fg-2 hover:border-line-2 hover:text-fg',
      )}
      {...rest}
    >
      {children}
    </button>
  )
}

interface Props {
  sessions: SessionMeta[]
  viewId: string | null
  beforeId: string | null
  afterId: string | null
  onView: (id: string) => void
  onBefore: (id: string | null) => void
  onAfter: (id: string | null) => void
  onEdit: (m: SessionMeta) => void
  onDelete: (m: SessionMeta) => void
  onExport: (m: SessionMeta) => void
}

/** Histórico de sessões reais — mesmo desenho da tabela da versão demonstrativa. */
export function HistoryTable({ sessions, viewId, beforeId, afterId, onView, onBefore, onAfter, onEdit, onDelete, onExport }: Props) {
  return (
    <div className="overflow-x-auto rounded-xl border border-line">
      <table className="w-full text-[13px]">
        <thead>
          <tr className="border-b border-line bg-surface text-left text-[11.5px] font-medium text-fg-3">
            <th className="px-5 py-2.5 font-medium">Sessão</th>
            <th className="px-3 py-2.5 font-medium">Quando</th>
            <th className="px-3 py-2.5 text-right font-medium">FPS médio</th>
            <th className="px-3 py-2.5 text-right font-medium">1% low</th>
            <th className="px-3 py-2.5 text-right font-medium">Duração</th>
            <th className="px-3 py-2.5 font-medium">Qualidade</th>
            <th className="px-5 py-2.5 text-right font-medium">Comparar como</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-line">
          {sessions.map((m) => {
            const isB = m.id === beforeId
            const isA = m.id === afterId
            const q = QUALITY[m.quality.level]
            return (
              <tr key={m.id} className={cn('group transition-colors', m.id === viewId || isA || isB ? 'bg-surface/70' : 'hover:bg-surface/40')}>
                <td className="max-w-[260px] px-5 py-2.5">
                  <button onClick={() => onView(m.id)} className="block w-full truncate text-left font-medium text-fg hover:text-accent" title={m.label}>
                    {m.label}
                  </button>
                  <span className="block truncate text-[11.5px] text-fg-3">
                    {m.source === 'synthetic' ? 'Sintético · teste' : m.source === 'assisted' ? 'Captura FRX' : 'Importado'}
                    {m.conditions.preset && ` · ${m.conditions.preset}`}
                    {m.conditions.resolution && ` · ${m.conditions.resolution}`}
                  </span>
                </td>
                <td className="px-3 py-2.5 whitespace-nowrap text-fg-2">{fmtDateTime(m.capturedAt ?? m.createdAt)}</td>
                <td className="tnum px-3 py-2.5 text-right font-mono text-fg">{num(m.stats.fps.avg, 1)}</td>
                <td className="tnum px-3 py-2.5 text-right font-mono text-fg-2">
                  {num(m.stats.fps.low1, 1)}
                  {m.stats.fps.lowsEstimated && m.stats.fps.low1 != null && <span className="text-caution">*</span>}
                </td>
                <td className="tnum px-3 py-2.5 text-right font-mono text-fg-2">{duration(m.stats.durationS)}</td>
                <td className="px-3 py-2.5">
                  <span className="inline-flex items-center gap-1.5 text-[12px]" style={{ color: q.color }} title={q.help}>
                    <span className="size-1.5" style={{ background: q.color }} aria-hidden />
                    {m.quality.level === 'good' ? 'Boa' : m.quality.level === 'usable' ? 'Com ressalvas' : 'Inadequada'}
                  </span>
                </td>
                <td className="px-5 py-2">
                  <div className="flex items-center justify-end gap-1">
                    <PickButton active={isB} onClick={() => onBefore(isB ? null : m.id)} disabled={isA}>
                      Antes
                    </PickButton>
                    <PickButton active={isA} onClick={() => onAfter(isA ? null : m.id)} disabled={isB}>
                      Depois
                    </PickButton>
                    <span className="mx-1 h-4 w-px bg-line-2" aria-hidden />
                    <IconButton label={`Editar ${m.label}`} onClick={() => onEdit(m)} className="size-7">
                      <Pencil className="size-3.5" />
                    </IconButton>
                    <IconButton label={`Exportar ${m.label}`} onClick={() => onExport(m)} className="size-7">
                      <Download className="size-3.5" />
                    </IconButton>
                    <IconButton label={`Excluir ${m.label}`} onClick={() => onDelete(m)} className="size-7 hover:text-bad">
                      <Trash2 className="size-3.5" />
                    </IconButton>
                  </div>
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}

export function DeleteSessionDialog({ meta, onClose, onConfirm }: { meta: SessionMeta | null; onClose: () => void; onConfirm: () => Promise<void> }) {
  const [busy, setBusy] = useState(false)
  return (
    <Dialog
      open={!!meta}
      onClose={onClose}
      locked={busy}
      title="Excluir sessão?"
      description={meta ? `“${meta.label}” e o log original guardado pelo FRX serão apagados. Isso não pode ser desfeito.` : undefined}
      width={460}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            Cancelar
          </Button>
          <Button
            variant="danger"
            loading={busy}
            icon={<Trash2 className="size-4" />}
            onClick={async () => {
              setBusy(true)
              await onConfirm()
              setBusy(false)
            }}
          >
            Excluir
          </Button>
        </>
      }
    >
      <p className="text-[13px] text-fg-2">O arquivo de log que você importou continua no lugar de onde veio.</p>
    </Dialog>
  )
}
