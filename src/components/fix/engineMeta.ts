import type { DynStatus, EvidenceLevel, OpState, PlanStatus, Purpose, Risk, Trust } from '@/types/fixEngine'

type Meta = { label: string; color: string; soft: string; help?: string }

export const TRUST: Record<Trust, Meta> = {
  reviewed: { label: 'Revisada', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)', help: 'Receita do catálogo local do FRX, revisada e com hash verificado. Pode ser aplicada com prévia, backup e rollback.' },
  candidate: { label: 'Candidata', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)', help: 'Escrita como receita, mas ainda não revisada. Só pode ser consultada.' },
  discovered: { label: 'Descoberta', color: 'var(--frx-text-2)', soft: 'var(--frx-raised)', help: 'Vista em uma fonte pública. Não é executável.' },
  unsupported: { label: 'Não suportada', color: 'var(--frx-text-3)', soft: 'var(--frx-raised)', help: 'O FRX não sabe aplicar este tipo de ajuste com segurança.' },
  revoked: { label: 'Revogada', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)', help: 'Retirada do catálogo por um problema conhecido.' },
  pack: { label: 'Smart Pack', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)', help: 'Composição de receitas validadas uma a uma e em conjunto, aplicada como uma única operação.' },
  generated: { label: 'Gerada automaticamente', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)', help: 'Gerada pelo FRX a partir de uma descoberta pública, com ações do conjunto restrito e validação do núcleo. Não foi revisada por pessoas.' },
}

/** Estado de cada descoberta depois do gerador dinâmico. */
export const DYN_STATUS: Record<DynStatus, Meta & { action: string }> = {
  generated: { label: 'Correção aplicável', action: 'Aplicar correção', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)' },
  reviewed: { label: 'Correção aplicável', action: 'Aplicar correção', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)' },
  'already-included': { label: 'Já incluída no Proton', action: 'Já incluída no Proton', color: 'var(--frx-info)', soft: 'var(--frx-info-soft)' },
  incompatible: { label: 'Incompatível com seu sistema', action: 'Incompatível com seu sistema', color: 'var(--frx-text-3)', soft: 'var(--frx-raised)' },
  'needs-validation': { label: 'Requer validação adicional', action: 'Requer validação adicional', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)' },
  blocked: { label: 'Bloqueada por segurança', action: 'Bloqueada por segurança', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)' },
  contested: { label: 'Relatos contestados', action: 'Relatos contestados', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)' },
  'no-actions': { label: 'Sem procedimento automatizável', action: 'Sem procedimento automatizável', color: 'var(--frx-text-2)', soft: 'var(--frx-raised)' },
  'insufficient-data': { label: 'Faltam informações', action: 'Faltam informações', color: 'var(--frx-text-2)', soft: 'var(--frx-raised)' },
  invalid: { label: 'Descoberta inválida', action: 'Descoberta inválida', color: 'var(--frx-text-3)', soft: 'var(--frx-raised)' },
}

export const EVIDENCE: Record<EvidenceLevel, string> = {
  documented: 'Evidência: documentada',
  corroborated: 'Evidência: vários relatos',
  single: 'Evidência: um relato',
  contested: 'Evidência: contestada',
  none: 'Evidência: nenhuma',
}

export const PLAN_STATUS: Record<PlanStatus, Meta> = {
  ready: { label: 'Aplicável', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)' },
  'requires-manual-step': { label: 'Aplicável · passo manual', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)' },
  conflict: { label: 'Conflito', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)' },
  incompatible: { label: 'Incompatível', color: 'var(--frx-text-3)', soft: 'var(--frx-raised)' },
  'already-applied': { label: 'Já aplicada', color: 'var(--frx-info)', soft: 'var(--frx-info-soft)' },
  'already-included': { label: 'Já incluída no Proton', color: 'var(--frx-info)', soft: 'var(--frx-info-soft)' },
  'not-executable': { label: 'Só consulta', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)' },
}

export const OP_STATE: Record<OpState, Meta> = {
  checking: { label: 'Conferindo', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)' },
  downloading: { label: 'Baixando arquivos', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)' },
  'verifying-artifacts': { label: 'Verificando arquivos baixados', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)' },
  'backing-up': { label: 'Criando backup', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)' },
  applying: { label: 'Aplicando', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)' },
  verifying: { label: 'Verificando', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)' },
  applied: { label: 'Aplicada', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)' },
  failed: { label: 'Não aplicada', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)' },
  conflict: { label: 'Conflito', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)' },
  'requires-manual-step': { label: 'Aguardando passo manual', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)' },
  'rolling-back': { label: 'Desfazendo', color: 'var(--frx-accent)', soft: 'var(--frx-accent-soft)' },
  'rolled-back': { label: 'Desfeita', color: 'var(--frx-text-2)', soft: 'var(--frx-raised)' },
  'recovery-required': { label: 'Requer recuperação', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)' },
  cancelled: { label: 'Cancelada antes da instalação', color: 'var(--frx-text-2)', soft: 'var(--frx-raised)' },
}

export const RISK: Record<Risk, Meta> = {
  low: { label: 'Risco baixo', color: 'var(--frx-good)', soft: 'var(--frx-good-soft)' },
  moderate: { label: 'Risco moderado', color: 'var(--frx-caution)', soft: 'var(--frx-caution-soft)' },
  high: { label: 'Risco alto', color: 'var(--frx-bad)', soft: 'var(--frx-bad-soft)' },
}

export const PURPOSE: Record<Purpose, string> = {
  diagnostic: 'Diagnóstico',
  compatibility: 'Compatibilidade',
  performance: 'Desempenho',
}

/** Etapas reais do executor, na ordem em que o núcleo as percorre. */
export const APPLY_STAGES: { state: OpState; label: string }[] = [
  { state: 'checking', label: 'Conferir o estado atual' },
  { state: 'backing-up', label: 'Criar backup verificado' },
  { state: 'applying', label: 'Gravar a alteração' },
  { state: 'verifying', label: 'Verificar no disco' },
]

export const FILE_APPLY_STAGES: { state: OpState; label: string }[] = [
  { state: 'checking', label: 'Conferir o jogo, hardware e destinos' },
  { state: 'downloading', label: 'Baixar para área temporária' },
  { state: 'verifying-artifacts', label: 'Verificar hash e conteúdo' },
  { state: 'backing-up', label: 'Criar backup verificado' },
  { state: 'applying', label: 'Instalar somente os arquivos autorizados' },
  { state: 'verifying', label: 'Verificar no disco' },
]

export const ROLLBACK_STAGES: { state: OpState; label: string }[] = [
  { state: 'rolling-back', label: 'Remover só o que a correção adicionou' },
  { state: 'rolled-back', label: 'Verificar no disco' },
]

export const isFinal = (s: OpState) => ['applied', 'failed', 'conflict', 'requires-manual-step', 'rolled-back', 'recovery-required', 'cancelled'].includes(s)
