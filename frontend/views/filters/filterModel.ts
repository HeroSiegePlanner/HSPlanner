import type { LootFilter, LootFilterTier, LootFilterType } from '../../types'
import {
  DEFAULT_RS,
  DEFAULT_SOC,
  DEFAULT_SOCH,
  DEFAULT_WTC,
} from '../../utils/lootfilter/codec'
import { FILTER_STATS } from '../../utils/lootfilter/constants'

export type CellState = 'hidden' | 'visible' | 'highlighted'

const RARITY_INDEXES = [0, 1, 2, 3, 4] as const

export function isRarityVisible(rs: number, rarity: number): boolean {
  return (rs & (1 << (5 + rarity))) !== 0
}

function isRarityHighlighted(rs: number, rarity: number): boolean {
  return (rs & (1 << rarity)) !== 0
}

export function rarityCellState(rs: number, rarity: number): CellState {
  if (!isRarityVisible(rs, rarity)) return 'hidden'
  return isRarityHighlighted(rs, rarity) ? 'highlighted' : 'visible'
}

function withType(
  filter: LootFilter,
  typeId: number,
  update: (type: LootFilterType) => LootFilterType,
): LootFilter {
  const type = filter.types[typeId]
  if (!type) return filter
  return { ...filter, types: { ...filter.types, [typeId]: update(type) } }
}

function withTier(
  filter: LootFilter,
  typeId: number,
  tier: number,
  update: (tier: LootFilterTier) => LootFilterTier,
): LootFilter {
  return withType(filter, typeId, (type) => ({
    ...type,
    tiers: type.tiers.map((t, i) => (i === tier ? update(t) : t)),
  }))
}

export function toggleRarityVisible(
  filter: LootFilter,
  typeId: number,
  tier: number,
  rarity: number,
): LootFilter {
  return withTier(filter, typeId, tier, (t) => {
    const visible = isRarityVisible(t.rs, rarity)
    const rs = visible
      ? t.rs & ~(1 << (5 + rarity)) & ~(1 << rarity)
      : t.rs | (1 << (5 + rarity))
    return { ...t, rs }
  })
}

export function toggleRarityHighlight(
  filter: LootFilter,
  typeId: number,
  tier: number,
  rarity: number,
): LootFilter {
  return withTier(filter, typeId, tier, (t) => {
    if (!isRarityVisible(t.rs, rarity)) return t
    return { ...t, rs: t.rs ^ (1 << rarity) }
  })
}

function setRarity(rs: number, rarity: number, visible: boolean): number {
  return visible
    ? rs | (1 << (5 + rarity))
    : rs & ~(1 << (5 + rarity)) & ~(1 << rarity)
}

export function toggleRarityRow(
  filter: LootFilter,
  typeId: number,
  rarity: number,
): LootFilter {
  const type = filter.types[typeId]
  if (!type) return filter
  const hide = type.tiers.some((t) => isRarityVisible(t.rs, rarity))
  return withType(filter, typeId, (t) => ({
    ...t,
    tiers: t.tiers.map((tier) => ({
      ...tier,
      rs: setRarity(tier.rs, rarity, !hide),
    })),
  }))
}

export function toggleRarityTier(
  filter: LootFilter,
  typeId: number,
  tier: number,
): LootFilter {
  const current = filter.types[typeId]?.tiers[tier]
  if (!current) return filter
  const hide = RARITY_INDEXES.some((r) => isRarityVisible(current.rs, r))
  return withTier(filter, typeId, tier, (t) => ({
    ...t,
    rs: RARITY_INDEXES.reduce((rs, r) => setRarity(rs, r, !hide), t.rs),
  }))
}

export function affixCellState(
  type: LootFilterType,
  tier: number,
  statId: number,
): CellState {
  const t = type.tiers[tier]!
  if (t.hidden.includes(statId)) return 'hidden'
  return t.highlighted.includes(statId) ? 'highlighted' : 'visible'
}

export function toggleAffixVisible(
  filter: LootFilter,
  typeId: number,
  tier: number,
  statId: number,
): LootFilter {
  return withTier(filter, typeId, tier, (t) => {
    if (t.hidden.includes(statId)) {
      return { ...t, hidden: t.hidden.filter((id) => id !== statId) }
    }
    return {
      ...t,
      hidden: [...t.hidden, statId],
      highlighted: t.highlighted.filter((id) => id !== statId),
    }
  })
}

export function toggleAffixHighlight(
  filter: LootFilter,
  typeId: number,
  tier: number,
  statId: number,
): LootFilter {
  return withTier(filter, typeId, tier, (t) => {
    if (t.hidden.includes(statId)) return t
    return {
      ...t,
      highlighted: t.highlighted.includes(statId)
        ? t.highlighted.filter((id) => id !== statId)
        : [...t.highlighted, statId],
    }
  })
}

export function isAffixRowAnyVisible(type: LootFilterType, statId: number): boolean {
  return type.tiers.some((t) => !t.hidden.includes(statId))
}

export function toggleAffixRow(
  filter: LootFilter,
  typeId: number,
  statId: number,
): LootFilter {
  return withType(filter, typeId, (type) => {
    const hide = isAffixRowAnyVisible(type, statId)
    return {
      ...type,
      tiers: type.tiers.map((t) => {
        const hidden = t.hidden.filter((id) => id !== statId)
        return hide
          ? {
              ...t,
              hidden: [...hidden, statId],
              highlighted: t.highlighted.filter((id) => id !== statId),
            }
          : { ...t, hidden }
      }),
    }
  })
}

