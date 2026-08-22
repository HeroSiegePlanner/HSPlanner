import { describe, expect, it } from 'vitest'
import { loadSeasonPatchSet } from '../load'
import { applyListPatch } from '../resolve'
import itemGrantedSkillsJson from '../../item-granted-skills.json'

type Rec = Record<string, unknown>

const itemModules = import.meta.glob<{ default: Rec[] }>('../../items/*.json', {
  eager: true,
})
const baseItems: Rec[] = Object.values(itemModules).flatMap((m) => m.default)

const gemModules = import.meta.glob<{ default: Rec[] }>('../../gems/*.json', {
  eager: true,
})
const gemsBase: Rec[] = Object.values(gemModules).flatMap((m) => m.default)

const { patches, errors } = loadSeasonPatchSet('s10')

function patchedItem(id: string): Rec {
  const out = applyListPatch(baseItems, patches.items, 'items')
  const found = out.data.find((i) => (i as Rec).id === id)
  if (!found) throw new Error(`item ${id} missing after patch`)
  return found as Rec
}

describe('S10 item changes', () => {
  it('loads the s10 patch set without validation errors', () => {
    expect(errors).toEqual([])
  })

  it("Ukko's Revenge nerfs lightning skill damage and stasis lightning", () => {
    const impl = patchedItem('weapons_heroic_ukkos_revenge').implicit as Rec
    expect(impl.lightning_skill_damage).toEqual([25, 40])
    expect(impl.extra_lightning_dmg_stasis).toEqual([20, 30])
  })

  it('Chaoswalkers nerfs extra damage per ailment', () => {
    const impl = patchedItem('boots_heroic_chaoswalkers').implicit as Rec
    expect(impl.extra_damage_ailments).toEqual([8, 15])
  })

  it('Glacier Talons nerfs the Blizzard on-kill proc rate to 4%', () => {
    const procs = patchedItem('claw_heroic_glacier_talons').procs as Rec[]
    expect(procs).toHaveLength(1)
    expect(procs[0]).toMatchObject({ trigger: 'on_kill', chance: 4 })
  })

  it('Amulet of Colosseum implicit is % increased attack speed, not flat APS', () => {
    const impl = patchedItem('amulet_heroic_amulet_of_colosseum').implicit as Rec
    expect(impl.increased_attack_speed).toEqual([5, 10])
    expect(impl.attacks_per_second).toBeUndefined()
    expect(impl.all_skills).toEqual([2, 4])
  })

  it('Angel gun gains base damage and APS moved out of implicit', () => {
    const angel = patchedItem('gun_satanic_angel')
    expect(angel.attackSpeed).toBe(1.75)
    expect(angel.damageMin).toBeGreaterThan(0)
    expect(angel.damageMax).toBeGreaterThanOrEqual(angel.damageMin as number)
    const impl = angel.implicit as Rec
    expect(impl.attacks_per_second).toBeUndefined()
    expect(impl.enhanced_damage).toEqual([430, 580])
  })

  it('every weapon has base damage and attack speed, none hides APS in implicit', () => {
    const out = applyListPatch(baseItems, patches.items, 'items')
    // Spears absent from hero-siege-helper; s10_ weapons are new-season
    // scaffolding the helper does not list yet.
    const missingInSource = new Set([
      'base_throwing_short_spear',
      'base_polearm_tribal_spear',
      's10_ethereal_musket',
      's10_grimtides_scimitar',
      's10_conjured_tentacle',
      's10_phantom_scimitar',
      's10_phantom_strike',
      's10_leviathans_spine',
    ])
    const weapons = out.data.filter(
      (i) => (i as Rec).slot === 'weapon' && !missingInSource.has((i as Rec).id as string),
    ) as Rec[]
    const noDamage = weapons.filter((w) => typeof w.damageMin !== 'number')
    const noAps = weapons.filter((w) => typeof w.attackSpeed !== 'number')
    const apsInImplicit = weapons.filter(
      (w) => ((w.implicit ?? {}) as Rec).attacks_per_second !== undefined,
    )
    expect(noDamage.map((w) => w.id)).toEqual([])
    expect(noAps.map((w) => w.id)).toEqual([])
    expect(apsInImplicit.map((w) => w.id)).toEqual([])
  })

  it('Shadow Lantern shield exists with its sentry package', () => {
    const sl = patchedItem('shield_heroic_shadow_lantern')
    expect(sl.slot).toBe('offhand')
    expect(sl.rarity).toBe('heroic')
    expect(sl.requiresLevel).toBe(98)
    expect(sl.implicit).toEqual({
      all_skills: [2, 3],
      skill_haste: [15, 35],
      sentry_damage: [25, 50],
      sentry_skills: [3, 5],
      sentry_duration: 33,
      sentry_max_amount: [1, 3],
    })
    expect(sl.uniqueEffects).toEqual(['Unholy', 'Unholy'])
    expect(sl.maxAffixes).toBe(2)
    expect(sl.randomAffixGroupId).toBe('random_unholy')
    expect(sl.sockets).toBe(3)
    expect(sl.maxSockets).toBe(4)
  })

  it('Ghost Armada charm carries its S10 stats', () => {
    const ga = patchedItem('s10_ghost_armada')
    expect(ga.slot).toBe('charm_1')
    expect(ga.grade).toBe('SS')
    expect(ga.requiresLevel).toBe(100)
    expect([ga.width, ga.height]).toEqual([3, 2])
    expect(ga.implicit).toEqual({
      all_skills: [3, 5],
      attack_radius: [25, 60],
      increased_attack_speed: [25, 50],
      deadly_blow: [10, 20],
      crushing_blow_chance: 25,
    })
  })

  it('Jar of Parasites charm: "Reduced by -25%" in game is a real 25% reduction', () => {
    const jar = patchedItem('s10_jar_of_parasites')
    expect(jar.grade).toBe('SS')
    expect(jar.requiresLevel).toBe(100)
    expect([jar.width, jar.height]).toEqual([1, 2])
    expect(jar.implicit).toEqual({
      all_skills: [2, 3],
      skill_haste: [10, 100],
      physical_damage_reduction: 25,
      magic_damage_reduction: 25,
    })
  })

  it('"Damage Taken Reduced by -10%" on Unholy Bible and Belt of Infinite Wealth is a real reduction', () => {
    const bible = patchedItem('book_satanic_satan_s_unholy_bible').implicit as Rec
    expect(bible.physical_damage_reduction).toBe(10)
    expect(bible.all_resistances).toBe(-50)
    const belt = patchedItem('belt_heroic_belt_of_infinite_wealth').implicit as Rec
    expect(belt.physical_damage_reduction).toBe(10)
    expect(belt.gold_find).toEqual([25, 40])
  })

  it('Ghastly Skull charm buffs summons and Will-O-Wisp with an orbiting-skull effect', () => {
    const skull = patchedItem('s10_ghastly_skull')
    expect(skull.grade).toBe('SS')
    expect(skull.requiresLevel).toBe(100)
    expect([skull.width, skull.height]).toEqual([1, 2])
    expect(skull.implicit).toEqual({ summon_skills: [3, 5] })
    expect(skull.skillBonuses).toEqual({ 'Will-O-Wisp': [1, 5] })
    expect(skull.uniqueEffects).toBeUndefined()
  })

  it("Captain's Anchor charm rolls a random element for its +5 skills", () => {
    const anchor = patchedItem('s10_captains_anchor')
    expect(anchor.grade).toBe('SS')
    expect(anchor.requiresLevel).toBe(100)
    expect([anchor.width, anchor.height]).toEqual([2, 2])
    expect(anchor.implicit).toEqual({
      random_skill_element: 5,
      phys_dmg_taken_as_cold: [10, 20],
      enemy_all_resist: 20,
      light_radius: [4, 8],
    })
  })

  it('Parasite Loop ring procs Sanguine Leech on cast', () => {
    const loop = patchedItem('s10_parasite_loop')
    expect(loop.slot).toBe('ring_1')
    expect(loop.grade).toBe('SS')
    expect(loop.requiresLevel).toBe(100)
    expect(loop.procs).toEqual([
      {
        trigger: 'on_cast',
        chance: 8,
        description: 'cast Sanguine Leech Level 1',
        details:
          'Summon a Sanguine Leech dealing damage and providing you increased life replenish for a short duration. Arcane Damage 12287, Life Replenish 800%',
      },
    ])
    expect(loop.implicit).toEqual({
      all_skills: [1, 2],
      skill_haste: [15, 35],
      enemy_all_resist: 15,
      increased_life: -15,
      all_resistances: [15, 25],
    })
  })

  it("Skeleton Crew's Band ring lets summons call a Ghost Crew member", () => {
    const band = patchedItem('s10_skeleton_crews_band')
    expect(band.slot).toBe('ring_1')
    expect(band.grade).toBe('SS')
    expect(band.requiresLevel).toBe(100)
    expect(band.skillBonuses).toEqual({ 'Ghost Crew': [1, 3] })
    expect(band.procs).toBeUndefined()
    expect(band.implicit).toEqual({
      summon_skills: [2, 4],
      all_skills: 2,
      summon_attack_speed: [15, 25],
      summon_reduced_dmg_taken: 15,
      enemy_all_resist: 15,
    })
  })

  it('Blood Maggot Pendant amulet buffs guardians at the cost of life', () => {
    const pendant = patchedItem('s10_blood_maggot_pendant')
    expect(pendant.slot).toBe('amulet')
    expect(pendant.grade).toBe('SS')
    expect(pendant.requiresLevel).toBe(100)
    expect([pendant.sockets, pendant.maxSockets]).toEqual([1, 1])
    expect(pendant.implicit).toEqual({
      all_skills: [2, 3],
      increased_attack_speed: [25, 35],
      enemy_arcane_resist: 15,
      magic_skill_damage: [20, 35],
      guardian_additional_attack: 15,
      guardian_attack_range: [50, 75],
      increased_life: -15,
    })
  })

  it("Grimtide's Necklace amulet is a magic-find piece", () => {
    const necklace = patchedItem('s10_grimtides_necklace')
    expect(necklace.grade).toBe('SS')
    expect(necklace.requiresLevel).toBe(100)
    expect([necklace.sockets, necklace.maxSockets]).toEqual([1, 1])
    expect(necklace.implicit).toEqual({
      life: [200, 250],
      all_resistances: 15,
      gold_find: [35, 50],
      magic_find: [80, 200],
    })
  })

  it('Infected Grasp gloves convert poison resistance into poison skill damage', () => {
    const gloves = patchedItem('s10_infected_grasp')
    expect(gloves.slot).toBe('gloves')
    expect(gloves.grade).toBe('SS')
    expect(gloves.requiresLevel).toBe(100)
    expect([gloves.defenseMin, gloves.defenseMax]).toEqual([120, 120])
    expect(gloves.implicit).toEqual({
      enhanced_defense: [35, 50],
      all_skills: 2,
      poison_skills: [2, 5],
      poison_resistance_converted_to_poison_damage: [20, 40],
      faster_cast_rate: [20, 35],
      increased_attack_speed: [20, 35],
      extra_poison_dmg_to_poisoned: [15, 25],
      poison_skill_damage: [20, 30],
      increased_poisoned_frequency: 20,
      poison_resistance: [40, 60],
    })
  })

  it("Leviathan's Ribcage armor procs Warrior's Path on hit", () => {
    const ribcage = patchedItem('s10_leviathans_ribcage')
    expect(ribcage.slot).toBe('armor')
    expect(ribcage.grade).toBe('SS')
    expect(ribcage.requiresLevel).toBe(100)
    expect([ribcage.defenseMin, ribcage.defenseMax]).toEqual([230, 310])
    expect([ribcage.sockets, ribcage.maxSockets]).toEqual([4, 4])
    expect(ribcage.procs).toEqual([
      {
        trigger: 'on_hit',
        chance: 4,
        description: "cast Warrior's Path Level 13",
        details:
          'Increases your attack damage and magic skill damage for a short period. Magic Skill Damage 39%, Attack Damage 58.50%',
      },
    ])
    expect(ribcage.implicit).toEqual({
      enhanced_defense: [375, 430],
      all_skills: 3,
      melee_skills: [8, 12],
      ranged_skills: [8, 12],
      reduced_movement_diminish: 25,
      attack_rating: [1400, 1800],
      life_steal: [6, 10],
      mana_steal: [6, 12],
      projectile_speed: 15,
      to_strength: 50,
      increased_dexterity: 6,
      enhanced_damage_based_on_level: 165,
    })
  })

  it("Captain's Attire armor grants Oasis Aura", () => {
    const attire = patchedItem('s10_captains_attire')
    expect(attire.slot).toBe('armor')
    expect(attire.grade).toBe('SS')
    expect(attire.requiresLevel).toBe(100)
    expect([attire.defenseMin, attire.defenseMax]).toEqual([110, 140])
    expect([attire.sockets, attire.maxSockets]).toEqual([5, 6])
    expect(attire.skillBonuses).toEqual({ 'Oasis Aura': [25, 35] })
    expect(attire.implicit).toEqual({
      enhanced_defense: [270, 400],
      all_skills: 2,
      life: [450, 600],
      increased_life: 8,
      physical_damage_reduction: [8, 18],
      magic_damage_reduction: [8, 18],
      all_resistances: [20, 30],
    })
  })

  it("Phantom's Step boots grant Phantom Momentum and a random element skill bonus", () => {
    const boots = patchedItem('s10_phantoms_step')
    expect(boots.slot).toBe('boots')
    expect(boots.grade).toBe('SS')
    expect(boots.requiresLevel).toBe(100)
    expect([boots.defenseMin, boots.defenseMax]).toEqual([72, 90])
    expect([boots.sockets, boots.maxSockets]).toEqual([2, 2])
    expect(boots.skillBonuses).toEqual({ 'Phantom Momentum': 1 })
    expect(boots.uniqueEffects).toEqual(['Movement Phasing'])
    expect(boots.implicit).toEqual({
      enhanced_defense: [180, 225],
      all_skills: [2, 4],
      random_skill_element: [4, 5],
      movement_speed: [75, 100],
      faster_cast_rate: 25,
      increased_attack_speed: 25,
      faster_hit_recovery: 120,
    })
  })

  it("Ghostplunderer's Marchers boots are the explosion pair", () => {
    const boots = patchedItem('s10_ghostplunderers_marchers')
    expect(boots.grade).toBe('SS')
    expect(boots.requiresLevel).toBe(100)
    expect([boots.defenseMin, boots.defenseMax]).toEqual([72, 90])
    expect([boots.sockets, boots.maxSockets]).toEqual([2, 2])
    expect(boots.uniqueEffects).toEqual(['Movement Phasing'])
    expect(boots.implicit).toEqual({
      enhanced_defense: [180, 225],
      all_skills: 2,
      explosion_skills: [5, 8],
      movement_speed: 80,
      explosion_damage: [20, 30],
      explosion_aoe: [30, 50],
      all_resistances: 15,
    })
  })

  it("Leviathan's Crown helmet is the AoE caster helm", () => {
    const crown = patchedItem('s10_leviathans_crown')
    expect(crown.slot).toBe('helmet')
    expect(crown.grade).toBe('SS')
    expect(crown.requiresLevel).toBe(100)
    expect([crown.defenseMin, crown.defenseMax]).toEqual([90, 140])
    expect([crown.sockets, crown.maxSockets]).toEqual([3, 4])
    expect(crown.implicit).toEqual({
      enhanced_defense: [140, 240],
      all_skills: 3,
      aoe_skills: [4, 8],
      spell_aoe_radius: [25, 35],
      all_attributes: [20, 40],
      all_resistances: [20, 30],
    })
  })

  it("Parasite Queen's Tiara helmet enrages summons through Scarlet Sacrifice", () => {
    const tiara = patchedItem('s10_parasite_queens_tiara')
    expect(tiara.grade).toBe('SS')
    expect(tiara.requiresLevel).toBe(100)
    expect([tiara.defenseMin, tiara.defenseMax]).toEqual([90, 140])
    expect([tiara.sockets, tiara.maxSockets]).toEqual([2, 3])
    expect(tiara.skillBonuses).toEqual({ 'Scarlet Sacrifice': [1, 3] })
    expect(tiara.procs).toBeUndefined()
    expect(tiara.implicit).toEqual({
      enhanced_defense: [140, 240],
      summon_skills: [5, 8],
      summon_attack_speed: 25,
      summon_damage: [15, 30],
      summon_movement_speed: 20,
    })
  })

  it('Overgrowth shield procs Sanguine Flow when struck', () => {
    const shield = patchedItem('s10_overgrowth')
    expect(shield.slot).toBe('offhand')
    expect(shield.grade).toBe('SS')
    expect(shield.requiresLevel).toBe(100)
    expect([shield.defenseMin, shield.defenseMax]).toEqual([150, 180])
    expect(shield.blockChance).toBe(80)
    expect([shield.sockets, shield.maxSockets]).toEqual([4, 6])
    expect(shield.procs).toEqual([
      {
        trigger: 'when_struck',
        chance: 10,
        description: 'cast Sanguine Flow Level 1',
        details: 'Spawn life globes around you. Damage 20%',
      },
    ])
    expect(shield.implicit).toEqual({
      enhanced_defense: [540, 640],
      life: [350, 550],
      faster_hit_recovery: 150,
      physical_damage_reduction: 15,
      magic_damage_reduction: 15,
      all_resistances: [50, 75],
      max_all_resistances: -5,
    })
  })

  it("Leviathan's Spine cane procs Odin's Fury on cast", () => {
    const spine = patchedItem('s10_leviathans_spine')
    expect(spine.slot).toBe('weapon')
    expect(spine.twoHanded).toBe(true)
    expect(spine.grade).toBe('SS')
    expect(spine.requiresLevel).toBe(100)
    expect([spine.damageMin, spine.damageMax, spine.attackSpeed]).toEqual([32, 42, 1.15])
    expect([spine.sockets, spine.maxSockets]).toEqual([5, 6])
    expect(spine.procs).toEqual([
      {
        trigger: 'on_cast',
        chance: 40,
        description: "cast Odin's Fury Level 99",
        details:
          'Powerful warcry which deals arcane damage and stuns monsters around you. Arcane Damage 98320',
      },
    ])
    expect(spine.implicit).toEqual({
      all_skills: [8, 12],
      suppress_spell_hits: 25,
      faster_cast_rate: [70, 100],
      phys_dmg_taken_as_arcane: 25,
      enemy_all_resist: [25, 40],
      magic_damage_reduction: 25,
      all_resistances: [45, 60],
    })
  })

  it('Phantom Strike bow procs Arrow Rain on attack', () => {
    const bow = patchedItem('s10_phantom_strike')
    expect(bow.twoHanded).toBe(true)
    expect(bow.grade).toBe('SS')
    expect(bow.requiresLevel).toBe(100)
    expect([bow.damageMin, bow.damageMax, bow.attackSpeed]).toEqual([120, 130, 1.25])
    expect([bow.sockets, bow.maxSockets]).toEqual([4, 6])
    expect(bow.procs).toEqual([
      {
        trigger: 'on_attack',
        chance: 20,
        description: 'cast Arrow Rain Level 100',
        details:
          'Unleash a rain of arrows from the sky dealing increased damage. Attack Damage 2700%, Attack Rating 300%',
      },
    ])
    expect(bow.implicit).toEqual({
      enhanced_damage: [900, 1000],
      all_skills: [3, 5],
      increased_attack_speed: [25, 40],
      crit_chance: [20, 30],
      crit_damage: [40, 60],
      defense_ignored: 40,
      increased_dexterity: 8,
    })
  })

  it('Phantom Scimitar procs Phantom Slice both on kill and on attack', () => {
    const sword = patchedItem('s10_phantom_scimitar')
    expect(sword.twoHanded).toBe(true)
    expect(sword.grade).toBe('SS')
    expect(sword.requiresLevel).toBe(100)
    expect([sword.damageMin, sword.damageMax, sword.attackSpeed]).toEqual([125, 150, 1.2])
    expect([sword.sockets, sword.maxSockets]).toEqual([5, 5])
    expect(sword.uniqueEffects).toEqual(['Attacks can hit multiple enemies'])
    const slice =
      "Summon Phantom Leviathan's essence to strike monsters dealing area of effect damage. Damage 100%"
    expect(sword.procs).toEqual([
      { trigger: 'on_kill', chance: 5, description: 'cast Phantom Slice Level 1', details: slice },
      { trigger: 'on_attack', chance: 25, description: 'cast Phantom Slice Level 1', details: slice },
    ])
    expect(sword.implicit).toEqual({
      enhanced_damage: [825, 960],
      all_skills: 2,
      physical_skills: [3, 5],
      crit_chance: 35,
      increased_attack_rating: 10,
      attack_rating: 1250,
      crit_damage: [90, 125],
      to_strength: [50, 75],
    })
  })

  it('Conjured Tentacle wand grants Heart Surge and converts cold resistance', () => {
    const wand = patchedItem('s10_conjured_tentacle')
    expect(wand.twoHanded).toBeUndefined()
    expect(wand.grade).toBe('SS')
    expect(wand.requiresLevel).toBe(100)
    expect([wand.damageMin, wand.damageMax, wand.attackSpeed]).toEqual([32, 42, 1.5])
    expect([wand.sockets, wand.maxSockets]).toEqual([3, 3])
    expect(wand.skillBonuses).toEqual({ 'Heart Surge': [1, 4] })
    expect(wand.implicit).toEqual({
      all_skills: [2, 3],
      cold_skills: [2, 5],
      cold_resistance_converted_to_cold_damage: 15,
      faster_cast_rate: 50,
      extra_dmg_to_deep_frozen: [25, 40],
      flat_cold_skill_damage: [35, 55],
      cold_skill_damage: [55, 75],
      enemy_cold_resist: [30, 45],
    })
  })

  it("Grimtide's Scimitar procs Tides of Chaos on attack", () => {
    const sword = patchedItem('s10_grimtides_scimitar')
    expect(sword.twoHanded).toBeUndefined()
    expect(sword.grade).toBe('SS')
    expect(sword.requiresLevel).toBe(100)
    expect([sword.damageMin, sword.damageMax, sword.attackSpeed]).toEqual([68, 78, 1.5])
    expect([sword.sockets, sword.maxSockets]).toEqual([3, 3])
    expect(sword.procs).toEqual([
      {
        trigger: 'on_attack',
        chance: 15,
        description: 'cast Tides of Chaos Level 1',
        details:
          'Conjure tides of chaos sending forward ghastly waves and summoning torrents dealing damage and stunning monsters on hit. Damage 20%',
      },
    ])
    expect(sword.implicit).toEqual({
      enhanced_damage: [550, 680],
      all_skills: 2,
      increased_attack_speed: [45, 70],
      crit_chance: 25,
      crit_damage: 55,
      defense_ignored: 35,
      extra_damage_stunned: [25, 40],
      to_strength: [20, 30],
    })
  })

  it('Ethereal Musket gun grants Spectral Scatter and pays with cooldown skill damage', () => {
    const gun = patchedItem('s10_ethereal_musket')
    expect(gun.twoHanded).toBe(true)
    expect(gun.grade).toBe('SS')
    expect(gun.requiresLevel).toBe(100)
    expect([gun.damageMin, gun.damageMax, gun.attackSpeed]).toEqual([160, 175, 1])
    expect([gun.sockets, gun.maxSockets]).toEqual([6, 6])
    expect(gun.skillBonuses).toEqual({ 'Spectral Scatter': [1, 3] })
    expect(gun.uniqueEffects).toEqual(['Piercing Attack'])
    expect(gun.implicit).toEqual({
      enhanced_damage: [930, 1120],
      all_skills: [4, 6],
      melee_range: 40,
      increased_attack_speed: [15, 30],
      less_dmg_with_cd_skills: 80,
      crit_chance: 25,
      life_steal: 10,
      crit_damage: 66,
      defense_ignored: 50,
    })
  })

  it('S10 granted skills carry their descriptions', () => {
    const names = (patches.itemGrantedSkills?.add ?? []).map((e) => (e as Rec).name)
    for (const n of [
      'Will-O-Wisp',
      'Ghost Crew',
      'Phantom Momentum',
      'Scarlet Sacrifice',
      'Heart Surge',
      'Spectral Scatter',
    ]) {
      expect(names).toContain(n)
    }
  })

  it('Parasitic Heart charm trades life for damage', () => {
    const heart = patchedItem('s10_parasitic_heart')
    expect(heart.grade).toBe('SS')
    expect(heart.requiresLevel).toBe(100)
    expect([heart.width, heart.height]).toEqual([1, 2])
    expect(heart.implicit).toEqual({
      all_skills: [2, 4],
      attack_damage: [35, 70],
      magic_skill_damage: [35, 70],
      increased_life: -25,
    })
  })

  it('Skull Axe grants the Demon Form proc buff', () => {
    const axe = patchedItem('relic_relic_skull_axe')
    expect(axe.procs).toEqual([
      {
        trigger: 'on_attack',
        chance: 20,
        description: 'cast Demon Form Level [1-10]',
      },
    ])
    expect(axe.skillBonuses).toEqual({ 'Demon Form': [1, 10] })
    expect(axe.implicit).toEqual({})
  })

  it('Demon Form buff scales 24%+6%/lvl attack damage, 8%+2%/lvl attack speed', () => {
    const out = applyListPatch(
      itemGrantedSkillsJson as Rec[],
      patches.itemGrantedSkills,
      'item-granted-skills',
      'name',
    )
    const df = out.data.find((s) => (s as Rec).name === 'Demon Form') as Rec
    expect(df).toBeDefined()
    expect(df.condition).toBe('demon_form_buff')
    const ps = df.passiveStats as Rec
    expect(ps.base).toEqual({ attack_damage: 18, increased_attack_speed: 6 })
    expect(ps.perRank).toEqual({ attack_damage: 6, increased_attack_speed: 2 })
  })

  it("Fallen God's Bloodlust nerfs attack-speed-to-FCR conversion 10% -> 7%", () => {
    const out = applyListPatch(
      itemGrantedSkillsJson as Rec[],
      patches.itemGrantedSkills,
      'item-granted-skills',
      'name',
    )
    const skill = out.data.find(
      (s) => (s as Rec).name === "Fallen God's Bloodlust",
    ) as Rec
    const perRank = (skill.passiveConverts as Rec).perRank as Rec[]
    expect(perRank[0].pct).toBe(7)
  })
})

