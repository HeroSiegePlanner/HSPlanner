import { getAugment, getItem, getRuneword } from '@data'
import { MAX_STARS, maxSocketsFor } from '../../../store/itemRules'
import { AUGMENT_MAX_LEVEL } from '../../../types'
import type { EquippedItem, SocketType } from '../../../types'

export function makeEquippedItem(baseId: string): EquippedItem | null {
  const base = getItem(baseId)
  if (!base) return null
  const initial = Math.min(base.sockets ?? 0, maxSocketsFor(baseId))
  return {
    baseId,
    affixes: [],
    socketCount: initial,
    socketed: Array(initial).fill(null),
    socketTypes: Array(initial).fill('normal'),
    stars: 0,
    forgedMods: [],
  }
}

export function withSocketCount(item: EquippedItem, count: number): EquippedItem {
  const max = maxSocketsFor(item.baseId, item.forgedMods)
  const clamped = Math.max(0, Math.min(max, count))
  const socketed = [...item.socketed]
  const socketTypes = [...item.socketTypes]
  while (socketed.length < clamped) {
    socketed.push(null)
    socketTypes.push('normal')
  }
  socketed.length = clamped
  socketTypes.length = clamped
  return { ...item, socketCount: clamped, socketed, socketTypes }
}

export function withSocketed(
  item: EquippedItem,
  idx: number,
  socketableId: string | null,
): EquippedItem {
  if (idx < 0 || idx >= item.socketCount) return item
  const socketed = [...item.socketed]
  socketed[idx] = socketableId
  return { ...item, socketed }
}

export function withSocketType(
  item: EquippedItem,
  idx: number,
  type: SocketType,
): EquippedItem {
  if (idx < 0 || idx >= item.socketCount) return item
  const socketTypes = [...item.socketTypes]
  socketTypes[idx] = type
  return { ...item, socketTypes }
}

export function withStars(item: EquippedItem, count: number): EquippedItem {
  const clamped = Math.max(0, Math.min(MAX_STARS, Math.floor(count)))
  if ((item.stars ?? 0) === clamped) return item
  return { ...item, stars: clamped }
}

export function withAffixAdded(
  item: EquippedItem,
  affixId: string,
  tier: number,
): EquippedItem {
  const base = getItem(item.baseId)
  if (base?.maxAffixes !== undefined && item.affixes.length >= base.maxAffixes) {
    return item
  }
  return { ...item, affixes: [...item.affixes, { affixId, tier, roll: 1 }] }
}

export function withAffixRemoved(item: EquippedItem, index: number): EquippedItem {
  if (index < 0 || index >= item.affixes.length) return item
  return { ...item, affixes: item.affixes.filter((_, i) => i !== index) }
}

export function withForgedModAdded(
  item: EquippedItem,
  modId: string,
  tier: number,
): EquippedItem {
  const forgedMods = [{ affixId: modId, tier, roll: 1 }]
  const newMax = maxSocketsFor(item.baseId, forgedMods)
  const socketCount = Math.min(item.socketCount, newMax)
  return {
    ...item,
    forgedMods,
    socketCount,
    socketed: item.socketed.slice(0, socketCount),
    socketTypes: item.socketTypes.slice(0, socketCount),
  }
}

export function withForgedModRemoved(item: EquippedItem, index: number): EquippedItem {
  const list = item.forgedMods ?? []
  if (index < 0 || index >= list.length) return item
  const forgedMods = list.filter((_, i) => i !== index)
  const newMax = maxSocketsFor(item.baseId, forgedMods)
  const socketCount = Math.min(item.socketCount, newMax)
  return {
    ...item,
    forgedMods,
    socketCount,
    socketed: item.socketed.slice(0, socketCount),
    socketTypes: item.socketTypes.slice(0, socketCount),
  }
}

export function withRuneword(item: EquippedItem, runewordId: string): EquippedItem {
  const base = getItem(item.baseId)
  const rw = getRuneword(runewordId)
  if (!base || !rw) return item
  if (base.rarity !== 'common') return item
  if (!rw.allowedBaseTypes.includes(base.baseType)) return item
  const cap = maxSocketsFor(item.baseId)
  if (rw.runes.length > cap) return item
  const socketed: (string | null)[] = [...rw.runes]
  const socketTypes = item.socketTypes.slice(0, rw.runes.length)
  while (socketTypes.length < rw.runes.length) socketTypes.push('normal')
  return { ...item, socketCount: rw.runes.length, socketed, socketTypes }
}

export function withAugment(
  item: EquippedItem,
  augmentId: string | null,
): EquippedItem {
  if (augmentId === null) {
    if (!item.augment) return item
    const { augment: _drop, ...rest } = item
    void _drop
    return rest
  }
  if (!getAugment(augmentId)) return item
  const level = item.augment?.id === augmentId ? item.augment.level : 1
  return { ...item, augment: { id: augmentId, level } }
}

export function withAugmentLevel(item: EquippedItem, level: number): EquippedItem {
  if (!item.augment) return item
  const clamped = Math.max(1, Math.min(AUGMENT_MAX_LEVEL, Math.round(level)))
  if (clamped === item.augment.level) return item
  return { ...item, augment: { ...item.augment, level: clamped } }
}