export function toggleTierColumn(
  filter: LootFilter,
  typeId: number,
  tier: number,
  statIds: number[] = FILTER_STATS.map((s) => s.id),
): LootFilter {
  return withTier(filter, typeId, tier, (t) => {
    const ids = new Set(statIds)
    const hide = statIds.some((id) => !t.hidden.includes(id))
    if (hide) {
      return {
        ...t,
        hidden: [...new Set([...t.hidden, ...statIds])],
        highlighted: t.highlighted.filter((id) => !ids.has(id)),
      }
    }
    return { ...t, hidden: t.hidden.filter((id) => !ids.has(id)) }
  })
}

export function isAffixRowEdited(type: LootFilterType, statId: number): boolean {
  return type.tiers.some(
    (t) => t.hidden.includes(statId) || t.highlighted.includes(statId),
  )
}

export function setAffixesVisible(
  filter: LootFilter,
  typeId: number,
  statIds: number[],
  visible: boolean,
): LootFilter {
  const ids = new Set(statIds)
  return withType(filter, typeId, (type) => ({
    ...type,
    tiers: type.tiers.map((t) =>
      visible
        ? { ...t, hidden: t.hidden.filter((id) => !ids.has(id)) }
        : {
            ...t,
            hidden: [...new Set([...t.hidden, ...statIds])],
            highlighted: t.highlighted.filter((id) => !ids.has(id)),
          },
    ),
  }))
}

export interface TypeSummary {
  hidden: Set<number>
  highlighted: Set<number>
  raritiesHidden: number
  edited: boolean
}

export function typeSummary(type: LootFilterType): TypeSummary {
  const hidden = new Set<number>()
  const highlighted = new Set<number>()
  let raritiesHidden = 0
  let rsEdited = false
  for (const tier of type.tiers) {
    for (const id of tier.hidden) hidden.add(id)
    for (const id of tier.highlighted) highlighted.add(id)
    if (tier.rs !== DEFAULT_RS) rsEdited = true
    for (const r of RARITY_INDEXES) {
      if (!isRarityVisible(tier.rs, r)) raritiesHidden += 1
    }
  }
  const socketsEdited = type.soc !== DEFAULT_SOC || type.soch !== DEFAULT_SOCH
  return {
    hidden,
    highlighted,
    raritiesHidden,
    edited: hidden.size > 0 || highlighted.size > 0 || rsEdited || socketsEdited,
  }
}

export interface FilterSummary {
  editedTypes: number
  hiddenStats: number
  highlightedStats: number
}

export function filterSummary(filter: LootFilter): FilterSummary {
  const hidden = new Set<number>()
  const highlighted = new Set<number>()
  let editedTypes = 0
  for (const type of Object.values(filter.types)) {
    const summary = typeSummary(type)
    if (summary.edited) editedTypes += 1
    for (const id of summary.hidden) hidden.add(id)
    for (const id of summary.highlighted) highlighted.add(id)
  }
  if (filter.wtc !== DEFAULT_WTC) editedTypes += 1
  return {
    editedTypes,
    hiddenStats: hidden.size,
    highlightedStats: highlighted.size,
  }
}

export function socketCellState(type: LootFilterType, socket: number): CellState {
  if ((type.soc & (1 << socket)) === 0) return 'hidden'
  return (type.soch & (1 << socket)) !== 0 ? 'highlighted' : 'visible'
}

export function toggleSocketVisible(
  filter: LootFilter,
  typeId: number,
  socket: number,
): LootFilter {
  return withType(filter, typeId, (type) => {
    const visible = (type.soc & (1 << socket)) !== 0
    return visible
      ? {
          ...type,
          soc: type.soc & ~(1 << socket),
          soch: type.soch & ~(1 << socket),
        }
      : { ...type, soc: type.soc | (1 << socket) }
  })
}

export function toggleSocketHighlight(
  filter: LootFilter,
  typeId: number,
  socket: number,
): LootFilter {
  return withType(filter, typeId, (type) => {
    if ((type.soc & (1 << socket)) === 0) return type
    return { ...type, soch: type.soch ^ (1 << socket) }
  })
}

export function isWeaponTypeEnabled(filter: LootFilter, bit: number): boolean {
  return (filter.wtc & (1 << bit)) !== 0
}

export function toggleWeaponType(filter: LootFilter, bit: number): LootFilter {
  return { ...filter, wtc: filter.wtc ^ (1 << bit) }
}

export function copyTypeConfig(
  filter: LootFilter,
  fromTypeId: number,
  toTypeIds: number[],
): LootFilter {
  const src = filter.types[fromTypeId]
  if (!src) return filter
  const types = { ...filter.types }
  for (const id of toTypeIds) {
    if (id === fromTypeId || !types[id]) continue
    types[id] = {
      soc: src.soc,
      soch: src.soch,
      tiers: src.tiers.map((t) => ({
        rs: t.rs,
        hidden: [...t.hidden],
        highlighted: [...t.highlighted],
      })),
    }
  }
  return { ...filter, types }
}
