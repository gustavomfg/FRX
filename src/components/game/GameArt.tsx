import { memo, useId, type ReactElement } from 'react'
import type { CoverArt } from '@/types'
import { rng } from '@/lib/random'
import { cn } from '@/lib/cn'

/**
 * Arte ilustrativa gerada para cada jogo fictício: cenas pixeladas em SVG
 * com paleta própria. Nada de assets de terceiros — e o mesmo jogo sempre
 * gera a mesma arte (semente fixa).
 */

function hex(h: string) {
  const n = parseInt(h.slice(1), 16)
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255]
}
function mix(a: string, b: string, t: number) {
  const A = hex(a)
  const B = hex(b)
  const c = A.map((v, i) => Math.round(v + (B[i] - v) * t))
  return `rgb(${c[0]} ${c[1]} ${c[2]})`
}

/** Cordilheira em degraus (silhueta pixelada). */
function ridge(r: () => number, W: number, H: number, base: number, amp: number, step: number, smooth = 0) {
  let d = `M0 ${H}V${base}`
  let y = base
  for (let x = 0; x < W; x += step) {
    const target = base - (smooth ? (Math.sin(x / smooth + r() * 0.6) * 0.5 + 0.5) * amp : r() * amp)
    y = Math.round(y * 0.45 + target * 0.55)
    d += `V${y}H${Math.min(W, x + step)}`
  }
  return d + `V${H}Z`
}

function pixelCircle(cx: number, cy: number, rad: number) {
  let d = ''
  for (let y = -rad; y <= rad; y++) {
    const w = Math.round(Math.sqrt(rad * rad - y * y))
    d += `M${cx - w} ${cy + y}h${w * 2}v1h-${w * 2}z`
  }
  return d
}

