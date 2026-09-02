import {
  entityAttackRate,
  entityAttackRateFixedKey,
  entityAttackSpeedKey,
  entityKindOfTag,
  entityRatesFrom,
} from './entityRates'
import { effectiveSkillTags, entityTagOf } from '../skills/skillTags'
import {
  detectRuneword,
  effectiveStars,
  gameConfig,
  getAffix,
  getClass,
  getGem,
  getItem,
  getRune,
  getSkillsByClass,
} from '@data'
import { getSeason } from '@data/seasons/registry'
import { groupGearSlots } from '../../views/gear/lib/slotGroups'
import { compactRange } from '../compactNumber'
import { manaCostAtRank } from './manaCost'
import { computeBuildPerformanceAsync, displayValuesNative } from '../calc/bridge'
import type { AffixValueOutput } from '../calc/bridge'
import {
  effectiveCap,
  formatValue,
  isZero,
  normalizeSkillName,
  rangedMax,
  rangedMin,
  shouldScaleImplicit,
  statDef,
} from '../item/stats'
import { APP_VERSION } from '../version'
import { stripHtml } from '../../components/buildSelect/buildDisplay'
import { decodeShareToBuild, encodeBuildToShare, type BuildSnapshot } from './shareBuild'
import { getActiveProfile, type SavedBuild } from './savedBuilds'
import { snapshotToDeps } from '../../components/buildSelect/usePreviewStats'
import { heroLevelFor } from './heroLevel'
import { groupEhpRows, formatEhp } from './ehp'
import { classIconPath, skillIconPath, socketIconPath } from './assetPaths'
import { buildGearTooltip } from './gearTooltip'
import type { TooltipModelDeps } from '../../components/itemTooltipModel'
import { buildIncarnation } from './incarnationSummary'
import { bonusSourceSynergy } from '../skill/synergyText'
import { computeSustainStats } from './sustainStats'
import type { BuildPerformance, PerSkillDps } from './buildPerformance'
import {
  ATTRIBUTE_ORDER,
  DEFENSE_KEYS,
  OFFENSE_KEYS,
  RESISTANCES,
  attributeRowTone,
  defenseRowTone,
  effectiveStatValue,
  offenseRowTone,
  resistanceRowTone,
} from './statSectionDefs'
import {
  CURRENT_SCHEMA_VERSION,
  MAX_SNAPSHOT_BYTES,
  MAX_TITLE_LENGTH,
  snapshotByteSize,
  type SharePayload,
  type SkillItem,
  type StatRow,
  type StatSection,
  type Tone,
} from './sharePayload'
import type { EquippedItem, Inventory, ItemBase, RangedValue, Skill } from '../../types'

const DPS_SCALE = 'billions'

const OFFENSE_KEY_LABELS: Record<string, string> = {
  enhanced_damage: 'Enhanced damage',
  attack_damage: 'Attack damage',
  increased_attack_speed: 'Attack speed',
  faster_cast_rate: 'Cast rate',
  crit_chance: 'Crit chance',
  crit_damage: 'Crit damage',
  life_steal: 'Life steal',
  mana_steal: 'Mana steal',
}

function statRow(
  label: string,
  key: string,
  stats: Record<string, RangedValue>,
  statsCombined: Record<string, RangedValue>,
  tone: Tone | undefined,
): StatRow | null {
  const value = effectiveStatValue(stats, statsCombined, key)
  if (isZero(value)) return null
  return { label, value: formatValue(value, key), tone: tone ?? undefined }
}

function buildOffenseSection(
  stats: Record<string, RangedValue>,
  statsCombined: Record<string, RangedValue>,
): StatSection {
  const rows = OFFENSE_KEYS.map((key) =>
    statRow(statDef(key)?.name ?? OFFENSE_KEY_LABELS[key] ?? key, key, stats, statsCombined, offenseRowTone(key)),
  ).filter((r): r is StatRow => r !== null)
  return { title: 'Offense', rows }
}

function buildDefenseSection(
  stats: Record<string, RangedValue>,
  statsCombined: Record<string, RangedValue>,
): StatSection {
  const rows = DEFENSE_KEYS.map((key) =>
    statRow(statDef(key)?.name ?? key, key, stats, statsCombined, defenseRowTone(key)),
  ).filter((r): r is StatRow => r !== null)
  const ehpRows = groupEhpRows({ ...stats, ...statsCombined }).map((r) => ({
    label: r.label,
    value: formatEhp(r.ehp),
    tone: 'gold' as const,
  }))
  return { title: 'Defense', rows: [...rows, ...ehpRows] }
}

