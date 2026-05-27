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

export type TreeEdge = readonly [number, number, number, number]
export const ALL_TREE_EDGES: TreeEdge[] = treeGraph.edges as unknown as TreeEdge[]

export const TREE_VIEWBOX: { x: number; y: number; w: number; h: number } = (() => {
  const parts = treeGraph.viewBox.split(' ').map(Number)
  return {
    x: parts[0] ?? 0,
    y: parts[1] ?? 0,
    w: parts[2] ?? 0,
    h: parts[3] ?? 0,
  }
})()

// First-write-wins when multiple nodes share a name.
const TREE_NODE_BY_NAME: Map<string, TreeNodeEntry> = (() => {
  const m = new Map<string, TreeNodeEntry>()
  for (const n of ALL_TREE_NODES) {
    if (!n.name) continue
    if (!m.has(n.name)) m.set(n.name, n)
  }
  return m
})()

// O(1) id lookup — built once at module load. Replaces a per-call O(n) scan
// that ran for every tree-source row in every SourceItem render.
const TREE_NODE_BY_ID: Map<number, TreeNodeEntry> = (() => {
  const m = new Map<number, TreeNodeEntry>()
  for (const n of ALL_TREE_NODES) {
    m.set(n.id, n)
  }
  return m
})()

export function findTreeNodeByName(name: string): TreeNodeEntry | undefined {
  return TREE_NODE_BY_NAME.get(name)
}

export function findTreeNodeById(id: number): TreeNodeEntry | undefined {
  return TREE_NODE_BY_ID.get(id)
}