function build(art: CoverArt, W: number, H: number, uid: string) {
  const r = rng(art.seed * 977 + W)
  const [sky, mid, light, glow, ink] = art.palette
  const els: ReactElement[] = []
  const bands = 6
  for (let i = 0; i < bands; i++) {
    const y = Math.round((H * 0.72 * i) / bands)
    els.push(<rect key={`b${i}`} x={0} y={y} width={W} height={Math.ceil((H * 0.72) / bands) + 1} fill={mix(sky, mid, i / (bands - 1))} />)
    if (i > 0) els.push(<rect key={`d${i}`} x={0} y={y - 1} width={W} height={2} fill={`url(#${uid}-dither)`} opacity={0.3} />)
  }
  els.push(<rect key="lower" x={0} y={Math.round(H * 0.72)} width={W} height={H} fill={mid} />)

  const night = art.orb === 'lua' || art.scene === 'espaco' || art.scene === 'caverna'
  if (night) {
    for (let i = 0; i < Math.round((W * H) / 90); i++) {
      els.push(<rect key={`s${i}`} x={Math.floor(r() * W)} y={Math.floor(r() * H * 0.55)} width={1} height={1} fill={glow} opacity={0.25 + r() * 0.6} />)
    }
  }

  const orbX = Math.round(W * (0.3 + r() * 0.45))
  const orbY = Math.round(H * (0.24 + r() * 0.12))
  const orbR = Math.round(Math.min(W, H) * 0.13)
  if (art.orb === 'sol') {
    els.push(<path key="halo" d={pixelCircle(orbX, orbY, orbR + 3)} fill={glow} opacity={0.18} />)
    els.push(<path key="orb" d={pixelCircle(orbX, orbY, orbR)} fill={glow} />)
  } else if (art.orb === 'lua') {
    els.push(<path key="orb" d={pixelCircle(orbX, orbY, orbR - 1)} fill={mix(glow, '#ffffff', 0.35)} />)
    els.push(<path key="cut" d={pixelCircle(orbX + 3, orbY - 2, orbR - 2)} fill={mix(sky, mid, 0.2)} />)
  } else if (art.orb === 'planeta') {
    els.push(<path key="orb" d={pixelCircle(orbX, orbY, orbR + 1)} fill={light} />)
    els.push(<path key="shade" d={pixelCircle(orbX + 2, orbY + 1, orbR)} fill={mix(light, sky, 0.35)} />)
    els.push(<rect key="ring" x={orbX - orbR - 5} y={orbY} width={(orbR + 5) * 2} height={1} fill={glow} />)
  }

  switch (art.scene) {
    case 'montanhas':
      els.push(<path key="r1" d={ridge(r, W, H, H * 0.5, H * 0.18, 3)} fill={mix(mid, light, 0.35)} />)
      els.push(<path key="r2" d={ridge(r, W, H, H * 0.64, H * 0.16, 4)} fill={mix(mid, ink, 0.25)} />)
      els.push(<path key="r3" d={ridge(r, W, H, H * 0.8, H * 0.1, 5)} fill={ink} />)
      break
    case 'dunas':
      els.push(<path key="r1" d={ridge(r, W, H, H * 0.6, H * 0.1, 2, 9)} fill={mix(light, glow, 0.25)} />)
      els.push(<path key="r2" d={ridge(r, W, H, H * 0.72, H * 0.1, 2, 13)} fill={light} />)
      els.push(<path key="r3" d={ridge(r, W, H, H * 0.86, H * 0.08, 3, 7)} fill={ink} />)
      break
    case 'mar': {
      const hz = Math.round(H * 0.6)
      els.push(<rect key="sea" x={0} y={hz} width={W} height={H - hz} fill={mix(mid, ink, 0.3)} />)
      for (let i = 0; i < 9; i++) {
        const y = hz + 2 + i * 3
        const w = Math.max(2, orbR * 2 - i * 2 + Math.floor(r() * 4))
        els.push(<rect key={`w${i}`} x={orbX - w / 2} y={y} width={w} height={1} fill={glow} opacity={0.65 - i * 0.06} />)
        els.push(<rect key={`x${i}`} x={Math.floor(r() * W)} y={y + 1} width={3 + Math.floor(r() * 6)} height={1} fill={light} opacity={0.25} />)
      }
      els.push(<path key="isle" d={ridge(r, W, H, H * 0.88, H * 0.06, 3)} fill={ink} />)
      break
    }
    case 'cidade': {
      let x = 0
      while (x < W) {
        const bw = 4 + Math.floor(r() * 7)
        const bh = Math.round(H * (0.18 + r() * 0.35))
        const top = H - bh - Math.round(H * 0.08)
        els.push(<rect key={`c${x}`} x={x} y={top} width={bw} height={H} fill={mix(ink, mid, 0.25 + r() * 0.25)} />)
        for (let wy = top + 2; wy < H - 6; wy += 3)
          for (let wx = x + 1; wx < x + bw - 1; wx += 2) if (r() > 0.72) els.push(<rect key={`p${wx}-${wy}`} x={wx} y={wy} width={1} height={1} fill={glow} opacity={0.85} />)
        x += bw + (r() > 0.6 ? 1 : 0)
      }
      els.push(<rect key="street" x={0} y={H - Math.round(H * 0.08)} width={W} height={H} fill={ink} />)
      break
    }
    case 'floresta': {
      els.push(<path key="hill" d={ridge(r, W, H, H * 0.66, H * 0.08, 4, 14)} fill={mix(mid, light, 0.2)} />)
      for (let i = 0; i < Math.round(W / 4.5); i++) {
        const tx = Math.floor(r() * W)
        const th = Math.round(H * (0.16 + r() * 0.2))
        const ty = Math.round(H * (0.74 + r() * 0.12)) - th
        const col = r() > 0.5 ? ink : mix(ink, mid, 0.4)
        for (let k = 0; k < th; k += 2) {
          const w = Math.max(1, Math.round((k / th) * 7))
          els.push(<rect key={`t${i}-${k}`} x={tx - Math.floor(w / 2)} y={ty + k} width={w} height={2} fill={col} />)
        }
      }
      els.push(<rect key="ground" x={0} y={Math.round(H * 0.86)} width={W} height={H} fill={ink} />)
      break
    }
    case 'espaco': {
      els.push(<path key="planet" d={pixelCircle(Math.round(W * 0.5), Math.round(H * 1.25), Math.round(H * 0.6))} fill={mix(mid, light, 0.3)} />)
      els.push(<path key="planet2" d={pixelCircle(Math.round(W * 0.5), Math.round(H * 1.3), Math.round(H * 0.6))} fill={ink} />)
      const sx = Math.round(W * 0.68)
      const sy = Math.round(H * 0.6)
      els.push(<rect key="st" x={sx} y={sy} width={6} height={3} fill={light} />)
      els.push(<rect key="st2" x={sx - 3} y={sy + 1} width={12} height={1} fill={glow} />)
      els.push(<rect key="st3" x={sx + 2} y={sy - 2} width={2} height={2} fill={glow} />)
      break
    }
    case 'caverna': {
      els.push(<path key="back" d={ridge(r, W, H, H * 0.72, H * 0.2, 3)} fill={mix(mid, ink, 0.4)} />)
      let d = `M0 0`
      for (let x = 0; x < W; x += 3) d += `V${Math.round(H * (0.06 + r() * 0.2))}H${x + 3}`
      els.push(<path key="roof" d={d + 'V0Z'} fill={ink} />)
      const lx = Math.round(W * (0.35 + r() * 0.3))
      const ly = Math.round(H * 0.62)
      els.push(<path key="glow2" d={pixelCircle(lx, ly, Math.round(W * 0.16))} fill={glow} opacity={0.12} />)
      els.push(<path key="glow1" d={pixelCircle(lx, ly, Math.round(W * 0.08))} fill={glow} opacity={0.22} />)
      els.push(<rect key="lamp" x={lx - 1} y={ly - 2} width={3} height={4} fill={glow} />)
      els.push(<rect key="lamp2" x={lx} y={ly - 4} width={1} height={2} fill={light} />)
      els.push(<path key="floor" d={ridge(r, W, H, H * 0.84, H * 0.06, 4)} fill={ink} />)
      break
    }
  }
  return els
}

interface GameArtProps {
  art: CoverArt
  variant?: 'cover' | 'banner' | 'thumb'
  className?: string
  title?: string
}

export const GameArt = memo(function GameArt({ art, variant = 'cover', className, title }: GameArtProps) {
  const uid = useId().replace(/:/g, '')
  const [W, H] = variant === 'banner' ? [180, 64] : variant === 'thumb' ? [40, 40] : [60, 80]
  return (
    <svg
      viewBox={`0 0 ${W} ${H}`}
      preserveAspectRatio="xMidYMid slice"
      className={cn('pixelated block h-full w-full', className)}
      role={title ? 'img' : undefined}
      aria-label={title ? `Arte ilustrativa de ${title}` : undefined}
      aria-hidden={title ? undefined : true}
    >
      <defs>
        <pattern id={`${uid}-dither`} width={2} height={2} patternUnits="userSpaceOnUse">
          <rect width={1} height={1} fill={art.palette[1]} />
          <rect x={1} y={1} width={1} height={1} fill={art.palette[0]} />
        </pattern>
      </defs>
      {build(art, W, H, uid)}
    </svg>
  )
})
