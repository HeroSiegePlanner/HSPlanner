import { compressToEncodedURIComponent } from 'lz-string'
import { describe, expect, it } from 'vitest'
import { makeSnapshot } from './buildSnapshot.fixture'
import {
  decodeShareToBuild,
  defaultEnemyResistances,
  encodeBuildToShare,
  type BuildSnapshot,
} from './shareBuild'

function snapshot(): BuildSnapshot {
  return makeSnapshot({
    level: 10,
    allocated: { strength: 1, dexterity: 0, intelligence: 0, energy: 0, vitality: 0, armor: 0 },
    allocatedTreeNodes: new Set([0, 2]),
  })
}

describe('share schema v2 season', () => {
  it('encodes with current season and decodes it back', () => {
    const code = encodeBuildToShare(snapshot())
    const decoded = decodeShareToBuild(code)
    expect(decoded).not.toBeNull()
    expect(decoded!.season).toBe('s9')
  })

  // Regression: v1 payloads (no `se` field) must decode as the legacy season 's9'
  it('v1 payload (no se field) decodes as legacy season s9', () => {
    const snap = snapshot()
    const v1Payload = {
      v: 1,
      c: snap.classId,
      l: snap.level,
      a: snap.allocated,
      i: snap.inventory,
      s: snap.skillRanks,
      ss: snap.subskillRanks,
      t: [...snap.allocatedTreeNodes].sort((x, y) => x - y),
      m: snap.activeSkillIds[0] ?? null,
      u: snap.activeAuraId,
      buf: snap.activeBuffs,
      ec: snap.enemyConditions,
      pt: snap.procToggles,
      er: defaultEnemyResistances(),
      kps: snap.killsPerSec,
    }
    const code = compressToEncodedURIComponent(JSON.stringify(v1Payload))
    const decoded = decodeShareToBuild(code)
    expect(decoded).not.toBeNull()
    expect(decoded!.season).toBe('s9')
    expect(decoded!.snapshot.classId).toBe('amazon')
    expect(decoded!.snapshot.level).toBe(10)
  })
})