function buildResistancesSection(stats: Record<string, RangedValue>): StatSection {
  const rows = RESISTANCES.map((r): StatRow | null => {
    const v = stats[r.key] ?? 0
    if (isZero(v)) return null
    const cap = effectiveCap(r.key, stats)
    const numeric = typeof v === 'number' ? v : rangedMax(v)
    const capped = cap !== undefined && numeric > cap
    const value = capped ? `${cap}% (${Math.round(numeric)}%)` : formatValue(v, r.key)
    return { label: `${r.label} res`, value, tone: resistanceRowTone(r.className) }
  }).filter((r): r is StatRow => r !== null)
  return { title: 'Resistances', rows }
}

function buildAttributesSection(attributes: Record<string, RangedValue>): StatSection {
  const rows = ATTRIBUTE_ORDER.map((key): StatRow | null => {
    const value = attributes[key]
    if (value === undefined) return null
    const label = gameConfig.attributes.find((a) => a.key === key)?.name ?? statDef(key)?.name ?? key
    return { label, value: formatValue(value, key), tone: attributeRowTone(key) }
  }).filter((r): r is StatRow => r !== null)
  return { title: 'Attributes', rows }
}

function formatNumRange(min: number, max: number): string {
  const fmt = (v: number): string => (Number.isInteger(v) ? String(v) : v.toFixed(2).replace(/\.?0+$/, ''))
  if (Math.abs(min - max) < 0.005) return fmt(min)
  return `${fmt(min)}–${fmt(max)}`
}

async function buildSustainSection(
  snapshot: BuildSnapshot,
  performance: BuildPerformance,
): Promise<StatSection | undefined> {
  const primarySkillId = snapshot.activeSkillIds[0]
  if (!primarySkillId) return undefined
  const skill = getSkillsByClass(snapshot.classId).find((s) => s.kind === 'active' && s.id === primarySkillId)
  if (!skill) return undefined

  const activeRank = snapshot.skillRanks[skill.id] ?? 0
  const [rankBonusMin, rankBonusMax] = performance.rankBonuses[normalizeSkillName(skill.name)] ?? [0, 0]
  const sustain = await computeSustainStats({
    skill,
    activeRank,
    rankBonusMin,
    rankBonusMax,
    stats: performance.stats,
    statsCombined: performance.statsCombined,
  })

  const rows: StatRow[] = []

  rows.push(
    sustain.effManaMin === undefined || sustain.effManaMax === undefined
      ? { label: 'Mana / cast', value: '—', tone: 'muted' }
      : { label: 'Mana / cast', value: formatNumRange(sustain.effManaMin, sustain.effManaMax), tone: 'blue' },
  )

  if (sustain.lifePerCastMax !== undefined && sustain.lifePerCastMax > 0) {
    rows.push({
      label: 'Life / cast',
      value: formatNumRange(sustain.lifePerCastMin ?? 0, sustain.lifePerCastMax),
      tone: 'red',
    })
  }

  // For an entity skill the player's cast rate only spawns them; the entity's
  // own swing rate is a separate row.
  const entityTag = entityTagOf(effectiveSkillTags(skill, snapshot.subskillRanks))
  const entityKind = entityTag ? entityKindOfTag(entityTag) : undefined
  const rateLabel = entityKind ? 'Spawn rate' : 'Cast rate'
  rows.push(
    sustain.effCastMin === undefined || sustain.effCastMax === undefined
      ? { label: rateLabel, value: '—', tone: 'muted' }
      : { label: rateLabel, value: `${formatNumRange(sustain.effCastMin, sustain.effCastMax)}/s` },
  )

  if (entityKind && entityTag) {
    const speedKey = entityAttackSpeedKey(entityKind)
    const swing = entityAttackRate(
      entityKind,
      entityRatesFrom(snapshot.entityRates, snapshot.entityAttacksPerSecond),
      [
        rangedMin(performance.stats[speedKey] ?? 0),
        rangedMax(performance.stats[speedKey] ?? 0),
      ],
      rangedMax(performance.stats[entityAttackRateFixedKey(entityKind)] ?? 0),
    )
    rows.push({
      label: `${entityTag} attack rate`,
      value: `${formatNumRange(swing.min, swing.max)}/s`,
    })
  }

  rows.push(
    sustain.manaPerSecMin === undefined || sustain.manaPerSecMax === undefined
      ? { label: 'Mana / sec', value: '—', tone: 'muted' }
      : {
          label: 'Mana / sec',
          value: formatNumRange(sustain.manaPerSecMin, sustain.manaPerSecMax),
          tone: sustain.sustainable ? 'green' : sustain.unsustainable ? 'red' : 'orange',
        },
  )

  rows.push({
    label: 'Mana regen',
    value: formatNumRange(sustain.manaRegenMin, sustain.manaRegenMax),
    tone: 'blue',
  })

  if (sustain.netMin !== undefined && sustain.netMax !== undefined) {
    rows.push({
      label: 'Net mana / sec',
      value: `${sustain.netMin >= 0 ? '+' : ''}${formatNumRange(sustain.netMin, sustain.netMax)}`,
      tone: sustain.netMin >= 0 ? 'green' : sustain.netMax < 0 ? 'red' : 'orange',
    })
  }

  if (sustain.uptimeMin !== undefined && sustain.uptimeMax !== undefined) {
    rows.push({
      label: 'Uptime',
      value: `${formatNumRange(Math.round(sustain.uptimeMin), Math.round(sustain.uptimeMax))}%`,
      tone: sustain.uptimeMin >= 100 ? 'green' : sustain.uptimeMax < 75 ? 'red' : 'orange',
    })
  }

  return { title: 'Sustain', rows }
}

