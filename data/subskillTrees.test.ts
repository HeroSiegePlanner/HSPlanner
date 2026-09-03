import { describe, expect, it } from 'vitest'
import gameConfigJson from './game-config.json'
import type { Skill, SubskillNode } from '../frontend/types'

const skillModules = import.meta.glob<{ default: Skill[] }>('./skills/*.json', {
  eager: true,
})

// 220 trees generated 2026-08 from the game's subskill tables plus the two
// original Stormweaver trees. The generator and its mapping decisions live in
// git history: the "feat(skills): sub-skill trees for all 222 skills" commit
// carries scripts/subskills/ (parser, mapping.json, audit reports).
const TREE_COUNT = 222
const MAX_POSITION_INDEX = 14

const STAT_KEYS = new Set(gameConfigJson.stats.map((s) => s.key))

interface Tree {
  classId: string
  skillId: string
  nodes: SubskillNode[]
}

const trees: Tree[] = Object.entries(skillModules).flatMap(([path, mod]) => {
  const classId = path.split('/').pop()?.replace(/\.json$/, '') ?? path
  return mod.default
    .filter((skill) => (skill.subskills?.length ?? 0) > 0)
    .map((skill) => ({
      classId,
      skillId: skill.id,
      nodes: skill.subskills ?? [],
    }))
})

const nodeKeys = (node: SubskillNode): string[] => [
  ...Object.keys(node.effects?.base ?? {}),
  ...Object.keys(node.effects?.perRank ?? {}),
  ...Object.keys(node.proc?.effects?.base ?? {}),
  ...Object.keys(node.proc?.effects?.perRank ?? {}),
]

describe('subskill trees', () => {
  it('ships one tree per skill listed in the source', () => {
    expect(trees).toHaveLength(TREE_COUNT)
  })

  it('only emits stat keys declared in game-config', () => {
    const unknown = new Set<string>()
    for (const tree of trees) {
      for (const node of tree.nodes) {
        for (const key of nodeKeys(node)) {
          if (!STAT_KEYS.has(key)) unknown.add(`${tree.skillId}/${node.id}: ${key}`)
        }
      }
    }
    expect([...unknown]).toEqual([])
  })

  it('lays every tree out on positions 0-14 with the core at 0', () => {
    for (const tree of trees) {
      const positions = tree.nodes.map((n) => n.positionIndex)
      expect(new Set(positions).size, `${tree.skillId}: duplicate positions`).toBe(
        positions.length,
      )
      for (const p of positions) {
        expect(p, `${tree.skillId}: position out of range`).toBeGreaterThanOrEqual(0)
        expect(p, `${tree.skillId}: position out of range`).toBeLessThanOrEqual(
          MAX_POSITION_INDEX,
        )
      }
      expect(positions, `${tree.skillId}: no core node`).toContain(0)
    }
  })

  it('gives every node a unique id and at least one rank', () => {
    for (const tree of trees) {
      const ids = tree.nodes.map((n) => n.id)
      expect(new Set(ids).size, `${tree.skillId}: duplicate node id`).toBe(ids.length)
      for (const node of tree.nodes) {
        expect(node.maxRank, `${tree.skillId}/${node.id}`).toBeGreaterThanOrEqual(1)
      }
    }
  })
})

