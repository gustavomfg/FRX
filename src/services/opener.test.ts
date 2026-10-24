import { describe, expect, it } from 'vitest'
import { isSafeExternalUrl } from './opener'

describe('isSafeExternalUrl', () => {
  it('só aceita HTTPS das fontes conhecidas', () => {
    expect(isSafeExternalUrl('https://github.com/ValveSoftware/Proton/issues/309')).toBe(true)
    expect(isSafeExternalUrl('https://www.protondb.com/app/200260')).toBe(true)
    expect(isSafeExternalUrl('http://github.com/x')).toBe(false)
    expect(isSafeExternalUrl('https://github.com.evil.io/x')).toBe(false)
    expect(isSafeExternalUrl('javascript:alert(1)')).toBe(false)
    expect(isSafeExternalUrl('file:///etc/passwd')).toBe(false)
  })
})
