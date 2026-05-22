import { gameConfig, getItem, isGearSlot } from '../data'
import type { ForgeKind } from '../data'
import {
  itemGrantedSkillRankFlatBonus,
  statStarFlatBonus,
  statStarPercentMultiplier,
} from './starScaling'
import type {
  AttributeKey,
  Inventory,
  RangedStatMap,
  RangedValue,
  Skill,
  StatDef,
} from '../types'
import { ELEMENTS } from './treeStats'

export type SourceType =
  | 'class'
  | 'allocated'
  | 'level'
  | 'attribute'
  | 'item'
  | 'socket'
  | 'skill'
  | 'subskill'
  | 'custom'
  | 'tree'

export function normalizeSkillName(name: string): string {
  return name.trim().toLowerCase()
}

const STAT_DEFS_MAP = new Map<string, StatDef>(
  gameConfig.stats.map((s) => [s.key, s]),
)

export interface SourceContribution {
  label: string
  sourceType: SourceType
  value: RangedValue
  forge?: {
    itemName: string
    modName: string
    kind: ForgeKind
  }
}

export interface ComputedStats {
  attributes: Record<AttributeKey, RangedValue>
  stats: RangedStatMap
  attributeSources: Record<AttributeKey, SourceContribution[]>
  statSources: Record<string, SourceContribution[]>
}

export function combineAdditiveAndMore(
  additive: RangedValue | undefined,
  more: RangedValue | undefined,
): RangedValue {
  const [aMin, aMax] =
    additive === undefined
      ? [0, 0]
      : typeof additive === 'number'
        ? [additive, additive]
        : additive
  const [mMin, mMax] =
    more === undefined
      ? [0, 0]
      : typeof more === 'number'
        ? [more, more]
        : more
  const round = (n: number) => Math.round(n * 1e6) / 1e6
  const min = round(((1 + aMin / 100) * (1 + mMin / 100) - 1) * 100)
  const max = round(((1 + aMax / 100) * (1 + mMax / 100) - 1) * 100)
  return min === max ? min : [min, max]
}

export function statName(key: string): string {
  const direct = STAT_DEFS_MAP.get(key)?.name
  if (direct) return direct
  if (key.endsWith('_more')) {
    const base = key.slice(0, -'_more'.length)
    const baseName = STAT_DEFS_MAP.get(base)?.name
    if (baseName) return `Total ${baseName}`
  }
  return key
}

export function statDef(key: string): StatDef | undefined {
  const direct = STAT_DEFS_MAP.get(key)
  if (direct) return direct
  if (key.endsWith('_more')) {
    const base = STAT_DEFS_MAP.get(key.slice(0, -'_more'.length))
    if (base) {
      return {
        ...base,
        key,
        name: `Total ${base.name}`,
        format: 'percent',
      }
    }
  }
  return undefined
}

export function effectiveCap(
  statKey: string,
  stats: RangedStatMap,
): number | undefined {
  const baseCap = statDef(statKey)?.cap
  if (baseCap === undefined) return undefined
  const mod = stats[`max_${statKey}`]
  if (mod === undefined) return baseCap
  return baseCap + rangedMax(mod)
}

export function isZero(v: RangedValue): boolean {
  if (typeof v === 'number') return v === 0
  return v[0] === 0 && v[1] === 0
}

export function rangedMin(v: RangedValue): number {
  return typeof v === 'number' ? v : v[0]
}

export function rangedMax(v: RangedValue): number {
  return typeof v === 'number' ? v : v[1]
}

export function formatValue(value: RangedValue, key: string): string {
  const def = statDef(key)
  const suffix = def?.format === 'percent' ? '%' : ''
  if (typeof value === 'number') {
    const sign = value >= 0 ? '+' : ''
    const num = Number.isInteger(value)
      ? value
      : Math.round(value * 100) / 100
    return `${sign}${num}${suffix}`
  }
  const [min, max] = value
  if (min === max) return formatValue(min, key)
  const sign = min >= 0 ? '+' : ''
  const minF = Number.isInteger(min) ? min : Math.round(min * 100) / 100
  const maxF = Number.isInteger(max) ? max : Math.round(max * 100) / 100
  return `${sign}[${minF}-${maxF}]${suffix}`
}

