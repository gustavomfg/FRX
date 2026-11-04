import { describe, expect, it } from 'vitest'
import { isSafeExternalUrl } from './opener'

describe('isSafeExternalUrl', () => {
  it('só aceita HTTPS das páginas da Arch Wiki', () => {
    expect(isSafeExternalUrl('https://wiki.archlinux.org/title/Wireless_network_configuration')).toBe(true)
    expect(isSafeExternalUrl('https://wiki.archlinux.org/api.php?action=query')).toBe(false)
    expect(isSafeExternalUrl('https://github.com/ValveSoftware/Proton/issues/309')).toBe(false)
    expect(isSafeExternalUrl('http://wiki.archlinux.org/title/X')).toBe(false)
    expect(isSafeExternalUrl('https://wiki.archlinux.org.evil.io/title/x')).toBe(false)
    expect(isSafeExternalUrl('https://user@wiki.archlinux.org/title/x')).toBe(false)
    expect(isSafeExternalUrl('javascript:alert(1)')).toBe(false)
    expect(isSafeExternalUrl('file:///etc/passwd')).toBe(false)
  })
})