// The requirement behind the subskill pipeline: every damage-relevant key the
// trees emit must land on a stat the engine actually multiplies with. The two
// lists below are the frozen contract with engine/src/calc — extend them
// only together with the Rust change (and its test) that consumes the key.
describe('damage coverage', () => {
  // Keys the damage pipeline reads (audited across build.rs, skills/damage.rs,
  // skills/attack.rs, skills/mod.rs, skills/ailment.rs, skills/conversion.rs;
  // each entry either predates the pipeline or has a dedicated Rust test).
  const CALC_CONSUMED = new Set<string>([
    'all_skills',
    'arcane_skills',
    'cold_skills',
    'fire_skills',
    'lightning_skills',
    'poison_skills',
    'physical_skills',
    'explosion_skills',
    'magic_skills',
    'projectile_skills',
    'arcane_skill_damage',
    'cold_skill_damage',
    'fire_skill_damage',
    'lightning_skill_damage',
    'poison_skill_damage',
    'magic_skill_damage',
    'physical_skill_damage',
    'explosion_skill_damage',
    'arcane_skill_damage_more',
    'cold_skill_damage_more',
    'fire_skill_damage_more',
    'lightning_skill_damage_more',
    'poison_skill_damage_more',
    'magic_skill_damage_more',
    'physical_skill_damage_more',
    'explosion_skill_damage_more',
    'flat_skill_damage',
    'flat_elemental_skill_damage',
    'flat_arcane_skill_damage',
    'flat_cold_skill_damage',
    'flat_fire_skill_damage',
    'flat_lightning_skill_damage',
    'flat_poison_skill_damage',
    'flat_magic_skill_damage',
    'flat_physical_skill_damage',
    'flat_explosion_skill_damage',
    'orbital_skill_damage',
    'orbital_skill_damage_more',
    'explosion_damage',
    'spell_aoe_damage',
    'spell_aoe_damage_more',
    'spell_projectile_damage',
    'extra_damage_stunned',
    'extra_damage_bleeding',
    'extra_damage_frozen',
    'extra_damage_poisoned',
    'extra_damage_burning',
    'extra_damage_stasis',
    'extra_damage_shadow_burning',
    'extra_damage_frost_bitten',
    'extra_dmg_to_deep_frozen',
    'extra_damage_low_life',
    'extra_fire_dmg_to_burning',
    'extra_poison_dmg_to_poisoned',
    'extra_physical_dmg_to_bleeding',
    'extra_lightning_dmg_stasis',
    'extra_lightning_dmg_to_stasis',
    'extra_lightning_dmg_slow',
    'extra_damage_ailments',
    'extra_dmg_to_monsters_afflicted_with_ailments',
    'extra_damage_bosses',
    'extra_damage_serrated_chains',
    'crit_chance',
    'crit_damage',
    'crit_damage_more',
    'spell_crit_chance',
    'spell_crit_damage',
    'multicast_chance',
    'projectile_count',
    'of_total_damage',
    'elemental_break',
    'elemental_break_on_spell',
    'elemental_break_on_strike',
    'lightning_break',
    'arcane_break',
    'fire_break',
    'cold_break',
    'poison_break',
    'armor_break',
    'ignore_arcane_res',
    'ignore_cold_res',
    'ignore_fire_res',
    'ignore_lightning_res',
    'ignore_poison_res',
    'ignore_physical_res',
    'ignore_magic_res',
    'ignore_explosion_res',
    'faster_cast_rate',
    'faster_cast_rate_more',
    'enhanced_damage',
    'enhanced_damage_more',
    'additive_physical_damage',
    'additive_fire_damage',
    'additive_cold_damage',
    'additive_lightning_damage',
    'additive_poison_damage',
    'additive_arcane_damage',
    'attack_damage',
    'increased_attack_speed',
    'increased_attack_speed_more',
    'attacks_per_second',
    'crushing_blow_modifier',
    'deadly_blow_chance',
    'deadly_blow',
    'deadly_blow_effectiveness',
    'hit_chance',
    'execute_below',
    'single_target_hit_cap',
    'extra_volleys_pct',
    'enemy_damage_taken_increased',
    'increased_burning_damage',
    'increased_bleeding_damage',
    'increased_poisoned_damage',
    'increased_stasis_damage',
    'increased_frost_bite_damage',
    'increased_shadow_burning_damage',
    'increased_permafrost_damage',
    'increased_rabies_damage',
    'ailment_damage_all',
    'increased_ailment_frequency',
    'increased_bleeding_frequency',
    'increased_poisoned_frequency',
    // conversion_* resolves per source in skills/conversion.rs
    ...gameConfigJson.stats
      .map((s) => s.key)
      .filter((k) => k.startsWith('conversion_')),
  ])

  // Attribute buffs move damage through the attribute pass
  // (damage.rs BonusSource::AttributePoint), not through a direct stat read.
  const CONSUMED_VIA_ATTRIBUTES = new Set<string>([
    'to_strength',
    'to_dexterity',
    'increased_strength',
    'increased_dexterity',
    'increased_all_attributes',
  ])

  // serrated_chains "Ignore Target Defense": the engine models no enemy
  // defense, so the key is mathematically neutral — visible on the card only.
  const EXCUSED = new Set<string>(['defense_ignored'])

  // What counts as a damage-relevant key in emitted data. Anything matching
  // one of these and not consumed/excused is a coverage regression.
  const DAMAGE_KEY_MATCHERS: RegExp[] = [
    /_skill_damage(_more)?$/,
    /^flat_.*_skill_damage$/,
    /^(of_total_damage|projectile_count|multicast_chance|attack_damage)$/,
    /^(spell_)?crit_(chance|damage)$/,
    /^(increased_attack_speed|faster_cast_rate)$/,
    /^extra_damage_/,
    /^extra_.*_dmg_/,
    /_break$/,
    /^increased_(burning|bleeding|poisoned|stasis|frost_bite|shadow_burning|permafrost|rabies|ailment)_(damage|frequency)$/,
    /^ailment_damage_all$/,
    /^conversion_/,
    /^(execute_below|enemy_damage_taken_increased)$/,
    /^deadly_blow(_effectiveness)?$/,
    /^(to|increased)_(strength|dexterity|all_attributes)$/,
    /^all_skills$/,
    /^defense_ignored$/,
  ]

  const isDamageKey = (key: string): boolean =>
    DAMAGE_KEY_MATCHERS.some((m) => m.test(key))

  const emittedKeys = new Set(
    trees.flatMap((tree) => tree.nodes.flatMap((node) => nodeKeys(node))),
  )

  it('every damage-relevant emitted key reaches the calculation', () => {
    const uncovered = [...emittedKeys]
      .filter(isDamageKey)
      .filter(
        (k) =>
          !CALC_CONSUMED.has(k) &&
          !CONSUMED_VIA_ATTRIBUTES.has(k) &&
          !EXCUSED.has(k),
      )
    expect(uncovered.sort()).toEqual([])
  })

  it('excuses exactly one damage-relevant key, and ships it', () => {
    expect([...EXCUSED]).toEqual(['defense_ignored'])
    expect(emittedKeys.has('defense_ignored')).toBe(true)
  })

  it('actually exercises the damage classification', () => {
    const classified = [...emittedKeys].filter(isDamageKey)
    // 222 trees emit damage notes on every skill — a collapse of the matcher
    // list (or of the data) should trip this long before coverage lies.
    expect(classified.length).toBeGreaterThan(60)
  })
})

