import { incarnationNodeInfo, incarnationTree } from '@data'

export interface TreeNodeEntry {
  id: number
  x: number
  y: number
  r: number
  name: string
  kind: string
  iconUrl?: string
}

const NODE_ICON_FILES = import.meta.glob<string>(
  '../../assets/atlas/nodes/*.png',
  { eager: true, query: '?url', import: 'default' },
)
const NODE_ICON_URL_BY_KEY: Record<string, string> = {}
for (const [path, url] of Object.entries(NODE_ICON_FILES)) {
  const file = path.split('/').pop() ?? ''
  const key = file.replace(/\.png$/i, '')
  NODE_ICON_URL_BY_KEY[key] = url
}

export const ALL_TREE_NODES: TreeNodeEntry[] = incarnationTree.nodes.map(
  (n) => ({
    id: n.id,
    x: n.x,
    y: n.y,
    r: n.r,
    name: incarnationNodeInfo[String(n.id)]?.t ?? '',
    kind: incarnationNodeInfo[String(n.id)]?.n ?? '',
    iconUrl: NODE_ICON_URL_BY_KEY[n.icon],
  }),
)

export const ALL_TREE_EDGES: ReadonlyArray<[number, number]> =
  incarnationTree.edges

const TREE_NODE_BY_NAME: Map<string, TreeNodeEntry> = (() => {
  const m = new Map<string, TreeNodeEntry>()
  for (const n of ALL_TREE_NODES) {
    if (!n.name) continue
    if (!m.has(n.name)) m.set(n.name, n)
  }
  return m
})()

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