export function fmtStats(stats: RangedStatMap): string {
  return Object.entries(stats)
    .map(([k, v]) => `${formatValue(v, k)} ${statName(k)}`)
    .join(', ')
}

export interface SkillDamageBreakdown {
  effectiveRankMin: number
  effectiveRankMax: number
  baseMin: number
  baseMax: number
  flatMin: number
  flatMax: number
  synergyMinPct: number
  synergyMaxPct: number
  skillDamageMinPct: number
  skillDamageMaxPct: number
  extraDamagePct: number
  extraDamageSources: Array<{ label: string; pct: number }>
  critChance: number
  critDamagePct: number
  critMultiplierAvg: number
  multicastChancePct: number
  multicastMultiplier: number
  projectileCount: number
  elementalBreakPct: number
  elementalBreakMultiplier: number
  enemyResistancePct: number
  resistanceIgnoredPct: number
  effectiveResistancePct: number
  resistanceMultiplier: number
  hitMin: number
  hitMax: number
  critMin: number
  critMax: number
  finalMin: number
  finalMax: number
  avgMin: number
  avgMax: number
}

export interface AttackSkillDamageBreakdown {
  effectiveRankMin: number
  effectiveRankMax: number
  weaponDamagePctMin: number
  weaponDamagePctMax: number
  skillFlatPhysMin: number
  skillFlatPhysMax: number
  attackRatingPctMin: number
  attackRatingPctMax: number
  physicalHitMin: number
  physicalHitMax: number
  physicalAvgMin: number
  physicalAvgMax: number
  poisonHitMin: number
  poisonHitMax: number
  poisonAvgMin: number
  poisonAvgMax: number
  combinedHitMin: number
  combinedHitMax: number
  combinedAvgMin: number
  combinedAvgMax: number
  attacksPerSecondMin: number
  attacksPerSecondMax: number
  dpsMin: number
  dpsMax: number
}

export interface WeaponDamageBreakdown {
  hasWeapon: boolean
  weaponName?: string
  weaponDamageMin: number
  weaponDamageMax: number
  enhancedDamageMinPct: number
  enhancedDamageMaxPct: number
  additivePhysicalMin: number
  additivePhysicalMax: number
  attackDamageMinPct: number
  attackDamageMaxPct: number
  extraDamagePct: number
  extraDamageSources: Array<{ label: string; pct: number }>
  critChance: number
  critDamagePct: number
  critMultiplierAvg: number
  attacksPerSecondMin: number
  attacksPerSecondMax: number
  hitMin: number
  hitMax: number
  critMin: number
  critMax: number
  avgMin: number
  avgMax: number
  dpsMin: number
  dpsMax: number
}

const EXTRA_DAMAGE_CONDITIONS: Array<{
  stat: string
  condition: string
  label: string
}> = [
  { stat: 'extra_damage_stunned', condition: 'stunned', label: 'Stunned' },
  { stat: 'extra_damage_bleeding', condition: 'bleeding', label: 'Bleeding' },
  { stat: 'extra_damage_frozen', condition: 'frozen', label: 'Frozen' },
  { stat: 'extra_damage_poisoned', condition: 'poisoned', label: 'Poisoned' },
  { stat: 'extra_damage_burning', condition: 'burning', label: 'Burning' },
  { stat: 'extra_damage_stasis', condition: 'stasis', label: 'Stasis' },
  {
    stat: 'extra_damage_shadow_burning',
    condition: 'shadow_burning',
    label: 'Shadow Burning',
  },
  {
    stat: 'extra_damage_frost_bitten',
    condition: 'frost_bitten',
    label: 'Frost Bitten',
  },
]

