import { etherTree } from '@data'
import type { EtherNode, EtherNodeType } from '../../types'

export const ETHER_NODES: EtherNode[] = etherTree.nodes
export const ETHER_EDGES: [number, number][] = etherTree.edges

export const [vbX = 0, vbY = 0, vbW = 1000, vbH = 800] = etherTree.viewBox
  .split(' ')
  .map(Number)

const ICON_FILES = import.meta.glob<string>('../../assets/atlas/nodes/*.png', {
  eager: true,
  query: '?url',
  import: 'default',
})
const ICON_URL_BY_KEY: Record<string, string> = {}
for (const [p, url] of Object.entries(ICON_FILES)) {
  const file = p.split('/').pop() ?? ''
  ICON_URL_BY_KEY[file.replace(/\.png$/i, '')] = url
}

export const ETHER_NODE_ICONS: {
  id: number
  x: number
  y: number
  r: number
  href: string
}[] = ETHER_NODES.flatMap((n) => {
  const href = ICON_URL_BY_KEY[n.icon]
  return href ? [{ id: n.id, x: n.x, y: n.y, r: n.r, href }] : []
})

export const ETHER_SEARCH_INDEX: { id: number; haystack: string }[] =
  ETHER_NODES.map((n) => {
    const stat = etherTree.stats[n.key]
    return {
      id: n.id,
      haystack: `${stat?.label ?? ''} ${stat?.desc ?? ''}`.toLowerCase(),
    }
  })

export function etherTypeLabel(t: EtherNodeType): string {
  if (t === 'root') return 'Starting Node'
  if (t === 'big') return 'Notable'
  return 'Minor'
}

export interface EtherNodePaint {
  isAlloc: boolean
  isPreview: boolean
  isRoot: boolean
  isBig: boolean
}

export function etherNodeFill({ isAlloc, isPreview, isRoot }: EtherNodePaint): string {
  if (isAlloc) return '#a574c9'
  if (isPreview) return 'rgba(165,116,201,0.28)'
  if (isRoot) return '#2e2838'
  return '#1c1d24'
}

export function etherNodeStroke({
  isAlloc,
  isPreview,
  isRoot,
  isBig,
}: EtherNodePaint): string {
  if (isAlloc) return '#d4cfbf'
  if (isPreview) return '#a574c9'
  if (isRoot) return '#c9a55a'
  if (isBig) return '#5a5448'
  return '#3a3528'
}

export function etherStrokeWidth(isRoot: boolean, isBig: boolean): number {
  if (isRoot) return 2.5
  return isBig ? 2 : 1
}

export const ETHER_EDGE_BASE = '#262b36'
export const ETHER_EDGE_ALLOC = '#8a5fb0'
export const ETHER_EDGE_PREVIEW = '#5a4a6e'
