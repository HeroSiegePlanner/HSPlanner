import { getAffix, getItem } from '@data'
import type { Inventory, LootFilter } from '../../types'
import { statDef, statName } from '../item/stats'
import { createDefaultLootFilter } from './codec'
import { FILTER_STATS } from './constants'

export interface BuildAffixStat {
  statKey: string
  format: 'flat' | 'percent'
}

const STAT_ALIASES: Record<string, string | { flat: string; percent: string }> = {
  fire_skill_damage: {
    flat: 'Flat Fire Skill Damage',
    percent: 'Fire Skill Damage %',
  },
  cold_skill_damage: {
    flat: 'Flat Cold Skill Damage',
    percent: 'Cold Skill Damage %',
  },
  lightning_skill_damage: {
    flat: 'Flat Lightning Skill Damage',
    percent: 'Lightning Skill Damage %',
  },
  poison_skill_damage: {
    flat: 'Flat Poison Skill Damage',
    percent: 'Poison Skill Damage %',
  },
  arcane_skill_damage: {
    flat: 'Flat Arcane Skill Damage',
    percent: 'Arcane Skill Damage %',
  },
  additive_physical_damage: {
    flat: 'Flat Physical Damage',
    percent: 'Physical Damage %',
  },
  attack_rating_pct: 'Attack Rating %',
  all_damage_taken_reduced_pct: 'All damage reduction',
  damage_taken_reduced: 'Damage reduced',
  magic_damage_reduction_pct: 'Magic damage reduction',
  extra_damage_burning: 'Extra damage to Burning',
  life_replenish_pct: 'Life Replenish',
  mana_replenish_pct: 'Mana Replenish',
  ranged_range: 'Attack Range',
}

function normalize(name: string): string {
  return name
    .toLowerCase()
    .trim()
    .replace(/^(to|increased|extra)\s+/, '')
    .replace(/%/g, ' percent')
    .replace(/\bmaximum\b/g, 'max')
    .replace(/\bresists?\b/g, 'resistance')
    .replace(/\bdefence\b/g, 'defense')
    .replace(/\bper sec(ond)?\b/g, 'per second')
    .replace(/\bfreq(uency)?\b/g, 'frequency')
    .replace(/[^a-z0-9]+/g, ' ')
    .trim()
}

const FILTER_ID_BY_NAME: ReadonlyMap<string, number> = FILTER_STATS.reduce(
  (map, stat) => {
    const key = normalize(stat.name)
    if (!map.has(key)) map.set(key, stat.id)
    return map
  },
  new Map<string, number>(),
)

function resolveFilterStatId({ statKey, format }: BuildAffixStat): number | null {
  const alias = STAT_ALIASES[statKey]
  const aliasName = typeof alias === 'string' ? alias : alias?.[format]
  if (aliasName) return FILTER_ID_BY_NAME.get(normalize(aliasName)) ?? null

  const base = normalize(statName(statKey))
  const candidates = format === 'percent' ? [`${base} percent`, base] : [base]
  for (const candidate of candidates) {
    const id = FILTER_ID_BY_NAME.get(candidate)
    if (id !== undefined) return id
  }
  return null
}

export function filterStatIdsFor(stats: BuildAffixStat[]): number[] {
  const ids = new Set<number>()
  for (const stat of stats) {
    const id = resolveFilterStatId(stat)
    if (id !== null) ids.add(id)
  }
  return [...ids]
}

function catalogFormat(statKey: string): 'flat' | 'percent' {
  return statDef(statKey)?.format === 'percent' ? 'percent' : 'flat'
}

export function collectBuildStats(inventory: Inventory): BuildAffixStat[] {
  const seen = new Map<string, BuildAffixStat>()
  const add = (statKey: string, format: 'flat' | 'percent') => {
    seen.set(`${statKey}:${format}`, { statKey, format })
  }

  for (const equipped of Object.values(inventory)) {
    if (!equipped) continue
    for (const { affixId } of [...equipped.affixes, ...(equipped.forgedMods ?? [])]) {
      const affix = getAffix(affixId)
      if (affix?.statKey) add(affix.statKey, affix.format)
    }
    const base = getItem(equipped.baseId)
    for (const statKey of Object.keys(base?.implicit ?? {})) {
      add(statKey, catalogFormat(statKey))
    }
  }
  return [...seen.values()]
}

export interface BuildFilterOptions {
  hideRest?: boolean
}

export function buildFilterForStats(
  statIds: number[],
  { hideRest = false }: BuildFilterOptions = {},
): LootFilter {
  const highlighted = [...new Set(statIds)]
  const wanted = new Set(highlighted)
  const hidden =
    hideRest && highlighted.length > 0
      ? FILTER_STATS.filter((s) => !wanted.has(s.id)).map((s) => s.id)
      : []

  const base = createDefaultLootFilter()
  const types = Object.fromEntries(
    Object.entries(base.types).map(([id, type]) => [
      Number(id),
      {
        ...type,
        tiers: type.tiers.map((tier) => ({
          ...tier,
          hidden: [...hidden],
          highlighted: [...highlighted],
        })),
      },
    ]),
  )
  return { ...base, types }
}
