const dateFmt = new Intl.DateTimeFormat('pt-BR', { day: '2-digit', month: 'short' })
const dateYearFmt = new Intl.DateTimeFormat('pt-BR', { day: '2-digit', month: 'short', year: 'numeric' })

/** Data curta; inclui o ano quando não é o ano corrente. */
export const fmtDate = (iso: string) => {
  const d = new Date(iso)
  return (d.getFullYear() === new Date().getFullYear() ? dateFmt : dateYearFmt).format(d).replace('.', '')
}
export const fmtNum = (n: number, digits = 0) =>
  (Object.is(n, -0) || Math.abs(n) < 0.5 * 10 ** -digits ? 0 : n).toLocaleString('pt-BR', { minimumFractionDigits: digits, maximumFractionDigits: digits })

export function greeting(date = new Date()) {
  const h = date.getHours()
  if (h < 5) return 'Boa madrugada'
  if (h < 12) return 'Bom dia'
  if (h < 18) return 'Boa tarde'
  return 'Boa noite'
}

export const uid = (p = 'id') => `${p}-${Math.random().toString(36).slice(2, 9)}`
