import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { depsToInput, type BuildPerformanceInput } from '../calc/bridge'
import { ADJ, START_IDS } from './treeGraph'
import { TREE_JEWELRY_IDS, TREE_WARP_IDS } from './treeStats'
import { VALUABLE_NODE_IDS } from './treeSuggest'
import type { BuildPerformanceDeps } from '../build/buildPerformance'

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

interface SuggestGraphPayload {
  adjacency: Record<string, number[]>
  startIds: number[]
  warpIds: number[]
  valuableIds: number[]
  jewelryIds: number[]
}

export interface SuggestPayload {
  perf: BuildPerformanceInput
  activeSkillIds: string[]
  graph: SuggestGraphPayload
  budget: number
}

function toAdjacency(adj: Map<number, Set<number>>): Record<string, number[]> {
  const adjacency: Record<string, number[]> = {}
  for (const [id, set] of adj) {
    adjacency[String(id)] = Array.from(set)
  }
  return adjacency
}

const GRAPH_PAYLOAD: SuggestGraphPayload = {
  adjacency: toAdjacency(ADJ),
  startIds: Array.from(START_IDS),
  warpIds: Array.from(TREE_WARP_IDS),
  valuableIds: Array.from(VALUABLE_NODE_IDS),
  jewelryIds: Array.from(TREE_JEWELRY_IDS),
}

export function buildSuggestPayload(
  deps: BuildPerformanceDeps,
  currentAllocation: Set<number>,
  budget: number,
): SuggestPayload {
  return {
    perf: {
      ...depsToInput(deps),
      allocatedTreeNodes: [...currentAllocation],
    },
    activeSkillIds: [...deps.activeSkillIds],
    graph: GRAPH_PAYLOAD,
    budget,
  }
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
  const input = buildSuggestPayload(deps, currentAllocation, budget)
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
