import { useEffect } from 'react'
import { AnimatePresence, motion } from 'motion/react'
import { CircleAlert, CircleCheck, Info, TriangleAlert, X } from 'lucide-react'
import { useAppState, useDispatch, type Toast } from '@/state/store'

const ICON = { ok: CircleCheck, info: Info, warn: TriangleAlert, error: CircleAlert }
const COLOR = { ok: 'var(--frx-good)', info: 'var(--frx-info)', warn: 'var(--frx-caution)', error: 'var(--frx-bad)' }

function ToastItem({ t }: { t: Toast }) {
  const dispatch = useDispatch()
  useEffect(() => {
    const id = setTimeout(() => dispatch({ type: 'dismissToast', id: t.id }), 4800)
    return () => clearTimeout(id)
  }, [t.id, dispatch])
  const Icon = ICON[t.tone]
  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 12, scale: 0.98 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, x: 24 }}
      transition={{ type: 'spring', stiffness: 500, damping: 38 }}
      className="pointer-events-auto flex w-[340px] items-start gap-3 rounded-lg border border-line-2 bg-raised px-4 py-3 shadow-[0_18px_40px_-16px_rgb(0_0_0/0.8)]"
      role="status"
    >
      <Icon className="mt-0.5 size-4 shrink-0" style={{ color: COLOR[t.tone] }} aria-hidden />
      <div className="min-w-0 flex-1">
        <p className="text-[13.5px] font-medium text-fg">{t.title}</p>
        {t.body && <p className="mt-0.5 text-[12.5px] text-fg-2">{t.body}</p>}
      </div>
      <button aria-label="Dispensar" onClick={() => dispatch({ type: 'dismissToast', id: t.id })} className="-mr-1 rounded-xs p-0.5 text-fg-3 hover:text-fg">
        <X className="size-3.5" />
      </button>
    </motion.div>
  )
}

export function Toaster() {
  const { toasts } = useAppState()
  return (
    <div className="pointer-events-none fixed right-5 bottom-5 z-[60] flex flex-col items-end gap-2" aria-live="polite">
      <AnimatePresence initial={false}>
        {toasts.map((t) => (
          <ToastItem key={t.id} t={t} />
        ))}
      </AnimatePresence>
    </div>
  )
}
