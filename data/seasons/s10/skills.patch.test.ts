import { describe, expect, it } from 'vitest'
import { loadSeasonPatchSet } from '../load'
import { applyListPatch } from '../resolve'

type Rec = Record<string, unknown>

const skillModules = import.meta.glob<{ default: Rec[] }>('../../skills/*.json', {
  eager: true,
})
const baseSkills: Rec[] = Object.values(skillModules).flatMap((m) => m.default)

const { patches, errors } = loadSeasonPatchSet('s10')

function patchedSkill(id: string): Rec {
  const out = applyListPatch(baseSkills, patches.skills, 'skills')
  const found = out.data.find((s) => (s as Rec).id === id)
  if (!found) throw new Error(`skill ${id} missing after patch`)
  return found as Rec
}

function subskill(skill: Rec, id: string): Rec {
  const found = (skill.subskills as Rec[]).find((s) => s.id === id)
  if (!found) throw new Error(`subskill ${id} missing`)
  return found
}

describe('S10 class changes', () => {
  it('loads the s10 patch set without validation errors', () => {
    expect(errors).toEqual([])
  })

  it('applies to known skill ids only', () => {
    const out = applyListPatch(baseSkills, patches.skills, 'skills')
    expect(out.errors).toEqual([])
  })

  it('Whirlwind attack damage scaling nerfed to 40% per point', () => {
    const as = patchedSkill('whirlwind').attackScaling as Rec
    expect((as.weaponDamagePct as Rec).perLevel).toBe(40)
  })

  it("Ymir's Champion buffed to 23.5% per point", () => {
    const as = patchedSkill('ymirs_champion').attackScaling as Rec
    expect((as.weaponDamagePct as Rec).perLevel).toBe(23.5)
  })

  it('Multishot Arrow Nova: 2 default projectiles, 2 per point', () => {
    const nova = subskill(patchedSkill('multishot'), 'arrow_nova')
    const fx = nova.effects as Rec
    expect((fx.base as Rec).projectile_count).toBe(2)
    expect((fx.perRank as Rec).projectile_count).toBe(2)
  })

  it('Pipe Bombs gains the Projectile tag and the Moonshine Madness synergy', () => {
    const pb = patchedSkill('pipe_bombs')
    expect(pb.tags).toContain('Projectile')
    const sources = (pb.bonusSources as Rec[]).map((b) => b.source)
    expect(sources).toContain('Moonshine Madness')
    expect(sources).not.toContain('Moonshine Molotov')
  })

  it('Raise Skeleton: starting damage 7, intelligence scaling 7.5%', () => {
    const rs = patchedSkill('raise_skeleton')
    expect((rs.damageFormula as Rec).base).toBe(7)
    const int = (rs.bonusSources as Rec[]).find((b) => b.source === 'Intelligence') as Rec
    expect(int.value).toBe(7.5)
  })

  it('Bone Spirit swaps Follow the Leader with Life Infusion', () => {
    const bs = patchedSkill('bone_spirit')
    expect(subskill(bs, 'follow_the_leader').positionIndex).toBe(3)
    expect(subskill(bs, 'life_infusion').positionIndex).toBe(8)
  })

  it('Jousting Master: 50% first point, 25% second, max 2 points', () => {
    const jm = subskill(patchedSkill('glorious_strike'), 'jousting_master')
    expect(jm.maxRank).toBe(2)
    const fx = jm.effects as Rec
    expect((fx.base as Rec).area_of_effect).toBe(25)
    expect((fx.perRank as Rec).area_of_effect).toBe(25)
  })

  it('Tire Fire carries the full in-game tag list (Attack + Melee restored)', () => {
    expect(patchedSkill('tire_fire').tags).toEqual([
      'Attack',
      'Active',
      'Melee',
      'Spell',
      'Projectile',
      'Explosion',
    ])
  })

  it('Death from Above becomes an attack with the Ranged tag', () => {
    const dfa = patchedSkill('death_from_above')
    expect(dfa.usesAttackSpeed).toBe(true)
    expect(dfa.tags).toContain('Ranged')
  })

  it('Age Proliferation damage nerfed to 7.5 per point', () => {
    expect((patchedSkill('age_proliferation').damageFormula as Rec).perLevel).toBe(7.5)
  })

  it("Hawk's Hunger crits nerfed to 3% chance / 15% damage per point", () => {
    const hh = subskill(patchedSkill('storm_hawk'), 'hawks_hunger')
    const per = (hh.effects as Rec).perRank as Rec
    expect(per.crit_chance).toBe(3)
    expect(per.crit_damage).toBe(15)
  })

  it('Caustic Spearhead synergies all convert to poison damage (sheet update)', () => {
    const sources = patchedSkill('caustic_spearhead').bonusSources as Rec[]
    const byName = Object.fromEntries(sources.map((b) => [b.source, b]))
    expect(byName['Noxious Strike'].stat).toBe('poison_skill_damage')
    expect(byName['Noxious Strike'].value).toBe(8)
    expect(byName['Envenom'].value).toBe(20)
    expect(byName['Intelligence'].value).toBe(7.5)
  })

  it('Bombardment ICBM nerfed to 40% damage / 8% AoE per point', () => {
    const icbm = subskill(patchedSkill('bombardment'), 'icbm')
    const per = (icbm.effects as Rec).perRank as Rec
    expect(per.of_total_damage).toBe(40)
    expect(per.area_of_effect).toBe(8)
  })

  it('Shield Wall gains a 5 second cooldown', () => {
    expect(patchedSkill('shield_wall').cooldown).toBe(5)
  })

  it("Exo Black Hole nodes nerfed (Empowered Singularity 60%, Devouring Growth 32%)", () => {
    const bh = patchedSkill('black_hole')
    const es = subskill(bh, 'empowered_singularity')
    expect(((es.effects as Rec).perRank as Rec).cold_skill_damage).toBe(60)
    const gd = subskill(bh, 'galaxy_devouring_growth')
    const proc = gd.proc as Rec
    expect(((proc.effects as Rec).perRank as Rec).cold_skill_damage).toBe(32)
  })
})

