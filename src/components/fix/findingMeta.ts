import type { Compatibility, Confidence, FixCategory, ProviderState } from '@/types/fixes'

export const CONFIDENCE: Record<Confidence, { label: string; color: string; soft: string; help: string }> = {
  verified: {
    label: 'Verificada',
    color: 'var(--frx-good)',
    soft: 'var(--frx-good-soft)',
    help: 'Declarada pelos mantenedores em notas oficiais de versão. Ainda assim, não foi testada no seu computador.',
  },
  curated: {
    label: 'Curada',
    color: 'var(--frx-info)',
    soft: 'var(--frx-info-soft)',
    help: 'Faz parte de um repositório mantido e revisado (UMU-Protonfixes). Não foi testada no seu computador.',
  },
  'community-reported': {
    label: 'Relato da comunidade',
    color: 'var(--frx-adjust)',
    soft: 'var(--frx-adjust-soft)',
    help: 'Mencionada por mais de uma pessoa em relatos públicos. Não é uma confirmação oficial.',
  },
  unverified: {
    label: 'Não verificada',
    color: 'var(--frx-caution)',
    soft: 'var(--frx-caution-soft)',
    help: 'Uma única menção ou conteúdo que não pôde ser analisado com segurança.',
  },
}

export const COMPATIBILITY: Record<Compatibility, { label: string; color: string; soft: string }> = {
  compatible: { label: 'Compatível com seu hardware', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)' },
  'already-included': { label: 'Já incluída na sua ferramenta', color: 'var(--frx-info)', soft: 'var(--frx-info-soft)' },
  unknown: { label: 'Compatibilidade desconhecida', color: 'var(--frx-text-2)', soft: 'var(--frx-raised)' },
  incompatible: { label: 'Incompatível com seu hardware', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)' },
}

export const CATEGORY: Record<FixCategory, string> = {
  compatibility: 'Compatibilidade',
  performance: 'Desempenho',
  graphics: 'Gráficos',
  audio: 'Áudio',
  video: 'Vídeo',
  physics: 'Física',
  launcher: 'Inicializador',
  input: 'Controles',
  network: 'Rede',
  other: 'Outros',
}

export const PROVIDER_STATE: Record<ProviderState, { label: string; color: string }> = {
  ok: { label: 'Consultado agora', color: 'var(--frx-good)' },
  cached: { label: 'Do cache', color: 'var(--frx-info)' },
  stale: { label: 'Cache desatualizado', color: 'var(--frx-caution)' },
  'no-data': { label: 'Nada encontrado', color: 'var(--frx-text-3)' },
  unavailable: { label: 'Indisponível', color: 'var(--frx-bad)' },
  'rate-limited': { label: 'Limite de consultas', color: 'var(--frx-caution)' },
  disabled: { label: 'Desativado', color: 'var(--frx-text-3)' },
}
