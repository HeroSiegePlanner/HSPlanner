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
    const fx = patchedItem('claw_heroic_glacier_talons').uniqueEffects as string[]
    expect(fx[0]).toBe('4% Chance after each Kill to cast Blizzard Level 80')
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
