import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { gameConfig, getSkillsByClass, getClass } from '../../data'
import type { Skill } from '../../types'
import { normalizeSkillName, rangedMax, rangedMin } from '../item/stats'
import { computeBuildStatsAsync } from '../../lib/calc/bridge'
import { ADJ, START_IDS } from './treeGraph'
import {
  TREE_JEWELRY_IDS,
  TREE_NODE_INFO,
  TREE_WARP_IDS,
  type TreeNodeInfo,
} from './treeStats'
import { VALUABLE_NODE_IDS } from './treeSuggest'
import type { BuildPerformanceDeps } from '../build/buildPerformance'
import type { RangedValue } from '../../types'

type Ranged = [number, number]

interface NativeSuggestStep {
  nodeId: number
  dpsBefore: number
  dpsAfter: number
  gain: number
  isFiller: boolean
}

export interface NativeSuggestResult {
  addedNodes: number[]
  sequence: NativeSuggestStep[]
  baseDps: number
  finalDps: number
  budgetUsed: number
  budgetRequested: number
  unsupportedLines: string[]
  usedStarts: number[]
}

function toRanged(value: RangedValue | undefined): Ranged {
  if (value === undefined) return [0, 0]
  return [rangedMin(value), rangedMax(value)]
}

function contributionsRecord(
  src: Record<string, Array<{ value: RangedValue }>> | undefined,
): Record<string, Ranged[]> {
  const out: Record<string, Ranged[]> = {}
  if (!src) return out
  for (const [k, sources] of Object.entries(src)) {
    if (!sources || sources.length === 0) continue
    const list: Ranged[] = []
    for (const s of sources) {
      const r = toRanged(s.value)
      if (r[0] === 0 && r[1] === 0) continue
      list.push(r)
    }
    if (list.length > 0) out[k] = list
  }
  return out
}

function skillRef(skill: Skill) {
  return {
    id: skill.id,
    name: skill.name,
    tags: skill.tags ?? [],
    damageType: skill.damageType ?? undefined,
    kind: skill.kind ?? undefined,
    damageFormula: skill.damageFormula ?? undefined,
    damagePerRank: skill.damagePerRank ?? undefined,
    bonusSources: skill.bonusSources ?? [],
    baseCastRate: skill.baseCastRate ?? undefined,
    proc: skill.proc
      ? {
          chance: skill.proc.chance,
          trigger: skill.proc.trigger,
          target: skill.proc.target,
        }
      : undefined,
  }
}

const GRAPH_PAYLOAD = (() => {
  const adjacency: Record<string, number[]> = {}
  for (const [id, set] of ADJ) {
    adjacency[String(id)] = Array.from(set)
  }
  return {
    adjacency,
    startIds: Array.from(START_IDS),
    warpIds: Array.from(TREE_WARP_IDS),
    valuableIds: Array.from(VALUABLE_NODE_IDS),
    jewelryIds: Array.from(TREE_JEWELRY_IDS),
  }
})()

const TREE_NODES_PAYLOAD: Record<string, TreeNodeInfo> =
  TREE_NODE_INFO as Record<string, TreeNodeInfo>

const GAME_CONFIG_PAYLOAD = {
  attributeKeys: (gameConfig.attributes ?? []).map((a) => a.key),
  defaultBaseAttributes: gameConfig.defaultBaseAttributes ?? {},
  defaultBaseStats: gameConfig.defaultBaseStats ?? {},
  defaultStatsPerAttribute: gameConfig.defaultStatsPerAttribute ?? {},
  attributeDividedStats: gameConfig.attributeDividedStats ?? {},
}

interface ProgressPayload {
  current: number
  total: number
}

export async function suggestNodesNative(
  deps: BuildPerformanceDeps,
  currentAllocation: Set<number>,
  budget: number,
  onProgress?: (current: number, total: number) => void,
): Promise<NativeSuggestResult> {
  // Compute baseline build stats WITHOUT tree (so Rust can incrementally add tree mods).
  const baseline = await computeBuildStatsAsync({
    ...deps,
    allocatedTreeNodes: new Set<number>(),
    treeSocketed: {},
  })

  const statContributions = contributionsRecord(baseline.statSources)
  const attrContributions = contributionsRecord(baseline.attributeSources)

  const cls = deps.classId ? getClass(deps.classId) : undefined
  const classInfo = cls
    ? {
        id: cls.id,
        name: cls.name,
        baseAttributes: cls.baseAttributes ?? {},
        baseStats: cls.baseStats ?? {},
        statsPerLevel: cls.statsPerLevel ?? {},
      }
    : undefined

  const allClassSkills = getSkillsByClass(deps.classId)
  const activeSkill = deps.mainSkillId
    ? allClassSkills.find((s) => s.id === deps.mainSkillId)
    : null

  const activeSkillRank = activeSkill
    ? (deps.skillRanks[activeSkill.id] ?? 0)
    : 0

  const skillRanksByName: Record<string, number> = {}
  for (const s of allClassSkills) {
    skillRanksByName[normalizeSkillName(s.name)] = deps.skillRanks[s.id] ?? 0
  }

  const enemyResistances: Record<string, number> = {}
  for (const [k, v] of Object.entries(deps.enemyResistances ?? {})) {
    enemyResistances[k] = v
  }

  const projectileCount =
    activeSkill && deps.skillProjectiles[activeSkill.id] !== undefined
      ? deps.skillProjectiles[activeSkill.id]
      : 1

  const input = {
    class: classInfo,
    level: deps.level,
    allocatedAttributes: deps.allocatedAttrs,
    statContributions,
    attrContributions,
    graph: GRAPH_PAYLOAD,
    treeNodes: TREE_NODES_PAYLOAD,
    allocatedTreeNodes: Array.from(currentAllocation),
    activeSkill: activeSkill ? skillRef(activeSkill) : undefined,
    activeSkillRank: Math.max(0, Math.floor(activeSkillRank)),
    skillRanksByName,
    inventory: deps.inventory,
    enemyConditions: deps.enemyConditions ?? {},
    playerConditions: deps.playerConditions ?? {},
    enemyResistances,
    projectileCount,
    budget,
    allSkills: allClassSkills.map(skillRef),
    gameConfig: GAME_CONFIG_PAYLOAD,
    procToggles: deps.procToggles ?? {},
    skillRanksById: deps.skillRanks ?? {},
    skillProjectiles: deps.skillProjectiles ?? {},
    killsPerSec: deps.killsPerSec ?? 0,
  }

  let unlisten: UnlistenFn | null = null
  if (onProgress) {
    unlisten = await listen<ProgressPayload>('suggest-progress', (e) => {
      onProgress(e.payload.current, e.payload.total)
    })
  }
  try {
    return await invoke<NativeSuggestResult>('suggest_tree_nodes', { input })
  } finally {
    if (unlisten) unlisten()
  }
}
