import { describe, expect, it, vi } from 'vitest'
import type * as DataModule from '@data'
import { buildGearTooltip, serializeTooltipModel } from './gearTooltip'
import { getItem, getItemImage } from '@data'
import type { EquippedItem } from '../../types'
import type {
  ItemTooltipModel,
  TooltipLine,
  TooltipLineStyle,
  TooltipModelDeps,
  TooltipSectionModel,
} from '../../components/itemTooltipModel'
import type { GearTooltip, GearTooltipLine } from './sharePayload'

vi.mock('@data', async (importOriginal) => {
  const actual = await importOriginal<typeof DataModule>()
  return { ...actual, getItemImage: vi.fn((_id: string): string | undefined => undefined) }
})

function modelWith(overrides: Partial<ItemTooltipModel>): ItemTooltipModel {
  return {
    name: 'Test Item',
    tone: 'rare',
    typeLine: 'Rare · Ring',
    imageId: 'test_item',
    sections: [],
    ...overrides,
  }
}

type TextOut = Extract<GearTooltipLine, { kind: 'text' }>

function findText(out: GearTooltip, text: string): TextOut | undefined {
  for (const section of out.sections) {
    for (const line of section.lines) {
      if (line.kind === 'text' && line.text === text) return line
    }
  }
  return undefined
}