describe('S10 item stat changes (second batch)', () => {
  const itemModules = import.meta.glob<{ default: Rec[] }>('../../items/*.json', {
    eager: true,
  })
  const baseItems: Rec[] = Object.values(itemModules).flatMap((m) => m.default)

  function patchedItem(id: string): Rec {
    const out = applyListPatch(baseItems, patches.items, 'items')
    const found = out.data.find((i) => (i as Rec).id === id)
    if (!found) throw new Error(`item ${id} missing after patch`)
    return found as Rec
  }

  it('Soulforged Ring swaps XP gain for XP below level 100', () => {
    const impl = patchedItem('ring_heroic_soulforged_ring').implicit as Rec
    expect(impl.experience_gain).toBeUndefined()
    expect(impl.increased_experience_gain_below_100).toBe(25)
  })

  it('Absolute Zero softens the cold res penalty and drops max cold res', () => {
    const impl = patchedItem('ring_unholy_absolute_zero').implicit as Rec
    expect(impl.cold_resistance).toBe(-30)
    expect(impl.max_cold_resistance).toBeUndefined()
  })

  it("Serpent's Tooth grants +15-20 to Envenom", () => {
    const sb = patchedItem('dagger_heroic_serpent_s_tooth').skillBonuses as Rec
    expect(sb.Envenom).toEqual([15, 20])
  })

  it('Mask of the Celestial gains flat +3 all skills', () => {
    const impl = patchedItem('helmet_angelic_mask_of_the_celestial').implicit as Rec
    expect(impl.all_skills).toBe(3)
  })

  it('Tablet of Awakening has built-in rainbow sockets at positions 2-4', () => {
    const tablet = patchedItem('charm_heroic_tablet_of_awakening')
    expect(tablet.rainbowSockets).toEqual([2, 3, 4])
  })

  it("Serpent's Tooth rolls attack speed instead of FCR and loses the Envenom proc", () => {
    const st = patchedItem('dagger_heroic_serpent_s_tooth')
    const impl = st.implicit as Rec
    expect(impl.faster_cast_rate).toBeUndefined()
    expect(impl.increased_attack_speed).toEqual([30, 45])
    expect(st.procs).toEqual([])
  })

  it("Ra's Band is promoted to SS heroic tier", () => {
    const rb = patchedItem('ring_satanic_ra_s_band')
    expect(rb.rarity).toBe('heroic')
    expect(rb.grade).toBe('SS')
  })

  it("Gurag's Fury tier-1 bonus drops to 300 life", () => {
    const setsJson = import.meta.glob<{ default: Rec[] }>('../../sets.json', {
      eager: true,
    })
    const baseSets: Rec[] = Object.values(setsJson).flatMap((m) => m.default)
    const out = applyListPatch(baseSets, patches.sets, 'sets')
    const gurag = out.data.find((s) => (s as Rec).id === 'gurag_s_fury') as Rec
    const bonuses = gurag.bonuses as Rec[]
    expect((bonuses[0].stats as Rec).life).toBe(300)
  })

  it('class-labelled set bonuses carry a class-scoped all_skills key', () => {
    const setsJson = import.meta.glob<{ default: Rec[] }>('../../sets.json', {
      eager: true,
    })
    const baseSets: Rec[] = Object.values(setsJson).flatMap((m) => m.default)
    const out = applyListPatch(baseSets, patches.sets, 'sets')
    const statKeys = patches.gameConfig?.stats?.add?.map((s) => s.key) ?? []

    let classScoped = 0
    for (const set of out.data as Rec[]) {
      for (const bonus of (set.bonuses ?? []) as Rec[]) {
        const stats = bonus.stats as Rec
        const labelled = ((bonus.descriptions ?? []) as string[]).some((d) =>
          /All Skills \(/.test(d),
        )
        if (!labelled) continue
        expect(stats.all_skills).toBeUndefined()
        const key = Object.keys(stats).find((k) => k.startsWith('all_skills_'))
        expect(key, `${set.id as string} lost its class-scoped key`).toBeDefined()
        expect(statKeys).toContain(key)
        classScoped++
      }
    }
    expect(classScoped).toBe(43)
  })

  it('unlabelled set bonuses keep the global all_skills key', () => {
    const setsJson = import.meta.glob<{ default: Rec[] }>('../../sets.json', {
      eager: true,
    })
    const baseSets: Rec[] = Object.values(setsJson).flatMap((m) => m.default)
    const out = applyListPatch(baseSets, patches.sets, 'sets')
    const gurag = out.data.find((s) => (s as Rec).id === 'gurag_s_fury') as Rec
    const tier2 = (gurag.bonuses as Rec[])[1].stats as Rec
    expect(tier2.all_skills).toBe(4)
  })

  it('game config gains the cold-res-to-cold-damage stat for Peg Leg', () => {
    const stats = patches.gameConfig?.stats?.add ?? []
    expect(stats.some((s) => s.key === 'cold_resistance_converted_to_cold_damage')).toBe(true)
    const impl = patchedItem('boots_unholy_peg_leg').implicit as Rec
    expect(impl.cold_resistance_converted_to_cold_damage).toBe(20)
  })
})
