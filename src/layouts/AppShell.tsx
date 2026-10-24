import { useEffect, useRef, useState } from 'react'
import { Outlet, useLocation, useNavigate } from 'react-router-dom'
import { MotionConfig, motion } from 'motion/react'
import { Sidebar } from './Sidebar'
import { CommandPalette } from './CommandPalette'
import { Toaster } from '@/components/ui/Toasts'
import { NAV } from '@/app/routes'
import { useMotionAttribute } from '@/state/motion'
import { useDispatch, useSettings } from '@/state/store'
import { useLibrary } from '@/state/library'
import { games } from '@/data/games'

function useNarrow(breakpoint = 1100) {
  const [narrow, setNarrow] = useState(() => window.innerWidth < breakpoint)
  useEffect(() => {
    const on = () => setNarrow(window.innerWidth < breakpoint)
    window.addEventListener('resize', on)
    return () => window.removeEventListener('resize', on)
  }, [breakpoint])
  return narrow
}

export function AppShell() {
  const motionLevel = useMotionAttribute()
  const narrow = useNarrow()
  const location = useLocation()
  const navigate = useNavigate()
  const main = useRef<HTMLElement>(null)
  const [palette, setPalette] = useState(false)
  const { scanOnStartup } = useSettings()
  const { mode } = useLibrary()
  const dispatch = useDispatch()

  // "Verificar a biblioteca ao abrir" — varredura simulada, uma vez por sessão.
  const scanned = useRef(false)
  useEffect(() => {
    // No modo real a varredura é feita pelo LibraryProvider; o aviso simulado só existe na demo.
    if (mode !== 'demo' || !scanOnStartup || scanned.current) return
    const t = setTimeout(() => {
      scanned.current = true
      dispatch({ type: 'toast', toast: { tone: 'info', title: 'Biblioteca verificada', body: `${games.length} jogos encontrados · simulado` } })
    }, 1200)
    return () => clearTimeout(t)
  }, [mode, scanOnStartup, dispatch])

  useEffect(() => {
    main.current?.scrollTo({ top: 0 })
  }, [location.pathname])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault()
        setPalette((p) => !p)
      }
      if (e.altKey && /^[1-5]$/.test(e.key)) {
        e.preventDefault()
        navigate(NAV[Number(e.key) - 1].to)
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [navigate])

  // Transições de página: só a entrada, curta, para manter a sensação de app nativo.
  const pageKey = location.pathname.split('/').slice(0, 3).join('/')

  return (
    <MotionConfig reducedMotion={motionLevel === 'off' ? 'always' : 'never'} transition={motionLevel === 'calm' ? { duration: 0.18 } : undefined}>
      {/* Com HashRouter, um href="#…" trocaria a rota — o foco é movido manualmente. */}
      <a
        href="#conteudo"
        onClick={(e) => {
          e.preventDefault()
          main.current?.focus()
        }}
        className="sr-only focus:not-sr-only focus:fixed focus:top-3 focus:left-3 focus:z-[70] focus:rounded-md focus:bg-accent focus:px-3 focus:py-2 focus:text-accent-ink">
        Pular para o conteúdo
      </a>
      <div className="flex h-full min-w-0">
        <Sidebar narrow={narrow} />
        <main ref={main} id="conteudo" className="relative min-w-0 flex-1 overflow-x-hidden overflow-y-auto" tabIndex={-1}>
          <motion.div
            key={pageKey}
            initial={{ opacity: 0, y: motionLevel === 'off' ? 0 : 6 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: motionLevel === 'off' ? 0 : 0.26, ease: [0.2, 0.8, 0.2, 1] }}
            className="min-h-full"
          >
            <Outlet />
          </motion.div>
        </main>
      </div>
      <CommandPalette open={palette} onClose={() => setPalette(false)} />
      <Toaster />
    </MotionConfig>
  )
}
