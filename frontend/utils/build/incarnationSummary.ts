import { getAffix, getGem, getRune, incarnationNodeInfo, incarnationTree } from '@data'
import { classifyTier } from '../../views/tree/treeData'
import { formatValue, socketableStatLines, statName } from '../item/stats'
import type { TreeSocketContent } from '../../types'
import { socketIconPath } from './assetPaths'
import type { Incarnation, IncarnationJewelry } from './sharePayload'

const STAT_SEPARATOR = ' · '
const EMPTY_SOCKET_LINE = '—'
const KEYSTONE_TIER = 'keystone'

function jewelryEntry(content: TreeSocketContent | null): IncarnationJewelry {
  if (!content) return { name: 'Empty socket', line: EMPTY_SOCKET_LINE }

  if (content.kind === 'item') {
    const source = getGem(content.id) ?? getRune(content.id)
    if (!source) return { name: 'Unknown socketable', line: content.id }
    const line = socketableStatLines(source.stats)
      .map((s) => s.text)
      .join(STAT_SEPARATOR)
    return { name: source.name, icon: socketIconPath(source.name), line }
  }

  const line = content.affixes
    .map((eq) => {
      const def = getAffix(eq.affixId)
      if (!def?.statKey || def.valueMin == null || def.valueMax == null) return null
      return `${formatValue([def.valueMin, def.valueMax], def.statKey)} ${statName(def.statKey)}`
    })
    .filter((text): text is string => text !== null)
    .join(STAT_SEPARATOR)
  return { name: 'Uncut Jewel', line }
}

export function buildIncarnation(
  allocatedTreeNodes: ReadonlySet<number>,
  treeSocketed: Record<number, TreeSocketContent | null>,
): Incarnation {
  const keystones: Incarnation['keystones'] = []
  const notables: Incarnation['notables'] = []
  const minorCounts = new Map<string, number>()
  const jewelry: Incarnation['jewelry'] = []

  for (const node of incarnationTree.nodes) {
    if (!allocatedTreeNodes.has(node.id)) continue
    const info = incarnationNodeInfo[String(node.id)]
    if (!info) continue

    switch (info.n) {
      case 'root':
      case 'warp':
        break
      case 'jewelry':
        jewelry.push(jewelryEntry(treeSocketed[node.id] ?? null))
        break
      case 'big':
        if (classifyTier(node.r) === KEYSTONE_TIER) {
          keystones.push({ name: info.t, lines: [...info.l] })
        } else {
          notables.push({ name: info.t, line: info.l.join(STAT_SEPARATOR) })
        }
        break
      default: {
        const text = info.l.join(STAT_SEPARATOR)
        if (text) minorCounts.set(text, (minorCounts.get(text) ?? 0) + 1)
      }
    }
  }

  const minors = [...minorCounts.entries()]
    .map(([text, count]) => ({ text, count }))
    .sort((a, b) => b.count - a.count)
  const allocated = allocatedTreeNodes.size
  const minorTotal = minors.reduce((sum, m) => sum + m.count, 0)

  return {
    countLabel: `${allocated} / ${incarnationTree.nodes.length} nodes`,
    tabLabel: `${allocated} nodes`,
    keystones,
    notables,
    minors,
    jewelry,
    summaryLabel: `${allocated} nodes · ${keystones.length} keystones · ${notables.length} notables · ${minorTotal} minors · ${jewelry.length} jewelry`,
  }
}
