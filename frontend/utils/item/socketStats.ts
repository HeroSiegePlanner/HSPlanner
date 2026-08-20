import { getGem, getRune } from '@data'
import { RAINBOW_MULTIPLIER } from '../../store/itemRules'
import type { EquippedItem, ItemBase, StatMap } from '../../types'

export interface SocketGroup {
  id: string
  name: string
  count: number
  stats: [string, number][]
}

// Some bases (e.g. Tablet of Awakening) have built-in rainbow sockets at fixed
// 1-indexed positions, regardless of the user's per-socket toggle.
export function isRainbowSocket(
  equipped: EquippedItem,
  index: number,
  base?: ItemBase,
): boolean {
  if (base?.rainbowSockets?.includes(index + 1)) return true
  return equipped.socketTypes[index] === 'rainbow'
}

// Sockets holding the same gem/rune are merged into one group (count = how many).
export function collectSocketGroups(
  equipped: EquippedItem,
  base?: ItemBase,
): SocketGroup[] {
  const groups = new Map<string, { name: string; count: number; stats: StatMap }>()
  for (let i = 0; i < equipped.socketed.length; i++) {
    const id = equipped.socketed[i]
    if (!id) continue
    const source = getGem(id) ?? getRune(id)
    if (!source) continue
    const mult = isRainbowSocket(equipped, i, base) ? RAINBOW_MULTIPLIER : 1
    const transform = base?.socketTransforms?.[id]
    const src = transform ?? source.stats
    const group = groups.get(id) ?? { name: source.name, count: 0, stats: {} }
    group.count += 1
    for (const [k, v] of Object.entries(src)) {
      group.stats[k] = (group.stats[k] ?? 0) + v * mult
    }
    groups.set(id, group)
  }
  return Array.from(groups, ([id, g]) => ({
    id,
    name: g.name,
    count: g.count,
    stats: Object.entries(g.stats).filter(([, v]) => v !== 0),
  })).filter((g) => g.stats.length > 0)
}

export function collectSocketStats(
  equipped: EquippedItem,
  base?: ItemBase,
): [string, number][] {
  const stats: StatMap = {}
  for (const group of collectSocketGroups(equipped, base)) {
    for (const [k, v] of group.stats) {
      stats[k] = (stats[k] ?? 0) + v
    }
  }
  return Object.entries(stats).filter(([, v]) => v !== 0)
}
