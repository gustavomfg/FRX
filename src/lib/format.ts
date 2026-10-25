const dateFmt = new Intl.DateTimeFormat('pt-BR', { day: '2-digit', month: 'short' })
const timeFmt = new Intl.DateTimeFormat('pt-BR', { hour: '2-digit', minute: '2-digit' })
const fullFmt = new Intl.DateTimeFormat('pt-BR', { day: '2-digit', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' })

const dateYearFmt = new Intl.DateTimeFormat('pt-BR', { day: '2-digit', month: 'short', year: 'numeric' })

/** Data curta; inclui o ano quando não é o ano corrente. */
export const fmtDate = (iso: string) => {
  const d = new Date(iso)
  return (d.getFullYear() === new Date().getFullYear() ? dateFmt : dateYearFmt).format(d).replace('.', '')
}
export const fmtTime = (iso: string) => timeFmt.format(new Date(iso))
export const fmtDateTime = (iso: string) => fullFmt.format(new Date(iso)).replace('.', '')

export function fmtRelative(iso: string, now = Date.now()) {
  const diff = Math.round((now - new Date(iso).getTime()) / 1000)
  if (diff < 45) return 'agora'
  const m = Math.round(diff / 60)
  if (m < 60) return `há ${m} min`
  const h = Math.round(m / 60)
  if (h < 24) return `há ${h} h`
  const d = Math.round(h / 24)
  if (d === 1) return 'ontem'
  if (d < 30) return `há ${d} dias`
  return fmtDate(iso)
}

export const fmtNum = (n: number, digits = 0) =>
  (Object.is(n, -0) || Math.abs(n) < 0.5 * 10 ** -digits ? 0 : n).toLocaleString('pt-BR', { minimumFractionDigits: digits, maximumFractionDigits: digits })

export const fmtPct = (n: number, digits = 0) => `${n > 0 ? '+' : n < 0 ? '−' : '±'}${fmtNum(Math.abs(n), digits)}%`

export function greeting(date = new Date()) {
  const h = date.getHours()
  if (h < 5) return 'Boa madrugada'
  if (h < 12) return 'Bom dia'
  if (h < 18) return 'Boa tarde'
  return 'Boa noite'
}

export const uid = (p = 'id') => `${p}-${Math.random().toString(36).slice(2, 9)}`
