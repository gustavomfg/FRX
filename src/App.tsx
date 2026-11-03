import { HashRouter, Route, Routes } from 'react-router-dom'
import { AppStoreProvider } from '@/state/store'
import { HardwareProvider } from '@/state/hardware'
import { AppShell } from '@/layouts/AppShell'
import { DashboardPage } from '@/pages/Dashboard'
import { SettingsPage } from '@/pages/Settings'
import { NotFoundPage } from '@/pages/NotFound'

// HashRouter funciona tanto no navegador quanto empacotado pelo Tauri (file/asset protocol).
export function App() {
  return (
    <AppStoreProvider>
      <HardwareProvider>
        <HashRouter>
          <Routes>
            <Route element={<AppShell />}>
              <Route index element={<DashboardPage />} />
              <Route path="configuracoes" element={<SettingsPage />} />
              <Route path="*" element={<NotFoundPage />} />
            </Route>
          </Routes>
        </HashRouter>
      </HardwareProvider>
    </AppStoreProvider>
  )
}