function collectExtraDamage(
  stats: RangedStatMap,
  enemyConditions: Record<string, boolean> | undefined,
): { pct: number; sources: Array<{ label: string; pct: number }> } {
  const sources: Array<{ label: string; pct: number }> = []
  let total = 0
  if (!enemyConditions) return { pct: 0, sources }
  let anyAilmentActive = false
  for (const m of EXTRA_DAMAGE_CONDITIONS) {
    if (!enemyConditions[m.condition]) continue
    anyAilmentActive = true
    const v = stats[m.stat]
    if (v === undefined) continue
    const avg = (rangedMin(v) + rangedMax(v)) / 2
    if (avg === 0) continue
    sources.push({ label: m.label, pct: avg })
    total += avg
  }
  if (anyAilmentActive) {
    const v = stats['extra_damage_ailments']
    if (v !== undefined) {
      const avg = (rangedMin(v) + rangedMax(v)) / 2
      if (avg !== 0) {
        sources.push({ label: 'Afflicted with Ailments', pct: avg })
        total += avg
      }
    }
  }
  return { pct: total, sources }
}

export function rolledAffixValue(
  affix: {
    sign: '+' | '-'
    format: 'flat' | 'percent'
    valueMin: number | null
    valueMax: number | null
  },
  roll: number,
): number {
  if (affix.valueMin === null || affix.valueMax === null) return 0
  const raw =
    affix.valueMin === affix.valueMax
      ? affix.valueMax
      : affix.valueMin + (affix.valueMax - affix.valueMin) * roll
  const rounded = affix.format === 'flat' ? Math.round(raw) : raw
  return affix.sign === '-' ? -rounded : rounded
}

function affixStarMultiplier(
  statKey: string | null,
  stars: number | undefined,
): number {
  return statStarPercentMultiplier(statKey, stars)
}

export function rolledAffixValueWithStars(
  affix: {
    sign: '+' | '-'
    format: 'flat' | 'percent'
    valueMin: number | null
    valueMax: number | null
    statKey: string | null
  },
  roll: number,
  stars: number | undefined,
): number {
  const base = rolledAffixValue(affix, roll)
  const mult = affixStarMultiplier(affix.statKey, stars)
  const flat = statStarFlatBonus(affix.statKey, stars)
  if (base === 0 && flat === 0) return 0
  const direction = affix.sign === '-' ? -1 : 1
  const scaled = base * mult + flat * direction
  const starsActive = (stars ?? 0) > 0 && (mult !== 1 || flat !== 0)
  if (starsActive) return Math.floor(scaled)
  return affix.format === 'flat' ? Math.round(scaled) : scaled
}

export function formatAffixRange(
  affix: {
    sign: '+' | '-'
    format: 'flat' | 'percent'
    valueMin: number | null
    valueMax: number | null
    statKey: string | null
  },
  stars?: number,
): string {
  if (affix.valueMin === null || affix.valueMax === null) return affix.sign
  const minSigned = rolledAffixValueWithStars(affix, 0, stars)
  const maxSigned = rolledAffixValueWithStars(affix, 1, stars)
  const fmtAbs = (v: number) => {
    const abs = Math.abs(v)
    return Number.isInteger(abs) ? abs : Math.round(abs * 100) / 100
  }
  const lo = fmtAbs(minSigned)
  const hi = fmtAbs(maxSigned)
  const sign =
    affix.sign === '-' || minSigned < 0 || maxSigned < 0 ? '-' : '+'
  const suffix = affix.format === 'percent' ? '%' : ''
  if (lo === hi) return `${sign}${hi}${suffix}`
  return `${sign}[${lo}-${hi}]${suffix}`
}

export function applyStarsToRangedValue(
  value: RangedValue,
  statKey: string,
  stars: number | undefined,
): RangedValue {
  if (!stars || stars <= 0) return value
  const flat =
    statKey === 'item_granted_skill_rank'
      ? itemGrantedSkillRankFlatBonus(stars)
      : statStarFlatBonus(statKey, stars)
  const mult = statStarPercentMultiplier(statKey, stars)
  if (mult === 1 && flat === 0) return value
  if (typeof value === 'number') {
    return Math.floor(value * mult + flat)
  }
  const [min, max] = value
  return [Math.floor(min * mult + flat), Math.floor(max * mult + flat)]
}

export function shouldScaleImplicit(isRuneword: boolean): boolean {
  return !isRuneword
}

