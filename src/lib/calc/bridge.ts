// TS-to-Rust calc bridge.

import { invoke } from '@tauri-apps/api/core'

import type { CustomStat, Inventory, RangedValue, Skill, TreeSocketContent } from '../../types'
import type { AttackSkillDamageBreakdown, SkillDamageBreakdown } from '../../utils/item/stats'
import type {
  BuildPerformance,
  BuildPerformanceDeps,
} from '../../utils/build/buildPerformance'
import type {
  ComputedStats,
  SourceContribution,
  SourceType,
} from '../../utils/item/stats'
import type { ForgeKind } from '../../data'
import { activeSeasonId } from '../../data'

// Single subscriber so the build store can surface Rust rejections via storageError.
type BridgeErrorListener = (err: Error) => void
let bridgeErrorListener: BridgeErrorListener | null = null

export function setBridgeErrorListener(fn: BridgeErrorListener | null): void {
  bridgeErrorListener = fn
}

// Exported so sibling IPC wrappers (e.g. utils/nativeDamage.ts) surface
// failures through the same storageError banner.
export function notifyBridgeError(err: unknown): Error {
  const wrapped = err instanceof Error ? err : new Error(String(err))
  if (bridgeErrorListener) {
    try {
      bridgeErrorListener(wrapped)
    } catch {
      /* swallow listener faults */
    }
  }
  return wrapped
}

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
  season?: string
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
  statsCombined: Record<string, RustRanged>
  itemSkillBonuses: Record<string, RustRanged>
  rankBonuses: Record<string, RustRanged>
}

async function computeBuildPerformanceNative(
  input: BuildPerformanceInput,
): Promise<BuildPerformanceOutput> {
  try {
    return await invoke<BuildPerformanceOutput>('calc_build_performance', { input })
  } catch (err) {
    throw notifyBridgeError(err)
  }
}

// Collapse [n, n] to n so legacy TS callers see the same shape.
function asRangedValue([min, max]: RustRanged): RangedValue {
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
    season: deps.season ?? activeSeasonId,
  }
}

// Converts a Rust [min, max] record into the legacy "number | [number, number]"
// map that the rest of the TS code expects.
function toRangedMap(
  rec: Record<string, RustRanged>,
): Record<string, RangedValue> {
  return Object.fromEntries(
    Object.entries(rec).map(([k, v]) => [k, asRangedValue(v)]),
  )
}

function toLegacyBuildPerformance(
  raw: BuildPerformanceOutput,
): BuildPerformance {
  return {
    attributes: toRangedMap(raw.attributes),
    stats: toRangedMap(raw.stats),
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
    statsCombined: toRangedMap(raw.statsCombined),
    itemSkillBonuses: raw.itemSkillBonuses,
    rankBonuses: raw.rankBonuses,
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
  statsCombined: Record<string, RustRanged>
  itemSkillBonuses: Record<string, RustRanged>
  rankBonuses: Record<string, RustRanged>
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
    attributes: toRangedMap(raw.attributes),
    stats: toRangedMap(raw.stats),
    attributeSources: convertSourceMap(raw.attributeSources),
    statSources: convertSourceMap(raw.statSources),
    statsCombined: toRangedMap(raw.statsCombined),
    itemSkillBonuses: raw.itemSkillBonuses,
    rankBonuses: raw.rankBonuses,
  }
}

// Adds per-stat source breakdown on top of computeBuildPerformanceAsync.
export async function computeBuildStatsAsync(
  deps: BuildPerformanceDeps,
): Promise<ComputedStats> {
  try {
    const raw = await invoke<BuildStatsRustOutput>('calc_build_stats', {
      input: depsToInput(deps),
    })
    return toLegacyBuildStats(raw)
  } catch (err) {
    throw notifyBridgeError(err)
  }
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
  try {
    const raw = await invoke<RustStatBreakdown>('calc_stat_breakdown', {
      input: {
        ...depsToInput(deps),
        statKey,
        kind,
      },
    })
    return toLegacyStatBreakdown(raw)
  } catch (err) {
    throw notifyBridgeError(err)
  }
}

