import { useEffect, useState } from 'react'
import { FileUp, Save } from 'lucide-react'
import type { Conditions, SessionMeta } from '@/types/performance'
import type { Game } from '@/types'
import { Dialog } from '@/components/ui/Dialog'
import { Button } from '@/components/ui/Button'
import { Select } from '@/components/ui/Controls'
import { cn } from '@/lib/cn'

/** Campos que o log não informa: resolução, preset, limite de FPS, VSync e notas. */
export interface FormValues {
  appId: string
  label: string
  notes: string
  resolution: string
  preset: string
  fpsLimit: string
  vsync: 'desconhecido' | 'ligado' | 'desligado'
  proton: string
}

export const emptyForm = (appId = ''): FormValues => ({ appId, label: '', notes: '', resolution: '', preset: '', fpsLimit: '', vsync: 'desconhecido', proton: '' })

export function toConditions(v: FormValues, base?: Conditions): Conditions {
  const s = (x: string) => (x.trim() ? x.trim() : null)
  return {
    proton: s(v.proton) ?? base?.proton ?? null,
    gpu: base?.gpu ?? null,
    driver: base?.driver ?? null,
    kernel: base?.kernel ?? null,
    os: base?.os ?? null,
    cpu: base?.cpu ?? null,
    cpuScheduler: base?.cpuScheduler ?? null,
    resolution: s(v.resolution),
    preset: s(v.preset),
    fpsLimit: s(v.fpsLimit),
    vsync: v.vsync === 'desconhecido' ? null : v.vsync === 'ligado',
  }
}

const input = 'h-9 w-full rounded-sm border border-line-2 bg-surface px-3 text-[13px] text-fg placeholder:text-fg-3 transition-colors hover:border-line-3 focus:border-accent-line focus:outline-none'

export function Field({ label, children, hint, className }: { label: string; children: React.ReactNode; hint?: string; className?: string }) {
  return (
    <label className={cn('block', className)}>
      <span className="mb-1.5 block text-[12px] text-fg-3">{label}</span>
      {children}
      {hint && <span className="mt-1 block text-[11.5px] text-fg-3">{hint}</span>}
    </label>
  )
}

export function SessionFields({ value, onChange, games, showGame = true, showLabel = true }: { value: FormValues; onChange: (v: FormValues) => void; games: Game[]; showGame?: boolean; showLabel?: boolean }) {
  const set = (patch: Partial<FormValues>) => onChange({ ...value, ...patch })
  return (
    <div className="grid grid-cols-2 gap-4">
      {showGame && (
        <Field label="Jogo" className="col-span-2">
          <Select
            label="Jogo"
            hideLabel
            className="[&_select]:h-9 [&_select]:w-full [&>div]:w-full"
            value={value.appId || 'nenhum'}
            onChange={(v) => set({ appId: v === 'nenhum' ? '' : v })}
            options={[{ value: 'nenhum', label: 'Sem jogo associado' }, ...games.map((g) => ({ value: g.id, label: g.title }))]}
          />
        </Field>
      )}
      {showLabel && (
        <Field label="Nome da sessão" className="col-span-2" hint="Ex.: “Antes do fix”, “Ultra 1440p – centro da cidade”.">
          <input className={input} value={value.label} maxLength={120} onChange={(e) => set({ label: e.target.value })} placeholder="Gerado automaticamente se ficar vazio" />
        </Field>
      )}
      <Field label="Resolução">
        <input className={input} value={value.resolution} maxLength={40} onChange={(e) => set({ resolution: e.target.value })} placeholder="Ex.: 1920x1080" />
      </Field>
      <Field label="Preset gráfico">
        <input className={input} value={value.preset} maxLength={60} onChange={(e) => set({ preset: e.target.value })} placeholder="Ex.: Alto" />
      </Field>
      <Field label="Limite de FPS">
        <input className={input} value={value.fpsLimit} maxLength={20} onChange={(e) => set({ fpsLimit: e.target.value })} placeholder="Ex.: sem limite, 60" />
      </Field>
      <Field label="VSync">
        <Select
          label="VSync"
          hideLabel
          className="[&_select]:h-9 [&_select]:w-full [&>div]:w-full"
          value={value.vsync}
          onChange={(v) => set({ vsync: v })}
          options={[
            { value: 'desconhecido', label: 'Não informado' },
            { value: 'desligado', label: 'Desligado' },
            { value: 'ligado', label: 'Ligado' },
          ]}
        />
      </Field>
      <Field label="Notas" className="col-span-2">
        <textarea
          className={cn(input, 'h-20 resize-none py-2')}
          value={value.notes}
          maxLength={4000}
          onChange={(e) => set({ notes: e.target.value })}
          placeholder="Trecho testado, o que mudou desde a última sessão…"
        />
      </Field>
    </div>
  )
}

