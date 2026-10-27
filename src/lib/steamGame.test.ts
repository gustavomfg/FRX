import { describe, expect, it } from 'vitest'
import { placeholderCover, steamToGame, toolLabel } from './steamGame'
import { batman } from '@/test/fixtures'

describe('steamToGame', () => {
  it('mapeia a instalação real sem inventar dados', () => {
    const g = steamToGame(batman)
    expect(g.id).toBe('200260')
    expect(g.origin).toBe('steam')
    expect(g.runner).toBe('Proton')
    expect(g.protonVersion).toBe('proton-cachyos-11.0-20261015 (SLR)')
    expect(g.studio).toBeUndefined()
    expect(g.genre).toBeUndefined()
    expect(g.status).toBe('nao-testado')
    expect(g.playtimeH).toBeCloseTo(46 / 60)
  })

  it('deixa claro quando a ferramenta não foi identificada', () => {
    const g = steamToGame({ ...batman, compatibility: { ...batman.compatibility, tool: null, prefixVersion: null, usesProton: null } })
    expect(g.protonVersion).toBe('Ferramenta não identificada')
    expect(g.runner).toBe('Desconhecido')
    expect(toolLabel(null)).toBeNull()
  })

  it('gera a mesma arte provisória para o mesmo AppID', () => {
    expect(placeholderCover('620')).toEqual(placeholderCover('620'))
  })
  it('mantém a paleta válida quando o hash do AppID ocupa o bit de sinal', () => {
    for (const id of ['620', '200260', '570', '730', '999999']) {
      const art = placeholderCover(id)
      expect(art.palette).toHaveLength(5)
      expect(art.palette.every((color) => /^#[0-9a-f]{6}$/i.test(color))).toBe(true)
      expect(['sol', 'lua', 'planeta', 'nenhum']).toContain(art.orb)
    }
  })
})
