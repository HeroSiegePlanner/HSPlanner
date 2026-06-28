import { describe, expect, it } from 'vitest'
import { isSafeUrl, sanitizeHtml } from './sanitizeHtml'

describe('sanitizeHtml', () => {
  it('returns empty string for empty input', () => {
    expect(sanitizeHtml('')).toBe('')
  })

  it('preserves whitelisted tags and text', () => {
    const out = sanitizeHtml('<p>Hello <strong>world</strong></p>')
    expect(out).toContain('<strong>world</strong>')
    expect(out).toContain('Hello')
  })

  it('strips <script> tags entirely', () => {
    const out = sanitizeHtml('safe<script>alert(1)</script>after')
    expect(out).not.toContain('<script')
    expect(out).not.toContain('alert(1)')
  })

  it('strips inline event handlers', () => {
    const out = sanitizeHtml('<a href="https://example.com" onclick="alert(1)">x</a>')
    expect(out).not.toMatch(/onclick/i)
    expect(out).toContain('href="https://example.com"')
  })

  it('strips on* attributes regardless of casing', () => {
    const out = sanitizeHtml('<p OnMouseOver="alert(1)">hi</p>')
    expect(out).not.toMatch(/onmouseover/i)
  })

  it('removes javascript: URLs from href', () => {
    const out = sanitizeHtml('<a href="javascript:alert(1)">click</a>')
    expect(out).not.toMatch(/javascript:/i)
  })

  it('removes data: URLs from href', () => {
    const out = sanitizeHtml('<a href="data:text/html,<script>alert(1)</script>">x</a>')
    expect(out).not.toMatch(/data:/i)
  })

  it('keeps http and https URLs in href', () => {
    const safe = sanitizeHtml('<a href="https://example.com">x</a>')
    expect(safe).toContain('href="https://example.com"')
    const insecure = sanitizeHtml('<a href="http://example.com">x</a>')
    expect(insecure).toContain('href="http://example.com"')
  })

  it('forces noopener on links', () => {
    const out = sanitizeHtml('<a href="https://example.com">x</a>')
    expect(out).toMatch(/target="_blank"/)
    expect(out).toMatch(/rel="noopener noreferrer nofollow"/)
  })

  it('drops iframe and its entire subtree', () => {
    const out = sanitizeHtml('<iframe src="evil"><strong>x</strong></iframe>')
    expect(out).not.toContain('<iframe')
    expect(out).not.toContain('evil')
  })

  it('lifts safe descendants out of disallowed but non-dangerous wrappers', () => {
    const out = sanitizeHtml('<center><strong>kept</strong></center>')
    expect(out).not.toContain('<center')
    expect(out).toContain('<strong>kept</strong>')
  })

  it('strips img tags (not in whitelist)', () => {
    const out = sanitizeHtml('<img src=x onerror=alert(1)>')
    expect(out).not.toContain('<img')
    expect(out).not.toMatch(/onerror/i)
  })

  it('strips dangerous descendants lifted out of disallowed wrappers', () => {
    const out = sanitizeHtml('<center><img src=x onerror=alert(1)></center>')
    expect(out).not.toContain('<img')
    expect(out).not.toMatch(/onerror/i)
  })

  it('strips event handlers on nested unknown wrappers (form)', () => {
    const out = sanitizeHtml('<form><img src=x onerror=alert(1)><b>keep</b></form>')
    expect(out).not.toContain('<img')
    expect(out).not.toMatch(/onerror/i)
    expect(out).toContain('<b>keep</b>')
  })

  it('strips style url() expressions', () => {
    const out = sanitizeHtml('<p style="background-color: url(javascript:alert(1))">x</p>')
    expect(out).not.toMatch(/url\(/)
    expect(out).not.toMatch(/javascript:/i)
  })

  it('strips style expression() injections', () => {
    const out = sanitizeHtml('<p style="color: expression(alert(1))">x</p>')
    expect(out).not.toMatch(/expression\(/)
  })

  it('keeps allow-listed style properties', () => {
    const out = sanitizeHtml('<p style="color: red; text-decoration: underline">x</p>')
    expect(out).toMatch(/color:\s*red/)
    expect(out).toMatch(/text-decoration:\s*underline/)
  })

  it('drops disallowed style properties', () => {
    const out = sanitizeHtml('<p style="position: fixed; color: red">x</p>')
    expect(out).not.toMatch(/position:/)
    expect(out).toMatch(/color:\s*red/)
  })

  it('drops disallowed attributes from FONT', () => {
    const out = sanitizeHtml('<font color="red" face="evil">x</font>')
    expect(out).toContain('color="red"')
    expect(out).not.toMatch(/face=/i)
  })

  it('strips svg/math (XSS via foreignObject)', () => {
    const out = sanitizeHtml('<svg><script>alert(1)</script></svg>')
    expect(out).not.toMatch(/<svg/i)
    expect(out).not.toMatch(/<script/i)
  })

  it('does not break on malformed input', () => {
    expect(() => sanitizeHtml('<<>><>><a<<<')).not.toThrow()
  })
})

describe('isSafeUrl', () => {
  it('accepts http(s), mailto, anchors, and relative paths', () => {
    expect(isSafeUrl('https://example.com')).toBe(true)
    expect(isSafeUrl('http://example.com')).toBe(true)
    expect(isSafeUrl('mailto:a@b.com')).toBe(true)
    expect(isSafeUrl('#anchor')).toBe(true)
    expect(isSafeUrl('/path')).toBe(true)
  })

  it('rejects javascript:, data:, file: schemes', () => {
    expect(isSafeUrl('javascript:alert(1)')).toBe(false)
    expect(isSafeUrl('data:text/html,x')).toBe(false)
    expect(isSafeUrl('file:///etc/passwd')).toBe(false)
  })

  it('handles whitespace before scheme', () => {
    expect(isSafeUrl('  javascript:alert(1)')).toBe(false)
    expect(isSafeUrl('  https://example.com')).toBe(true)
  })
})
