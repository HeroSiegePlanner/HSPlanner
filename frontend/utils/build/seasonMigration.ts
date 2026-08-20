import { gems, items, runes, skills } from '@data'
import type { EquippedItem, Inventory, SlotKey } from '../../types'
import type { BuildSnapshot } from './shareBuild'

export function clearSeasonBoundAllocations(snap: BuildSnapshot): BuildSnapshot {
  return {
    ...snap,
    allocatedTreeNodes: new Set<number>(),
    allocatedEtherNodes: new Set<number>(),
    treeSocketed: {},
  }
}

const knownItemIds = new Set(items.map((i) => i.id))
const knownSkillIds = new Set(skills.map((s) => s.id))
const knownSubskillKeys = new Set(
  skills.flatMap((s) => (s.subskills ?? []).map((ss) => `${s.id}:${ss.id}`)),
)
const knownSocketableIds = new Set([
  ...gems.map((g) => g.id),
  ...runes.map((r) => r.id),
])

function pruneItem(item: EquippedItem): EquippedItem | null {
  if (!knownItemIds.has(item.baseId)) return null
  const socketed = item.socketed.map((id) =>
    id && knownSocketableIds.has(id) ? id : null,
  )
  return { ...item, socketed }
}

function pruneInventory(inventory: Inventory): Inventory {
  const out: Inventory = {}
  for (const [slot, item] of Object.entries(inventory)) {
    if (!item) continue
    const pruned = pruneItem(item)
    if (pruned) out[slot as SlotKey] = pruned
  }
  return out
}

function pruneRankMap(
  ranks: Record<string, number>,
  isKnown: (key: string) => boolean,
): Record<string, number> {
  return Object.fromEntries(Object.entries(ranks).filter(([k]) => isKnown(k)))
}

export function pruneUnknownIds(snap: BuildSnapshot): BuildSnapshot {
  return {
    ...snap,
    inventory: pruneInventory(snap.inventory),
    mercInventory: pruneInventory(snap.mercInventory),
    skillRanks: pruneRankMap(snap.skillRanks, (id) => knownSkillIds.has(id)),
    subskillRanks: pruneRankMap(snap.subskillRanks, (key) =>
      knownSubskillKeys.has(key),
    ),
    activeSkillIds: snap.activeSkillIds.filter((id) => knownSkillIds.has(id)),
    activeAuraId:
      snap.activeAuraId && knownSkillIds.has(snap.activeAuraId)
        ? snap.activeAuraId
        : null,
  }
}
