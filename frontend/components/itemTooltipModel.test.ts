import { describe, expect, it } from 'vitest'
import { buildItemTooltipModel } from './itemTooltipModel'
import type { TooltipModelDeps } from './itemTooltipModel'
import {
  FORGE_KIND_LABEL,
  getAugment,
  getCrystalMod,
  getGem,
  getItem,
  getItemGrantedSkillByName,
  getItemSet,
  getRuneword,
} from '@data'
import { formatValue, statName } from '../utils/item/stats'
import { collectSocketGroups } from '../utils/item/socketStats'
import type { EquippedItem } from '../types'

function eq(baseId: string, over: Partial<EquippedItem> = {}): EquippedItem {
  return {
    baseId,
    affixes: [],
    socketCount: 0,
    socketed: [],
    socketTypes: [],
    ...over,
  }
}

const emptyDisplay = (): TooltipModelDeps['display'] => ({
  implicitScaled: {},
  skillRankScaled: {},
  affixRanges: [],
})

function deps(over: Partial<TooltipModelDeps> = {}): TooltipModelDeps {
  return { display: emptyDisplay(), inventory: {}, ...over }
}

describe('buildItemTooltipModel', () => {
  it('puts base stats first as row lines (Defense/Damage/Block/Attacks per sec)', () => {
    const base = getItem('sword_angelic_st_mika_s_zweih_nder')
    if (!base) throw new Error('fixture item missing from game data')
    const model = buildItemTooltipModel(base, eq(base.id), deps())
    expect(model.sections[0].header).toBeUndefined()
    expect(model.sections[0].lines).toEqual([
      { kind: 'row', label: 'Damage', value: `${base.damageMin}–${base.damageMax}` },
      { kind: 'row', label: 'Attacks / sec', value: `${base.attackSpeed}` },
    ])
  })

  it('renders implicit section with custom-override badge', () => {
    const base = getItem('boots_satanic_boots_of_wild')
    if (!base) throw new Error('fixture item missing from game data')
    expect(base.implicit).toHaveProperty('movement_speed')
    const model = buildItemTooltipModel(
      base,
      eq(base.id, { implicitOverrides: { movement_speed: 77 } }),
      deps(),
    )
    const impl = model.sections.find((s) => s.header?.text === 'Implicit')
    if (!impl) throw new Error('implicit section missing')
    expect(impl.header).toEqual({ text: 'Implicit', tone: 'gold' })
    expect(impl.lines).toContainEqual({
      kind: 'text',
      text: `${formatValue(77, 'movement_speed')} ${statName('movement_speed')}`,
      style: 'implicit',
      badge: 'custom',
    })
  })

  it('splits affixes into standard (affix) and Unholy Affixes section (unholy)', () => {
    const base = getItem('boots_satanic_boots_of_wild')
    if (!base) throw new Error('fixture item missing from game data')
    const model = buildItemTooltipModel(
      base,
      eq(base.id, {
        affixes: [
          { affixId: '15_30_to_life_t1_bear', tier: 1, roll: 0, customValue: 99 },
          { affixId: 'random_unholy_to_strength', tier: 1, roll: 0, customValue: 50 },
        ],
      }),
      deps(),
    )
    const standard = model.sections.find(
      (s) => !s.header && s.lines.some((l) => l.kind === 'text' && l.style === 'affix'),
    )
    if (!standard) throw new Error('standard affix section missing')
    const stdLine = standard.lines.find((l) => l.kind === 'text' && l.style === 'affix')
    expect(stdLine).toMatchObject({ kind: 'text', style: 'affix', badge: 'custom' })
    expect(stdLine && stdLine.kind === 'text' && stdLine.text).toMatch(/^\+99 .*Life$/)
    const unholy = model.sections.find((s) => s.header?.text === 'Unholy Affixes')
    if (!unholy) throw new Error('unholy affix section missing')
    expect(unholy.header).toEqual({ text: 'Unholy Affixes', tone: 'pink' })
    const unLine = unholy.lines.find((l) => l.kind === 'text' && l.style === 'unholy')
    expect(unLine).toMatchObject({ kind: 'text', style: 'unholy', badge: 'custom' })
    expect(unLine && unLine.kind === 'text' && unLine.text).toMatch(/^\+50 .*Strength$/)
  })

  it('builds granted skill entries with rank suffix, desc and computed lines', () => {
    const base = getItem('boots_heroic_pearlescent_dream')
    if (!base) throw new Error('fixture item missing from game data')
    const skill = getItemGrantedSkillByName('Holy Aura')
    if (!skill) throw new Error('granted skill Holy Aura missing from game data')
    const model = buildItemTooltipModel(
      base,
      eq(base.id),
      deps({
        display: {
          implicitScaled: {},
          skillRankScaled: { 'Holy Aura': [3, 3] },
          affixRanges: [],
        },
      }),
    )
    const section = model.sections.find((s) => s.header?.text === 'Granted Skill Effects')
    if (!section) throw new Error('granted skill section missing')
    expect(section.header).toEqual({ text: 'Granted Skill Effects', tone: 'orange' })
    const entry = section.lines.find((l) => l.kind === 'entry' && l.title === 'Holy Aura')
    if (!entry || entry.kind !== 'entry') throw new Error('Holy Aura entry missing')
    expect(entry).toMatchObject({
      kind: 'entry',
      title: 'Holy Aura',
      suffix: 'rank 3',
      desc: skill.description,
    })
    expect(entry.lines).toEqual([
      `${formatValue(6, 'attack_damage')} ${statName('attack_damage')}`,
      `${formatValue(7.5, 'magic_skill_damage')} ${statName('magic_skill_damage')}`,
    ])
  })

  it('adds the flat base to conversion lines (Radiant Power on the Mantle)', () => {
    const base = getItem('body_armor_unholy_grand_arch_wizard_s_mantle')
    if (!base) throw new Error('fixture item missing from game data')
    const model = buildItemTooltipModel(
      base,
      eq(base.id),
      deps({
        display: {
          implicitScaled: {},
          skillRankScaled: { 'Radiant Power': [5, 15] },
          affixRanges: [],
        },
      }),
    )
    const section = model.sections.find((s) => s.header?.text === 'Granted Skill Effects')
    const entry = section?.lines.find((l) => l.kind === 'entry' && l.title === 'Radiant Power')
    if (!entry || entry.kind !== 'entry') throw new Error('Radiant Power entry missing')
    expect(entry.suffix).toBe('rank 5-15')
    expect(entry.lines).toEqual([
      `0.95–1.45% of ${statName('mana')} added as ${statName('magic_skill_damage')}`,
    ])
  })

  it('uses runeword name, rare tone and runeword stat lines when detected', () => {
    const base = getItem('helmet_normal_cap')
    if (!base) throw new Error('fixture item missing from game data')
    const rw = getRuneword('rw_desert_s_wrath')
    if (!rw) throw new Error('runeword rw_desert_s_wrath missing from game data')
    const socketed = ['rune_nut', 'rune_pul', 'rune_old', 'rune_um']
    const model = buildItemTooltipModel(
      base,
      eq(base.id, {
        socketed,
        socketCount: 4,
        socketTypes: ['normal', 'normal', 'normal', 'normal'],
      }),
      deps(),
    )
    expect(model.name).toBe(rw.name)
    expect(model.tone).toBe('rare')
    expect(model.typeLine.startsWith('Runeword · ')).toBe(true)
    const section = model.sections.find((s) =>
      s.lines.some((l) => l.kind === 'text' && l.style === 'runeword'),
    )
    if (!section) throw new Error('runeword stat section missing')
    for (const [k, v] of Object.entries(rw.stats)) {
      expect(section.lines).toContainEqual({
        kind: 'text',
        text: `${formatValue(v as number, k)} ${statName(k)}`,
        style: 'runeword',
      })
    }
  })

  it('adds Forged section with FORGE_KIND_LABEL and forged style lines', () => {
    const base = getItem('boots_satanic_boots_of_wild')
    if (!base) throw new Error('fixture item missing from game data')
    const mod = getCrystalMod('crystal_satanic_to_strength')
    if (!mod) throw new Error('crystal mod missing from game data')
    const model = buildItemTooltipModel(
      base,
      eq(base.id, { forgedMods: [{ affixId: mod.id, tier: 1, roll: 1 }] }),
      deps(),
    )
    const section = model.sections.find((s) => s.header?.text?.startsWith('Forged · '))
    if (!section) throw new Error('forged section missing')
    expect(section.header).toEqual({
      text: `Forged · ${FORGE_KIND_LABEL.satanic_crystal}`,
      tone: 'red',
    })
    expect(section.lines).toContainEqual({
      kind: 'text',
      text: mod.description,
      style: 'forged',
    })
  })

  it('adds From Sockets lines and set section with pieces trailing and active flags', () => {
    const base = getItem('amulet_satanic_anubis_oculus')
    if (!base || !base.setId) throw new Error('fixture set item missing from game data')
    const set = getItemSet(base.setId)
    if (!set) throw new Error('item set missing from game data')
    const gem = getGem('gem_chipped_amethyst')
    if (!gem) throw new Error('gem missing from game data')
    const equipped = eq(base.id, {
      socketed: [gem.id],
      socketCount: 1,
      socketTypes: ['normal'],
    })
    const member = eq(base.id)
    const bonus = set.bonuses[0]

    const active = buildItemTooltipModel(
      base,
      equipped,
      deps({ inventory: { a: member, b: member, c: member } }),
    )
    const sockets = active.sections.find((s) => s.header?.text === 'From Sockets')
    if (!sockets) throw new Error('From Sockets section missing')
    expect(sockets.header).toEqual({ text: 'From Sockets', tone: 'gold' })
    expect(sockets.lines).toEqual(
      collectSocketGroups(equipped, base).map((group) => ({
        kind: 'entry',
        style: 'socket',
        title: group.name,
        icon: group.name,
        lines: group.stats.map(
          ([k, v]) => `${formatValue(v, k)} ${statName(k)}`,
        ),
      })),
    )
    const setActive = active.sections.find((s) => s.header?.text === set.name)
    if (!setActive) throw new Error('set section (active) missing')
    expect(setActive.header).toEqual({
      text: set.name,
      tone: 'green',
      trailing: `3/${set.items.length} pieces`,
    })
    expect(setActive.lines).toContainEqual({
      kind: 'entry',
      title: `${bonus.pieces}-Set (active)`,
      style: 'set-active',
      lines: bonus.descriptions ?? [],
    })
    expect(setActive.lines.at(-1)).toEqual({
      kind: 'entry',
      title: 'Set items',
      style: 'set-items',
      lines: set.items.map(
        (piece) =>
          `${piece.itemId === base.id ? '✓' : '·'} ${piece.name} (${piece.slot})`,
      ),
    })

    const inactive = buildItemTooltipModel(
      base,
      equipped,
      deps({ inventory: { a: member, b: member } }),
    )
    const setInactive = inactive.sections.find((s) => s.header?.text === set.name)
    if (!setInactive) throw new Error('set section (inactive) missing')
    expect(setInactive.header).toEqual({
      text: set.name,
      tone: 'green',
      trailing: `2/${set.items.length} pieces`,
    })
    const locked = set.bonuses.find((b) => b.pieces > 2)
    if (!locked) throw new Error('set has no bonus above 2 pieces')
    expect(setInactive.lines).toContainEqual({
      kind: 'entry',
      title: `${locked.pieces}-Set`,
      style: 'set-inactive',
      lines: locked.descriptions ?? [],
    })
  })

  it('adds procs, special effects and Not Yet Supported with footnote', () => {
    const stMika = getItem('sword_angelic_st_mika_s_zweih_nder')
    if (!stMika) throw new Error('fixture item missing from game data')
    if (!stMika.procs || stMika.procs.length === 0) throw new Error('fixture has no procs')
    const base = {
      ...stMika,
      uniqueEffects: ['Attacks can hit multiple enemies', 'Some Unsupported Mod'],
    }
    const model = buildItemTooltipModel(base, eq(base.id), deps())

    const proc0 = stMika.procs[0]
    const procSec = model.sections.find((s) =>
      s.lines.some((l) => l.kind === 'entry' && l.style === 'proc'),
    )
    if (!procSec) throw new Error('proc section missing')
    const procLine = procSec.lines.find((l) => l.kind === 'entry' && l.style === 'proc')
    if (!procLine || procLine.kind !== 'entry') throw new Error('proc entry missing')
    expect(procLine.title).toContain(proc0.description)
    expect(procLine.title).toContain(`${proc0.chance}%`)
    expect(procLine.desc).toBe(proc0.details)

    const special = model.sections.find((s) => s.header?.text === 'Special Effects')
    if (!special) throw new Error('special effects section missing')
    expect(special.header).toEqual({ text: 'Special Effects', tone: 'gold' })
    expect(special.lines).toContainEqual({
      kind: 'text',
      text: 'Attacks can hit multiple enemies',
      style: 'special',
    })

    const notSup = model.sections.find((s) => s.header?.text === 'Not Yet Supported')
    if (!notSup) throw new Error('not-yet-supported section missing')
    expect(notSup.header).toEqual({ text: 'Not Yet Supported', tone: 'muted' })
    expect(notSup.lines).toContainEqual({
      kind: 'text',
      text: 'Some Unsupported Mod',
      style: 'unsupported',
    })
    expect(notSup.footnote).toBe('These mods are not yet calculated by the planner.')
  })

  it('renders the Angelic Augment section with the stats of the selected level', () => {
    const base = getItem('boots_satanic_boots_of_wild')
    if (!base) throw new Error('fixture item missing from game data')
    const augment = getAugment('spell_slinger')
    if (!augment) throw new Error('fixture augment missing from game data')
    const level = 7
    const tier = augment.levels[level - 1]!
    const model = buildItemTooltipModel(
      base,
      eq(base.id, { augment: { id: augment.id, level } }),
      deps(),
    )
    const section = model.sections.find(
      (s) => s.header?.text === 'Angelic Augment',
    )
    if (!section) throw new Error('augment section missing')
    expect(section.lines).toEqual([
      {
        kind: 'entry',
        title: augment.name,
        icon: augment.id,
        suffix: `level ${level}`,
        desc: augment.description,
        lines: Object.entries(tier.stats).map(
          ([k, v]) => `${formatValue(v as number, k)} ${statName(k)}`,
        ),
      },
    ])
  })

  it('composes footer from Req Level, iLvl and Tier', () => {
    const boots = getItem('boots_satanic_boots_of_wild')
    if (!boots) throw new Error('fixture item missing from game data')
    expect(boots.requiresLevel).toBeDefined()
    expect(boots.grade).toBeDefined()
    const base = { ...boots, itemLevel: 60 }
    const model = buildItemTooltipModel(base, eq(base.id), deps())
    expect(model.footer).toBe(
      `Req Level ${boots.requiresLevel} · iLvl 60 · Tier ${boots.grade}`,
    )
  })
})