// ---------- passive_stats_at_rank / mana_cost_at_rank ----------
// Replace the former TS stats.ts helpers; the math now lives in Rust calc/passive.rs.

export async function passiveStatsAtRankNative(
  skill: Skill,
  rank: number,
): Promise<Record<string, number>> {
  try {
    return await invoke('passive_stats_at_rank', { skill, rank })
  } catch (err) {
    throw notifyBridgeError(err)
  }
}

export async function manaCostAtRankNative(
  skill: Skill,
  rank: number,
): Promise<number | null> {
  try {
    return await invoke('mana_cost_at_rank', { skill, rank })
  } catch (err) {
    throw notifyBridgeError(err)
  }
}

// ---------- parse_custom_stats ----------
// Replaces the former TS parseCustomStatValue; the parser lives in Rust
// calc/custom_stat.rs.

export async function parseCustomStatsNative(
  values: string[],
): Promise<([number, number] | null)[]> {
  try {
    return await invoke<([number, number] | null)[]>('parse_custom_stats', {
      values,
    })
  } catch (err) {
    throw notifyBridgeError(err)
  }
}

// ---------- display_values ----------
// Batched affix/star display math; replaces the former TS
// rolledAffixValue*/applyStarsToRangedValue helpers.

export interface AffixValueRequest {
  affix: unknown
  roll?: number
  stars?: number | null
}

export interface ScaledValueRequest {
  value: [number, number]
  statKey: string
  stars?: number | null
}

export interface AffixValueOutput {
  value: number
  rangeMin: number
  rangeMax: number
}

export interface DisplayValuesOutput {
  affixes: AffixValueOutput[]
  scaled: [number, number][]
}

export async function displayValuesNative(input: {
  affixes?: AffixValueRequest[]
  scaled?: ScaledValueRequest[]
}): Promise<DisplayValuesOutput> {
  try {
    return await invoke<DisplayValuesOutput>('display_values', {
      input,
      season: activeSeasonId,
    })
  } catch (err) {
    throw notifyBridgeError(err)
  }
}

// ---------- classify_tree_nodes ----------
// Replaces the former TS classifyNodeLines RULES engine; the regexes live in
// Rust calc/tree/parse.rs. Fetched once and cached by TreeView.

export interface NodeLineClassification {
  parsed: string[]
  unsupported: string[]
}

export async function classifyTreeNodesNative(): Promise<
  Record<string, NodeLineClassification>
> {
  try {
    return await invoke<Record<string, NodeLineClassification>>(
      'classify_tree_nodes',
      { season: activeSeasonId },
    )
  } catch (err) {
    throw notifyBridgeError(err)
  }
}

// ---------- subskill_aggregation ----------
// Replaces the former TS subtree.ts aggregation; the math lives in Rust
// calc/subskill.rs.

export interface AppliedStateInfo {
  state: string
  trigger: string
  chance: number
  amount?: number
}

export interface SubtreeAggregation {
  stats: Record<string, number>
  procStats: Record<string, number>
  appliedStates: AppliedStateInfo[]
}

interface RustAppliedState {
  state: string
  trigger: string
  chance: number
  amount: number | null
}

interface SubskillAggregationRustOutput {
  stats: Record<string, number>
  procStats: Record<string, number>
  appliedStates: RustAppliedState[]
}

export async function subskillAggregationNative(
  classId: string,
  skillId: string,
  subskillRanks: Record<string, number>,
  enemyConditions: Record<string, boolean>,
): Promise<SubtreeAggregation> {
  try {
    const raw = await invoke<SubskillAggregationRustOutput>(
      'subskill_aggregation',
      {
        input: {
          classId,
          skillId,
          subskillRanks,
          enemyConditions,
          season: activeSeasonId,
        },
      },
    )
    return {
      stats: raw.stats,
      procStats: raw.procStats,
      appliedStates: raw.appliedStates.map((s) => ({
        state: s.state,
        trigger: s.trigger,
        chance: s.chance,
        ...(s.amount !== null ? { amount: s.amount } : {}),
      })),
    }
  } catch (err) {
    throw notifyBridgeError(err)
  }
}

// Test seam: depsToInput is module-private; expose it for unit tests only.
export const __depsToInputForTest = depsToInput
