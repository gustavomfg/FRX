import { CircleAlert, HardDrive, History } from 'lucide-react'
import { useLibrary } from '@/state/library'

/** Selo de origem para dados reais — par do `DemoTag`, no mesmo desenho. */
export function SourceTag({ children = 'Steam' }: { children?: string }) {
  return (
    <span
      className="inline-flex h-[18px] items-center rounded-[3px] border border-good/35 px-1.5 font-pixel text-[9px] tracking-[0.06em] text-good uppercase"
      title="Dados reais detectados neste computador"
    >
      {children}
    </span>
  )
}

/** Avisos da varredura: bibliotecas fora do ar, erros e dados vindos do cache. */
export function LibraryNotices() {
  const { scan, error, phase, fromCache } = useLibrary()
  const offline = scan?.libraries.filter((l) => l.status !== 'available') ?? []
  const otherWarnings = (scan?.warnings ?? []).filter((w) => !w.startsWith('Biblioteca indisponível') && !w.startsWith('Sem permissão'))
  if (!error && !offline.length && !otherWarnings.length && !(fromCache && phase === 'error')) return null
  return (
    <div className="mt-4 space-y-2" aria-live="polite">
      {error && (
        <p className="flex items-start gap-2.5 rounded-md border border-bad/25 bg-bad-soft px-4 py-2.5 text-[13px] text-bad">
          <CircleAlert className="mt-0.5 size-4 shrink-0" aria-hidden />
          <span>
            Não foi possível verificar a biblioteca: {error}
            {scan && ' Mostrando a última varredura salva.'}
          </span>
        </p>
      )}
      {offline.map((l) => (
        <p key={l.path} className="flex items-start gap-2.5 rounded-md border border-caution/25 bg-caution-soft px-4 py-2.5 text-[13px] text-caution">
          <HardDrive className="mt-0.5 size-4 shrink-0" aria-hidden />
          <span>
            {l.status === 'unavailable' ? 'Biblioteca indisponível — o disco pode estar desmontado: ' : 'Sem permissão para ler a biblioteca: '}
            <code className="font-mono text-[12px]">{l.path}</code>. Os jogos dela aparecem como “Indisponível”.
          </span>
        </p>
      ))}
      {otherWarnings.length > 0 && (
        <details className="rounded-md border border-line px-4 py-2.5 text-[12.5px] text-fg-2">
          <summary className="flex cursor-pointer items-center gap-2 text-fg-2">
            <History className="size-3.5" aria-hidden /> {otherWarnings.length} aviso{otherWarnings.length > 1 ? 's' : ''} da varredura
          </summary>
          <ul className="mt-2 space-y-1 font-mono text-[11.5px] text-fg-3">
            {otherWarnings.map((w) => (
              <li key={w}>{w}</li>
            ))}
          </ul>
        </details>
      )}
    </div>
  )
}