export function aggregateItemSkillBonuses(
  inventory: Inventory,
): Record<string, [number, number]> {
  const out: Record<string, [number, number]> = {}
  for (const [slotKey, item] of Object.entries(inventory)) {
    if (!item) continue
    const base = getItem(item.baseId)
    if (!base?.skillBonuses) continue
    const stars = isGearSlot(slotKey) ? item.stars : undefined
    for (const [skillName, val] of Object.entries(base.skillBonuses)) {
      const scaled = applyStarsToRangedValue(val, 'item_granted_skill_rank', stars)
      const min = Math.round(rangedMin(scaled))
      const max = Math.round(rangedMax(scaled))
      const key = normalizeSkillName(skillName)
      const cur = out[key] ?? [0, 0]
      out[key] = [cur[0] + min, cur[1] + max]
    }
  }
  return out
}

export function effectiveRankRangeFor(
  skill: Skill,
  baseRank: number,
  stats: RangedStatMap,
  itemSkillBonuses: Record<string, [number, number]>,
): [number, number] {
  if (baseRank <= 0) return [0, 0]
  const all = stats.all_skills ?? 0
  const elem = skill.damageType
    ? (stats[`${skill.damageType}_skills`] ?? 0)
    : 0
  const item = itemSkillBonuses[normalizeSkillName(skill.name)] ?? [0, 0]
  return [
    baseRank + rangedMin(all) + rangedMin(elem) + item[0],
    baseRank + rangedMax(all) + rangedMax(elem) + item[1],
  ]
}

