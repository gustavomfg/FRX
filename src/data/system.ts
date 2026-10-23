import type { SystemInfo } from '@/types'

/** Perfil de sistema DEMONSTRATIVO — nenhuma detecção real de hardware é feita. */
export const systemInfo: SystemInfo = {
  distro: 'Fedora Linux 43 (demo)',
  kernel: '6.17.4',
  desktop: 'KDE Plasma 6.5 · Wayland',
  cpu: 'Ryzen 7 7800X3D · 8 núcleos',
  gpu: 'Radeon RX 7800 XT',
  driver: 'Mesa 25.2.3 (RADV)',
  vulkan: 'Vulkan 1.4',
  ramGb: 32,
  vramGb: 16,
  steamRuntime: 'Steam Linux Runtime 3.0 (sniper)',
  protonVersions: ['Proton 9.0-4', 'Proton Experimental', 'Proton 8.0-5', 'GE-Proton9-20'],
  defaultProton: 'Proton 9.0-4',
}