function capitalize(value: string): string {
  return value.length > 0 ? value.charAt(0).toUpperCase() + value.slice(1) : value
}

function skillType(skill: Skill): string {
  const kind = capitalize(skill.kind)
  return skill.damageType ? `${kind} · ${capitalize(skill.damageType)}` : kind
}

function buildSkillRows(
  skill: Skill,
  perSkill: PerSkillDps | undefined,
  isMain: boolean,
  effRankMin: number,
  effRankMax: number,
): StatRow[] {
  const rows: StatRow[] = []
  if (perSkill?.hitDpsMin !== undefined && perSkill?.hitDpsMax !== undefined) {
    rows.push({
      label: 'Hit DPS',
      value: compactRange(perSkill.hitDpsMin, perSkill.hitDpsMax, DPS_SCALE),
      tone: 'gold',
      ...(isMain ? { glow: true } : {}),
    })
  }
  const manaMin = manaCostAtRank(skill, Math.max(1, effRankMin))
  const manaMax = manaCostAtRank(skill, Math.max(1, effRankMax))
  if (manaMin !== undefined && manaMax !== undefined) {
    rows.push({
      label: 'Mana / cast',
      value: manaMin === manaMax ? String(manaMax) : `${manaMin}–${manaMax}`,
      tone: 'blue',
    })
  }
  if (skill.baseCooldown !== undefined) {
    rows.push({ label: 'Cooldown', value: `${skill.baseCooldown}s` })
  }
  return rows
}

function buildSkillItem(
  skill: Skill,
  rank: number,
  performance: BuildPerformance,
  isMain: boolean,
  classId: string | null,
): SkillItem {
  const [bonusMin, bonusMax] = performance.rankBonuses[normalizeSkillName(skill.name)] ?? [0, 0]
  const fromItems =
    bonusMax > 0 ? (bonusMin === bonusMax ? `+${bonusMin}` : `+${bonusMin}–${bonusMax}`) : undefined
  const effectiveRank =
    bonusMax > 0
      ? bonusMin === bonusMax
        ? String(rank + bonusMin)
        : `${rank + bonusMin}–${rank + bonusMax}`
      : undefined
  const rows = buildSkillRows(
    skill,
    performance.perSkill?.find((p) => p.id === skill.id),
    isMain,
    rank + bonusMin,
    rank + bonusMax,
  )
  const synergies = (skill.bonusSources ?? []).map(bonusSourceSynergy)
  return {
    icon: classId ? skillIconPath(classId, skill.id) : undefined,
    name: skill.name,
    type: skillType(skill),
    ...(isMain ? { main: true } : {}),
    rank,
    max: skill.maxRank,
    ...(fromItems ? { fromItems } : {}),
    ...(effectiveRank ? { effectiveRank } : {}),
    ...(skill.description ? { desc: skill.description } : {}),
    ...(rows.length > 0 ? { rows } : {}),
    ...(synergies.length > 0 ? { synergies } : {}),
  }
}

function buildSkillGroups(
  snapshot: BuildSnapshot,
  performance: BuildPerformance,
): NonNullable<SharePayload['skills']>['groups'] {
  const classSkills = getSkillsByClass(snapshot.classId)
  const mainId = snapshot.activeSkillIds[0]
  const byGroup = new Map<string, SkillItem[]>()
  const order: string[] = []
  for (const skill of classSkills) {
    const rank = snapshot.skillRanks[skill.id] ?? 0
    if (rank <= 0) continue
    const groupName = skill.tree ?? 'General'
    let items = byGroup.get(groupName)
    if (!items) {
      items = []
      byGroup.set(groupName, items)
      order.push(groupName)
    }
    items.push(buildSkillItem(skill, rank, performance, skill.id === mainId, snapshot.classId))
  }
  return order.map((name) => ({ name, items: byGroup.get(name)! }))
}