describe('S10 new items (scaffolding, no affixes yet)', () => {
  const NEW_ITEM_IDS = [
    's10_captains_anchor', 's10_ghastly_skull', 's10_grimtides_necklace',
    's10_skeleton_crews_band', 's10_ethereal_musket', 's10_grimtides_scimitar',
    's10_ghostplunderers_marchers', 's10_captains_attire', 's10_parasitic_heart',
    's10_parasite_queens_tiara', 's10_blood_maggot_pendant', 's10_conjured_tentacle',
    's10_overgrowth', 's10_infected_grasp', 's10_jar_of_parasites', 's10_parasite_loop',
    's10_ghost_armada', 's10_phantom_scimitar', 's10_leviathans_crown', 's10_phantom_strike',
    's10_leviathans_spine', 's10_leviathans_ribcage', 's10_phantoms_step', 's10_leviathans_blood',
  ]

  const resolvedItems = (): Rec[] =>
    applyListPatch(baseItems, patches.items, 'items').data as Rec[]

  it('adds every new item as a net-new id (no collision, all present)', () => {
    const baseIds = new Set(baseItems.map((i) => i.id))
    const patchedIds = new Set(resolvedItems().map((i) => i.id))
    for (const id of NEW_ITEM_IDS) {
      expect(baseIds.has(id)).toBe(false)
      expect(patchedIds.has(id)).toBe(true)
    }
  })

  it('routes new items to the right slots', () => {
    const items = resolvedItems()
    const byId = (id: string) => items.find((i) => i.id === id) as Rec
    expect(byId('s10_captains_anchor').slot).toBe('charm_1')
    expect(byId('s10_leviathans_blood').slot).toBe('potion_1')
    expect(byId('s10_overgrowth').slot).toBe('offhand')
    expect(byId('s10_phantom_scimitar').twoHanded).toBe(true)
    expect(byId('s10_phantom_strike').twoHanded).toBe(true)
  })

  it("adds Cthulhu's Soul Gem to the gems collection", () => {
    const gems = applyListPatch(gemsBase, patches.gems, 'gems').data as Rec[]
    const gem = gems.find((g) => g.id === 's10_cthulhus_soul_gem') as Rec
    expect(gem).toBeDefined()
    expect(gem.name).toBe("Cthulhu's Soul Gem")
  })
})
