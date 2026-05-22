// All the TS calls to the Rust calc go through here.

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
  // Rust deserialises this array into a HashSet on its side.
  allocatedTreeNodes?: number[]
  // JSON object keys must be strings, so node ids are stringified here.
  treeSocketed?: Record<string, TreeSocketContent>
  mainSkillId?: string | null
  enemyConditions?: Record<string, boolean>
  playerConditions?: Record<string, boolean>
  skillProjectiles?: Record<string, number>
  enemyResistances?: Record<string, number>
  procToggles?: Record<string, boolean>
  killsPerSec?: number
}

// Rust always returns ranges as [min, max] tuples. TS code mostly expects
// "number | [number, number]" — see asRangedValue() below for the conversion.
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

function computeBuildPerformanceNative(
  input: BuildPerformanceInput,
): Promise<BuildPerformanceOutput> {
  return invoke<BuildPerformanceOutput>('calc_build_performance', { input })
}

// Collapse [n, n] back to just n, so the rest of the TS code stays the same.
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

// The TS UI uses Set<number> / Record<number, ...>; Rust wants plain arrays
// and string-keyed objects, so we convert here before sending.
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

// Async replacement for the old TS computeBuildPerformance — same input/output
// shape, just goes through Rust.
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

// Same as computeBuildPerformanceAsync but also returns the per-stat source
// breakdown that StatsView / SkillsView / ItemTooltip render.
export async function computeBuildStatsAsync(
  deps: BuildPerformanceDeps,
): Promise<ComputedStats> {
  const raw = await invoke<BuildStatsRustOutput>('calc_build_stats', {
    input: depsToInput(deps),
  })
  return toLegacyBuildStats(raw)
}
