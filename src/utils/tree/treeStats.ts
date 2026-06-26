import { treeNodeInfo } from '../../data'
import type { TreeNodeInfo } from '../../data/seasons/patchTypes'

export type { TreeNodeInfo }

export const TREE_NODE_INFO = treeNodeInfo

export const TREE_WARP_IDS = new Set<number>(
  Object.entries(TREE_NODE_INFO)
    .filter(([, info]) => info.n === 'warp')
    .map(([id]) => Number(id)),
)

export const TREE_JEWELRY_IDS = new Set<number>(
  Object.entries(TREE_NODE_INFO)
    .filter(([, info]) => info.n === 'jewelry')
    .map(([id]) => Number(id)),
)

export type SelfConditionKey = 'crit_chance_below_40' | 'life_below_40'

export const SELF_CONDITION_LABELS: Record<SelfConditionKey, string> = {
  crit_chance_below_40: 'Critical Strike Chance is below 40% (auto)',
  life_below_40: 'Current Life is below 40% of Maximum',
}

export const SELF_CONDITION_KEYS: SelfConditionKey[] = [
  'crit_chance_below_40',
  'life_below_40',
]
