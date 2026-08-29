import { getAffix, getAugment, getItem, getRuneword } from '@data'
import { MAX_STARS, maxSocketsFor } from '../../../store/itemRules'
import { AUGMENT_MAX_LEVEL } from '../../../types'
import type { EquippedItem, SkillElement, SocketType } from '../../../types'

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

export function withRandomSkill(
  item: EquippedItem,
  skillId: string | null,
): EquippedItem {
  if (skillId === null) {
    if (!item.randomSkillId) return item
    const { randomSkillId: _drop, ...rest } = item
    void _drop
    return rest
  }
  if (item.randomSkillId === skillId) return item
  return { ...item, randomSkillId: skillId }
}

export function withRandomElement(
  item: EquippedItem,
  element: SkillElement | null,
): EquippedItem {
  if (element === null) {
    if (!item.randomSkillElement) return item
    const { randomSkillElement: _drop, ...rest } = item
    void _drop
    return rest
  }
  if (item.randomSkillElement === element) return item
  return { ...item, randomSkillElement: element }
}

export function withSubskillBoost(
  item: EquippedItem,
  skillId: string | null,
): EquippedItem {
  if (skillId === null) {
    if (!item.subskillBoostSkillId) return item
    const { subskillBoostSkillId: _drop, ...rest } = item
    void _drop
    return rest
  }
  if (item.subskillBoostSkillId === skillId) return item
  return { ...item, subskillBoostSkillId: skillId }
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

export function withAffixRoll(
  item: EquippedItem,
  index: number,
  roll: number,
  affixId?: string,
): EquippedItem {
  if (index < 0 || index >= item.affixes.length) return item
  const clamped = Math.max(0, Math.min(1, roll))
  // A group slider spans every tier, so a big enough roll moves the affix to another tier.
  const tier = affixId ? getAffix(affixId)?.tier : undefined
  const affixes = item.affixes.map((a, i) => {
    if (i !== index) return a
    const { customValue: _drop, ...rest } = a
    void _drop
    return {
      ...rest,
      roll: clamped,
      ...(affixId && tier !== undefined ? { affixId, tier } : {}),
    }
  })
  return { ...item, affixes }
}

function withOverrideKey(
  item: EquippedItem,
  field: 'implicitOverrides' | 'skillBonusOverrides',
  key: string,
  value: number | null,
): EquippedItem {
  const cur = item[field] ?? {}
  if (value === null) {
    if (!(key in cur)) return item
    const { [key]: _drop, ...rest } = cur
    void _drop
    if (Object.keys(rest).length > 0) return { ...item, [field]: rest }
    const { [field]: _all, ...bare } = item
    void _all
    return bare
  }
  if (cur[key] === value) return item
  return { ...item, [field]: { ...cur, [key]: value } }
}

export function withImplicitOverride(
  item: EquippedItem,
  statKey: string,
  value: number | null,
): EquippedItem {
  return withOverrideKey(item, 'implicitOverrides', statKey, value)
}

export function withSkillBonusOverride(
  item: EquippedItem,
  skillName: string,
  value: number | null,
): EquippedItem {
  return withOverrideKey(item, 'skillBonusOverrides', skillName, value)
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
