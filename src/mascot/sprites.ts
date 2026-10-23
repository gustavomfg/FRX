/**
 * Pim — o pequeno zelador do FRX.
 *
 * Criatura original: corpo de marshmallow, gorro de tricô amarelo-manteiga
 * com pompom e botinhas. Os quadros são montados proceduralmente sobre uma
 * grade 24×20, para que cada pose reaproveite a mesma anatomia pixel a pixel.
 */

export type PixelKey = 'W' | 'S' | 'K' | 'Y' | 'H' | 'P' | 'M' | 'G' | 'A' | 'T'

export const PIXEL_COLORS: Record<PixelKey, string> = {
  W: 'var(--pim-body)',
  S: 'var(--pim-shade)',
  K: 'var(--pim-ink)',
  Y: 'var(--pim-hat)',
  H: 'var(--pim-hat-shade)',
  P: 'var(--pim-cheek)',
  M: 'var(--pim-metal)',
  G: 'var(--pim-glass)',
  A: 'var(--frx-accent-strong)',
  T: 'var(--frx-text-3)',
}

export const GRID_W = 24
export const GRID_H = 20
const OX = 4 // deslocamento do personagem na grade
const OY = 3

export type Grid = (PixelKey | null)[][]

type Eyes = 'open' | 'closed' | 'left' | 'right' | 'up' | 'down' | 'happy'
type Mouth = 'smile' | 'neutral' | 'open' | 'frown' | 'o'
type Arms = 'down' | 'waveA' | 'waveB' | 'up' | 'hold' | 'holdUp'
type Prop = 'magnifier' | 'magnifierLow' | 'wrenchUp' | 'wrenchDown' | null
type Fx = 'z1' | 'z2' | 'z3' | 'sparkA' | 'sparkB' | 'sweatA' | 'sweatB' | 'question' | 'questionDim' | 'note'

export interface FrameSpec {
  eyes?: Eyes
  mouth?: Mouth
  arms?: Arms
  prop?: Prop
  fx?: Fx[]
  dy?: number
  sitting?: boolean
  cheeks?: boolean
}

// Corpo base (16×16). '.' = transparente.
const BODY = [
  '................',
  '.......YY.......',
  '......YYYY......',
  '.......YY.......',
  '.....YYYYYY.....',
  '....YYYYYYYY....',
  '...YYYYYYYYYY...',
  '..HHHHHHHHHHHH..',
  '...WWWWWWWWWW...',
  '..WWWWWWWWWWWW..',
  '..WWWWWWWWWWWW..',
  '..WWWWWWWWWWWW..',
  '..WWWWWWWWWWWW..',
  '..SWWWWWWWWWWS..',
  '...SSWWWWWWSS...',
  '....HH....HH....',
]

const EYES: Record<Eyes, [number, number][]> = {
  open: [[5, 9], [5, 10], [10, 9], [10, 10]],
  closed: [[4, 10], [5, 10], [10, 10], [11, 10]],
  left: [[4, 9], [4, 10], [9, 9], [9, 10]],
  right: [[6, 9], [6, 10], [11, 9], [11, 10]],
  up: [[5, 8], [5, 9], [10, 8], [10, 9]],
  down: [[5, 10], [5, 11], [10, 10], [10, 11]],
  happy: [[4, 10], [5, 9], [6, 10], [9, 10], [10, 9], [11, 10]],
}

const MOUTH: Record<Mouth, [number, number, PixelKey][]> = {
  smile: [[6, 11, 'K'], [7, 12, 'K'], [8, 12, 'K'], [9, 11, 'K']],
  neutral: [[7, 12, 'K'], [8, 12, 'K']],
  open: [[6, 11, 'K'], [7, 11, 'K'], [8, 11, 'K'], [9, 11, 'K'], [7, 12, 'P'], [8, 12, 'P']],
  frown: [[6, 12, 'K'], [7, 11, 'K'], [8, 11, 'K'], [9, 12, 'K']],
  o: [[7, 11, 'K'], [8, 11, 'K'], [7, 12, 'K'], [8, 12, 'K']],
}

const ARMS: Record<Arms, [number, number, PixelKey][]> = {
  down: [[1, 12, 'W'], [1, 13, 'S'], [14, 12, 'W'], [14, 13, 'S']],
  waveA: [[1, 12, 'W'], [1, 13, 'S'], [14, 11, 'W'], [15, 10, 'W'], [15, 9, 'W']],
  waveB: [[1, 12, 'W'], [1, 13, 'S'], [14, 11, 'W'], [15, 11, 'W'], [16, 10, 'W']],
  up: [[1, 11, 'W'], [0, 10, 'W'], [0, 9, 'W'], [14, 11, 'W'], [15, 10, 'W'], [15, 9, 'W']],
  hold: [[1, 12, 'W'], [1, 13, 'S'], [14, 12, 'W'], [15, 12, 'W']],
  holdUp: [[1, 12, 'W'], [1, 13, 'S'], [14, 11, 'W'], [15, 11, 'W']],
}