const EMPTY_ITEM_DISPLAY: TooltipModelDeps['display'] = {
  implicitScaled: {},
  skillRankScaled: {},
  affixRanges: [],
}

async function computeItemDisplay(
  base: ItemBase,
  equipped: EquippedItem,
  scaleImplicit: boolean,
): Promise<TooltipModelDeps['display']> {
  const stars = effectiveStars(base.slot, equipped.stars, base.rarity)
  const toPair = (v: RangedValue): [number, number] => [rangedMin(v), rangedMax(v)]
  const implicitEntries = scaleImplicit && base.implicit ? Object.entries(base.implicit) : []
  const skillEntries = base.skillBonuses ? Object.entries(base.skillBonuses) : []
  const equippedAffixes = equipped.affixes ?? []
  const affixDefs = equippedAffixes.map((eq) => getAffix(eq.affixId))
  const affixReqs = equippedAffixes
    .map((eq, i) => ({ eq, def: affixDefs[i] }))
    .filter((x) => x.def)
    .map((x) => ({ affix: x.def, roll: x.eq.roll ?? 0, stars }))
  const scaled = [
    // Star scaling keys off the resolved stat (engine rewrites
    // random_skill_element to {element}_skills before scaling).
    ...implicitEntries.map(([k, v]) => ({
      value: toPair(v),
      statKey:
        k === 'random_skill_element' && equipped.randomSkillElement
          ? `${equipped.randomSkillElement}_skills`
          : k,
      stars,
    })),
    ...skillEntries.map(([, v]) => ({ value: toPair(v), statKey: 'item_granted_skill_rank', stars })),
  ]
  if (affixReqs.length === 0 && scaled.length === 0) return EMPTY_ITEM_DISPLAY

  const res = await displayValuesNative({ affixes: affixReqs, scaled })
  const implicitScaled: Record<string, [number, number]> = {}
  implicitEntries.forEach(([k], i) => {
    implicitScaled[k] = res.scaled[i]!
  })
  const skillRankScaled: Record<string, [number, number]> = {}
  skillEntries.forEach(([name], i) => {
    skillRankScaled[name] = res.scaled[implicitEntries.length + i]!
  })
  const affixRanges: (AffixValueOutput | null)[] = []
  let cursor = 0
  for (const def of affixDefs) {
    affixRanges.push(def ? (res.affixes[cursor++] ?? null) : null)
  }
  return { implicitScaled, skillRankScaled, affixRanges }
}

function toModelInventory(inventory: Inventory): Record<string, EquippedItem | null> {
  const out: Record<string, EquippedItem | null> = {}
  for (const [slot, item] of Object.entries(inventory)) out[slot] = item ?? null
  return out
}

async function buildGearItems(inventory: Inventory): Promise<NonNullable<SharePayload['gear']>['items']> {
  const modelInventory = toModelInventory(inventory)
  const { gear: gearSlots } = groupGearSlots(gameConfig.slots)
  const items: NonNullable<SharePayload['gear']>['items'] = []
  for (const slotDef of gearSlots) {
    const equipped = inventory[slotDef.key]
    if (!equipped) continue
    const base = getItem(equipped.baseId)
    if (!base) continue
    const runeword = detectRuneword(base, equipped.socketed)
    const display = await computeItemDisplay(base, equipped, shouldScaleImplicit(!!runeword))
    const tooltip = buildGearTooltip(base, equipped, { display, inventory: modelInventory })
    const sockets = equipped.socketed
      .map((id, i) => ({ id, rainbow: equipped.socketTypes[i] === 'rainbow' }))
      .filter((s): s is { id: string; rainbow: boolean } => !!s.id)
      .map((s) => {
        const icon = socketIconPath(getGem(s.id)?.name ?? getRune(s.id)?.name ?? s.id)
        return icon ? { icon, ...(s.rainbow ? { rainbow: true } : {}) } : null
      })
      .filter((s): s is { icon: string; rainbow?: boolean } => !!s)
    items.push({
      slot: slotDef.name,
      name: tooltip.name,
      rarity: base.rarity,
      sockets,
      tooltip,
    })
  }
  return items
}

