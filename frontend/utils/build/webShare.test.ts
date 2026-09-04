import { describe, expect, it, vi } from 'vitest'
import type * as BridgeModule from '../calc/bridge'
import type { BuildPerformance } from './buildPerformance'
import type { SkillCost } from './skillCost'
import type { Skill } from '../../types'

const FAKE_PERFORMANCE: BuildPerformance = {
  attributes: {},
  stats: {},
  damage: null,
  attackDamage: null,
  hitDpsMin: undefined,
  hitDpsMax: undefined,
  avgHitDpsMin: undefined,
  avgHitDpsMax: undefined,
  procDpsMin: 0,
  procDpsMax: 0,
  combinedDpsMin: undefined,
  combinedDpsMax: undefined,
  activeSkillName: null,
  statsCombined: {},
  itemSkillBonuses: {},
  rankBonuses: {},
  perSkill: [],
  ehp: { entries: [], worst: null },
  defenseInsights: [],
  skillCosts: {},
}

const FAKE_COST: SkillCost = {
  effRankMin: 3,
  effRankMax: 3,
  baseManaMin: 20,
  baseManaMax: 20,
  mcrMax: 0,
  baseRate: 1.2,
  speedMax: 0,
  manaMin: 20,
  manaMax: 20,
  lifeMin: 0,
  lifeMax: 0,
  castRateMin: 1.2,
  castRateMax: 1.2,
  entityRate: null,
  manaPerSecMin: 24,
  manaPerSecMax: 24,
  manaRegenMin: 30,
  manaRegenMax: 30,
  sustainable: true,
  unsustainable: false,
  netMin: 6,
  netMax: 6,
  uptimeMin: 100,
  uptimeMax: 100,
}

vi.mock('../calc/bridge', async (importOriginal) => {
  const mod = await importOriginal<typeof BridgeModule>()
  return {
    ...mod,
    computeBuildPerformanceAsync: vi.fn(async () => FAKE_PERFORMANCE),
    displayValuesNative: vi.fn(async (input: { affixes?: unknown[]; scaled?: unknown[] }) => ({
      affixes: (input.affixes ?? []).map(() => ({ value: 0, rangeMin: 20, rangeMax: 20 })),
      scaled: (input.scaled ?? []).map(() => [3, 3] as [number, number]),
    })),
  }
})

import { buildSharePayload } from './webShare'
import { sharePayloadSchema, MAX_TITLE_LENGTH, type SharePayload, type SkillItem } from './sharePayload'
import { createBuild } from './savedBuilds'
import { encodeBuildToShare, type BuildSnapshot } from './shareBuild'
import { getClass, getSkillsByClass, classes, incarnationTree, gems, runes, items } from '@data'
import { socketIconPath } from './assetPaths'
import { normalizeSkillName } from '../item/stats'
import { computeBuildPerformanceAsync } from '../calc/bridge'
import { bonusSourceSynergy } from '../skill/synergyText'

const CLASS_ID = classes[0]?.id
if (!CLASS_ID) throw new Error('no classes loaded in game data')
const CLASS_SKILLS = getSkillsByClass(CLASS_ID)

const perf = vi.mocked(computeBuildPerformanceAsync)

function minimalSnapshot(classId: string): BuildSnapshot {
  const skill = getSkillsByClass(classId).find((s) => s.kind === 'active')
  return {
    classId,
    level: 50,
    allocated: {},
    inventory: {},
    skillRanks: skill ? { [skill.id]: 3 } : {},
    subskillRanks: {},
    allocatedTreeNodes: new Set<number>(),
    activeSkillIds: skill ? [skill.id] : [],
    activeAuraId: null,
    activeBuffs: {},
    enemyConditions: {},
    playerConditions: {},
    skillProjectiles: {},
    enemyResistances: {},
    procToggles: {},
    disabledPotions: {},
    killsPerSec: 1,
    customStats: [],
    treeSocketed: {},
    allocatedEtherNodes: new Set<number>(),
    mercClassId: null,
    mercSkillRanks: {},
    mercInventory: {},
    mercDisabledAuras: {},
  }
}

function savedFrom(name: string, snapshot: BuildSnapshot) {
  const saved = createBuild(name, snapshot, undefined, '', null)
  saved.profiles[0]!.code = encodeBuildToShare(snapshot)
  return saved
}

function snapshotWith(skillRanks: Record<string, number>, activeSkillIds: string[]): BuildSnapshot {
  return { ...minimalSnapshot(CLASS_ID!), skillRanks, activeSkillIds }
}

