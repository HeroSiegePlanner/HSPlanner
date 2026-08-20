import { describe, expect, it } from 'vitest'
import { getItem, incarnationNodeInfo, items } from '@data'
import { canOffhand, isOffhandLocked, withValidOffhand } from './dualWield'

function nodeId(title: string): number {
  const entry = Object.entries(incarnationNodeInfo).find(([, i]) => i.t === title)
  if (!entry) throw new Error(`no incarnation node titled ${title}`)
  return Number(entry[0])
}

function base(baseType: string, twoHanded: boolean) {
  const item = items.find(
    (i) => i.baseType === baseType && !!i.twoHanded === twoHanded,
  )
  if (!item) throw new Error(`no ${twoHanded ? '2H' : '1H'} ${baseType}`)
  return item
}

const SWORD_1H = base('Sword', false)
const SWORD_2H = base('Sword', true)
const AXE_2H = base('Axe', true)
const BOW = base('Bow', true)
const GUN_1H = base('Gun', false)
const WAND = base('Wand', false)
const SHIELD = items.find((i) => i.baseType === 'Shield')!

const NONE = new Set<number>()
const GRIP = new Set([nodeId('Hercules Grip')])
const WANDS = new Set([nodeId('Master of Wands')])

describe('canOffhand — base game rules, no incarnation notes', () => {
  it('takes a second one-handed weapon', () => {
    expect(canOffhand(SWORD_1H, SWORD_1H, NONE)).toBe(true)
  })

  it('takes a shield next to a one-handed weapon', () => {
    expect(canOffhand(SHIELD, SWORD_1H, NONE)).toBe(true)
  })

  it('refuses a two-handed weapon in the offhand', () => {
    expect(canOffhand(SWORD_2H, SWORD_1H, NONE)).toBe(false)
  })

  it('refuses a wand without Master of Wands', () => {
    expect(canOffhand(WAND, WAND, NONE)).toBe(false)
  })

  it('locks the offhand behind any two-handed mainhand', () => {
    expect(isOffhandLocked(SWORD_2H, NONE)).toBe(true)
    expect(isOffhandLocked(SWORD_1H, NONE)).toBe(false)
    expect(isOffhandLocked(undefined, NONE)).toBe(false)
  })
})

describe('canOffhand — Hercules Grip', () => {
  it('frees the offhand under a two-handed sword', () => {
    expect(isOffhandLocked(SWORD_2H, GRIP)).toBe(false)
    expect(canOffhand(AXE_2H, SWORD_2H, GRIP)).toBe(true)
    expect(canOffhand(SWORD_1H, SWORD_2H, GRIP)).toBe(true)
  })

  it('takes a two-handed axe next to a one-handed sword', () => {
    expect(canOffhand(AXE_2H, SWORD_1H, GRIP)).toBe(true)
  })

  it('still blocks a shield under a two-handed weapon', () => {
    expect(canOffhand(SHIELD, SWORD_2H, GRIP)).toBe(false)
  })

  it('stays within swords, maces and axes once a hand is two-handed', () => {
    expect(canOffhand(GUN_1H, SWORD_2H, GRIP)).toBe(false)
    expect(canOffhand(GUN_1H, SWORD_1H, GRIP)).toBe(true)
  })

  it('does not cover two-handed weapons outside swords, maces and axes', () => {
    expect(isOffhandLocked(BOW, GRIP)).toBe(true)
    expect(canOffhand(BOW, SWORD_1H, GRIP)).toBe(false)
  })
})

describe('canOffhand — Master of Wands', () => {
  it('takes a second wand', () => {
    expect(canOffhand(WAND, WAND, WANDS)).toBe(true)
  })
})

describe('withValidOffhand', () => {
  const equipped = (baseId: string) => ({ baseId, socketed: [], affixes: [] })

  it('keeps a dual-wielded wand while the node is allocated', () => {
    const inv = { weapon: equipped(WAND.id), offhand: equipped(WAND.id) }
    expect(withValidOffhand(inv, WANDS)).toBe(inv)
  })

  it('drops the offhand wand once the node is gone', () => {
    const inv = { weapon: equipped(WAND.id), offhand: equipped(WAND.id) }
    expect(withValidOffhand(inv, NONE).offhand).toBeUndefined()
  })

  it('drops a shield when a two-handed weapon goes in the mainhand', () => {
    const inv = { weapon: equipped(SWORD_2H.id), offhand: equipped(SHIELD.id) }
    expect(withValidOffhand(inv, NONE).offhand).toBeUndefined()
  })

  it('keeps a second one-handed sword with no nodes at all', () => {
    const inv = { weapon: equipped(SWORD_1H.id), offhand: equipped(SWORD_1H.id) }
    expect(withValidOffhand(inv, NONE)).toBe(inv)
  })

  it('leaves an untouched offhand alone when the base is unknown', () => {
    const inv = { offhand: equipped('__nope__') }
    expect(withValidOffhand(inv, NONE)).toBe(inv)
    expect(getItem('__nope__')).toBeUndefined()
  })
})