function ring(cx: number, cy: number): [number, number][] {
  return [
    [cx - 1, cy - 2], [cx, cy - 2], [cx + 1, cy - 2],
    [cx - 2, cy - 1], [cx + 2, cy - 1],
    [cx - 2, cy], [cx + 2, cy],
    [cx - 2, cy + 1], [cx + 2, cy + 1],
    [cx - 1, cy + 2], [cx, cy + 2], [cx + 1, cy + 2],
  ]
}

function magnifier(cy: number): [number, number, PixelKey][] {
  const cx = 17
  const out: [number, number, PixelKey][] = []
  for (const [x, y] of ring(cx, cy)) out.push([x, y, 'M'])
  for (let x = cx - 1; x <= cx + 1; x++) for (let y = cy - 1; y <= cy + 1; y++) out.push([x, y, 'G'])
  out.push([cx - 1, cy - 1, 'W'])
  out.push([15, cy + 2, 'M'])
  return out
}

const PROPS: Record<Exclude<Prop, null>, [number, number, PixelKey][]> = {
  magnifier: magnifier(8),
  magnifierLow: magnifier(9),
  wrenchUp: [
    [15, 10, 'M'], [15, 9, 'M'], [15, 8, 'M'],
    [14, 7, 'M'], [16, 7, 'M'], [14, 6, 'M'], [16, 6, 'M'],
  ],
  wrenchDown: [
    [16, 11, 'M'], [17, 11, 'M'],
    [18, 10, 'M'], [18, 12, 'M'], [19, 10, 'M'], [19, 12, 'M'],
  ],
}

const FX: Record<Fx, [number, number, PixelKey][]> = {
  z1: zed(15, 4, 'T'),
  z2: [...zed(15, 4, 'T'), ...zed(17, 1, 'T')],
  z3: [...zed(17, 1, 'T')],
  sparkA: [...plus(-3, 3), ...plus(16, 0), [18, 7, 'A']],
  sparkB: [...plus(-4, 0), ...plus(17, 5), [-2, 8, 'A']],
  sweatA: [[15, 7, 'G'], [15, 8, 'G']],
  sweatB: [[15, 9, 'G'], [15, 10, 'G']],
  question: [[16, 0, 'A'], [17, 0, 'A'], [18, 1, 'A'], [17, 2, 'A'], [17, 4, 'A']],
  questionDim: [[16, 0, 'T'], [17, 0, 'T'], [18, 1, 'T'], [17, 2, 'T'], [17, 4, 'T']],
  note: [[17, 2, 'A'], [17, 3, 'A'], [17, 4, 'A'], [18, 2, 'A'], [16, 4, 'A']],
}

function zed(x: number, y: number, c: PixelKey): [number, number, PixelKey][] {
  return [[x, y, c], [x + 1, y, c], [x + 2, y, c], [x + 1, y + 1, c], [x, y + 2, c], [x + 1, y + 2, c], [x + 2, y + 2, c]]
}
function plus(x: number, y: number): [number, number, PixelKey][] {
  return [[x + 1, y, 'A'], [x, y + 1, 'A'], [x + 1, y + 1, 'A'], [x + 2, y + 1, 'A'], [x + 1, y + 2, 'A']]
}

export function buildFrame(spec: FrameSpec): Grid {
  const g: Grid = Array.from({ length: GRID_H }, () => Array<PixelKey | null>(GRID_W).fill(null))
  const dy = (spec.dy ?? 0) + (spec.sitting ? 1 : 0)
  const put = (x: number, y: number, c: PixelKey, shift = true) => {
    const gx = x + OX
    const gy = y + OY + (shift ? dy : 0)
    if (gx >= 0 && gx < GRID_W && gy >= 0 && gy < GRID_H) g[gy][gx] = c
  }
  BODY.forEach((row, y) => {
    if (spec.sitting && y === 15) return
    for (let x = 0; x < row.length; x++) {
      const ch = row[x]
      if (ch !== '.') put(x, y, ch as PixelKey)
    }
  })
  if (spec.cheeks !== false) {
    put(3, 11, 'P')
    put(12, 11, 'P')
  }
  for (const [x, y] of EYES[spec.eyes ?? 'open']) put(x, y, 'K')
  for (const [x, y, c] of MOUTH[spec.mouth ?? 'neutral']) put(x, y, c)
  for (const [x, y, c] of ARMS[spec.arms ?? 'down']) put(x, y, c)
  if (spec.prop) for (const [x, y, c] of PROPS[spec.prop]) put(x, y, c)
  for (const f of spec.fx ?? []) for (const [x, y, c] of FX[f]) put(x, y, c, false)
  return g
}

/** Cada pose é uma sequência de quadros com duração em "ticks" (1 tick = 125 ms). */
export type Pose =
  | 'idle'
  | 'wave'
  | 'rest'
  | 'inspect'
  | 'fix'
  | 'observe'
  | 'celebrate'
  | 'worried'
  | 'confused'
  | 'sleep'

