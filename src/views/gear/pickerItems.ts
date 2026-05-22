import { getItemImage, items } from '../../data'
import { formatValue, statName } from '../../utils/item/stats'
import type { PickerRow } from '../../components/PickerModal'
import type { ItemBase, SlotKey } from '../../types'
import { RARITY_LABEL, RARITY_ORDER } from './lib/rarity'

function slotGroup(slotKey: SlotKey): string {
  // Strips a trailing `_N` suffix from a slot key so paired slots (e.g. `ring_1`, `ring_2`) collapse into a single shared group ("ring"). Used by `itemsForSlot` and the inventory move logic.
  return slotKey.replace(/_\d+$/, '')
}

function buildItemSearchTerms(i: ItemBase): string {
  // Aggregates every searchable surface of a base item (implicits, unique effects, procs, skill bonuses, description, flavor) into a single lowercase string used by the gear PickerModal so users can filter by affix or effect text — not just by item name.
  const parts: string[] = [i.name, i.baseType]
  if (i.grade) parts.push(`Grade ${i.grade}`)
  if (i.implicit) {
    for (const [k, v] of Object.entries(i.implicit)) {
      parts.push(statName(k))
      parts.push(formatValue(v, k))
    }
  }
  if (i.uniqueEffects) parts.push(...i.uniqueEffects)
  if (i.procs) {
    for (const p of i.procs) {
      parts.push(p.description)
      if (p.details) parts.push(p.details)
    }
  }
  if (i.skillBonuses) {
    for (const [skill, val] of Object.entries(i.skillBonuses)) {
      parts.push(skill)
      parts.push(formatValue(val, skill))
    }
  }
  if (i.description) parts.push(i.description)
  if (i.flavor) parts.push(i.flavor)
  if (i.setId) parts.push(i.setId)
  return parts.join(' ').toLowerCase()
}

export function pickerItemsForSlot(slotKey: SlotKey): PickerRow[] {
  // Builds the modal-picker rows for the supplied gear slot, mirroring the legacy itemsForSlot (sorted by rarity then name) but enriched with rarity, group, baseType, a one-line meta string, and a full searchable-text blob (built by `buildItemSearchTerms`) so users can filter by affix/effect text. Used by ItemPickerLauncher.
  const group = slotGroup(slotKey)
  const matching = items
    .filter((i) => i.slot === slotKey || slotGroup(i.slot) === group)
    .slice()
    .sort((a, b) => {
      const ra = RARITY_ORDER[a.rarity] ?? 99
      const rb = RARITY_ORDER[b.rarity] ?? 99
      if (ra !== rb) return ra - rb
      return a.name.localeCompare(b.name)
    })
  return matching.map((i) => {
    const parts: string[] = [i.baseType]
    if (i.grade) parts.push(`Grade ${i.grade}`)
    if (i.baseType === 'Charm') parts.push(`${i.width ?? 1}×${i.height ?? 1}`)
    if (i.defenseMin !== undefined && i.defenseMax !== undefined)
      parts.push(`Def ${i.defenseMin}–${i.defenseMax}`)
    if (i.damageMin !== undefined && i.damageMax !== undefined)
      parts.push(`Dmg ${i.damageMin}–${i.damageMax}`)
    if (i.blockChance !== undefined) parts.push(`Block ${i.blockChance}%`)
    if (i.sockets !== undefined) {
      const max = i.maxSockets ?? i.sockets
      parts.push(
        max > i.sockets
          ? `${i.sockets}/${max} sockets`
          : `${i.sockets} sockets`,
      )
    }
    return {
      id: i.id,
      name: i.name,
      rarity: i.rarity,
      kindLabel: i.baseType,
      group: RARITY_LABEL[i.rarity],
      meta: parts.join(' · '),
      searchTerms: buildItemSearchTerms(i),
      iconUrl: getItemImage(i.id),
    }
  })
}