export function computeSkillDamage(
  skill: Skill,
  allocatedRank: number,
  attributes: Record<AttributeKey, RangedValue>,
  stats: RangedStatMap,
  skillRanksByName: Record<string, number>,
  itemSkillBonuses: Record<string, [number, number]>,
  enemyConditions?: Record<string, boolean>,
  enemyResistances?: Record<string, number>,
  skillsByNormalizedName?: Record<string, Skill>,
  projectileCount?: number,
): SkillDamageBreakdown | null {
  if (allocatedRank === 0) return null
  const hasFormula = !!skill.damageFormula
  const hasTable =
    !!skill.damagePerRank && skill.damagePerRank.length > 0
  if (!hasFormula && !hasTable) return null

  const allSkills = stats.all_skills ?? 0
  const allSkillsMin = rangedMin(allSkills)
  const allSkillsMax = rangedMax(allSkills)
  let elementSkillsMin = 0
  let elementSkillsMax = 0
  if (skill.damageType) {
    const es = stats[`${skill.damageType}_skills`] ?? 0
    elementSkillsMin = rangedMin(es)
    elementSkillsMax = rangedMax(es)
  }
  const itemBonus =
    itemSkillBonuses[normalizeSkillName(skill.name)] ?? [0, 0]
  const effectiveRankMin =
    allocatedRank + allSkillsMin + elementSkillsMin + itemBonus[0]
  const effectiveRankMax =
    allocatedRank + allSkillsMax + elementSkillsMax + itemBonus[1]

  // Clamp to >= 0: linear formulas can extrapolate negative at low ranks,
  // a skill never deals negative damage. Mirrors Rust damage.rs.
  let baseMin: number
  let baseMax: number
  if (skill.damageFormula) {
    baseMin = Math.max(
      0,
      skill.damageFormula.base + skill.damageFormula.perLevel * effectiveRankMin,
    )
    baseMax = Math.max(
      0,
      skill.damageFormula.base + skill.damageFormula.perLevel * effectiveRankMax,
    )
  } else {
    const table = skill.damagePerRank!
    const idxMin = Math.min(Math.max(effectiveRankMin, 1), table.length) - 1
    const idxMax = Math.min(Math.max(effectiveRankMax, 1), table.length) - 1
    baseMin = Math.max(0, table[idxMin]?.min ?? 0)
    baseMax = Math.max(0, table[idxMax]?.max ?? 0)
  }

  const flatKeys: string[] = ['flat_skill_damage', 'flat_elemental_skill_damage']
  if (skill.damageType) flatKeys.push(`flat_${skill.damageType}_skill_damage`)
  let flatMin = 0
  let flatMax = 0
  for (const k of flatKeys) {
    const v = stats[k] ?? 0
    flatMin += rangedMin(v)
    flatMax += rangedMax(v)
  }

  let synergyMinPct = 0
  let synergyMaxPct = 0
  for (const b of skill.bonusSources ?? []) {
    const sourceKey = normalizeSkillName(b.source)
    if (b.per === 'attribute_point') {
      const attrKey = gameConfig.attributes.find(
        (a) =>
          a.name.toLowerCase() === sourceKey || a.key.toLowerCase() === sourceKey,
      )?.key
      if (attrKey) {
        const attr = attributes[attrKey] ?? 0
        synergyMinPct += rangedMin(attr) * b.value
        synergyMaxPct += rangedMax(attr) * b.value
      }
    } else if (b.per === 'skill_level') {
      const baseRank = skillRanksByName[sourceKey] ?? 0
      if (baseRank > 0) {
        const srcSkill = skillsByNormalizedName?.[sourceKey]
        if (srcSkill) {
          const [effMin, effMax] = effectiveRankRangeFor(
            srcSkill,
            baseRank,
            stats,
            itemSkillBonuses,
          )
          synergyMinPct += effMin * b.value
          synergyMaxPct += effMax * b.value
        } else {
          synergyMinPct += baseRank * b.value
          synergyMaxPct += baseRank * b.value
        }
      }
    }
  }

  const magicDmg = stats.magic_skill_damage ?? 0
  const elementKey = skill.damageType
    ? (`${skill.damageType}_skill_damage` as const)
    : null
  const elementDmg = elementKey ? (stats[elementKey] ?? 0) : 0
  const skillDamageMinPct = rangedMin(magicDmg) + rangedMin(elementDmg)
  const skillDamageMaxPct = rangedMax(magicDmg) + rangedMax(elementDmg)

  const magicMore = stats.magic_skill_damage_more ?? 0
  const elementMoreKey = elementKey ? (`${elementKey}_more` as const) : null
  const elementMore = elementMoreKey ? (stats[elementMoreKey] ?? 0) : 0
  const skillMoreMultMin =
    (1 + rangedMin(magicMore) / 100) * (1 + rangedMin(elementMore) / 100)
  const skillMoreMultMax =
    (1 + rangedMax(magicMore) / 100) * (1 + rangedMax(elementMore) / 100)

  const extra = collectExtraDamage(stats, enemyConditions)
  const extraMult = 1 + extra.pct / 100
  const isSpell = !!skill.tags?.includes('Spell')
  const critChance = isSpell
    ? rangedMax(stats.spell_crit_chance ?? 0)
    : rangedMax(stats.crit_chance ?? 0)
  const critDamagePct = isSpell
    ? rangedMax(stats.spell_crit_damage ?? 0)
    : rangedMax(stats.crit_damage ?? 0)
  const critDamageMore = isSpell ? 0 : rangedMax(stats.crit_damage_more ?? 0)
  const critMoreMult = 1 + critDamageMore / 100
  const critMultOnCrit = (1 + critDamagePct / 100) * critMoreMult
  const critChanceClamped = Math.max(0, Math.min(95, critChance)) / 100
  const critMultAvg =
    1 - critChanceClamped + critChanceClamped * critMultOnCrit

  const enemyResPct = skill.damageType
    ? (enemyResistances?.[skill.damageType] ?? 0)
    : 0
  const rawIgnorePct = skill.damageType
    ? rangedMax(stats[`ignore_${skill.damageType}_res`] ?? 0)
    : 0
  const ignoreResPct = Math.max(0, Math.min(100, rawIgnorePct))
  const effectiveResPct = enemyResPct * (1 - ignoreResPct / 100)
  const resistanceMult = 1 - effectiveResPct / 100

  const isElementalSkill =
    !!skill.damageType &&
    (ELEMENTS as readonly string[]).includes(skill.damageType)
  const elementalBreakPct = isElementalSkill
    ? Math.max(
        0,
        rangedMax(stats.elemental_break ?? 0) +
          (isSpell
            ? rangedMax(stats.elemental_break_on_spell ?? 0)
            : rangedMax(stats.elemental_break_on_strike ?? 0)),
      )
    : 0
  const elementalBreakMultiplier = 1 + elementalBreakPct / 100

  const lightningBreakPct =
    skill.damageType === 'lightning' && enemyConditions?.lightning_break
      ? Math.max(0, rangedMax(stats.lightning_break ?? 0))
      : 0
  const lightningBreakMultiplier = 1 + lightningBreakPct / 100

  const hitMin =
    (baseMin + flatMin) *
    (1 + synergyMinPct / 100) *
    (1 + skillDamageMinPct / 100) *
    skillMoreMultMin *
    extraMult *
    elementalBreakMultiplier *
    lightningBreakMultiplier *
    resistanceMult
  const hitMax =
    (baseMax + flatMax) *
    (1 + synergyMaxPct / 100) *
    (1 + skillDamageMaxPct / 100) *
    skillMoreMultMax *
    extraMult *
    elementalBreakMultiplier *
    lightningBreakMultiplier *
    resistanceMult

  const critMin = hitMin * critMultOnCrit
  const critMax = hitMax * critMultOnCrit
  const multicastChancePct = isSpell
    ? Math.max(0, rangedMax(stats.multicast_chance ?? 0))
    : 0
  const multicastMultiplier = 1 + multicastChancePct / 100
  const projectiles = Math.max(1, Math.floor(projectileCount ?? 1))
  const avgMin = hitMin * critMultAvg * multicastMultiplier * projectiles
  const avgMax = hitMax * critMultAvg * multicastMultiplier * projectiles

  const finalMin = Math.floor(hitMin)
  const finalMax = Math.floor(hitMax)

  return {
    effectiveRankMin,
    effectiveRankMax,
    baseMin,
    baseMax,
    flatMin,
    flatMax,
    synergyMinPct,
    synergyMaxPct,
    skillDamageMinPct,
    skillDamageMaxPct,
    extraDamagePct: extra.pct,
    extraDamageSources: extra.sources,
    critChance,
    critDamagePct,
    critMultiplierAvg: critMultAvg,
    multicastChancePct,
    multicastMultiplier,
    projectileCount: projectiles,
    elementalBreakPct,
    elementalBreakMultiplier,
    enemyResistancePct: enemyResPct,
    resistanceIgnoredPct: ignoreResPct,
    effectiveResistancePct: effectiveResPct,
    resistanceMultiplier: resistanceMult,
    hitMin: Math.floor(hitMin),
    hitMax: Math.floor(hitMax),
    critMin: Math.floor(critMin),
    critMax: Math.floor(critMax),
    finalMin,
    finalMax,
    avgMin: Math.floor(avgMin),
    avgMax: Math.floor(avgMax),
  }
}