describe('serializeTooltipModel', () => {
  it('maps model styles to contract tones', () => {
    const styles: TooltipLineStyle[] = [
      'implicit',
      'runeword',
      'socket',
      'special',
      'affix',
      'unholy',
      'forged',
      'set-active',
      'set-inactive',
      'proc',
      'unsupported',
      'muted',
      'affix-missing',
      'unholy-missing',
    ]
    const lines: TooltipLine[] = styles.map((style) => ({ kind: 'text', text: style, style }))
    const out = serializeTooltipModel(modelWith({ sections: [{ lines }] }))

    const toneOf = (t: string) => findText(out, t)?.tone
    expect(toneOf('implicit')).toBe('gold')
    expect(toneOf('runeword')).toBe('gold')
    expect(toneOf('socket')).toBe('gold')
    expect(toneOf('special')).toBe('gold')
    expect(toneOf('affix')).toBe('yellow')
    expect(toneOf('unholy')).toBe('pink')
    expect(toneOf('forged')).toBe('red')
    expect(toneOf('set-active')).toBe('green')
    expect(toneOf('set-inactive')).toBe('muted')
    expect(toneOf('proc')).toBe('good')
    expect(toneOf('unsupported')).toBe('muted')
    expect(toneOf('muted')).toBe('muted')
    expect(toneOf('affix-missing')).toBe('yellow')
    expect(toneOf('unholy-missing')).toBe('pink')
    expect(findText(out, 'affix-missing')?.italic).toBe(true)
    expect(findText(out, 'unholy-missing')?.italic).toBe(true)
    expect(findText(out, 'affix')?.italic).toBeUndefined()
    expect(findText(out, 'implicit')?.italic).toBeUndefined()
  })

  it('passes header tones, trailing and footnote through unchanged', () => {
    const footnote = 'These mods are not yet calculated by the planner.'
    const sections: TooltipSectionModel[] = [
      { header: { text: 'Implicit', tone: 'gold' }, lines: [] },
      { header: { text: 'Granted Skill Effects', tone: 'orange' }, lines: [] },
      { header: { text: 'Forged · Satanic Crystal', tone: 'red' }, lines: [] },
      { header: { text: 'Unholy Affixes', tone: 'pink' }, lines: [] },
      { header: { text: 'Doomguard Regalia', tone: 'green', trailing: '3/5 pieces' }, lines: [] },
      { header: { text: 'Not Yet Supported', tone: 'muted' }, lines: [], footnote },
    ]
    const out = serializeTooltipModel(modelWith({ sections }))

    expect(out.sections.map((s) => s.header?.tone)).toEqual([
      'gold',
      'orange',
      'red',
      'pink',
      'green',
      'muted',
    ])
    expect(out.sections[4].header?.trailing).toBe('3/5 pieces')
    expect(out.sections[5].footnote).toBe(footnote)
    expect(out.sections[0].header).toEqual({ text: 'Implicit', tone: 'gold' })
    expect('footnote' in out.sections[0]).toBe(false)
  })

  it('emits image only when getItemImage(base.id) resolves', () => {
    vi.mocked(getItemImage).mockReturnValueOnce('/assets/game/stormlash.abc123.png')
    const withImage = serializeTooltipModel(modelWith({ imageId: 'stormlash' }))
    expect(withImage.image).toBe('items/stormlash.png')

    vi.mocked(getItemImage).mockReturnValueOnce(undefined)
    const withoutImage = serializeTooltipModel(modelWith({ imageId: 'stormlash' }))
    expect(withoutImage.image).toBeUndefined()
    expect('image' in withoutImage).toBe(false)
  })

  it('keeps section order from the model', () => {
    const sections: TooltipSectionModel[] = [
      { lines: [{ kind: 'row', label: 'Defense', value: '10–20' }] },
      { header: { text: 'Implicit', tone: 'gold' }, lines: [{ kind: 'text', text: 'A', style: 'implicit' }] },
      { header: { text: 'Unholy Affixes', tone: 'pink' }, lines: [{ kind: 'text', text: 'B', style: 'unholy' }] },
      { lines: [{ kind: 'text', text: 'C', style: 'muted' }] },
    ]
    const out = serializeTooltipModel(modelWith({ sections }))

    expect(out.sections).toHaveLength(4)
    expect(out.sections[0].lines[0]).toEqual({ kind: 'row', label: 'Defense', value: '10–20' })
    expect(out.sections[1].header?.text).toBe('Implicit')
    expect(out.sections[2].header?.text).toBe('Unholy Affixes')
    expect(out.sections[3].lines[0]).toEqual({ kind: 'text', text: 'C', tone: 'muted' })
  })

  it('serializes entry lines with suffix/desc/lines and badge marks', () => {
    const sections: TooltipSectionModel[] = [
      {
        header: { text: 'Granted Skill Effects', tone: 'orange' },
        lines: [
          {
            kind: 'entry',
            title: 'Frost Nova',
            suffix: 'rank 3',
            desc: 'Chills nearby foes',
            lines: ['+10 Cold Damage', '2s duration'],
          },
        ],
      },
      {
        header: { text: 'Doomguard Regalia', tone: 'green', trailing: '2/4 pieces' },
        lines: [{ kind: 'entry', title: '2-Set (active)', style: 'set-active', lines: ['+50 to Life'] }],
      },
      { lines: [{ kind: 'entry', title: 'Proc thing', style: 'proc', lines: [] }] },
      { lines: [{ kind: 'text', text: '+99 to Life', style: 'affix', badge: 'custom' }] },
    ]
    const out = serializeTooltipModel(modelWith({ sections }))

    expect(out.sections[0].lines[0]).toEqual({
      kind: 'entry',
      title: 'Frost Nova',
      suffix: 'rank 3',
      desc: 'Chills nearby foes',
      lines: ['+10 Cold Damage', '2s duration'],
    })
    expect(out.sections[1].lines[0]).toEqual({
      kind: 'entry',
      title: '2-Set (active)',
      tone: 'green',
      lines: ['+50 to Life'],
    })
    expect(out.sections[2].lines[0]).toEqual({ kind: 'entry', title: 'Proc thing', tone: 'good' })
    expect(out.sections[3].lines[0]).toEqual({
      kind: 'text',
      text: '+99 to Life',
      tone: 'yellow',
      badge: 'custom',
    })
  })
})

describe('buildGearTooltip', () => {
  it('wires buildItemTooltipModel through the serializer for a real item', () => {
    const base = getItem('boots_satanic_boots_of_wild')
    if (!base) throw new Error('fixture item missing from game data — pick a real id from src/data/items')
    const equipped: EquippedItem = {
      baseId: base.id,
      affixes: [],
      socketCount: 0,
      socketed: [],
      socketTypes: [],
    }
    const deps: TooltipModelDeps = {
      display: { implicitScaled: {}, skillRankScaled: {}, affixRanges: [] },
      inventory: {},
    }
    const tooltip = buildGearTooltip(base, equipped, deps)

    expect(tooltip.name).toBe(base.name)
    expect(tooltip.rarity).toBe(base.rarity)
    expect(tooltip.typeLine).toContain(base.baseType)
    expect(Array.isArray(tooltip.sections)).toBe(true)
  })
})