export async function buildSharePayload(build: SavedBuild): Promise<SharePayload> {
  const profile = getActiveProfile(build)
  if (!profile) throw new Error('Build has no active profile to share')
  const decoded = decodeShareToBuild(profile.code)
  if (!decoded) throw new Error('Build code could not be decoded')
  const { snapshot } = decoded

  const deps = snapshotToDeps(snapshot)
  const performance = await computeBuildPerformanceAsync({ ...deps, season: build.season })

  const cls = snapshot.classId ? getClass(snapshot.classId) : undefined
  const seasonName = getSeason(build.season)?.name ?? build.season
  const heroLevel = heroLevelFor(snapshot)

  const dps =
    performance.combinedDpsMin !== undefined && performance.combinedDpsMax !== undefined
      ? compactRange(performance.combinedDpsMin, performance.combinedDpsMax, DPS_SCALE)
      : '0'

  const offense = buildOffenseSection(performance.stats, performance.statsCombined)
  const defense = buildDefenseSection(performance.stats, performance.statsCombined)
  const resistances = buildResistancesSection(performance.stats)
  const attributes = buildAttributesSection(performance.attributes)
  const sustain = await buildSustainSection(snapshot, performance)

  const skillGroups = buildSkillGroups(snapshot, performance)
  const skillPointsSpent = Object.values(snapshot.skillRanks).reduce((a, b) => a + b, 0)

  const gearItems = await buildGearItems(snapshot.inventory)

  const incarnation = buildIncarnation(snapshot.allocatedTreeNodes, snapshot.treeSocketed)

  const code = encodeBuildToShare(snapshot, build.notes, build.season)

  const payload: SharePayload = {
    schemaVersion: CURRENT_SCHEMA_VERSION,
    appVersion: APP_VERSION,
    code,
    meta: {
      title: build.name.slice(0, MAX_TITLE_LENGTH),
      className: cls?.name ?? snapshot.classId ?? 'Unknown',
      classId: snapshot.classId ?? '',
      level: snapshot.level,
      heroLevel,
      seasonLabel: seasonName,
      tags: build.tags,
      primaryAttribute: cls?.primaryAttribute,
      classIcon: snapshot.classId ? classIconPath(snapshot.classId) : undefined,
      dps,
    },
    profiles: [
      {
        name: 'Current',
        dps,
        statSections: [offense, defense, resistances, attributes, ...(sustain ? [sustain] : [])],
      },
    ],
    skills: skillGroups.length > 0 ? { pointsLabel: `${skillPointsSpent} pts`, groups: skillGroups } : undefined,
    gear: gearItems.length > 0 ? { items: gearItems } : undefined,
    incarnation: snapshot.allocatedTreeNodes.size > 0 ? incarnation : undefined,
    notes: build.notes ? stripHtml(build.notes) : undefined,
  }

  return payload
}

export type WebShareErrorKind = 'too-large' | 'validation' | 'server' | 'network' | 'not-configured' | 'unknown'

export class WebShareError extends Error {
  readonly kind: WebShareErrorKind
  constructor(kind: WebShareErrorKind, message: string) {
    super(message)
    this.name = 'WebShareError'
    this.kind = kind
  }
}

function getCreateUrl(): string | null {
  const raw = import.meta.env.VITE_WEB_SHARE_CREATE_URL
  return typeof raw === 'string' && raw.trim() ? raw.trim() : null
}

export function isWebShareConfigured(): boolean {
  return getCreateUrl() !== null
}

function errorFromStatus(status: number): WebShareError {
  if (status === 400) return new WebShareError('validation', 'This build could not be validated for web share.')
  if (status === 413) return new WebShareError('too-large', 'This build is too large to share to the web.')
  if (status >= 500) return new WebShareError('server', 'The share server had a problem. Try again shortly.')
  return new WebShareError('unknown', `Web share failed (${status}).`)
}

export async function postWebShare(payload: SharePayload): Promise<{ id: string; url: string }> {
  const url = getCreateUrl()
  if (!url) throw new WebShareError('not-configured', 'Web sharing is not configured in this build.')
  if (snapshotByteSize(payload) > MAX_SNAPSHOT_BYTES) {
    throw new WebShareError('too-large', 'This build is too large to share to the web.')
  }

  let res: Response
  try {
    res = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    })
  } catch {
    throw new WebShareError('network', 'Network error. Check your connection.')
  }

  if (res.status !== 201) throw errorFromStatus(res.status)
  const data = (await res.json()) as { id?: string; url?: string }
  if (!data.id || !data.url) throw new WebShareError('unknown', 'Unexpected response from the share server.')
  return { id: data.id, url: data.url }
}
