import { describe, expect, it } from 'vitest'
import { CURRENT_SCHEMA_VERSION, sharePayloadSchema, snapshotByteSize } from './sharePayload'
import { SHARE_PAYLOAD_FIXTURE } from './sharePayload.fixture'

describe('sharePayloadSchema v2', () => {
  it('accepts the canonical fixture', () => {
    expect(sharePayloadSchema.safeParse(SHARE_PAYLOAD_FIXTURE).success).toBe(true)
  })

  it('rejects schemaVersion 1', () => {
    const v1 = { ...SHARE_PAYLOAD_FIXTURE, schemaVersion: 1 }
    expect(sharePayloadSchema.safeParse(v1).success).toBe(false)
  })

  it('accepts a minimal payload without optional sections', () => {
    const minimal = {
      schemaVersion: CURRENT_SCHEMA_VERSION,
      appVersion: '0.11.0',
      code: 'HSP://TEST',
      meta: SHARE_PAYLOAD_FIXTURE.meta,
      profiles: [{ name: 'Current', dps: '1M', statSections: [] }],
    }
    expect(sharePayloadSchema.safeParse(minimal).success).toBe(true)
  })

  it('rejects a tooltip line with unknown kind', () => {
    const bad = structuredClone(SHARE_PAYLOAD_FIXTURE)
    // @ts-expect-error — celowo zepsuty kind
    bad.gear!.items[0]!.tooltip.sections[0]!.lines[0] = { kind: 'mystery', text: 'x' }
    expect(sharePayloadSchema.safeParse(bad).success).toBe(false)
  })

  it('rejects skills without groups array', () => {
    const bad = structuredClone(SHARE_PAYLOAD_FIXTURE) as Record<string, unknown>
    bad.skills = { pointsLabel: '63 pts', items: [] }
    expect(sharePayloadSchema.safeParse(bad).success).toBe(false)
  })

  it('rejects minor node with count below 1', () => {
    const bad = structuredClone(SHARE_PAYLOAD_FIXTURE)
    bad.incarnation!.minors[0]!.count = 0
    expect(sharePayloadSchema.safeParse(bad).success).toBe(false)
  })

  it('snapshotByteSize excludes code', () => {
    const small = snapshotByteSize({ ...SHARE_PAYLOAD_FIXTURE, code: 'HSP://A' })
    const big = snapshotByteSize({ ...SHARE_PAYLOAD_FIXTURE, code: 'HSP://' + 'A'.repeat(10_000) })
    expect(small).toBe(big)
  })
})
