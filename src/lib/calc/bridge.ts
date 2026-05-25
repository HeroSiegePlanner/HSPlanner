// TS-to-Rust calc bridge.

import { invoke } from '@tauri-apps/api/core'

import type { CustomStat, Inventory, RangedValue, TreeSocketContent } from '../../types'
import type { AttackSkillDamageBreakdown, SkillDamageBreakdown } from '../../utils/stats'
import type {
  BuildPerformance,
  BuildPerformanceDeps,
} from '../../utils/buildPerformance'
import type {
  ComputedStats,
  SourceContribution,
  SourceType,
} from '../../utils/stats'
import type { ForgeKind } from '../../data'

// ---------- compute_build_performance ----------

export interface BuildPerformanceInput {
  classId?: string | null
  level?: number
  allocatedAttrs?: Record<string, number>
  inventory?: Inventory
  skillRanks?: Record<string, number>
  subskillRanks?: Record<string, number>
  activeAuraId?: string | null
  activeBuffs?: Record<string, boolean>
  customStats?: CustomStat[]
  // Rust deserialises into HashSet.
  allocatedTreeNodes?: number[]
  // Node ids stringified because JSON keys must be strings.
  treeSocketed?: Record<string, TreeSocketContent>
  mainSkillId?: string | null
  enemyConditions?: Record<string, boolean>
  playerConditions?: Record<string, boolean>
  skillProjectiles?: Record<string, number>
  enemyResistances?: Record<string, number>
  procToggles?: Record<string, boolean>
  killsPerSec?: number
}

// Rust returns [min, max]; TS expects `number | [number, number]` — see asRangedValue.
export type RustRanged = [number, number]

export interface BuildPerformanceOutput {
  attributes: Record<string, RustRanged>
  stats: Record<string, RustRanged>
  damage: SkillDamageBreakdown | null
  attackDamage: AttackSkillDamageBreakdown | null
  hitDpsMin: number | null
  hitDpsMax: number | null
  avgHitDpsMin: number | null
  avgHitDpsMax: number | null
  procDpsMin: number
  procDpsMax: number
  combinedDpsMin: number | null
  combinedDpsMax: number | null
  activeSkillName: string | null
}

export function computeBuildPerformanceNative(
  input: BuildPerformanceInput,
): Promise<BuildPerformanceOutput> {
  return invoke<BuildPerformanceOutput>('calc_build_performance', { input })
}

// Collapse [n, n] to n so legacy TS callers see the same shape.
export function asRangedValue([min, max]: RustRanged): RangedValue {
  return min === max ? min : [min, max]
}

// ---------- legacy-shape adapters ----------

function filterTreeSocketed(
  socketed: Record<number, TreeSocketContent | null>,
): Record<string, TreeSocketContent> {
  const out: Record<string, TreeSocketContent> = {}
  for (const [k, v] of Object.entries(socketed)) {
    if (v !== null) out[k] = v
  }
  return out
}

// TS Set<number>/Record<number,...> -> Rust arrays/string-keyed objects.
function depsToInput(deps: BuildPerformanceDeps): BuildPerformanceInput {
  return {
    classId: deps.classId,
    level: deps.level,
    allocatedAttrs: deps.allocatedAttrs,
    inventory: deps.inventory,
    skillRanks: deps.skillRanks,
    subskillRanks: deps.subskillRanks,
    activeAuraId: deps.activeAuraId,
    activeBuffs: deps.activeBuffs,
    customStats: deps.customStats,
    allocatedTreeNodes: [...deps.allocatedTreeNodes],
    treeSocketed: filterTreeSocketed(deps.treeSocketed),
    mainSkillId: deps.mainSkillId,
    enemyConditions: deps.enemyConditions,
    playerConditions: deps.playerConditions,
    skillProjectiles: deps.skillProjectiles,
    enemyResistances: deps.enemyResistances,
    procToggles: deps.procToggles,
    killsPerSec: deps.killsPerSec,
  }
}

function toLegacyBuildPerformance(
  raw: BuildPerformanceOutput,
): BuildPerformance {
  return {
    attributes: Object.fromEntries(
      Object.entries(raw.attributes).map(([k, v]) => [k, asRangedValue(v)]),
    ),
    stats: Object.fromEntries(
      Object.entries(raw.stats).map(([k, v]) => [k, asRangedValue(v)]),
    ),
    damage: raw.damage,
    attackDamage: raw.attackDamage,
    hitDpsMin: raw.hitDpsMin ?? undefined,
    hitDpsMax: raw.hitDpsMax ?? undefined,
    avgHitDpsMin: raw.avgHitDpsMin ?? undefined,
    avgHitDpsMax: raw.avgHitDpsMax ?? undefined,
    procDpsMin: raw.procDpsMin,
    procDpsMax: raw.procDpsMax,
    combinedDpsMin: raw.combinedDpsMin ?? undefined,
    combinedDpsMax: raw.combinedDpsMax ?? undefined,
    activeSkillName: raw.activeSkillName,
  }
}

export async function computeBuildPerformanceAsync(
  deps: BuildPerformanceDeps,
): Promise<BuildPerformance> {
  const raw = await computeBuildPerformanceNative(depsToInput(deps))
  return toLegacyBuildPerformance(raw)
}

// ---------- compute_build_stats (with source maps) ----------

