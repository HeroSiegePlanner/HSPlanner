import { affixes, getAffix, items } from '@data'
import { descriptionWithoutValue } from '../../../utils/item/itemTextShared'
import type { Affix } from '../../../types'

export interface AffixGroup {
  groupId: string
  tiers: Affix[]
  /** Description of one tier, with its range widened to cover every tier. */
  label: string
  /** Tier holding the group's strongest value; where a freshly added affix starts. */
  topTier: Affix
}

export interface TierRange {
  rangeMin: number
  rangeMax: number
}

// Item grades, reused for affix tiers. Only five groups reach past tier 5.
const TIER_GRADES = ['D', 'C', 'B', 'A', 'S']

export function affixTierLabel(tier: number): string {
  return TIER_GRADES[tier - 1] ?? `T${tier}`
}

function num(value: number): string {
  const abs = Math.abs(value)
  return String(Number.isInteger(abs) ? abs : Math.round(abs * 100) / 100)
}

// Descriptions embed their range verbatim, so swapping that token keeps the prose intact.
function rangeToken(min: number, max: number): string {
  return num(min) === num(max) ? num(min) : `[${num(min)}-${num(max)}]`
}

/** An affix description with its value removed, so the stat can be labelled on its own. */
export function affixStatLabel(affix: Affix): string {
  const stripped = descriptionWithoutValue(affix.description)
  if (stripped && stripped !== affix.description.trim()) return stripped
  // Some descriptions print their range mid-sentence, where the leading-value strip misses it.
  if (affix.valueMin !== null && affix.valueMax !== null) {
    const token = rangeToken(affix.valueMin, affix.valueMax)
    if (affix.description.includes(token)) {
      return affix.description
        .replace(token, '')
        .replace(/\s*%/, '')
        .replace(/\s+/g, ' ')
        .trim()
    }
  }
  return affix.description.trim()
}

/** Swaps the range printed in a description for one concrete value; null if it has none. */
export function describeAffixValue(affix: Affix, value: number): string | null {
  if (affix.valueMin === null || affix.valueMax === null) return null
  const token = rangeToken(affix.valueMin, affix.valueMax)
  return affix.description.includes(token)
    ? affix.description.replace(token, num(value))
    : null
}

function groupLabel(tiers: Affix[]): string {
  const rollable = tiers.filter((t) => t.valueMin !== null && t.valueMax !== null)
  const ref = rollable[0]
  if (!ref) return tiers[0]?.description ?? ''
  const lo = Math.min(...rollable.map((t) => Math.abs(t.valueMin!)))
  const hi = Math.max(...rollable.map((t) => Math.abs(t.valueMax!)))
  const own = rangeToken(ref.valueMin!, ref.valueMax!)
  const all = rangeToken(lo, hi)
  return ref.description.includes(own)
    ? ref.description.replace(own, all)
    : ref.description
}

function topTierOf(tiers: Affix[]): Affix {
  return tiers.reduce((best, t) =>
    (t.valueMax ?? -Infinity) > (best.valueMax ?? -Infinity) ? t : best,
  )
}

export function buildAffixGroups(list: Affix[]): AffixGroup[] {
  const byGroup = new Map<string, Affix[]>()
  for (const a of list) {
    const cur = byGroup.get(a.groupId)
    if (cur) cur.push(a)
    else byGroup.set(a.groupId, [a])
  }
  return [...byGroup].map(([groupId, members]) => {
    const tiers = members.slice().sort((a, b) => a.tier - b.tier)
    return { groupId, tiers, label: groupLabel(tiers), topTier: topTierOf(tiers) }
  })
}

// Unholy-style pools are one flat bag of unrelated stats, not a tier ladder, and
// only the bases that roll them can ever offer them.
const RANDOM_POOL_GROUP_IDS = new Set(
  items.map((i) => i.randomAffixGroupId).filter((id): id is string => !!id),
)

export function isRandomPoolAffix(affix: Affix): boolean {
  return RANDOM_POOL_GROUP_IDS.has(affix.groupId)
}

const TIERS_BY_GROUP = new Map<string, Affix[]>()
for (const a of affixes) {
  const cur = TIERS_BY_GROUP.get(a.groupId)
  if (cur) cur.push(a)
  else TIERS_BY_GROUP.set(a.groupId, [a])
}
for (const tiers of TIERS_BY_GROUP.values()) tiers.sort((a, b) => a.tier - b.tier)

export function affixTiers(affixId: string): Affix[] {
  const affix = getAffix(affixId)
  if (!affix) return []
  return TIERS_BY_GROUP.get(affix.groupId) ?? [affix]
}

function magnitudes(range: TierRange): [number, number] {
  const a = Math.abs(range.rangeMin)
  const b = Math.abs(range.rangeMax)
  return [Math.min(a, b), Math.max(a, b)]
}

/**
 * Magnitudes spanned by every rollable tier, or null when nothing in the group
 * rolls. Magnitudes, not signed values, so a bigger slider position always means
 * a stronger affix — negative affixes get stronger as they go down.
 */
export function groupBounds(
  tiers: Affix[],
  ranges: (TierRange | null)[],
): { lo: number; hi: number } | null {
  let lo = Infinity
  let hi = -Infinity
  tiers.forEach((tier, i) => {
    const range = ranges[i]
    if (!range || tier.valueMin === null || tier.valueMax === null) return
    const [min, max] = magnitudes(range)
    lo = Math.min(lo, min)
    hi = Math.max(hi, max)
  })
  return lo <= hi ? { lo, hi } : null
}

/**
 * Lowest tier that can roll `value` — the cheapest way to reach it. Tier ranges
 * overlap and sometimes leave gaps, so an unreachable value picks the nearest tier.
 */
export function tierIndexForValue(
  tiers: Affix[],
  ranges: (TierRange | null)[],
  value: number,
): number {
  const target = Math.abs(value)
  let nearest = -1
  let nearestDistance = Infinity
  for (let i = 0; i < tiers.length; i++) {
    const range = ranges[i]
    const tier = tiers[i]!
    if (!range || tier.valueMin === null || tier.valueMax === null) continue
    const [lo, hi] = magnitudes(range)
    if (target >= lo && target <= hi) return i
    const distance = target < lo ? lo - target : target - hi
    if (distance < nearestDistance) {
      nearestDistance = distance
      nearest = i
    }
  }
  return nearest
}
