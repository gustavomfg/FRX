import { useEffect, useRef, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { AnimatePresence, motion } from 'motion/react'
import { X } from 'lucide-react'
import { IconButton } from './Button'
import { cn } from '@/lib/cn'

interface DialogProps {
  open: boolean
  onClose: () => void
  title: ReactNode
  description?: ReactNode
  children: ReactNode
  footer?: ReactNode
  /** Impede fechar por Esc/clique fora durante operações. */
  locked?: boolean
  width?: number
  aside?: ReactNode
}

const FOCUSABLE = 'button:not([disabled]),[href],input:not([disabled]),select,textarea,[tabindex]:not([tabindex="-1"])'

export function Dialog({ open, onClose, title, description, children, footer, locked, width = 560, aside }: DialogProps) {
  const panel = useRef<HTMLDivElement>(null)
  const restore = useRef<HTMLElement | null>(null)

  useEffect(() => {
    if (!open) return
    restore.current = document.activeElement as HTMLElement
    const t = setTimeout(() => panel.current?.querySelector<HTMLElement>(FOCUSABLE)?.focus(), 30)
    return () => {
      clearTimeout(t)
      restore.current?.focus?.()
    }
  }, [open])

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape' && !locked) {
      e.stopPropagation()
      onClose()
    }
    if (e.key === 'Tab' && panel.current) {
      const els = [...panel.current.querySelectorAll<HTMLElement>(FOCUSABLE)]
      if (!els.length) return
      const first = els[0]
      const last = els[els.length - 1]
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault()
        last.focus()
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault()
        first.focus()
      }
    }
  }

  return createPortal(
    <AnimatePresence>
      {open && (
        <motion.div
          className="fixed inset-0 z-50 grid place-items-center p-6"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.16 }}
          onKeyDown={onKeyDown}
        >
          <div className="absolute inset-0 bg-(--frx-scrim)" onClick={() => !locked && onClose()} aria-hidden />
          <motion.div
            ref={panel}
            role="dialog"
            aria-modal="true"
            aria-label={typeof title === 'string' ? title : undefined}
            className={cn('relative flex max-h-[min(720px,calc(100vh-48px))] w-full flex-col overflow-hidden rounded-xl border border-line-2 bg-surface shadow-[0_30px_80px_-20px_rgb(0_0_0/0.7)]')}
            style={{ maxWidth: width }}
            initial={{ y: 14, scale: 0.98, opacity: 0 }}
            animate={{ y: 0, scale: 1, opacity: 1 }}
            exit={{ y: 8, scale: 0.98, opacity: 0 }}
            transition={{ type: 'spring', stiffness: 420, damping: 34 }}
          >
            <div className="flex items-start justify-between gap-4 px-6 pt-5 pb-4">
              <div className="flex min-w-0 items-start gap-4">
                {aside}
                <div className="min-w-0">
                  <h2 className="font-display text-[19px] font-semibold tracking-[-0.02em]">{title}</h2>
                  {description && <p className="mt-1 text-[13.5px] text-fg-2">{description}</p>}
                </div>
              </div>
              <IconButton label="Fechar" onClick={onClose} disabled={locked} className="-mt-1 -mr-2 disabled:opacity-30">
                <X className="size-4" />
              </IconButton>
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto px-6 pb-5">{children}</div>
            {footer && <div className="flex items-center justify-end gap-2 border-t border-line bg-bg/40 px-6 py-3.5">{footer}</div>}
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>,
    document.body,
  )
}
