import { lazy, Suspense } from 'react'
import { HashRouter, Route, Routes } from 'react-router-dom'
import { AppStoreProvider } from '@/state/store'
import { LibraryProvider } from '@/state/library'
import { FixReportsProvider } from '@/state/fixes'
import { PerformanceProvider } from '@/state/performance'
import { FixEngineProvider } from '@/state/fixEngine'
import { AppShell } from '@/layouts/AppShell'
import { HomePage } from '@/pages/Home'
import { LibraryPage } from '@/pages/Library'
import { GameDetailPage } from '@/pages/GameDetail'
import { FixPacksPage } from '@/pages/FixPacks'
import { SettingsPage } from '@/pages/Settings'
import { NotFoundPage } from '@/pages/NotFound'

// O Performance Lab (Recharts) é carregado sob demanda para acelerar a abertura.
const PerformancePage = lazy(() => import('@/pages/Performance').then((m) => ({ default: m.PerformancePage })))

// HashRouter funciona tanto no navegador quanto empacotado pelo Tauri (file/asset protocol).
export function App() {
  return (
    <AppStoreProvider>
      <LibraryProvider>
        <FixReportsProvider>
          <FixEngineProvider>
          <PerformanceProvider>
          <HashRouter>
            <Routes>
              <Route element={<AppShell />}>
                <Route index element={<HomePage />} />
                <Route path="biblioteca" element={<LibraryPage />} />
                <Route path="biblioteca/:gameId" element={<GameDetailPage />} />
                <Route path="correcoes" element={<FixPacksPage />} />
                <Route
                  path="performance"
                  element={
                    <Suspense fallback={<div className="m-10 h-[320px] rounded-xl skeleton" aria-label="Carregando Performance Lab" />}>
                      <PerformancePage />
                    </Suspense>
                  }
                />
                <Route path="configuracoes" element={<SettingsPage />} />
                <Route path="*" element={<NotFoundPage />} />
              </Route>
            </Routes>
          </HashRouter>
          </PerformanceProvider>
          </FixEngineProvider>
        </FixReportsProvider>
      </LibraryProvider>
    </AppStoreProvider>
  )
}