export function computeWeaponDamage(
  inventory: Inventory,
  stats: RangedStatMap,
  enemyConditions?: Record<string, boolean>,
): WeaponDamageBreakdown {
  const weapon = inventory.weapon
  const base = weapon ? getItem(weapon.baseId) : undefined
  const hasWeapon =
    !!base && base.damageMin !== undefined && base.damageMax !== undefined

  // Unarmed: every character has a 2-6 base physical damage when no weapon is equipped.
  const weaponDamageMin = hasWeapon ? base!.damageMin! : 2
  const weaponDamageMax = hasWeapon ? base!.damageMax! : 6

  const ed = stats.enhanced_damage ?? 0
  const enhancedDamageMinPct = rangedMin(ed)
  const enhancedDamageMaxPct = rangedMax(ed)
  const edMore = stats.enhanced_damage_more ?? 0
  const enhancedMoreMinMult = 1 + rangedMin(edMore) / 100
  const enhancedMoreMaxMult = 1 + rangedMax(edMore) / 100

  const addPhys = stats.additive_physical_damage ?? 0
  const additivePhysicalMin = rangedMin(addPhys)
  const additivePhysicalMax = rangedMax(addPhys)

  const atkDmg = stats.attack_damage ?? 0
  const attackDamageMinPct = rangedMin(atkDmg)
  const attackDamageMaxPct = rangedMax(atkDmg)

  const extra = collectExtraDamage(stats, enemyConditions)
  const extraMult = 1 + extra.pct / 100

  const critChance = rangedMax(stats.crit_chance ?? 0)
  const critDamagePct = rangedMax(stats.crit_damage ?? 0)
  const critDamageMore = rangedMax(stats.crit_damage_more ?? 0)
  const critMoreMult = 1 + critDamageMore / 100
  const critMultOnCrit = (1 + critDamagePct / 100) * critMoreMult
  const critChanceClamped = Math.max(0, Math.min(95, critChance)) / 100
  const critMultAvg =
    1 - critChanceClamped + critChanceClamped * critMultOnCrit

  const baseMin =
    weaponDamageMin * (1 + enhancedDamageMinPct / 100) * enhancedMoreMinMult +
    additivePhysicalMin
  const baseMax =
    weaponDamageMax * (1 + enhancedDamageMaxPct / 100) * enhancedMoreMaxMult +
    additivePhysicalMax

  const hitMinRaw =
    baseMin * (1 + attackDamageMinPct / 100) * extraMult
  const hitMaxRaw =
    baseMax * (1 + attackDamageMaxPct / 100) * extraMult

  const critMinRaw = hitMinRaw * critMultOnCrit
  const critMaxRaw = hitMaxRaw * critMultOnCrit
  const avgMinRaw = hitMinRaw * critMultAvg
  const avgMaxRaw = hitMaxRaw * critMultAvg

  const ias = stats.increased_attack_speed ?? 0
  const iasMore = stats.increased_attack_speed_more ?? 0
  const baseAps = rangedMax(stats.attacks_per_second ?? 0)
  const iasMin = rangedMin(ias)
  const iasMax = rangedMax(ias)
  const iasMoreMinMult = 1 + rangedMin(iasMore) / 100
  const iasMoreMaxMult = 1 + rangedMax(iasMore) / 100
  const apsMin = baseAps * (1 + iasMin / 100) * iasMoreMinMult
  const apsMax = baseAps * (1 + iasMax / 100) * iasMoreMaxMult

  const dpsMinRaw = avgMinRaw * apsMin
  const dpsMaxRaw = avgMaxRaw * apsMax

  return {
    hasWeapon,
    weaponName: base?.name,
    weaponDamageMin,
    weaponDamageMax,
    enhancedDamageMinPct,
    enhancedDamageMaxPct,
    additivePhysicalMin,
    additivePhysicalMax,
    attackDamageMinPct,
    attackDamageMaxPct,
    extraDamagePct: extra.pct,
    extraDamageSources: extra.sources,
    critChance,
    critDamagePct,
    critMultiplierAvg: critMultAvg,
    attacksPerSecondMin: apsMin,
    attacksPerSecondMax: apsMax,
    hitMin: Math.floor(hitMinRaw),
    hitMax: Math.floor(hitMaxRaw),
    critMin: Math.floor(critMinRaw),
    critMax: Math.floor(critMaxRaw),
    avgMin: Math.floor(avgMinRaw),
    avgMax: Math.floor(avgMaxRaw),
    dpsMin: Math.floor(dpsMinRaw),
    dpsMax: Math.floor(dpsMaxRaw),
  }
}

export function passiveStatsAtRank(
  skill: Skill,
  rank: number,
): Record<string, number> {
  if (!skill.passiveStats || rank <= 0) return {}
  const { base, perRank } = skill.passiveStats
  const out: Record<string, number> = {}
  if (base) for (const [k, v] of Object.entries(base)) out[k] = v
  if (perRank) {
    for (const [k, v] of Object.entries(perRank)) {
      out[k] = (out[k] ?? 0) + v * (rank - 1)
    }
  }
  for (const k of Object.keys(out)) {
    out[k] = Math.round(out[k]! * 1000) / 1000
  }
  return out
}

export function manaCostAtRank(skill: Skill, rank: number): number | undefined {
  if (rank <= 0) rank = 1
  if (skill.manaCostFormula) {
    return Math.floor(
      skill.manaCostFormula.base +
        skill.manaCostFormula.perLevel * (rank - 1),
    )
  }
  const exact = skill.ranks.find((r) => r.rank === rank)
  if (exact?.manaCost !== undefined) return exact.manaCost
  return skill.ranks[0]?.manaCost
}
