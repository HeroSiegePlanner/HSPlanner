import { heroSiegeTree, nodeIcons } from '../../data'
import { TREE_JEWELRY_IDS, TREE_NODE_INFO } from '../../utils/tree/treeStats'
import type { TooltipTone } from '../../components/tooltip-tones'

type RawNode = [id: number, x: number, y: number, r: number]
type RawEdge = [x1: number, y1: number, x2: number, y2: number]

export interface TreeNode {
  id: number
  x: number
  y: number
  r: number
  tier: 'minor' | 'notable' | 'keystone'
}

export function tierTone(
  nodeType: string | undefined,
  tier: TreeNode['tier'],
): TooltipTone {
  if (nodeType === 'jewelry') return 'rare'
  if (nodeType === 'warp') return 'rare'
  if (nodeType === 'root') return 'angelic'
  if (nodeType === 'big' || tier === 'keystone' || tier === 'notable') return 'rare'
  return 'neutral'
}

export function tierLabel(
  nodeType: string | undefined,
  tier: TreeNode['tier'],
): string {
  if (nodeType === 'jewelry') return 'Jewelry Socket'
  if (nodeType === 'warp') return 'Warp Node'
  if (nodeType === 'root') return 'Starting Node'
  if (nodeType === 'big') return 'Notable'
  if (tier === 'keystone') return 'Keystone'
  return 'Minor'
}

export function classifyTier(r: number): TreeNode['tier'] {
  if (r >= 12) return 'keystone'
  if (r >= 10) return 'notable'
  return 'minor'
}

const VIEW_BOX = heroSiegeTree.viewBox
export const NODES: TreeNode[] = (heroSiegeTree.nodes as RawNode[]).map(
  ([id, x, y, r]) => ({
    id,
    x,
    y,
    r,
    tier: classifyTier(r),
  }),
)
export const EDGES: RawEdge[] = heroSiegeTree.edges as RawEdge[]

export const [vbX = 0, vbY = 0, vbW = 1000, vbH = 800] = VIEW_BOX.split(' ').map(
  Number,
)

export const POS_ID_MAP = (() => {
  const m = new Map<string, number>()
  for (const n of NODES) {
    m.set(`${Math.round(n.x * 10)}_${Math.round(n.y * 10)}`, n.id)
  }
  return m
})()

export const SEARCH_INDEX: { id: number; haystack: string }[] = Object.entries(
  TREE_NODE_INFO,
).map(([id, info]) => ({
  id: Number(id),
  haystack: (info.t + ' ' + info.l.join(' ')).toLowerCase(),
}))

const NODE_ICON_FILES = import.meta.glob<string>(
  '../../assets/atlas/nodes/*.png',
  { eager: true, query: '?url', import: 'default' },
)
const NODE_ICON_URL_BY_KEY: Record<string, string> = {}
for (const [p, url] of Object.entries(NODE_ICON_FILES)) {
  const file = p.split('/').pop() ?? ''
  const key = file.replace(/\.png$/i, '')
  NODE_ICON_URL_BY_KEY[key] = url
}

const NODE_ICON_KEY_BY_ID: Record<string, string> = nodeIcons
export const NODE_ICONS: {
  id: number
  x: number
  y: number
  r: number
  href: string
}[] = NODES.flatMap((n) => {
  const key = NODE_ICON_KEY_BY_ID[String(n.id)]
  const href = key ? NODE_ICON_URL_BY_KEY[key] : undefined
  return href ? [{ id: n.id, x: n.x, y: n.y, r: n.r, href }] : []
})

export const JEWELRY_NODES = NODES.filter((n) => TREE_JEWELRY_IDS.has(n.id))
