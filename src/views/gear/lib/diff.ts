import { computeBuildPerformanceAsync } from '../../../lib/calc/bridge'
import { gameConfig, getItem } from '../../../data'
import { rangedBounds, type BuildPerformance, type BuildPerformanceDeps } from '../../../utils/build/buildPerformance'
import type { EquippedItem, Inventory, ItemRarity, SlotKey } from '../../../types'

export interface BuildSummary extends BuildPerformance {
  itemBaseId: string | null
  itemName: string | null
  itemRarity: ItemRarity | null
  itemSockets: number
  itemSocketsMax: number
}

export type BuildSummaryDeps = Omit<BuildPerformanceDeps, 'inventory'>

export async function computeBuildSummary(
  inventory: Inventory,
  slot: SlotKey,
  deps: BuildSummaryDeps,
): Promise<BuildSummary> {
  const performance = await computeBuildPerformanceAsync({ ...deps, inventory })
  const equipped = inventory[slot]
  const base = equipped ? getItem(equipped.baseId) : null
  const baseSockets = base?.sockets ?? 0
  const baseSocketsMax = base?.maxSockets ?? baseSockets
  return {
    ...performance,
    itemBaseId: equipped?.baseId ?? null,
    itemName: base?.name ?? null,
    itemRarity: base?.rarity ?? null,
    itemSockets: equipped?.socketCount ?? baseSockets,
    itemSocketsMax: baseSocketsMax,
  }
}

type DiffKind = 'up' | 'down' | 'same' | 'new' | 'lost'

export interface StatDiff {
  key: string
  label: string
  beforeMin: number
  beforeMax: number
  afterMin: number
  afterMax: number
  delta: number
  kind: DiffKind
  unit?: 'pct'
}

function classifyDiff(before: number, after: number): DiffKind {
  const eps = 0.001
  const beforeZero = Math.abs(before) < eps
  const afterZero = Math.abs(after) < eps
  if (beforeZero && afterZero) return 'same'
  if (beforeZero && !afterZero) return 'new'
  if (!beforeZero && afterZero) return 'lost'
  if (Math.abs(after - before) < eps) return 'same'
  return after > before ? 'up' : 'down'
}

export const ITEM_AFFIX_KEYS: ReadonlyArray<{ key: string; label: string; unit?: 'pct' }> = [
  { key: 'defense', label: 'Defense' },
  { key: 'enhanced_defense', label: 'Enhanced Defense', unit: 'pct' },
  { key: 'all_skills', label: '+ All Skills' },
  { key: 'enhanced_damage', label: 'Enhanced Damage', unit: 'pct' },
]

export const BUILD_STAT_KEYS: ReadonlyArray<{ key: string; label: string; unit?: 'pct' }> = [
  { key: 'life', label: 'Life' },
  { key: 'mana', label: 'Mana' },
  { key: 'crit_chance', label: 'Crit Chance', unit: 'pct' },
  { key: 'crit_damage', label: 'Crit Damage', unit: 'pct' },
  { key: 'fire_resist', label: 'Fire Resist', unit: 'pct' },
  { key: 'cold_resist', label: 'Cold Resist', unit: 'pct' },
  { key: 'lightning_resist', label: 'Lightning Resist', unit: 'pct' },
  { key: 'poison_resist', label: 'Poison Resist', unit: 'pct' },
  { key: 'magic_find', label: 'Magic Find', unit: 'pct' },
  { key: 'gold_find', label: 'Gold Find', unit: 'pct' },
]

export function pickStatDiffsByKeys(
  before: BuildSummary,
  after: BuildSummary,
  keys: ReadonlyArray<{ key: string; label: string; unit?: 'pct' }>,
): StatDiff[] {
  const out: StatDiff[] = []
  for (const { key, label, unit } of keys) {
    const b = rangedBounds(before.stats[key])
    const a = rangedBounds(after.stats[key])
    const beforeAvg = (b.min + b.max) / 2
    const afterAvg = (a.min + a.max) / 2
    const kind = classifyDiff(beforeAvg, afterAvg)
    if (kind === 'same') continue
    out.push({
      key,
      label,
      beforeMin: b.min,
      beforeMax: b.max,
      afterMin: a.min,
      afterMax: a.max,
      delta: afterAvg - beforeAvg,
      kind,
      unit,
    })
  }
  return out
}

