import { useEffect, useMemo, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { AnimatePresence, motion } from 'motion/react'
import { CornerDownLeft, Search, Wrench } from 'lucide-react'
import { NAV } from '@/app/routes'
import { useLibrary } from '@/state/library'
import { CoverImage } from '@/components/game/CoverImage'
import { fixPacks } from '@/data/fixpacks'
import { GameArt } from '@/components/game/GameArt'
import { Kbd } from '@/components/ui/Controls'
import { cn } from '@/lib/cn'

interface Entry {
  id: string
  group: 'Páginas' | 'Jogos' | 'Fix Packs'
  label: string
  hint?: string
  to: string
  icon: React.ReactNode
}

export function CommandPalette({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [q, setQ] = useState('')
  const [index, setIndex] = useState(0)
  const navigate = useNavigate()
  const input = useRef<HTMLInputElement>(null)
  const { games, mode } = useLibrary()

  const all = useMemo<Entry[]>(
    () => [
      ...NAV.map((n) => ({ id: n.to, group: 'Páginas' as const, label: n.label, hint: `Alt ${n.shortcut}`, to: n.to, icon: <n.icon className="size-4 text-fg-3" /> })),
      ...games.map((g) => ({
        id: g.id,
        group: 'Jogos' as const,
        label: g.title,
        hint: g.protonVersion,
        to: `/biblioteca/${g.id}`,
        icon: (
          <span className="block size-5 overflow-hidden rounded-[4px]">
            {g.origin === 'steam' ? <CoverImage game={g} kind="header" variant="thumb" /> : <GameArt art={g.cover} variant="thumb" />}
          </span>
        ),
      })),
      ...(mode === 'demo'
        ? fixPacks.map((f) => ({ id: f.id, group: 'Fix Packs' as const, label: f.name, hint: games.find((g) => g.id === f.gameId)?.title, to: `/correcoes?pack=${f.id}`, icon: <Wrench className="size-4 text-fg-3" /> }))
        : []),
    ],
    [games, mode],
  )

  const results = useMemo(() => {
    const s = q.trim().toLowerCase()
    const list = s ? all.filter((e) => e.label.toLowerCase().includes(s) || e.hint?.toLowerCase().includes(s)) : all.filter((e) => e.group !== 'Fix Packs')
    return list.slice(0, 12)
  }, [q, all])

  useEffect(() => {
    if (open) {
      setQ('')
      setIndex(0)
      setTimeout(() => input.current?.focus(), 20)
    }
  }, [open])
  useEffect(() => setIndex(0), [q])

  const go = (e: Entry | undefined) => {
    if (!e) return
    navigate(e.to)
    onClose()
  }

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setIndex((i) => Math.min(results.length - 1, i + 1))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setIndex((i) => Math.max(0, i - 1))
    } else if (e.key === 'Enter') {
      go(results[index])
    } else if (e.key === 'Escape') {
      onClose()
    }
  }

  let lastGroup = ''
  return (
    <AnimatePresence>
      {open && (
        <motion.div className="fixed inset-0 z-50 flex justify-center pt-[14vh]" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.12 }}>
          <div className="absolute inset-0 bg-(--frx-scrim)" onClick={onClose} aria-hidden />
          <motion.div
            role="dialog"
            aria-modal="true"
            aria-label="Busca rápida"
            className="relative h-fit w-full max-w-[560px] overflow-hidden rounded-xl border border-line-2 bg-surface shadow-[0_30px_80px_-20px_rgb(0_0_0/0.75)]"
            initial={{ y: -8, scale: 0.985 }}
            animate={{ y: 0, scale: 1 }}
            exit={{ y: -6, scale: 0.985 }}
            transition={{ type: 'spring', stiffness: 520, damping: 36 }}
            onKeyDown={onKey}
          >
            <div className="flex items-center gap-3 border-b border-line px-4">
              <Search className="size-4 text-fg-3" aria-hidden />
              <input
                ref={input}
                value={q}
                onChange={(e) => setQ(e.target.value)}
                placeholder="Ir para página, jogo ou correção…"
                className="h-12 flex-1 bg-transparent text-[15px] text-fg placeholder:text-fg-3 focus:outline-none"
                role="combobox"
                aria-expanded="true"
                aria-controls="cmdk-list"
                aria-activedescendant={results[index] ? `cmdk-${results[index].id}` : undefined}
              />
              <Kbd>Esc</Kbd>
            </div>
            <ul id="cmdk-list" role="listbox" className="max-h-[380px] overflow-y-auto p-2">
              {results.length === 0 && <li className="px-3 py-8 text-center text-[13px] text-fg-3">Nada encontrado para “{q}”.</li>}
              {results.map((e, i) => {
                const header = e.group !== lastGroup
                lastGroup = e.group
                return (
                  <li key={e.group + e.id} role="presentation">
                    {header && <div className="eyebrow px-3 pt-3 pb-2">{e.group}</div>}
                    <div
                      id={`cmdk-${e.id}`}
                      role="option"
                      aria-selected={i === index}
                      onMouseMove={() => setIndex(i)}
                      onClick={() => go(e)}
                      className={cn('flex h-10 cursor-pointer items-center gap-3 rounded-md px-3 text-[13.5px]', i === index ? 'bg-raised text-fg' : 'text-fg-2')}
                    >
                      {e.icon}
                      <span className="flex-1 truncate">{e.label}</span>
                      {e.hint && <span className="truncate text-[12px] text-fg-3">{e.hint}</span>}
                      {i === index && <CornerDownLeft className="size-3.5 text-fg-3" aria-hidden />}
                    </div>
                  </li>
                )
              })}
            </ul>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}
