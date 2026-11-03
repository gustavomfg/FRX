import { CircuitBoard, Moon, Touchpad, Volume2, Wifi, type LucideIcon } from 'lucide-react'
import type { HardwareSymptom } from '@/types/hardware'

export interface SymptomMeta {
  id: HardwareSymptom
  label: string
  hint: string
  icon: LucideIcon
}

export const SYMPTOMS: SymptomMeta[] = [
  { id: 'wifi', label: 'Wi-Fi', hint: 'Não conecta, cai ou está lento', icon: Wifi },
  { id: 'audio', label: 'Áudio', hint: 'Sem som, microfone ou alto-falantes', icon: Volume2 },
  { id: 'touchpad', label: 'Touchpad', hint: 'Não responde, gestos ou cliques', icon: Touchpad },
  { id: 'suspend', label: 'Suspensão', hint: 'Não suspende ou não volta', icon: Moon },
  { id: 'firmware', label: 'Firmware', hint: 'BIOS/UEFI e atualizações', icon: CircuitBoard },
]

export const isSymptom = (v: string | null): v is HardwareSymptom => SYMPTOMS.some((s) => s.id === v)