export const POSES: Record<Pose, { frames: FrameSpec[]; ticks: number[] }> = {
  idle: {
    frames: [{ mouth: 'smile' }, { mouth: 'smile', dy: 1 }, { mouth: 'smile' }, { eyes: 'closed', mouth: 'smile' }],
    ticks: [8, 8, 10, 1],
  },
  wave: {
    frames: [
      { arms: 'waveA', mouth: 'smile' },
      { arms: 'waveB', mouth: 'smile' },
      { arms: 'waveA', mouth: 'smile' },
      { arms: 'waveB', mouth: 'smile', eyes: 'happy' },
      { arms: 'down', mouth: 'smile' },
      { arms: 'down', mouth: 'smile', eyes: 'closed' },
      { arms: 'down', mouth: 'smile' },
    ],
    ticks: [3, 3, 3, 3, 16, 1, 10],
  },
  rest: {
    frames: [
      { sitting: true, eyes: 'closed', mouth: 'neutral', fx: ['z1'] },
      { sitting: true, eyes: 'closed', mouth: 'neutral', fx: ['z2'], dy: 0 },
      { sitting: true, eyes: 'closed', mouth: 'neutral', fx: ['z3'] },
      { sitting: true, eyes: 'closed', mouth: 'neutral' },
      { sitting: true, eyes: 'left', mouth: 'neutral' },
      { sitting: true, eyes: 'closed', mouth: 'neutral' },
    ],
    ticks: [6, 6, 6, 8, 12, 2],
  },
  inspect: {
    frames: [
      { eyes: 'right', mouth: 'neutral', arms: 'holdUp', prop: 'magnifier' },
      { eyes: 'right', mouth: 'neutral', arms: 'hold', prop: 'magnifierLow' },
      { eyes: 'right', mouth: 'o', arms: 'holdUp', prop: 'magnifier' },
      { eyes: 'up', mouth: 'neutral', arms: 'holdUp', prop: 'magnifier', fx: ['questionDim'] },
    ],
    ticks: [5, 5, 4, 6],
  },
  fix: {
    frames: [
      { eyes: 'right', mouth: 'neutral', arms: 'holdUp', prop: 'wrenchUp' },
      { eyes: 'right', mouth: 'o', arms: 'hold', prop: 'wrenchDown', fx: ['sparkA'] },
      { eyes: 'right', mouth: 'neutral', arms: 'holdUp', prop: 'wrenchUp' },
      { eyes: 'right', mouth: 'o', arms: 'hold', prop: 'wrenchDown', fx: ['sparkB'] },
    ],
    ticks: [3, 2, 3, 2],
  },
  observe: {
    frames: [{ eyes: 'left' }, { eyes: 'left' }, { eyes: 'closed' }, { eyes: 'up' }, { eyes: 'left', mouth: 'smile' }],
    ticks: [16, 6, 1, 6, 10],
  },
  celebrate: {
    frames: [
      { arms: 'up', eyes: 'happy', mouth: 'open', dy: -2, fx: ['sparkA'] },
      { arms: 'up', eyes: 'happy', mouth: 'open', dy: 0, fx: ['sparkB'] },
    ],
    ticks: [3, 3],
  },
  worried: {
    frames: [
      { eyes: 'down', mouth: 'frown', fx: ['sweatA'] },
      { eyes: 'down', mouth: 'frown', fx: ['sweatB'] },
      { eyes: 'left', mouth: 'frown' },
    ],
    ticks: [6, 6, 8],
  },
  confused: {
    frames: [
      { eyes: 'up', mouth: 'neutral', fx: ['question'] },
      { eyes: 'up', mouth: 'neutral', fx: ['questionDim'] },
      { eyes: 'left', mouth: 'o', fx: ['question'] },
    ],
    ticks: [6, 4, 8],
  },
  sleep: {
    frames: [
      { sitting: true, eyes: 'closed', fx: ['z1'] },
      { sitting: true, eyes: 'closed', fx: ['z2'] },
      { sitting: true, eyes: 'closed', fx: ['z3'] },
    ],
    ticks: [8, 8, 8],
  },
}

/** Converte uma grade em um path SVG por cor (poucos nós no DOM). */
export function gridToPaths(g: Grid, maxRows = GRID_H) {
  const byColor = new Map<PixelKey, string[]>()
  for (let y = 0; y < Math.min(maxRows, GRID_H); y++) {
    let x = 0
    while (x < GRID_W) {
      const c = g[y][x]
      if (!c) {
        x++
        continue
      }
      let w = 1
      while (x + w < GRID_W && g[y][x + w] === c) w++
      const list = byColor.get(c) ?? []
      list.push(`M${x} ${y}h${w}v1h-${w}z`)
      byColor.set(c, list)
      x += w
    }
  }
  return [...byColor.entries()].map(([key, segs]) => ({ key, d: segs.join('') }))
}

const cache = new Map<string, ReturnType<typeof gridToPaths>>()
export function framePaths(pose: Pose, index: number, maxRows = GRID_H) {
  const k = `${pose}:${index}:${maxRows}`
  let v = cache.get(k)
  if (!v) {
    v = gridToPaths(buildFrame(POSES[pose].frames[index]), maxRows)
    cache.set(k, v)
  }
  return v
}