interface RustForge {
  itemName: string
  modName: string
  kind: ForgeKind
}

interface RustSourceContribution {
  label: string
  sourceType: SourceType
  value: RustRanged
  forge?: RustForge
}

interface BuildStatsRustOutput {
  attributes: Record<string, RustRanged>
  stats: Record<string, RustRanged>
  attributeSources: Record<string, RustSourceContribution[]>
  statSources: Record<string, RustSourceContribution[]>
}

function convertContribution(raw: RustSourceContribution): SourceContribution {
  const out: SourceContribution = {
    label: raw.label,
    sourceType: raw.sourceType,
    value: asRangedValue(raw.value),
  }
  if (raw.forge) {
    out.forge = raw.forge
  }
  return out
}

function convertSourceMap(
  raw: Record<string, RustSourceContribution[]>,
): Record<string, SourceContribution[]> {
  const out: Record<string, SourceContribution[]> = {}
  for (const [k, list] of Object.entries(raw)) {
    out[k] = list.map(convertContribution)
  }
  return out
}

function toLegacyBuildStats(raw: BuildStatsRustOutput): ComputedStats {
  return {
    attributes: Object.fromEntries(
      Object.entries(raw.attributes).map(([k, v]) => [k, asRangedValue(v)]),
    ),
    stats: Object.fromEntries(
      Object.entries(raw.stats).map(([k, v]) => [k, asRangedValue(v)]),
    ),
    attributeSources: convertSourceMap(raw.attributeSources),
    statSources: convertSourceMap(raw.statSources),
  }
}

// Adds per-stat source breakdown on top of computeBuildPerformanceAsync.
export async function computeBuildStatsAsync(
  deps: BuildPerformanceDeps,
): Promise<ComputedStats> {
  const raw = await invoke<BuildStatsRustOutput>('calc_build_stats', {
    input: depsToInput(deps),
  })
  return toLegacyBuildStats(raw)
}

// ---------- calc_stat_breakdown (per-key explainability) ----------

export interface StatTypeSubtotal {
  sourceType: SourceType
  sum: RangedValue
  count: number
}

export interface StatBreakdown {
  statKey: string
  statName: string
  isPercent: boolean
  hasMore: boolean
  hasIncreased: boolean
  additiveSum: RangedValue
  additiveSources: SourceContribution[]
  additiveByType: StatTypeSubtotal[]
  increasedSum: RangedValue
  increasedSources: SourceContribution[]
  increasedByType: StatTypeSubtotal[]
  moreSum: RangedValue
  moreSources: SourceContribution[]
  moreByType: StatTypeSubtotal[]
  combined: RangedValue
}

export type StatBreakdownKind = 'stat' | 'attribute'

interface RustStatTypeSubtotal {
  sourceType: SourceType
  sum: RustRanged
  count: number
}

interface RustStatBreakdown {
  statKey: string
  statName: string
  isPercent: boolean
  hasMore: boolean
  hasIncreased: boolean
  additiveSum: RustRanged
  additiveSources: RustSourceContribution[]
  additiveByType: RustStatTypeSubtotal[]
  increasedSum: RustRanged
  increasedSources: RustSourceContribution[]
  increasedByType: RustStatTypeSubtotal[]
  moreSum: RustRanged
  moreSources: RustSourceContribution[]
  moreByType: RustStatTypeSubtotal[]
  combined: RustRanged
}

function convertSubtotal(raw: RustStatTypeSubtotal): StatTypeSubtotal {
  return {
    sourceType: raw.sourceType,
    sum: asRangedValue(raw.sum),
    count: raw.count,
  }
}

function toLegacyStatBreakdown(raw: RustStatBreakdown): StatBreakdown {
  return {
    statKey: raw.statKey,
    statName: raw.statName,
    isPercent: raw.isPercent,
    hasMore: raw.hasMore,
    hasIncreased: raw.hasIncreased,
    additiveSum: asRangedValue(raw.additiveSum),
    additiveSources: raw.additiveSources.map(convertContribution),
    additiveByType: raw.additiveByType.map(convertSubtotal),
    increasedSum: asRangedValue(raw.increasedSum),
    increasedSources: raw.increasedSources.map(convertContribution),
    increasedByType: raw.increasedByType.map(convertSubtotal),
    moreSum: asRangedValue(raw.moreSum),
    moreSources: raw.moreSources.map(convertContribution),
    moreByType: raw.moreByType.map(convertSubtotal),
    combined: asRangedValue(raw.combined),
  }
}

// Re-runs the same pipeline as computeBuildStatsAsync to guarantee identical numbers, then collapses one key into per-source-type subtotals + combined.
export async function computeStatBreakdownAsync(
  deps: BuildPerformanceDeps,
  statKey: string,
  kind: StatBreakdownKind = 'stat',
): Promise<StatBreakdown> {
  const raw = await invoke<RustStatBreakdown>('calc_stat_breakdown', {
    input: {
      ...depsToInput(deps),
      statKey,
      kind,
    },
  })
  return toLegacyStatBreakdown(raw)
}

// Escape hatch for unwrapped commands.
export function invokeCalc<TResult>(
  cmd: string,
  args: Record<string, unknown> = {},
): Promise<TResult> {
  return invoke<TResult>(cmd, args)
}
