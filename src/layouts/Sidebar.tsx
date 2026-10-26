import { NavLink } from 'react-router-dom'
import { PanelLeftClose, PanelLeftOpen } from 'lucide-react'
import { NAV } from '@/app/routes'
import { Logo } from '@/components/brand/Logo'
import { DemoTag } from '@/components/ui/Status'
import { Kbd } from '@/components/ui/Controls'
import { useDispatch, useFixSummary, useSettings } from '@/state/store'
import { systemInfo } from '@/data/system'
import { useLibrary } from '@/state/library'
import { useFixReports } from '@/state/fixes'
import { SourceTag } from '@/components/game/LibraryNotices'
import { cn } from '@/lib/cn'
import { isRelevant } from '@/lib/findings'

export function Sidebar({ narrow }: { narrow: boolean }) {
  const settings = useSettings()
  const dispatch = useDispatch()
  const { available } = useFixSummary()
  const { reports } = useFixReports()
  const compact = narrow || settings.compactSidebar
  const { mode, system } = useLibrary()
  const real = mode === 'real'
  const fixBadge = real
    ? Object.values(reports).reduce((n, r) => n + r.findings.filter((f) => isRelevant(f) && f.kind !== 'report').length, 0)
    : available.length
  const gpu = system?.primaryGpu
  const sysRows: [string, string][] = real
    ? [
        ['GPU', gpu?.name ?? gpu?.vendor.toUpperCase() ?? '—'],
        ['Driver', gpu ? [gpu.driver, gpu.driverVersion].filter(Boolean).join(' ') || '—' : '—'],
        ['Kernel', system?.kernel ?? '—'],
      ]
    : [
        ['GPU', systemInfo.gpu],
        ['Driver', systemInfo.driver.split(' (')[0]],
        ['Proton', systemInfo.defaultProton],
      ]

  return (
    <aside
      className={cn(
        'relative z-10 flex h-full shrink-0 flex-col border-r border-line bg-bg transition-[width] duration-(--frx-dur-slow) ease-frx',
        compact ? 'w-[68px]' : 'w-[232px]',
      )}
    >
      <div className={cn('flex h-16 items-center', compact ? 'justify-center' : 'justify-between px-5')}>
        <Logo compact={compact} />
        {!compact && (real ? <SourceTag>v0.4</SourceTag> : <DemoTag />)}
      </div>

      <nav aria-label="Principal" className={cn('mt-3 flex flex-col gap-0.5', compact ? 'px-2.5' : 'px-3')}>
        {NAV.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.to === '/'}
            title={compact ? item.label : undefined}
            className={({ isActive }) =>
              cn(
                'group relative flex h-9 items-center gap-3 rounded-md text-[13.5px] font-medium transition-colors duration-(--frx-dur-fast)',
                compact ? 'justify-center' : 'px-3',
                isActive ? 'bg-surface text-fg' : 'text-fg-2 hover:bg-surface/60 hover:text-fg',
              )
            }
          >
            {({ isActive }) => (
              <>
                {/* Indicador "pixel" do item ativo */}
                <span
                  aria-hidden
                  className={cn(
                    'absolute top-1/2 -left-3 h-4 w-[3px] -translate-y-1/2 bg-accent transition-[opacity,transform] duration-(--frx-dur) ease-frx',
                    compact && '-left-2.5',
                    isActive ? 'scale-y-100 opacity-100' : 'scale-y-0 opacity-0',
                  )}
                />
                <item.icon className={cn('size-[17px] shrink-0', isActive ? 'text-accent' : 'text-fg-3 group-hover:text-fg-2')} strokeWidth={1.9} aria-hidden />
                {!compact && (
                  <>
                    <span className="flex-1 truncate">{item.label}</span>
                    {item.to === '/correcoes' && fixBadge > 0 && (
                      <span className="tnum rounded-[4px] bg-accent-soft px-1.5 font-mono text-[11px] leading-[18px] text-accent" aria-label={`${fixBadge} ${real ? 'descobertas' : 'disponíveis'}`}>
                        {fixBadge}
                      </span>
                    )}
                  </>
                )}
              </>
            )}
          </NavLink>
        ))}
      </nav>

      <div className="mt-auto">
        {!compact && (
          <div className="mx-5 mb-4 border-t border-line pt-4">
            <div className="eyebrow mb-2.5">{real ? 'Sistema' : 'Sistema · demo'}</div>
            <dl className="space-y-1 text-[12px]">
              {sysRows.map(([k, v]) => (
                <div key={k} className="flex justify-between gap-2">
                  <dt className="text-fg-3">{k}</dt>
                  <dd className="truncate text-fg-2" title={v}>
                    {v}
                  </dd>
                </div>
              ))}
            </dl>
          </div>
        )}
        <div className={cn('flex items-center border-t border-line py-3', compact ? 'justify-center' : 'justify-between px-5')}>
          {!compact && (
            <span className="flex items-center gap-1.5 text-[11.5px] text-fg-3">
              <Kbd>Ctrl</Kbd>
              <Kbd>K</Kbd>
              <span className="ml-1">buscar</span>
            </span>
          )}
          {!narrow && (
            <button
              onClick={() => dispatch({ type: 'settings', patch: { compactSidebar: !settings.compactSidebar } })}
              className="grid size-7 place-items-center rounded-sm text-fg-3 transition-colors hover:bg-surface hover:text-fg"
              aria-label={compact ? 'Expandir barra lateral' : 'Recolher barra lateral'}
              title={compact ? 'Expandir barra lateral' : 'Recolher barra lateral'}
            >
              {compact ? <PanelLeftOpen className="size-4" /> : <PanelLeftClose className="size-4" />}
            </button>
          )}
        </div>
      </div>
    </aside>
  )
}
