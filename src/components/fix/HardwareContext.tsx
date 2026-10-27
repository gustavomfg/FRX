import type { EnvironmentContext, HardwareMatch, HardwareRequirements } from '@/types/steam'

export function HardwareRequirementsSummary({ requirements, compact = false }: { requirements?: HardwareRequirements; compact?: boolean }) {
  if (!requirements) return null
  const fields = [
    ['Fabricante', requirements.gpuVendor?.toUpperCase()],
    ['Modelo de GPU', requirements.gpuModels.join(', ')],
    ['ID PCI', requirements.gpuDeviceIds.join(', ')],
    ['Driver', requirements.driverVersions.join(', ')],
    ['Proton', requirements.protonVersions.join(', ')],
    ['Arquitetura', requirements.cpuArch],
  ].filter((field) => field[1])
  if (!fields.length) return null
  if (compact) return <span className="text-[12px] text-fg-3" title={fields.map(([name, value]) => `${name}: ${value}`).join(' · ')}>{requirements.gpuVendor ? `Escopo ${requirements.gpuVendor.toUpperCase()}` : 'Requisitos específicos de hardware'}</span>
  return <div className="space-y-1 text-[12.5px] text-fg-2"><p className="font-medium text-fg">Requisitos publicados pela fonte</p><dl className="space-y-1">{fields.map(([name, value]) => <div key={name} className="flex flex-wrap gap-x-2"><dt className="text-fg-3">{name}:</dt><dd>{value}</dd></div>)}</dl></div>
}

/** A provável GPU do desktop e o renderizador medido têm proveniências distintas. */
export function HardwareContext({ environment, match }: { environment?: EnvironmentContext | null; match?: HardwareMatch | null }) {
  if (!environment && !match) return null
  const selected = environment?.selection === 'measured' ? environment.selectedGpu : null
  return (
    <section aria-label="Contexto de hardware da correção" className="space-y-2 text-[12.5px] text-fg-2">
      <p className="font-medium text-fg">Jogo e hardware considerados</p>
      {environment && <dl className="space-y-1">
        <div className="flex flex-wrap gap-x-2"><dt className="text-fg-3">GPU usada pelo jogo:</dt><dd>{selected ? selected.name ?? `${selected.vendor.toUpperCase()} ${selected.vendorId}:${selected.deviceId}` : 'Não confirmada por uma captura'}</dd></div>
        {!!environment.detectedGpus.length && <div className="flex flex-wrap gap-x-2"><dt className="text-fg-3">GPUs detectadas:</dt><dd>{environment.detectedGpus.map((gpu) => gpu.name ?? `${gpu.vendor.toUpperCase()} ${gpu.vendorId}:${gpu.deviceId}`).join(' · ')}</dd></div>}
        {selected && <div className="flex flex-wrap gap-x-2"><dt className="text-fg-3">Driver da GPU medida:</dt><dd>{[selected.driver, selected.driverVersion].filter(Boolean).join(' ') || 'Não informado'}</dd></div>}
        {selected?.pciAddress && <div className="flex flex-wrap gap-x-2"><dt className="text-fg-3">Identidade PCI medida:</dt><dd>{selected.vendorId}:{selected.deviceId} · {selected.pciAddress}</dd></div>}
        <div className="flex flex-wrap gap-x-2"><dt className="text-fg-3">Driver Vulkan ativo:</dt><dd>{environment.activeVulkanDriver ?? 'Não confirmado'}</dd></div>
        {environment.protonVersion && <div className="flex flex-wrap gap-x-2"><dt className="text-fg-3">Proton:</dt><dd>{environment.protonVersion}{environment.protonOrigin ? ` · ${environment.protonOrigin}` : ''}</dd></div>}
      </dl>}
      {match && <>
        <p className={match.state === 'incompatible' ? 'text-bad' : match.state === 'unknown' ? 'text-caution' : 'text-fg-2'}>{match.state === 'incompatible' ? 'Hardware incompatível' : match.state === 'unknown' ? 'Compatibilidade de hardware não confirmada' : 'Requisitos de hardware correspondentes'}</p>
        {match.reasons.map((reason) => <p key={reason}>{reason}</p>)}
        {match.limitations.map((limitation) => <p key={limitation} className="text-fg-3">{limitation}</p>)}
      </>}
    </section>
  )
}