export function attrDiffs(before: BuildSummary, after: BuildSummary): StatDiff[] {
  const out: StatDiff[] = []
  for (const attr of gameConfig.attributes) {
    const b = rangedBounds(before.attributes[attr.key])
    const a = rangedBounds(after.attributes[attr.key])
    const beforeAvg = (b.min + b.max) / 2
    const afterAvg = (a.min + a.max) / 2
    const kind = classifyDiff(beforeAvg, afterAvg)
    if (kind === 'same') continue
    out.push({
      key: attr.key,
      label: attr.name,
      beforeMin: b.min,
      beforeMax: b.max,
      afterMin: a.min,
      afterMax: a.max,
      delta: afterAvg - beforeAvg,
      kind,
    })
  }
  return out
}

export function socketDiff(before: BuildSummary, after: BuildSummary): StatDiff | null {
  const kind = classifyDiff(before.itemSockets, after.itemSockets)
  if (kind === 'same') return null
  return {
    key: 'sockets',
    label: 'Sockets',
    beforeMin: before.itemSockets,
    beforeMax: before.itemSockets,
    afterMin: after.itemSockets,
    afterMax: after.itemSockets,
    delta: after.itemSockets - before.itemSockets,
    kind,
  }
}

export function hitDpsDiff(before: BuildSummary, after: BuildSummary): StatDiff | null {
  const beforeMin = before.hitDpsMin ?? 0
  const beforeMax = before.hitDpsMax ?? 0
  const afterMin = after.hitDpsMin ?? 0
  const afterMax = after.hitDpsMax ?? 0
  const beforeAvg = (beforeMin + beforeMax) / 2
  const afterAvg = (afterMin + afterMax) / 2
  const kind = classifyDiff(beforeAvg, afterAvg)
  if (
    kind === 'same' &&
    before.hitDpsMin === undefined &&
    after.hitDpsMin === undefined
  ) {
    return null
  }
  return {
    key: 'hit_dps',
    label: 'Hit DPS',
    beforeMin,
    beforeMax,
    afterMin,
    afterMax,
    delta: afterAvg - beforeAvg,
    kind,
  }
}

export function combinedDpsDiff(
  before: BuildSummary,
  after: BuildSummary,
): StatDiff | null {
  const beforeMin = before.combinedDpsMin ?? 0
  const beforeMax = before.combinedDpsMax ?? 0
  const afterMin = after.combinedDpsMin ?? 0
  const afterMax = after.combinedDpsMax ?? 0
  const beforeAvg = (beforeMin + beforeMax) / 2
  const afterAvg = (afterMin + afterMax) / 2
  const kind = classifyDiff(beforeAvg, afterAvg)
  if (
    kind === 'same' &&
    before.combinedDpsMin === undefined &&
    after.combinedDpsMin === undefined
  ) {
    return null
  }
  return {
    key: 'combined_dps',
    label: 'Combined DPS',
    beforeMin,
    beforeMax,
    afterMin,
    afterMax,
    delta: afterAvg - beforeAvg,
    kind,
  }
}

export function avgHitDiff(before: BuildSummary, after: BuildSummary): StatDiff | null {
  const beforeMin = before.damage !== null ? before.damage.avgMin : 0
  const beforeMax = before.damage !== null ? before.damage.avgMax : 0
  const afterMin = after.damage !== null ? after.damage.avgMin : 0
  const afterMax = after.damage !== null ? after.damage.avgMax : 0
  const beforeAvg = (beforeMin + beforeMax) / 2
  const afterAvg = (afterMin + afterMax) / 2
  const kind = classifyDiff(beforeAvg, afterAvg)
  if (kind === 'same' && before.damage === null && after.damage === null) {
    return null
  }
  return {
    key: 'avg_hit',
    label: 'Average Hit',
    beforeMin,
    beforeMax,
    afterMin,
    afterMax,
    delta: afterAvg - beforeAvg,
    kind,
  }
}

export type Verdict = 'upgrade' | 'downgrade' | 'sidegrade'

