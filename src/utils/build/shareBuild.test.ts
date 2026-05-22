import { compressToEncodedURIComponent } from 'lz-string'
import { describe, expect, it } from 'vitest'
import {
  type BuildSnapshot,
  decodeShareToBuild,
  encodeBuildToShare,
  parseBuildCodeFromInput,
} from './shareBuild'

function makeSnapshot(overrides: Partial<BuildSnapshot> = {}): BuildSnapshot {
  // Test helper that returns a complete BuildSnapshot pre-filled with sensible defaults, allowing each test to override only the fields it cares about. Used throughout the share-encoding tests to keep individual cases small and focused.
  return {
    classId: 'stormweaver',
    level: 50,
    allocated: { strength: 10 },
    inventory: {},
    skillRanks: { fireball: 5 },
    subskillRanks: {},
    allocatedTreeNodes: new Set([1, 2, 3]),
    mainSkillId: 'fireball',
    activeAuraId: null,
    activeBuffs: {},
    enemyConditions: {},
    playerConditions: {},
    skillProjectiles: {},
    enemyResistances: {},
    procToggles: {},
    killsPerSec: 1,
    customStats: [],
    ...overrides,
  }
}

describe('encode/decode round-trip', () => {
  it('round-trips a basic snapshot', () => {
    const snap = makeSnapshot()
    const code = encodeBuildToShare(snap)
    const decoded = decodeShareToBuild(code)
    expect(decoded).not.toBeNull()
    expect(decoded!.snapshot.classId).toBe('stormweaver')
    expect(decoded!.snapshot.level).toBe(50)
    expect(decoded!.snapshot.skillRanks.fireball).toBe(5)
    expect([...decoded!.snapshot.allocatedTreeNodes]).toEqual([1, 2, 3])
  })

  it('preserves and sanitizes notes through the round-trip', () => {
    const snap = makeSnapshot()
    const code = encodeBuildToShare(snap, '<p>safe<script>alert(1)</script></p>')
    const decoded = decodeShareToBuild(code)
    expect(decoded).not.toBeNull()
    expect(decoded!.notes).not.toContain('<script')
    expect(decoded!.notes).not.toContain('alert(1)')
    expect(decoded!.notes).toContain('safe')
  })
})

describe('decodeShareToBuild — invalid input', () => {
  it('returns null for empty string', () => {
    expect(decodeShareToBuild('')).toBeNull()
  })

  it('returns null for non-base64 garbage', () => {
    expect(decodeShareToBuild('!!!@@@###')).toBeNull()
  })

  it('returns null for valid lz-string but invalid JSON', () => {
    const code = compressToEncodedURIComponent('not json {[')
    expect(decodeShareToBuild(code)).toBeNull()
  })

  it('returns null for JSON missing required fields', () => {
    const code = compressToEncodedURIComponent(JSON.stringify({ v: 1 }))
    expect(decodeShareToBuild(code)).toBeNull()
  })

  it('returns null for wrong-shaped fields', () => {
    const code = compressToEncodedURIComponent(
      JSON.stringify({
        v: 1,
        c: 'x',
        l: 'fifty',
        a: {},
        i: {},
        s: {},
        ss: {},
        t: [],
        m: null,
        u: null,
        buf: {},
        ec: {},
        pt: {},
        kps: 1,
      }),
    )
    expect(decodeShareToBuild(code)).toBeNull()
  })

  it('rejects payloads with too many tree nodes', () => {
    const huge = Array.from({ length: 50_000 }, (_, i) => i)
    const code = compressToEncodedURIComponent(
      JSON.stringify({
        v: 1,
        c: 'x',
        l: 1,
        a: {},
        i: {},
        s: {},
        ss: {},
        t: huge,
        m: null,
        u: null,
        buf: {},
        ec: {},
        pt: {},
        kps: 1,
      }),
    )
    expect(decodeShareToBuild(code)).toBeNull()
  })

  it('rejects oversized input strings', () => {
    const oversized = 'A'.repeat(300_000)
    expect(decodeShareToBuild(oversized)).toBeNull()
  })

  it('clamps absurd levels into a sane range', () => {
    const snap = makeSnapshot({ level: 999_999_999 })
    const code = encodeBuildToShare(snap)
    const decoded = decodeShareToBuild(code)
    expect(decoded).not.toBeNull()
    expect(decoded!.snapshot.level).toBeLessThanOrEqual(10_000)
    expect(decoded!.snapshot.level).toBeGreaterThanOrEqual(1)
  })

  it('rejects non-finite numbers in records', () => {
    const code = compressToEncodedURIComponent(
      JSON.stringify({
        v: 1,
        c: 'x',
        l: 1,
        a: { strength: Number.NaN },
        i: {},
        s: {},
        ss: {},
        t: [],
        m: null,
        u: null,
        buf: {},
        ec: {},
        pt: {},
        kps: 1,
      }),
    )
    expect(decodeShareToBuild(code)).toBeNull()
  })

  it('rejects mismatched schema version', () => {
    const json = JSON.stringify({
      v: 999,
      c: null,
      l: 1,
      a: {},
      i: {},
      s: {},
      ss: {},
      t: [],
      m: null,
      u: null,
      buf: {},
      ec: {},
      pt: {},
      kps: 1,
    })
    const badCode = compressToEncodedURIComponent(json)
    expect(decodeShareToBuild(badCode)).toBeNull()
  })
})

describe('parseBuildCodeFromInput', () => {
  it('extracts code from a hash URL', () => {
    expect(parseBuildCodeFromInput('https://example.com/#b=ABC123')).toBe('ABC123')
  })

  it('extracts code from a query URL', () => {
    expect(parseBuildCodeFromInput('https://example.com/?b=ABC123')).toBe('ABC123')
  })

  it('returns raw code when no URL pattern is found', () => {
    expect(parseBuildCodeFromInput('  ABC123  ')).toBe('ABC123')
  })

  it('returns trimmed input on empty match', () => {
    expect(parseBuildCodeFromInput('  hello  ')).toBe('hello')
  })
})