function allItems(payload: SharePayload): SkillItem[] {
  return payload.skills?.groups.flatMap((g) => g.items) ?? []
}

function itemByName(payload: SharePayload, name: string): SkillItem | undefined {
  return allItems(payload).find((i) => i.name === name)
}

function cap(word: string): string {
  return word.charAt(0).toUpperCase() + word.slice(1)
}

const treeOf = (s: Skill): string => s.tree ?? 'General'

describe('buildSharePayload', () => {
  it('produces a schema-valid payload with the library title, tags and one profile', async () => {
    const snapshot = minimalSnapshot(CLASS_ID!)
    perf.mockResolvedValueOnce({
      ...FAKE_PERFORMANCE,
      skillCosts: { [snapshot.activeSkillIds[0]!]: FAKE_COST },
    })
    const code = encodeBuildToShare(snapshot, 'Test notes')
    const saved = createBuild('My Test Build', snapshot, undefined, 'Test notes', null)
    saved.tags = ['HC']
    saved.profiles[0]!.code = code

    const payload = await buildSharePayload(saved)

    expect(sharePayloadSchema.safeParse(payload).success).toBe(true)
    expect(payload.meta.title).toBe('My Test Build')
    expect(payload.meta.classId).toBe(CLASS_ID)
    expect(payload.meta.className).toBe(getClass(CLASS_ID!)?.name)
    expect(payload.meta.tags).toEqual(['HC'])
    expect(payload.profiles).toHaveLength(1)
    expect(payload.profiles[0]!.statSections.map((s) => s.title)).toEqual([
      'Offense',
      'Defense',
      'Resistances',
      'Attributes',
      'Sustain',
    ])
    expect(payload.notes).toBe('Test notes')
  })

  it('labels attribute rows with gameConfig display names, not raw keys', async () => {
    perf.mockResolvedValueOnce({
      ...FAKE_PERFORMANCE,
      attributes: { strength: 104, energy: [430, 564] },
    })
    const snapshot = minimalSnapshot(CLASS_ID!)
    const code = encodeBuildToShare(snapshot)
    const saved = createBuild('Attr Labels', snapshot, undefined, '', null)
    saved.profiles[0]!.code = code

    const payload = await buildSharePayload(saved)

    const attributes = payload.profiles[0]!.statSections.find((s) => s.title === 'Attributes')
    expect(attributes?.rows.map((r) => r.label)).toEqual(['Strength', 'Energy'])
  })

  it('clamps an overlong build name to the schema title limit', async () => {
    const snapshot = minimalSnapshot(CLASS_ID!)
    const code = encodeBuildToShare(snapshot)
    const longName = 'A'.repeat(500)
    const saved = createBuild(longName, snapshot, undefined, '', null)
    saved.profiles[0]!.code = code

    const payload = await buildSharePayload(saved)

    expect(sharePayloadSchema.safeParse(payload).success).toBe(true)
    expect(payload.meta.title.length).toBeLessThanOrEqual(MAX_TITLE_LENGTH)
    expect(payload.meta.title).toBe(longName.slice(0, MAX_TITLE_LENGTH))
  })

  it('payload carries incarnation when nodes allocated', async () => {
    const snapshot: BuildSnapshot = {
      ...minimalSnapshot(CLASS_ID!),
      allocatedTreeNodes: new Set([incarnationTree.nodes[0]!.id]),
    }
    const saved = createBuild('Incarnation Build', snapshot, undefined, '', null)

    const payload = await buildSharePayload(saved)

    expect(payload.incarnation?.countLabel).toMatch(/\/ \d+ nodes$/)
  })

  it('rainbow sockets flagged in gear', async () => {
    const realItemId = items[0]?.id
    if (!realItemId) throw new Error('no items loaded in game data')
    const socketable = [...gems, ...runes].find((g) => socketIconPath(g.name))
    if (!socketable) throw new Error('no gem/rune with a matching socketable icon in current game data')

    const snapshot: BuildSnapshot = {
      ...minimalSnapshot(CLASS_ID!),
      inventory: {
        weapon: {
          baseId: realItemId,
          affixes: [],
          socketCount: 1,
          socketed: [socketable.id],
          socketTypes: ['rainbow'],
        },
      },
    }
    const saved = createBuild('Rainbow Socket Build', snapshot, undefined, '', null)

    const payload = await buildSharePayload(saved)

    const flags = payload.gear!.items.flatMap((i) => i.sockets.map((s) => s.rainbow ?? false))
    expect(flags).toContain(true)
  })

  it('groups all skills with rank > 0 by skill.tree, preserving class skill order', async () => {
    const ranks = Object.fromEntries(CLASS_SKILLS.map((s) => [s.id, 1]))
    const saved = savedFrom('All Skills', snapshotWith(ranks, []))

    const payload = await buildSharePayload(saved)

    const expectedGroupOrder: string[] = []
    for (const s of CLASS_SKILLS) {
      const t = treeOf(s)
      if (!expectedGroupOrder.includes(t)) expectedGroupOrder.push(t)
    }
    expect(payload.skills?.groups.map((g) => g.name)).toEqual(expectedGroupOrder)

    for (const group of payload.skills!.groups) {
      const expectedNames = CLASS_SKILLS.filter((s) => treeOf(s) === group.name).map((s) => s.name)
      expect(group.items.map((i) => i.name)).toEqual(expectedNames)
    }
    const zeroRankSaved = savedFrom('Some Skills', snapshotWith({ [CLASS_SKILLS[0]!.id]: 2 }, []))
    const zeroPayload = await buildSharePayload(zeroRankSaved)
    expect(allItems(zeroPayload)).toHaveLength(1)
  })

  it('includes non-active skills (passives, buffs), not only active skills', async () => {
    const passive = CLASS_SKILLS.find((s) => s.kind === 'passive')
    const buff = CLASS_SKILLS.find((s) => s.kind === 'buff')
    const active = CLASS_SKILLS.find((s) => s.kind === 'active')
    if (!passive || !active) throw new Error('class lacks a passive/active skill to exercise')

    const ranks: Record<string, number> = { [passive.id]: 5, [active.id]: 5 }
    if (buff) ranks[buff.id] = 5
    const saved = savedFrom('Every Kind', snapshotWith(ranks, [active.id]))

    const payload = await buildSharePayload(saved)

    const names = allItems(payload).map((i) => i.name)
    expect(names).toContain(passive.name)
    expect(names).toContain(active.name)
    if (buff) expect(names).toContain(buff.name)
  })

  it('marks activeSkillIds[0] as main and leaves other skills unmarked', async () => {
    const actives = CLASS_SKILLS.filter((s) => s.kind === 'active')
    const [a, b] = actives
    if (!a || !b) throw new Error('class needs two active skills')
    const saved = savedFrom('Main Skill', snapshotWith({ [a.id]: 10, [b.id]: 10 }, [a.id, b.id]))

    const payload = await buildSharePayload(saved)

    expect(itemByName(payload, a.name)?.main).toBe(true)
    expect(itemByName(payload, b.name)?.main).toBeUndefined()
  })

  it('formats type as "Kind · DamageType" capitalized, kind alone without damageType', async () => {
    const dmgSkill = CLASS_SKILLS.find((s) => s.kind === 'active' && s.damageType)
    const plainSkill = CLASS_SKILLS.find((s) => !s.damageType)
    if (!dmgSkill?.damageType || !plainSkill) throw new Error('class lacks the needed skill shapes')

    const saved = savedFrom(
      'Types',
      snapshotWith({ [dmgSkill.id]: 4, [plainSkill.id]: 4 }, [dmgSkill.id]),
    )

    const payload = await buildSharePayload(saved)

    expect(itemByName(payload, dmgSkill.name)?.type).toBe(`${cap(dmgSkill.kind)} · ${cap(dmgSkill.damageType)}`)
    expect(itemByName(payload, dmgSkill.name)?.type).toContain(' · ')
    expect(itemByName(payload, plainSkill.name)?.type).toBe(cap(plainSkill.kind))
    expect(itemByName(payload, plainSkill.name)?.type).not.toContain('·')
  })

  it('formats fromItems "+4" / "+2–6" and effectiveRank "16" / "14–18" from rank bonuses; omits when zero', async () => {
    const [a, b, c] = CLASS_SKILLS
    if (!a || !b || !c) throw new Error('class needs three skills')
    const snapshot = snapshotWith({ [a.id]: 12, [b.id]: 12, [c.id]: 12 }, [])
    const saved = savedFrom('Rank Bonuses', snapshot)

    perf.mockResolvedValueOnce({
      ...FAKE_PERFORMANCE,
      rankBonuses: {
        [normalizeSkillName(a.name)]: [4, 4],
        [normalizeSkillName(b.name)]: [2, 6],
      },
    })

    const payload = await buildSharePayload(saved)

    expect(itemByName(payload, a.name)?.fromItems).toBe('+4')
    expect(itemByName(payload, a.name)?.effectiveRank).toBe('16')
    expect(itemByName(payload, b.name)?.fromItems).toBe('+2–6')
    expect(itemByName(payload, b.name)?.effectiveRank).toBe('14–18')
    expect(itemByName(payload, c.name)?.fromItems).toBeUndefined()
    expect(itemByName(payload, c.name)?.effectiveRank).toBeUndefined()
  })

  it('emits rows: Hit DPS (gold, from perSkillDps), Mana / cast (blue), Cooldown — each only when available', async () => {
    const manaActive = CLASS_SKILLS.find(
      (s) => s.kind === 'active' && s.ranks[0]?.manaCost !== undefined,
    )
    const otherActive = CLASS_SKILLS.find(
      (s) => s.kind === 'active' && s.id !== manaActive?.id,
    )
    const cdSkill = CLASS_SKILLS.find((s) => s.baseCooldown !== undefined)
    const passive = CLASS_SKILLS.find((s) => s.kind === 'passive')
    if (!manaActive || !otherActive || !cdSkill || !passive) {
      throw new Error('class lacks the skill shapes needed for the rows test')
    }

    const ranks: Record<string, number> = {
      [manaActive.id]: 8,
      [otherActive.id]: 8,
      [cdSkill.id]: 8,
      [passive.id]: 8,
    }
    const saved = savedFrom('Rows', snapshotWith(ranks, [manaActive.id, otherActive.id]))

    perf.mockResolvedValueOnce({
      ...FAKE_PERFORMANCE,
      perSkill: [
        { id: manaActive.id, name: manaActive.name, hitDpsMin: 1e9, hitDpsMax: 1e9 },
        { id: otherActive.id, name: otherActive.name, hitDpsMin: 2e9, hitDpsMax: 2e9 },
      ],
      skillCosts: { [manaActive.id]: { ...FAKE_COST, baseManaMin: 12, baseManaMax: 12 } },
    })

    const payload = await buildSharePayload(saved)

    const rowsOf = (name: string) => itemByName(payload, name)?.rows ?? []
    const rowByLabel = (name: string, label: string) => rowsOf(name).find((r) => r.label === label)

    const mainDps = rowByLabel(manaActive.name, 'Hit DPS')
    expect(mainDps?.tone).toBe('gold')
    expect(mainDps?.glow).toBe(true)
    expect(rowByLabel(manaActive.name, 'Mana / cast')).toMatchObject({
      value: '12',
      tone: 'blue',
    })

    const otherDps = rowByLabel(otherActive.name, 'Hit DPS')
    expect(otherDps?.tone).toBe('gold')
    expect(otherDps?.glow).toBeUndefined()

    expect(rowByLabel(cdSkill.name, 'Cooldown')).toMatchObject({
      value: `${cdSkill.baseCooldown}s`,
    })
    expect(rowByLabel(cdSkill.name, 'Hit DPS')).toBeUndefined()

    expect(itemByName(payload, passive.name)?.rows).toBeUndefined()
  })

  it('builds synergies from bonusSources with the same text as SkillEffectsBlock', async () => {
    const withBonus = CLASS_SKILLS.find((s) => (s.bonusSources?.length ?? 0) > 0)
    if (!withBonus) throw new Error('class lacks a skill with bonusSources')
    const saved = savedFrom('Synergies', snapshotWith({ [withBonus.id]: 12 }, []))

    const payload = await buildSharePayload(saved)

    expect(itemByName(payload, withBonus.name)?.synergies).toEqual(
      (withBonus.bonusSources ?? []).map(bonusSourceSynergy),
    )
  })

  it('sets pointsLabel to "<spent> pts" and omits the removed tree section', async () => {
    const [a, b] = CLASS_SKILLS
    if (!a || !b) throw new Error('class needs two skills')
    const saved = savedFrom('Points', snapshotWith({ [a.id]: 7, [b.id]: 5 }, [a.id]))

    const payload = await buildSharePayload(saved)

    expect(payload.skills?.pointsLabel).toBe('12 pts')
    expect('tree' in payload).toBe(false)
    expect(sharePayloadSchema.safeParse(payload).success).toBe(true)
  })
})
