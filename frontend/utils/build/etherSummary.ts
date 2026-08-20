import { etherTree } from '@data'
import { ETHER_NODE_BY_ID } from '../tree/etherGraph'

export const ETHER_MAGIC_FIND_KEY = 'etherUnSmall01'

export interface EtherSummaryEntry {
  key: string
  label: string
  desc: string
  count: number
  valuePer: number
  total: number
  isPercent: boolean
}

export function parseEtherValue(value: string): {
  num: number
  isPercent: boolean
} {
  const isPercent = value.endsWith('%')
  const num = Number.parseFloat(value)
  return { num: Number.isFinite(num) ? num : 0, isPercent }
}

export function summarizeEtherNodes(
  allocated: Iterable<number>,
): EtherSummaryEntry[] {
  const counts = new Map<string, number>()
  for (const id of allocated) {
    const node = ETHER_NODE_BY_ID.get(id)
    if (!node) continue
    counts.set(node.key, (counts.get(node.key) ?? 0) + 1)
  }
  const out: EtherSummaryEntry[] = []
  for (const [key, count] of counts) {
    const stat = etherTree.stats[key]
    if (!stat) continue
    const { num, isPercent } = parseEtherValue(stat.value)
    out.push({
      key,
      label: stat.label,
      desc: stat.desc,
      count,
      valuePer: num,
      total: Math.round(num * count * 100) / 100,
      isPercent,
    })
  }
  return out.sort(
    (a, b) => b.count - a.count || a.label.localeCompare(b.label),
  )
}

export function etherMagicFindTotal(allocated: Iterable<number>): number {
  let count = 0
  for (const id of allocated) {
    if (ETHER_NODE_BY_ID.get(id)?.key === ETHER_MAGIC_FIND_KEY) count++
  }
  const stat = etherTree.stats[ETHER_MAGIC_FIND_KEY]
  if (!stat) return 0
  return Math.round(parseEtherValue(stat.value).num * count * 100) / 100
}

export function formatEtherTotal(entry: {
  total: number
  isPercent: boolean
}): string {
  const num = Number.isInteger(entry.total)
    ? entry.total
    : Math.round(entry.total * 100) / 100
  return `+${num}${entry.isPercent ? '%' : ''}`
}

const ETHER_REGIONS: Record<string, { label: string; color: string }> = {
  Un: { label: 'Universal', color: '#a574c9' },
  Ow: { label: 'Overworld', color: '#7fd966' },
  Ct: { label: 'Chaos Tower', color: '#e05c5c' },
  Cp: { label: 'Chaos Pillars', color: '#e0985c' },
  SR: { label: 'Shadow Realm', color: '#7a8ce0' },
  Pe: { label: 'Prime Evil', color: '#c94f6d' },
  Ur: { label: 'Unstable Rift', color: '#5cd8d0' },
  Min: { label: 'Mining', color: '#c98a3a' },
  EB: { label: 'Eternal Battlefield', color: '#b8b04a' },
  CS: { label: 'Cursed Spirit', color: '#66d9a8' },
  US: { label: 'Unholy Siege', color: '#e07adb' },
  Dng: { label: 'Dungeons', color: '#8f9bb0' },
  Rg: { label: 'Ruby Gardens', color: '#f27a9d' },
  Cc: { label: 'Colossal Creatures', color: '#e8d84a' },
}

export const ETHER_REGION_FALLBACK_COLOR = '#9aa0ab'

function etherRegionMeta(key: string): { label: string; color: string } {
  const m = key.match(/^ether([A-Z][a-zA-Z]*?)(?:Small|Big)\d/)
  const code = m?.[1]
  return (
    (code && ETHER_REGIONS[code]) || {
      label: 'Other',
      color: ETHER_REGION_FALLBACK_COLOR,
    }
  )
}

export function etherRegionLabel(key: string): string {
  return etherRegionMeta(key).label
}

export function etherRegionColor(key: string): string {
  return etherRegionMeta(key).color
}

export interface EtherSummaryGroup {
  region: string
  color: string
  entries: EtherSummaryEntry[]
}

export function groupEtherSummary(
  entries: EtherSummaryEntry[],
): EtherSummaryGroup[] {
  const byRegion = new Map<string, EtherSummaryGroup>()
  for (const entry of entries) {
    const { label, color } = etherRegionMeta(entry.key)
    const group = byRegion.get(label)
    if (group) group.entries.push(entry)
    else byRegion.set(label, { region: label, color, entries: [entry] })
  }
  return [...byRegion.values()].sort((a, b) => {
    if (a.region === 'Universal') return -1
    if (b.region === 'Universal') return 1
    return a.region.localeCompare(b.region)
  })
}
