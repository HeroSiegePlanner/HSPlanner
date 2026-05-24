import treeGraph from '../data/hero-siege-tree.json'
import treeNodeInfo from '../data/tree-nodes.json'
import nodeIconsMap from '../data/node-icons.json'

export interface TreeNodeEntry {
  id: number
  x: number
  y: number
  r: number
  name: string
  kind: string
  iconUrl?: string
}

const NODE_INFO_BY_ID: Record<string, { t: string; n: string }> =
  treeNodeInfo as Record<string, { t: string; n: string }>

// Eagerly resolved sprite URLs keyed by the icon-file name (without extension).
// Vite's import.meta.glob copies the PNG into the bundle and returns its public
// URL; matching keys come from node-icons.json (`{nodeId: "Strength_Life_..."}`)
// We mirror TreeView's resolution scheme so the mini-map shows the same art.
const NODE_ICON_FILES = import.meta.glob<string>(
  '../assets/atlas/nodes/*.png',
  { eager: true, query: '?url', import: 'default' },
)
const NODE_ICON_URL_BY_KEY: Record<string, string> = {}
for (const [path, url] of Object.entries(NODE_ICON_FILES)) {
  const file = path.split('/').pop() ?? ''
  const key = file.replace(/\.png$/i, '')
  NODE_ICON_URL_BY_KEY[key] = url
}
const NODE_ICON_KEY_BY_ID = nodeIconsMap as Record<string, string>

// Flat list of every tree node with its (x, y, r) and resolved display
// name + sprite URL. Built once at module-load; ~1.6k entries. The JSON
// shape is `[id, x, y, r]` per node, but TS infers it as `number[]` so we
// cast through a tuple type before destructuring.
type RawNode = [number, number, number, number]
export const ALL_TREE_NODES: TreeNodeEntry[] = (
  treeGraph.nodes as RawNode[]
).map(([id, x, y, r]) => {
  const iconKey = NODE_ICON_KEY_BY_ID[String(id)]
  return {
    id,
    x,
    y,
    r,
    name: NODE_INFO_BY_ID[String(id)]?.t ?? '',
    kind: NODE_INFO_BY_ID[String(id)]?.n ?? '',
    iconUrl: iconKey ? NODE_ICON_URL_BY_KEY[iconKey] : undefined,
  }
})

// Raw edge segments as `[x1, y1, x2, y2]`. Each tree edge is a single line
// between two nodes; we don't need adjacency here because the mini-map just
// paints the silhouette without highlighting allocated paths.
export type TreeEdge = readonly [number, number, number, number]
export const ALL_TREE_EDGES: TreeEdge[] = treeGraph.edges as unknown as TreeEdge[]

// `viewBox` of the tree SVG, split into numeric components for reuse.
// Uses `?? 0` fallbacks because TS treats `arr[n]` as possibly-undefined
// under noUncheckedIndexedAccess; in practice every viewBox we ship has 4
// components.
export const TREE_VIEWBOX: { x: number; y: number; w: number; h: number } = (() => {
  const parts = treeGraph.viewBox.split(' ').map(Number)
  return {
    x: parts[0] ?? 0,
    y: parts[1] ?? 0,
    w: parts[2] ?? 0,
    h: parts[3] ?? 0,
  }
})()

// Reverse-lookup by display name. First-write-wins when several nodes share a
// name (e.g. repeated "+3% Movement Speed" minor nodes) so the highlight
// at least lands on a plausible one.
const TREE_NODE_BY_NAME: Map<string, TreeNodeEntry> = (() => {
  const m = new Map<string, TreeNodeEntry>()
  for (const n of ALL_TREE_NODES) {
    if (!n.name) continue
    if (!m.has(n.name)) m.set(n.name, n)
  }
  return m
})()

export function findTreeNodeByName(name: string): TreeNodeEntry | undefined {
  return TREE_NODE_BY_NAME.get(name)
}

export function findTreeNodeById(id: number): TreeNodeEntry | undefined {
  return ALL_TREE_NODES.find((n) => n.id === id)
}
