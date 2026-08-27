import {
  compressToEncodedURIComponent,
  decompressFromEncodedURIComponent,
} from 'lz-string'
import { describe, expect, it } from 'vitest'
import { makeSnapshot as makeBaseSnapshot } from './buildSnapshot.fixture'
import {
  type BuildSnapshot,
  decodeShareToBuild,
  encodeBuildToShare,
  parseBuildCodeFromInput,
} from './shareBuild'

function makeSnapshot(overrides: Partial<BuildSnapshot> = {}): BuildSnapshot {
  return makeBaseSnapshot({
    classId: 'stormweaver',
    level: 50,
    allocated: { strength: 10 },
    skillRanks: { fireball: 5 },
    allocatedTreeNodes: new Set([1, 2, 3]),
    activeSkillIds: ['fireball'],
    enemyResistances: {},
    ...overrides,
  })
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
    expect(decoded!.snapshot.activeSkillIds).toEqual(['fireball'])
  })

  it('round-trips disabledPotions through share', () => {
    const snap = makeSnapshot({ disabledPotions: { potion_1: true } })
    const decoded = decodeShareToBuild(encodeBuildToShare(snap))
    expect(decoded!.snapshot.disabledPotions).toEqual({ potion_1: true })
  })

  it('round-trips skill bonus overrides on an equipped item', () => {
    const snap = makeSnapshot({
      inventory: {
        amulet: {
          baseId: 'amulet_heroic_gryphon_s_claw',
          affixes: [],
          socketCount: 0,
          socketed: [],
          socketTypes: [],
          skillBonusOverrides: { Execute: 15 },
        },
      },
    })
    const decoded = decodeShareToBuild(encodeBuildToShare(snap))
    expect(decoded!.snapshot.inventory.amulet?.skillBonusOverrides).toEqual({
      Execute: 15,
    })
  })

  it('round-trips a picked random skill on an equipped item', () => {
    const snap = makeSnapshot({
      inventory: {
        charm_1: {
          baseId: 'charm_satanic_engineer_s_mini_drone',
          affixes: [],
          socketCount: 0,
          socketed: [],
          socketTypes: [],
          randomSkillId: 'gunner_drone',
        },
      },
    })
    const decoded = decodeShareToBuild(encodeBuildToShare(snap))
    expect(decoded!.snapshot.inventory.charm_1?.randomSkillId).toBe('gunner_drone')
  })

  it('round-trips the picked random skill element', () => {
    const snap = makeSnapshot({
      inventory: {
        boots: {
          baseId: 's10_phantoms_step',
          affixes: [],
          socketCount: 0,
          socketed: [],
          socketTypes: [],
          randomSkillElement: 'cold',
        },
      },
    })
    const decoded = decodeShareToBuild(encodeBuildToShare(snap))
    expect(decoded!.snapshot.inventory.boots?.randomSkillElement).toBe('cold')
  })

  it('defaults disabledPotions to empty when absent from the payload', () => {
    const decoded = decodeShareToBuild(encodeBuildToShare(makeSnapshot()))
    expect(decoded!.snapshot.disabledPotions).toEqual({})
  })

  it('migrates a legacy single-skill `m` string to activeSkillIds', () => {
    const code = compressToEncodedURIComponent(
      JSON.stringify({
        v: 1,
        c: 'stormweaver',
        l: 10,
        a: {},
        i: {},
        s: {},
        ss: {},
        t: [],
        m: 'fireball',
        u: null,
        buf: {},
        ec: {},
        pt: {},
        kps: 1,
      }),
    )
    const decoded = decodeShareToBuild(code)
    expect(decoded).not.toBeNull()
    expect(decoded!.snapshot.activeSkillIds).toEqual(['fireball'])
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

describe('ether + merc fields', () => {
  it('round-trips ether nodes and merc state', () => {
    const snap = makeSnapshot({
      allocatedEtherNodes: new Set([19, 38, 44]),
      mercClassId: 'merc_knight',
      mercSkillRanks: { taunt: 5, defenses: 3 },
      mercInventory: {
        helmet: {
          baseId: 'helmet_common_cap',
          affixes: [],
          socketCount: 0,
          socketed: [],
          socketTypes: [],
          stars: 0,
          forgedMods: [],
        },
      },
    })
    const decoded = decodeShareToBuild(encodeBuildToShare(snap))
    expect(decoded).not.toBeNull()
    expect([...decoded!.snapshot.allocatedEtherNodes].sort((a, b) => a - b)).toEqual([
      19, 38, 44,
    ])
    expect(decoded!.snapshot.mercClassId).toBe('merc_knight')
    expect(decoded!.snapshot.mercSkillRanks).toEqual({ taunt: 5, defenses: 3 })
    expect(decoded!.snapshot.mercInventory.helmet?.baseId).toBe(
      'helmet_common_cap',
    )
  })

  it('defaults ether and merc state when absent from the payload', () => {
    const decoded = decodeShareToBuild(encodeBuildToShare(makeSnapshot()))
    expect(decoded!.snapshot.allocatedEtherNodes.size).toBe(0)
    expect(decoded!.snapshot.mercClassId).toBeNull()
    expect(decoded!.snapshot.mercSkillRanks).toEqual({})
    expect(decoded!.snapshot.mercInventory).toEqual({})
    expect(decoded!.snapshot.mercDisabledAuras).toEqual({})
  })

  it('round-trips disabled merc auras', () => {
    const snap = makeSnapshot({ mercDisabledAuras: { 'holy aura': true } })
    const decoded = decodeShareToBuild(encodeBuildToShare(snap))
    expect(decoded!.snapshot.mercDisabledAuras).toEqual({ 'holy aura': true })
  })

  it('omits empty ether/merc fields from the encoded payload', () => {
    const code = encodeBuildToShare(makeSnapshot())
    const json = decompressFromEncodedURIComponent(code)
    const payload = JSON.parse(json!) as Record<string, unknown>
    expect(payload).not.toHaveProperty('et')
    expect(payload).not.toHaveProperty('mc')
    expect(payload).not.toHaveProperty('ms')
    expect(payload).not.toHaveProperty('mi')
  })
})

describe('legacy incarnation field (`it`)', () => {
  it('merges legacy `it` node ids into allocatedTreeNodes on decode', () => {
    const code = encodeBuildToShare(makeSnapshot())
    const payload = JSON.parse(
      decompressFromEncodedURIComponent(code)!,
    ) as Record<string, unknown>
    payload.it = [5, 11]
    const legacyCode = compressToEncodedURIComponent(JSON.stringify(payload))

    const decoded = decodeShareToBuild(legacyCode)
    expect(decoded).not.toBeNull()
    expect([...decoded!.snapshot.allocatedTreeNodes].sort((a, b) => a - b)).toEqual(
      [1, 2, 3, 5, 11],
    )
  })

  it('never writes the legacy field when encoding', () => {
    const code = encodeBuildToShare(makeSnapshot())
    const json = decompressFromEncodedURIComponent(code)
    const payload = JSON.parse(json!) as Record<string, unknown>
    expect(payload).not.toHaveProperty('it')
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