// Sprites are resolved by filename alone (SubtreeOverlay#subskillSpriteKey):
// <classId>_<skillId>_subskill_<positionIndex>.png. A typo in a dropped-in
// folder would otherwise fail silently as a missing icon.
describe('subskill sprites', () => {
  const spriteFiles = Object.keys(
    import.meta.glob('../frontend/assets/subskills/**/*.png'),
  ).map((p) => p.split('/').pop()!.replace(/\.png$/i, ''))

  const spriteKeys = new Set(
    trees.flatMap((t) =>
      t.nodes.map(
        (n) => `${t.classId}_${t.skillId}_subskill_${n.positionIndex}`,
      ),
    ),
  )

  it('names every sprite after an existing tree node', () => {
    expect(spriteFiles.filter((k) => !spriteKeys.has(k)).sort()).toEqual([])
  })

  it('ships complete trees — no half-covered skill', () => {
    const perSkill = new Map<string, number>()
    for (const key of spriteFiles) {
      const skill = key.replace(/_subskill_\d+$/, '')
      perSkill.set(skill, (perSkill.get(skill) ?? 0) + 1)
    }
    const incomplete = [...perSkill]
      .filter(([, n]) => n !== MAX_POSITION_INDEX + 1)
      .map(([skill, n]) => `${skill}: ${n}`)
    expect(incomplete.sort()).toEqual([])
  })
})
