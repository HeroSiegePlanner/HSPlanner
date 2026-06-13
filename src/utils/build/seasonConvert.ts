import {
  activeSeasonId,
  getAffix,
  getAugment,
  getCrystalMod,
  getGem,
  getItem,
  getRune,
  getRuneword,
  skills,
  treeNodeInfo,
} from '../../data'
import { reachableFromAny, START_IDS } from '../tree/treeGraph'
import type { EquippedItem, Inventory, TreeSocketContent } from '../../types'
import type { BuildSnapshot } from './shareBuild'

export interface SeasonConversionReport {
  fromSeason: string
  toSeason: string
  removedTreeNodes: number[]
  orphanedTreeNodes: number[]
  freedTreePoints: number
  removedItems: { slot: string; baseId: string }[]
  removedAffixes: { slot: string; affixId: string }[]
  removedForgedMods: { slot: string; affixId: string }[]
  removedSocketables: { slot: string; id: string }[]
  removedRunewords: { slot: string; runewordId: string }[]
  removedAugments: { slot: string; id: string }[]
  removedSkills: string[]
  removedSubskills: string[]
  removedTreeSockets: number[]
  removedUncutAffixes: { nodeId: number; affixId: string }[]
  hasChanges: boolean
}

const SKILL_IDS = new Set(skills.map((s) => s.id))
const SUBSKILL_IDS = new Set(
  skills.flatMap((s) => (s.subskills ?? []).map((n) => n.id)),
)

function knownSocketable(id: string): boolean {
  return getGem(id) !== undefined || getRune(id) !== undefined
}

function knownForgedMod(affixId: string): boolean {
  return getCrystalMod(affixId) !== undefined || getAffix(affixId) !== undefined
}

// Lossy conversion: every id is validated against the ACTIVE resolved
// dataset; the report feeds the conversion modal (spec §5).
export function convertSnapshotToActiveSeason(
  snapshot: BuildSnapshot,
  fromSeason: string,
): { snapshot: BuildSnapshot; report: SeasonConversionReport } {
  const report: SeasonConversionReport = {
    fromSeason,
    toSeason: activeSeasonId,
    removedTreeNodes: [],
    orphanedTreeNodes: [],
    freedTreePoints: 0,
    removedItems: [],
    removedAffixes: [],
    removedForgedMods: [],
    removedSocketables: [],
    removedRunewords: [],
    removedAugments: [],
    removedSkills: [],
    removedSubskills: [],
    removedTreeSockets: [],
    removedUncutAffixes: [],
    hasChanges: false,
  }

  const inventory: Inventory = {}
  for (const [slot, item] of Object.entries(snapshot.inventory)) {
    if (!item) continue
    const converted = convertItem(slot, item, report)
    if (converted) inventory[slot] = converted
  }

  const knownNodes = new Set<number>()
  for (const id of snapshot.allocatedTreeNodes) {
    if (treeNodeInfo[String(id)]) knownNodes.add(id)
    else report.removedTreeNodes.push(id)
  }
  const reachable = reachableFromAny(START_IDS, knownNodes)
  for (const id of knownNodes) {
    if (!reachable.has(id)) report.orphanedTreeNodes.push(id)
  }
  report.freedTreePoints = snapshot.allocatedTreeNodes.size - reachable.size

  const treeSocketed: Record<number, TreeSocketContent | null> = {}
  for (const [idStr, content] of Object.entries(snapshot.treeSocketed)) {
    const id = Number(idStr)
    if (content == null) continue
    if (!reachable.has(id)) {
      report.removedTreeSockets.push(id)
      continue
    }
    if (
      content.kind === 'item' &&
      !knownSocketable(content.id) &&
      getItem(content.id) === undefined
    ) {
      report.removedTreeSockets.push(id)
      continue
    }
    if (content.kind === 'uncut') {
      const kept = content.affixes.filter((a) => {
        if (getAffix(a.affixId) !== undefined) return true
        report.removedUncutAffixes.push({ nodeId: id, affixId: a.affixId })
        return false
      })
      treeSocketed[id] = { ...content, affixes: kept }
      continue
    }
    treeSocketed[id] = content
  }

  const skillRanks: Record<string, number> = {}
  for (const [id, rank] of Object.entries(snapshot.skillRanks)) {
    if (SKILL_IDS.has(id)) skillRanks[id] = rank
    else report.removedSkills.push(id)
  }
  const subskillRanks: Record<string, number> = {}
  for (const [id, rank] of Object.entries(snapshot.subskillRanks)) {
    if (SUBSKILL_IDS.has(id)) subskillRanks[id] = rank
    else report.removedSubskills.push(id)
  }

  const mainSkillId =
    snapshot.mainSkillId && SKILL_IDS.has(snapshot.mainSkillId)
      ? snapshot.mainSkillId
      : null
  const activeAuraId =
    snapshot.activeAuraId && SKILL_IDS.has(snapshot.activeAuraId)
      ? snapshot.activeAuraId
      : null

  // every array field in the report is a removal list
  report.hasChanges =
    Object.values(report).some((v) => Array.isArray(v) && v.length > 0) ||
    mainSkillId !== snapshot.mainSkillId ||
    activeAuraId !== snapshot.activeAuraId

  return {
    snapshot: {
      ...snapshot,
      inventory,
      allocatedTreeNodes: reachable,
      treeSocketed,
      skillRanks,
      subskillRanks,
      mainSkillId,
      activeAuraId,
    },
    report,
  }
}

function convertItem(
  slot: string,
  item: EquippedItem,
  report: SeasonConversionReport,
): EquippedItem | null {
  if (getItem(item.baseId) === undefined) {
    report.removedItems.push({ slot, baseId: item.baseId })
    return null
  }
  const affixes = item.affixes.filter((a) => {
    if (getAffix(a.affixId) !== undefined) return true
    report.removedAffixes.push({ slot, affixId: a.affixId })
    return false
  })
  const forgedMods = item.forgedMods?.filter((a) => {
    if (knownForgedMod(a.affixId)) return true
    report.removedForgedMods.push({ slot, affixId: a.affixId })
    return false
  })
  const socketed = item.socketed.map((id) => {
    if (id == null || knownSocketable(id)) return id
    report.removedSocketables.push({ slot, id })
    return null
  })
  let runewordId = item.runewordId
  if (runewordId && getRuneword(runewordId) === undefined) {
    report.removedRunewords.push({ slot, runewordId })
    runewordId = undefined
  }
  let augment = item.augment
  if (augment && getAugment(augment.id) === undefined) {
    report.removedAugments.push({ slot, id: augment.id })
    augment = undefined
  }
  return {
    ...item,
    affixes,
    ...(forgedMods !== undefined ? { forgedMods } : {}),
    socketed,
    runewordId,
    augment,
  }
}