export function computeVerdict(
  before: BuildSummary,
  after: BuildSummary,
): Verdict {
  if (
    before.hitDpsMin !== undefined &&
    after.hitDpsMin !== undefined &&
    before.hitDpsMax !== undefined &&
    after.hitDpsMax !== undefined
  ) {
    const b = (before.hitDpsMin + before.hitDpsMax) / 2
    const a = (after.hitDpsMin + after.hitDpsMax) / 2
    if (b > 0) {
      const ratio = (a - b) / b
      if (ratio > 0.02) return 'upgrade'
      if (ratio < -0.02) return 'downgrade'
      return 'sidegrade'
    }
    if (a > b) return 'upgrade'
    if (a < b) return 'downgrade'
  }
  let netUp = 0
  let netDown = 0
  for (const d of [
    ...pickStatDiffsByKeys(before, after, ITEM_AFFIX_KEYS),
    ...pickStatDiffsByKeys(before, after, BUILD_STAT_KEYS),
    ...attrDiffs(before, after),
  ]) {
    if (d.kind === 'up' || d.kind === 'new') netUp += 1
    if (d.kind === 'down' || d.kind === 'lost') netDown += 1
  }
  if (netUp > netDown && netUp - netDown >= 2) return 'upgrade'
  if (netDown > netUp && netDown - netUp >= 2) return 'downgrade'
  return 'sidegrade'
}

function formatScalar(n: number, unit?: 'pct'): string {
  const abs = Math.abs(n)
  const rounded =
    Math.abs(n - Math.round(n)) < 0.05 ? Math.round(n) : Math.round(n * 10) / 10
  if (unit === 'pct') return `${rounded}%`
  if (abs >= 1000) {
    return `${(n / 1000).toFixed(abs >= 10000 ? 1 : 2)}k`
  }
  return `${rounded}`
}

export function formatStatNum(min: number, max: number, unit?: 'pct'): string {
  if (Math.abs(max - min) < 0.001) return formatScalar(min, unit)
  return `${formatScalar(min, unit)}-${formatScalar(max, unit)}`
}

export function formatDeltaNum(n: number, unit?: 'pct', kind?: DiffKind): string {
  if (kind === 'new') return 'new'
  if (kind === 'lost') return 'lost'
  if (kind === 'same') return '='
  const sign = n > 0 ? '+' : ''
  const abs = Math.abs(n)
  const rounded = Math.abs(n - Math.round(n)) < 0.05 ? Math.round(n) : Math.round(n * 10) / 10
  if (unit === 'pct') return `${sign}${rounded}%`
  if (abs >= 1000) {
    return `${sign}${(n / 1000).toFixed(n >= 10000 || n <= -10000 ? 1 : 2)}k`
  }
  return `${sign}${rounded}`
}

export function isSameEquipped(a: EquippedItem, b: EquippedItem): boolean {
  if (a.baseId !== b.baseId) return false
  if ((a.stars ?? 0) !== (b.stars ?? 0)) return false
  if (a.socketCount !== b.socketCount) return false
  const aSocketed = a.socketed ?? []
  const bSocketed = b.socketed ?? []
  if (aSocketed.length !== bSocketed.length) return false
  for (let i = 0; i < aSocketed.length; i++) {
    if (aSocketed[i] !== bSocketed[i]) return false
  }
  const aTypes = a.socketTypes ?? []
  const bTypes = b.socketTypes ?? []
  if (aTypes.length !== bTypes.length) return false
  for (let i = 0; i < aTypes.length; i++) {
    if (aTypes[i] !== bTypes[i]) return false
  }
  const aAffixes = a.affixes ?? []
  const bAffixes = b.affixes ?? []
  if (aAffixes.length !== bAffixes.length) return false
  for (let i = 0; i < aAffixes.length; i++) {
    if (
      aAffixes[i]?.affixId !== bAffixes[i]?.affixId ||
      aAffixes[i]?.tier !== bAffixes[i]?.tier ||
      aAffixes[i]?.roll !== bAffixes[i]?.roll
    ) {
      return false
    }
  }
  const aMods = a.forgedMods ?? []
  const bMods = b.forgedMods ?? []
  if (aMods.length !== bMods.length) return false
  for (let i = 0; i < aMods.length; i++) {
    if (
      aMods[i]?.affixId !== bMods[i]?.affixId ||
      aMods[i]?.tier !== bMods[i]?.tier
    ) {
      return false
    }
  }
  return (
    a.augment?.id === b.augment?.id && a.augment?.level === b.augment?.level
  )
}