export function ImportDialog({
  open,
  path,
  games,
  defaultAppId,
  stage,
  onClose,
  onSubmit,
}: {
  open: boolean
  path: string | null
  games: Game[]
  defaultAppId: string
  stage: string | null
  onClose: () => void
  onSubmit: (v: FormValues) => Promise<void>
}) {
  const [v, setV] = useState<FormValues>(emptyForm(defaultAppId))
  const [busy, setBusy] = useState(false)
  useEffect(() => {
    if (open) setV(emptyForm(defaultAppId))
  }, [open, defaultAppId])
  const stageLabel: Record<string, string> = { reading: 'Lendo o arquivo…', analyzing: 'Calculando estatísticas…', saving: 'Salvando no histórico…' }
  return (
    <Dialog
      open={open}
      onClose={onClose}
      locked={busy}
      width={620}
      title="Importar benchmark"
      description={path ? path.split('/').pop() : undefined}
      footer={
        <>
          <span className="mr-auto text-[12px] text-fg-3" aria-live="polite">
            {busy ? (stage ? stageLabel[stage] ?? 'Processando…' : 'Processando…') : 'O arquivo original não é alterado.'}
          </span>
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            Cancelar
          </Button>
          <Button
            variant="primary"
            icon={<FileUp className="size-4" />}
            loading={busy}
            onClick={async () => {
              setBusy(true)
              await onSubmit(v)
              setBusy(false)
            }}
          >
            Importar
          </Button>
        </>
      }
    >
      <SessionFields value={v} onChange={setV} games={games} />
    </Dialog>
  )
}

export function EditSessionDialog({ meta, games, onClose, onSave }: { meta: SessionMeta | null; games: Game[]; onClose: () => void; onSave: (v: FormValues) => Promise<void> }) {
  const [v, setV] = useState<FormValues>(emptyForm())
  const [busy, setBusy] = useState(false)
  useEffect(() => {
    if (meta)
      setV({
        appId: meta.appId ?? '',
        label: meta.label,
        notes: meta.notes,
        resolution: meta.conditions.resolution ?? '',
        preset: meta.conditions.preset ?? '',
        fpsLimit: meta.conditions.fpsLimit ?? '',
        vsync: meta.conditions.vsync == null ? 'desconhecido' : meta.conditions.vsync ? 'ligado' : 'desligado',
        proton: meta.conditions.proton ?? '',
      })
  }, [meta])
  return (
    <Dialog
      open={!!meta}
      onClose={onClose}
      locked={busy}
      width={620}
      title="Editar sessão"
      description="Os números medidos não mudam; só nome, notas e condições informadas."
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            Cancelar
          </Button>
          <Button
            variant="primary"
            icon={<Save className="size-4" />}
            loading={busy}
            onClick={async () => {
              setBusy(true)
              await onSave(v)
              setBusy(false)
            }}
          >
            Salvar
          </Button>
        </>
      }
    >
      <SessionFields value={v} onChange={setV} games={games} />
    </Dialog>
  )
}
