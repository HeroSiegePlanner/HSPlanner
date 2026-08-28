import { describe, expect, it } from 'vitest'
import { skills } from './index'

type Rec = Record<string, unknown>

function skill(id: string): Rec {
  const found = skills.find((s) => s.id === id)
  if (!found) throw new Error(`skill ${id} missing`)
  return found as unknown as Rec
}

function subskill(parent: Rec, id: string): Rec {
  const found = (parent.subskills as Rec[]).find((s) => s.id === id)
  if (!found) throw new Error(`subskill ${id} missing`)
  return found
}

describe('S10 class changes', () => {
  it('Whirlwind attack damage scaling nerfed to 40% per point', () => {
    const as = skill('whirlwind').attackScaling as Rec
    expect((as.weaponDamagePct as Rec).perLevel).toBe(40)
  })

  it("Ymir's Champion buffed to 23.5% per point", () => {
    const as = skill('ymirs_champion').attackScaling as Rec
    expect((as.weaponDamagePct as Rec).perLevel).toBe(23.5)
  })

  it('Multishot Arrow Nova: 2 default projectiles, 2 per point', () => {
    const nova = subskill(skill('multishot'), 'arrow_nova')
    const fx = nova.effects as Rec
    expect((fx.base as Rec).projectile_count).toBe(2)
    expect((fx.perRank as Rec).projectile_count).toBe(2)
  })

  it('Pipe Bombs gains the Projectile tag and the Moonshine Madness synergy', () => {
    const pb = skill('pipe_bombs')
    expect(pb.tags).toContain('Projectile')
    const sources = (pb.bonusSources as Rec[]).map((b) => b.source)
    expect(sources).toContain('Moonshine Madness')
    expect(sources).not.toContain('Moonshine Molotov')
  })

  it('Raise Skeleton: starting damage 7, intelligence scaling 7.5%', () => {
    const rs = skill('raise_skeleton')
    expect((rs.damageFormula as Rec).base).toBe(7)
    const int = (rs.bonusSources as Rec[]).find((b) => b.source === 'Intelligence') as Rec
    expect(int.value).toBe(7.5)
  })

  it('Bone Spirit swaps Follow the Leader with Life Infusion', () => {
    const bs = skill('bone_spirit')
    expect(subskill(bs, 'follow_the_leader').positionIndex).toBe(3)
    expect(subskill(bs, 'life_infusion').positionIndex).toBe(8)
  })

  it('Jousting Master: 50% first point, 25% second, max 2 points', () => {
    const jm = subskill(skill('glorious_strike'), 'jousting_master')
    expect(jm.maxRank).toBe(2)
    const fx = jm.effects as Rec
    expect((fx.base as Rec).area_of_effect).toBe(25)
    expect((fx.perRank as Rec).area_of_effect).toBe(25)
  })

  it('Tire Fire carries the full in-game tag list (Attack + Melee restored)', () => {
    expect(skill('tire_fire').tags).toEqual([
      'Attack',
      'Active',
      'Melee',
      'Spell',
      'Projectile',
      'Explosion',
    ])
  })

  it('Death from Above becomes an attack with the Ranged tag', () => {
    const dfa = skill('death_from_above')
    expect(dfa.usesAttackSpeed).toBe(true)
    expect(dfa.tags).toContain('Ranged')
  })

  it('Age Proliferation damage nerfed to 7.5 per point', () => {
    expect((skill('age_proliferation').damageFormula as Rec).perLevel).toBe(7.5)
  })

  it("Hawk's Hunger crits nerfed to 3% chance / 15% damage per point", () => {
    const hh = subskill(skill('storm_hawk'), 'hawks_hunger')
    const per = (hh.effects as Rec).perRank as Rec
    expect(per.crit_chance).toBe(3)
    expect(per.crit_damage).toBe(15)
  })

  it('Caustic Spearhead synergies all convert to poison damage (sheet update)', () => {
    const sources = skill('caustic_spearhead').bonusSources as Rec[]
    const byName = Object.fromEntries(sources.map((b) => [b.source as string, b]))
    expect(byName['Noxious Strike'].stat).toBe('poison_skill_damage')
    expect(byName['Noxious Strike'].value).toBe(8)
    expect(byName['Envenom'].value).toBe(20)
    expect(byName['Intelligence'].value).toBe(7.5)
  })

  it('Bombardment ICBM nerfed to 40% damage / 8% AoE per point', () => {
    const icbm = subskill(skill('bombardment'), 'icbm')
    const per = (icbm.effects as Rec).perRank as Rec
    expect(per.of_total_damage).toBe(40)
    expect(per.area_of_effect).toBe(8)
  })

  it('Shield Wall gains a 5 second cooldown', () => {
    expect(skill('shield_wall').cooldown).toBe(5)
  })

  it('Exo Black Hole nodes nerfed (Empowered Singularity 60%, Devouring Growth 32%)', () => {
    const bh = skill('black_hole')
    const es = subskill(bh, 'empowered_singularity')
    expect(((es.effects as Rec).perRank as Rec).cold_skill_damage).toBe(60)
    const gd = subskill(bh, 'galaxy_devouring_growth')
    const proc = gd.proc as Rec
    expect(((proc.effects as Rec).perRank as Rec).cold_skill_damage).toBe(32)
  })
})
