import { create } from 'zustand'
import {
  classes,
  gameConfig,
  getAugment,
  getClass,
  getItem,
  getRuneword,
  skills as ALL_SKILLS,
} from '../data'
import { AUGMENT_MAX_LEVEL } from '../types'
import type {
  AttributeKey,
  CustomStat,
  EquippedItem,
  Inventory,
  SlotKey,
  SocketType,
  TreeSocketContent,
} from '../types'
import {
  addProfile as storeAddProfile,
  commitProfileSnapshot as storeCommitProfile,
  createBuild as storeCreateBuild,
  deleteBuild as storeDeleteBuild,
  duplicateBuild as storeDuplicateBuild,
  duplicateProfile as storeDuplicateProfile,
  getSavedBuild,
  loadProfileSnapshot,
  moveBuildToFolder as storeMoveBuildToFolder,
  removeProfile as storeRemoveProfile,
  renameProfile as storeRenameProfile,
  renameBuild as storeRenameBuild,
  setActiveProfile as storeSetActiveProfile,
  setBuildFavorite as storeSetBuildFavorite,
  setBuildNotes as storeSetBuildNotes,
  setBuildTags as storeSetBuildTags,
  type Folder,
  type SavedBuild,
} from '../utils/build/savedBuilds'
import {
  createFolder as storeCreateFolder,
  deleteFolder as storeDeleteFolder,
  renameFolder as storeRenameFolder,
} from '../utils/build/savedFolders'
import { guardStorage } from './storageError'
import { sanitizeHtml } from '../utils/sanitizeHtml'
import {
  defaultEnemyResistances,
  type BuildSnapshot,
} from '../utils/build/shareBuild'
import { ADJ, findPath, reachableFromAny, START_IDS } from '../utils/tree/treeGraph'

type AttrMap = Record<AttributeKey, number>

export const RAINBOW_MULTIPLIER = 1.5

export const MAX_STARS = 5

interface BuildState {
  classId: string | null
  level: number
  allocated: AttrMap
  inventory: Inventory
  skillRanks: Record<string, number>
  allocatedTreeNodes: Set<number>
  treeSocketed: Record<number, TreeSocketContent | null>
  mainSkillId: string | null
  activeAuraId: string | null
  procToggles: Record<string, boolean>
  killsPerSec: number
  activeBuffs: Record<string, boolean>
  enemyConditions: Record<string, boolean>
  playerConditions: Record<string, boolean>
  skillProjectiles: Record<string, number>
  enemyResistances: Record<string, number>
  subskillRanks: Record<string, number>
  activeBuildId: string | null
  activeProfileId: string | null
  savedBuildsVersion: number
  storageError: string | null
  notes: string
  customStats: CustomStat[]
}

interface BuildActions {
  setClass: (id: string) => void
  setLevel: (lvl: number) => void
  incAttr: (key: AttributeKey, amount?: number) => void
  decAttr: (key: AttributeKey, amount?: number) => void
  resetAttrs: () => void
  equipItem: (slot: SlotKey, baseId: string) => void
  unequipItem: (slot: SlotKey) => void
  replaceEquippedItem: (slot: SlotKey, equipped: EquippedItem) => void
  setSocketCount: (slot: SlotKey, count: number) => void
  setSocketed: (slot: SlotKey, idx: number, socketableId: string | null) => void
  setSocketType: (slot: SlotKey, idx: number, type: SocketType) => void
  setStars: (slot: SlotKey, count: number) => void
  exportBuildSnapshot: () => BuildSnapshot
  importBuildSnapshot: (snapshot: BuildSnapshot, notes?: string) => void
  applyRuneword: (slot: SlotKey, runewordId: string) => void
  setAugment: (augmentId: string | null) => void
  setAugmentLevel: (level: number) => void
  addAffix: (slot: SlotKey, affixId: string, tier: number) => void
  removeAffix: (slot: SlotKey, index: number) => void
  setAffixRoll: (slot: SlotKey, index: number, roll: number) => void
  addForgedMod: (slot: SlotKey, modId: string, tier: number) => void
  removeForgedMod: (slot: SlotKey, index: number) => void
  moveItem: (fromSlot: SlotKey, toSlot: SlotKey) => void
  setSkillRank: (skillId: string, rank: number, maxRank: number) => void
  incSkillRank: (skillId: string, maxRank: number) => void
  decSkillRank: (skillId: string) => void
  resetSkillRanks: () => void
  toggleTreeNode: (nodeId: number) => void
  applySuggestedNodes: (ids: Iterable<number>) => void
  resetTreeNodes: () => void
  setTreeSocketed: (nodeId: number, content: TreeSocketContent | null) => void
  setMainSkill: (skillId: string | null) => void
  setActiveAura: (skillId: string | null) => void
  setProcToggle: (skillId: string, enabled: boolean) => void
  setKillsPerSec: (rate: number) => void
  setBuffActive: (skillId: string, enabled: boolean) => void
  setEnemyCondition: (key: string, enabled: boolean) => void
  setPlayerCondition: (key: string, enabled: boolean) => void
  setSkillProjectiles: (skillId: string, count: number | null) => void
  setEnemyResistance: (damageType: string, value: number | null) => void
  setSubskillRank: (
    skillId: string,
    subskillId: string,
    rank: number,
    maxRank: number,
  ) => void
  incSubskillRank: (
    skillId: string,
    subskillId: string,
    maxRank: number,
  ) => void
  decSubskillRank: (skillId: string, subskillId: string) => void
  resetSubskillsFor: (skillId: string) => void
  setNotes: (html: string) => void
  commitBuildNotes: () => boolean
  addCustomStat: (init?: Partial<CustomStat>) => void
  updateCustomStat: (index: number, patch: Partial<CustomStat>) => void
  removeCustomStat: (index: number) => void
  loadSavedBuild: (buildId: string, profileId?: string) => boolean
  switchActiveProfile: (profileId: string) => boolean
  commitActiveProfile: () => boolean
  addProfileToActiveBuild: (name: string) => string | null
  duplicateActiveProfile: (profileId: string) => string | null
  renameActiveProfile: (profileId: string, name: string) => boolean
  removeActiveProfile: (profileId: string) => boolean
  detachFromBuild: () => void
  resetBuild: () => void
  bindToBuild: (buildId: string, profileId: string) => void
  deleteSavedBuild: (buildId: string) => void
  renameSavedBuild: (buildId: string, name: string) => boolean
  saveCurrentAsNewBuild: (
    name: string,
    notes?: string,
    folderId?: string | null,
  ) => SavedBuild | null
  duplicateSavedBuild: (buildId: string) => SavedBuild | null
  setSavedBuildFavorite: (buildId: string, favorite: boolean) => boolean
  setSavedBuildTags: (buildId: string, tags: string[]) => boolean
  moveSavedBuildToFolder: (
    buildId: string,
    folderId: string | null,
  ) => boolean
  switchSavedBuildProfile: (buildId: string, profileId: string) => boolean
  addSavedBuildProfile: (buildId: string, name: string) => string | null
  renameSavedBuildProfile: (
    buildId: string,
    profileId: string,
    name: string,
  ) => boolean
  duplicateSavedBuildProfile: (
    buildId: string,
    profileId: string,
  ) => string | null
  removeSavedBuildProfile: (buildId: string, profileId: string) => boolean
  createSavedFolder: (name: string, parentId: string | null) => Folder | null
  renameSavedFolder: (folderId: string, name: string) => boolean
  deleteSavedFolder: (folderId: string, cascade: boolean) => boolean
  dismissStorageError: () => void
}

function emptyAllocation(): AttrMap {
  // Returns an attribute map with every game-defined attribute initialised to zero. Used to seed the build store and to reset attribute allocations.
  return gameConfig.attributes.reduce<AttrMap>((acc, a) => {
    acc[a.key] = 0
    return acc
  }, {})
}


function bumpSavedBuilds(
  set: (fn: (s: BuildState) => Partial<BuildState>) => void,
) {
  // Increments the `savedBuildsVersion` counter via the supplied zustand setter so subscribed components know to re-read from localStorage. Used after every successful mutation of the persisted SavedBuilds list.
  set((s) => ({ savedBuildsVersion: s.savedBuildsVersion + 1 }))
}

function snapshotPatch(snap: BuildSnapshot) {
  // Translates a BuildSnapshot into the slice of build-store state needed by every "load profile" code path, deliberately excluding `notes` (which live on the build, not the profile). Used by importBuildSnapshot, loadSavedBuild, switchActiveProfile, duplicateActiveProfile, and removeActiveProfile.
  return {
    classId: snap.classId,
    level: snap.level,
    allocated: snap.allocated,
    inventory: snap.inventory,
    skillRanks: snap.skillRanks,
    subskillRanks: snap.subskillRanks,
    allocatedTreeNodes: snap.allocatedTreeNodes,
    treeSocketed: snap.treeSocketed ?? {},
    mainSkillId: snap.mainSkillId,
    activeAuraId: snap.activeAuraId,
    activeBuffs: snap.activeBuffs,
    enemyConditions: snap.enemyConditions,
    playerConditions: snap.playerConditions ?? {},
    skillProjectiles: snap.skillProjectiles ?? {},
    enemyResistances: snap.enemyResistances ?? defaultEnemyResistances(),
    procToggles: snap.procToggles,
    killsPerSec: snap.killsPerSec,
    customStats: snap.customStats ?? [],
  }
}

const HARD_SOCKET_CAP = 6
export const BONUS_SOCKET_MOD_ID = 'crystal_add_socket'

function hasBonusSocketMod(
  forgedMods?: { affixId: string }[] | null,
): boolean {
  // Returns true when the supplied forged-mod list contains the BONUS_SOCKET_MOD_ID crystal mod. Used by maxSocketsFor (and gear UI) to decide whether to extend an item's socket cap by one.
  return !!forgedMods?.some((m) => m.affixId === BONUS_SOCKET_MOD_ID)
}

export function maxSocketsFor(
  baseId: string,
  forgedMods?: { affixId: string }[] | null,
): number {
  // Computes the maximum number of sockets an item can have at the moment, starting from `maxSockets`/`sockets` on the base, adding one if the bonus-socket forged mod is present, and clamping to HARD_SOCKET_CAP. Used by the gear UI and the equip/forge actions to validate socket counts.
  const base = getItem(baseId)
  if (!base) return 0
  let cap = base.maxSockets ?? base.sockets ?? 0
  if (hasBonusSocketMod(forgedMods)) cap += 1
  return Math.min(cap, HARD_SOCKET_CAP)
}

export const useBuild = create<BuildState & BuildActions>((set, get) => ({
  classId: classes[0]?.id ?? null,
  level: 1,
  allocated: emptyAllocation(),
  inventory: {},
  skillRanks: {},
  allocatedTreeNodes: new Set<number>(),
  treeSocketed: {},
  mainSkillId: null,
  activeAuraId: null,
  procToggles: {},
  killsPerSec: 1,
  activeBuffs: {},
  enemyConditions: {},
  playerConditions: {},
  skillProjectiles: {},
  enemyResistances: defaultEnemyResistances(),
  subskillRanks: {},
  activeBuildId: null,
  activeProfileId: null,
  savedBuildsVersion: 0,
  storageError: null,
  notes: '',
  customStats: [],

  setClass: (id) =>
    set((s) => {
      // Switches the active character class, resetting all class-scoped state (allocated points, skills, subskills, main skill / aura / buffs / procs, notes, custom stats) and detaching from any saved build/profile because a class change effectively starts a new build.
      if (s.classId === id) return s
      return {
        classId: id,
        allocated: emptyAllocation(),
        skillRanks: {},
        mainSkillId: null,
        activeAuraId: null,
        procToggles: {},
        activeBuffs: {},
        subskillRanks: {},
        activeBuildId: null,
        activeProfileId: null,
        notes: '',
        customStats: [],
      }
    }),

  setNotes: (html) =>
    set((s) => {
      // Replaces the in-memory notes HTML, running it through sanitizeHtml first and short-circuiting when the cleaned value matches the current state. Used by the NotesView editor on every change.
      const cleaned = sanitizeHtml(html)
      return s.notes === cleaned ? s : { notes: cleaned }
    }),

  commitBuildNotes: () =>
    guardStorage(
      (m) => set({ storageError: m }),
      false,
      () => {
        // Persists the current `notes` field onto the active SavedBuild (and bumps the savedBuilds version). No-op when no build is currently active. Used by NotesView to flush edits to disk.
        const s = get()
        if (!s.activeBuildId) return false
        const ok = storeSetBuildNotes(s.activeBuildId, s.notes) !== null
        if (ok) bumpSavedBuilds(set)
        return ok
      },
    ),

  addCustomStat: (init = {}) =>
    set((s) => ({
      // Appends a new (possibly partially-prefilled) custom stat row to the list. Used by ConfigView's "Add custom stat" button.
      customStats: [
        ...s.customStats,
        {
          statKey: init.statKey ?? '',
          value: init.value ?? '',
        },
      ],
    })),

  updateCustomStat: (index, patch) =>
    set((s) => {
      // Patches the custom-stat row at `index` with the partial fields in `patch`, no-op when the index is out of range. Used by ConfigView when the user edits a custom stat row.
      if (index < 0 || index >= s.customStats.length) return s
      const next = s.customStats.map((cs, i) =>
        i === index ? { ...cs, ...patch } : cs,
      )
      return { customStats: next }
    }),

  removeCustomStat: (index) =>
    set((s) => {
      // Removes the custom-stat row at `index`. Used by ConfigView's per-row delete button.
      if (index < 0 || index >= s.customStats.length) return s
      return { customStats: s.customStats.filter((_, i) => i !== index) }
    }),

  toggleTreeNode: (nodeId) =>
    set((s) => {
      // Toggles a talent-tree node's allocation. When deallocating, prunes any nodes that are no longer reachable from a START node; when allocating, automatically allocates the cheapest path connecting the click target to the existing allocation (or to a starting node if the tree is empty). Used by TreeView clicks.
      const cur = s.allocatedTreeNodes
      if (cur.has(nodeId)) {
        const next = new Set(cur)
        next.delete(nodeId)
        const stillReachable = reachableFromAny(START_IDS, next)
        return { allocatedTreeNodes: stillReachable }
      }
      const sources = new Set<number>([...cur, ...START_IDS])
      const path = findPath(sources, nodeId)
      if (!path) return s
      const next = new Set(cur)
      for (const id of path) next.add(id)
      return { allocatedTreeNodes: next }
    }),

  applySuggestedNodes: (ids) =>
    set((s) => {
      // Bulk-allocates a pre-validated set of tree node ids (typically produced by `suggestNodes`). Only includes the START_IDS that are actually adjacent to (or contained in) the union — historically the modal pulled in all 8 starts which polluted the allocation with unused class entry points. Used by the Suggest Nodes modal to apply an algorithmic recommendation, then prunes anything that became unreachable as a safety net.
      const next = new Set(s.allocatedTreeNodes)
      for (const id of ids) next.add(id)
      for (const sid of START_IDS) {
        if (next.has(sid)) continue
        const nbrs = ADJ.get(sid)
        if (!nbrs) continue
        for (const nb of nbrs) {
          if (next.has(nb)) {
            next.add(sid)
            break
          }
        }
      }
      const reachable = reachableFromAny(START_IDS, next)
      return { allocatedTreeNodes: reachable }
    }),

  resetTreeNodes: () => set({ allocatedTreeNodes: new Set<number>(), treeSocketed: {} }),
  // Clears every allocated talent-tree node and any tree-socket content. Used by TreeView's reset button.

  setTreeSocketed: (nodeId, content) =>
    set((s) => {
      // Sets (or clears with null) the content of a jewelry tree socket. Used by JewelSocketPicker.
      const next = { ...s.treeSocketed }
      if (content == null) delete next[nodeId]
      else next[nodeId] = content
      return { treeSocketed: next }
    }),

  setMainSkill: (skillId) => set({ mainSkillId: skillId }),
  // Marks a skill as the build's main skill, which the StatsView uses to drive the headline damage breakdown.

  setActiveAura: (skillId) => set({ activeAuraId: skillId }),
  // Sets which aura skill is currently active so its passive stats contribute to the build. Used by SkillsView when the user enables an aura.

  setProcToggle: (skillId, enabled) =>
    set((s) => {
      // Adds or removes a skill id from the `procToggles` map (deletion when disabled to keep the object minimal). Used by SkillsView to opt skill procs into the stat aggregation.
      const next = { ...s.procToggles }
      if (enabled) next[skillId] = true
      else delete next[skillId]
      return { procToggles: next }
    }),

  setKillsPerSec: (rate) =>
    set({ killsPerSec: Math.max(0, rate) }),
  // Sets the assumed kills-per-second number used by on-kill proc damage approximations, clamped to non-negative.

  setBuffActive: (skillId, enabled) =>
    set((s) => {
      // Adds or removes a skill id from the `activeBuffs` map. Used by SkillsView when the user toggles a buff so its passiveStats apply.
      const next = { ...s.activeBuffs }
      if (enabled) next[skillId] = true
      else delete next[skillId]
      return { activeBuffs: next }
    }),

  setEnemyCondition: (key, enabled) =>
    set((s) => {
      // Toggles a single enemy-condition flag (e.g. "burning", "frozen", "low_life"). Used by ConfigView to gate conditional stat contributions and `extra_damage_*` bonuses.
      const next = { ...s.enemyConditions }
      if (enabled) next[key] = true
      else delete next[key]
      return { enemyConditions: next }
    }),

  setPlayerCondition: (key, enabled) =>
    set((s) => {
      // Toggles a single player self-condition flag (e.g. "crit_chance_below_40", "life_below_40"). Used by ConfigView to gate tree-node mods that only apply when the player is in a specific state.
      const next = { ...s.playerConditions }
      if (enabled) next[key] = true
      else delete next[key]
      return { playerConditions: next }
    }),

  setSkillProjectiles: (skillId, count) =>
    set((s) => {
      // Sets a manual projectile-count override for a specific skill (multiplies that skill's hit/avg/DPS by `count`). Pass `null` (or any non-positive / non-finite value) to clear the override and fall back to the default of 1. Used by ConfigView so the user can model multi-projectile skills like Multi Shot or Fan of Knives without skill metadata.
      const next = { ...s.skillProjectiles }
      if (
        count === null ||
        !Number.isFinite(count) ||
        (count as number) <= 1
      ) {
        delete next[skillId]
      } else {
        next[skillId] = Math.max(1, Math.floor(count as number))
      }
      return { skillProjectiles: next }
    }),

  setEnemyResistance: (damageType, value) =>
    set((s) => {
      // Sets the assumed enemy resistance percentage for a given damage type, or removes the key when the value is null/non-finite (which lets the default cover it). Used by ConfigView's resistance sliders.
      const next = { ...s.enemyResistances }
      if (value === null || !Number.isFinite(value)) {
        delete next[damageType]
      } else {
        next[damageType] = value
      }
      return { enemyResistances: next }
    }),

  setSubskillRank: (skillId, subskillId, rank, maxRank) => {
    // Sets a subskill node's rank, clamped against its individual maxRank, against the unallocated subskill point budget at the current level, and against zero. Removes the entry when the resulting rank is zero. Used by SubtreeOverlay click and slider interactions.
    const { subskillRanks, level } = get()
    const total = subskillPointsFor(level)
    const key = subskillKey(skillId, subskillId)
    const cur = subskillRanks[key] ?? 0
    const otherSpent = Object.entries(subskillRanks).reduce(
      (s, [k, r]) => (k === key ? s : s + r),
      0,
    )
    const clamped = Math.max(0, Math.min(maxRank, rank, total - otherSpent))
    if (clamped === cur) return
    const next = { ...subskillRanks }
    if (clamped === 0) delete next[key]
    else next[key] = clamped
    set({ subskillRanks: next })
  },

  incSubskillRank: (skillId, subskillId, maxRank) => {
    // Convenience helper that increments a subskill rank by one through `setSubskillRank` so the same clamping rules apply. Used by SubtreeOverlay's "+" button.
    const cur =
      get().subskillRanks[subskillKey(skillId, subskillId)] ?? 0
    get().setSubskillRank(skillId, subskillId, cur + 1, maxRank)
  },

  decSubskillRank: (skillId, subskillId) => {
    // Decrements a subskill rank by one (no-op at zero). Used by SubtreeOverlay's "-" button.
    const key = subskillKey(skillId, subskillId)
    const cur = get().subskillRanks[key] ?? 0
    if (cur <= 0) return
    set((s) => {
      const next = { ...s.subskillRanks }
      if (cur - 1 === 0) delete next[key]
      else next[key] = cur - 1
      return { subskillRanks: next }
    })
  },

  resetSubskillsFor: (skillId) =>
    set((s) => {
      // Drops every subskillRanks entry that belongs to the supplied parent skill (the `${skillId}:` prefix). Used by SubtreeOverlay's "Reset subskills" action.
      const next: Record<string, number> = {}
      for (const [k, v] of Object.entries(s.subskillRanks)) {
        if (!k.startsWith(`${skillId}:`)) next[k] = v
      }
      return { subskillRanks: next }
    }),

  setLevel: (lvl) => {
    // Sets the character level, clamped to the [1, gameConfig.maxCharacterLevel] range. Used by CharacterView's level field.
    const clamped = Math.max(1, Math.min(gameConfig.maxCharacterLevel, lvl))
    set({ level: clamped })
  },

  incAttr: (key, amount = 1) => {
    // Spends up to `amount` unallocated attribute points on the supplied attribute key, capped by the remaining budget at the current level. Used by CharacterView's "+" button.
    const { allocated, level } = get()
    const total = Object.values(allocated).reduce((s, v) => s + v, 0)
    const available = attrPointsFor(level) - total
    const step = Math.min(amount, available)
    if (step <= 0) return
    set({ allocated: { ...allocated, [key]: (allocated[key] ?? 0) + step } })
  },

  decAttr: (key, amount = 1) => {
    // Refunds up to `amount` allocated points from the given attribute, never going below zero. Used by CharacterView's "-" button.
    const { allocated } = get()
    const cur = allocated[key] ?? 0
    const step = Math.min(amount, cur)
    if (step <= 0) return
    set({ allocated: { ...allocated, [key]: cur - step } })
  },

  resetAttrs: () => set({ allocated: emptyAllocation() }),
  // Clears every allocated attribute point. Used by CharacterView's reset button.

  equipItem: (slot, baseId) => {
    // Equips a base item into the named slot, seeding socket count/types/stars and enforcing the two-handed rule (offhand cannot coexist with a 2H weapon, and equipping a 2H weapon clears any offhand). Used by GearView's item picker.
    const base = getItem(baseId)
    if (!base) return
    set((s) => {
      if (slot === 'offhand') {
        const cur = s.inventory.weapon
        const w = cur ? getItem(cur.baseId) : undefined
        if (w?.twoHanded) return s
      }
      const initial = Math.min(base.sockets ?? 0, maxSocketsFor(baseId))
      const item: EquippedItem = {
        baseId,
        affixes: [],
        socketCount: initial,
        socketed: Array(initial).fill(null),
        socketTypes: Array(initial).fill('normal'),
        stars: 0,
        forgedMods: [],
      }
      const next = { ...s.inventory, [slot]: item }
      if (slot === 'weapon' && base.twoHanded) {
        delete next.offhand
      }
      return { inventory: next }
    })
  },

  unequipItem: (slot) => {
    // Removes any equipped item from the named slot. Used by GearView's "remove" action.
    set((s) => {
      const next = { ...s.inventory }
      delete next[slot]
      return { inventory: next }
    })
  },

  replaceEquippedItem: (slot, equipped) => {
    set((s) => {
      if (!getItem(equipped.baseId)) return s
      return { inventory: { ...s.inventory, [slot]: equipped } }
    })
  },

  setSocketCount: (slot, count) => {
    // Resizes the equipped item's socket array up to `count`, clamped by `maxSocketsFor`, padding new sockets with null/normal and truncating extras. Used by GearView's socket-count stepper.
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      const max = maxSocketsFor(cur.baseId, cur.forgedMods)
      const clamped = Math.max(0, Math.min(max, count))
      const socketed = [...cur.socketed]
      const socketTypes = [...cur.socketTypes]
      while (socketed.length < clamped) {
        socketed.push(null)
        socketTypes.push('normal')
      }
      socketed.length = clamped
      socketTypes.length = clamped
      return {
        inventory: {
          ...s.inventory,
          [slot]: { ...cur, socketCount: clamped, socketed, socketTypes },
        },
      }
    })
  },

  setSocketed: (slot, idx, socketableId) => {
    // Inserts (or clears) a runeword/gem id into the socket at `idx` on the equipped item in `slot`. Used by GearView's socket picker.
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur || idx < 0 || idx >= cur.socketCount) return s
      const socketed = [...cur.socketed]
      socketed[idx] = socketableId
      return {
        inventory: { ...s.inventory, [slot]: { ...cur, socketed } },
      }
    })
  },

  setSocketType: (slot, idx, type) => {
    // Sets a socket's type (normal vs rainbow) at index `idx`. Used by GearView when the user converts a socket to rainbow.
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur || idx < 0 || idx >= cur.socketCount) return s
      const socketTypes = [...cur.socketTypes]
      socketTypes[idx] = type
      return {
        inventory: { ...s.inventory, [slot]: { ...cur, socketTypes } },
      }
    })
  },

  setStars: (slot, count) => {
    // Sets the star count of the equipped item, clamped to the [0, MAX_STARS] range. Used by GearView's star control.
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      const clamped = Math.max(0, Math.min(MAX_STARS, Math.floor(count)))
      if ((cur.stars ?? 0) === clamped) return s
      return {
        inventory: { ...s.inventory, [slot]: { ...cur, stars: clamped } },
      }
    })
  },

  exportBuildSnapshot: () => {
    // Returns a fresh BuildSnapshot describing the live in-memory state, suitable for share-link encoding or saved-build persistence. Used by ShareButton, the saved-builds layer and every profile-mutating action that needs to capture current state.
    const s = get()
    return {
      classId: s.classId,
      level: s.level,
      allocated: s.allocated,
      inventory: s.inventory,
      skillRanks: s.skillRanks,
      subskillRanks: s.subskillRanks,
      allocatedTreeNodes: s.allocatedTreeNodes,
      treeSocketed: s.treeSocketed,
      mainSkillId: s.mainSkillId,
      activeAuraId: s.activeAuraId,
      activeBuffs: s.activeBuffs,
      enemyConditions: s.enemyConditions,
      playerConditions: s.playerConditions,
      skillProjectiles: s.skillProjectiles,
      enemyResistances: s.enemyResistances,
      procToggles: s.procToggles,
      killsPerSec: s.killsPerSec,
      customStats: s.customStats,
    }
  },

  importBuildSnapshot: (snapshot, notes = '') => {
    // Replaces the live state with the supplied snapshot (and notes), detaching from any saved build/profile so the imported build behaves as unsaved freeform until the user explicitly saves it. Used by share-URL imports and paste-import flows.
    set(() => ({
      ...snapshotPatch(snapshot),
      notes,
      activeBuildId: null,
      activeProfileId: null,
    }))
  },

  bindToBuild: (buildId, profileId) =>
    set((cur) => ({
      // Marks the live state as belonging to the named saved build/profile without importing a snapshot, then bumps the savedBuilds version. Used by BuildsMenu when associating an in-memory build with a freshly-created saved entry.
      activeBuildId: buildId,
      activeProfileId: profileId,
      savedBuildsVersion: cur.savedBuildsVersion + 1,
    })),

  detachFromBuild: () =>
    set(() => ({ activeBuildId: null, activeProfileId: null })),
  // Detaches the live state from any saved build/profile so further edits are not auto-committed. Used when the user explicitly leaves a saved build.

  resetBuild: () =>
    set(() => ({
      // Resets the live state to a brand-new blank build (default class, level 1,
      // nothing allocated) and detaches from any saved build. Used by the Build
      // Select "New" action so the planner opens on a clean slate.
      classId: classes[0]?.id ?? null,
      level: 1,
      allocated: emptyAllocation(),
      inventory: {},
      skillRanks: {},
      allocatedTreeNodes: new Set<number>(),
      treeSocketed: {},
      mainSkillId: null,
      activeAuraId: null,
      procToggles: {},
      killsPerSec: 1,
      activeBuffs: {},
      enemyConditions: {},
      playerConditions: {},
      skillProjectiles: {},
      enemyResistances: defaultEnemyResistances(),
      subskillRanks: {},
      activeBuildId: null,
      activeProfileId: null,
      notes: '',
      customStats: [],
    })),

  deleteSavedBuild: (buildId) =>
    guardStorage<void>(
      (m) => set({ storageError: m }),
      undefined,
      () => {
        // Deletes a SavedBuild from disk and, if it was the active build, also detaches the live state. Used by BuildsMenu's delete action.
        storeDeleteBuild(buildId)
        set((cur) => {
          const detach = cur.activeBuildId === buildId
          return {
            savedBuildsVersion: cur.savedBuildsVersion + 1,
            ...(detach
              ? { activeBuildId: null, activeProfileId: null }
              : {}),
          }
        })
      },
    ),

  renameSavedBuild: (buildId, name) =>
    guardStorage(
      (m) => set({ storageError: m }),
      false,
      () => {
        // Renames a SavedBuild on disk and bumps the savedBuilds version. Returns false when the build does not exist. Used by BuildsMenu's inline rename.
        const ok = storeRenameBuild(buildId, name) !== null
        if (ok) bumpSavedBuilds(set)
        return ok
      },
    ),

  commitActiveProfile: () =>
    guardStorage(
      (m) => set({ storageError: m }),
      false,
      () => {
        // Persists the current in-memory state into the active SavedProfile of the active SavedBuild. No-op when nothing is active. Used by autosave-style flows and the "Save" button.
        const s = get()
        if (!s.activeBuildId || !s.activeProfileId) return false
        const snap = s.exportBuildSnapshot()
        const result = storeCommitProfile(
          s.activeBuildId,
          s.activeProfileId,
          snap,
        )
        if (result) bumpSavedBuilds(set)
        return result !== null
      },
    ),

  loadSavedBuild: (buildId, profileId) =>
    guardStorage(
      (m) => set({ storageError: m }),
      false,
      () => {
        // Loads a SavedBuild (optionally a specific profile) into the live state, after auto-committing whatever profile was previously active. Used by BuildsMenu when the user opens a saved build.
        const cur = get()
        if (cur.activeBuildId && cur.activeProfileId) {
          storeCommitProfile(
            cur.activeBuildId,
            cur.activeProfileId,
            cur.exportBuildSnapshot(),
          )
        }
        const build = getSavedBuild(buildId)
        if (!build) return false
        const targetProfileId =
          profileId && build.profiles.some((p) => p.id === profileId)
            ? profileId
            : build.activeProfileId
        const snap = loadProfileSnapshot(buildId, targetProfileId)
        if (!snap) return false
        storeSetActiveProfile(buildId, targetProfileId)
        set((s) => ({
          ...snapshotPatch(snap),
          notes: sanitizeHtml(build.notes ?? ''),
          activeBuildId: buildId,
          activeProfileId: targetProfileId,
          savedBuildsVersion: s.savedBuildsVersion + 1,
        }))
        return true
      },
    ),

  switchActiveProfile: (profileId) =>
    guardStorage(
      (m) => set({ storageError: m }),
      false,
      () => {
        // Switches the active profile within the currently active SavedBuild, auto-committing the outgoing profile so unsaved edits survive. Returns true when the switch succeeded. Used by ProfileSwitcher.
        const s = get()
        if (!s.activeBuildId) return false
        if (s.activeProfileId === profileId) return true
        if (s.activeProfileId) {
          storeCommitProfile(
            s.activeBuildId,
            s.activeProfileId,
            s.exportBuildSnapshot(),
          )
        }
        const snap = loadProfileSnapshot(s.activeBuildId, profileId)
        if (!snap) return false
        storeSetActiveProfile(s.activeBuildId, profileId)
        set((cur) => ({
          ...snapshotPatch(snap),
          activeProfileId: profileId,
          savedBuildsVersion: cur.savedBuildsVersion + 1,
        }))
        return true
      },
    ),

  addProfileToActiveBuild: (name) =>
    guardStorage<string | null>(
      (m) => set({ storageError: m }),
      null,
      () => {
        // Forks the current state into a new profile inside the active SavedBuild, auto-committing the outgoing profile first, and activates the new profile. Returns the new profile id, or null when no build is active. Used by ProfileSwitcher's "Add profile" action.
        const s = get()
        if (!s.activeBuildId) return null
        if (s.activeProfileId) {
          storeCommitProfile(
            s.activeBuildId,
            s.activeProfileId,
            s.exportBuildSnapshot(),
          )
        }
        const result = storeAddProfile(
          s.activeBuildId,
          name,
          s.exportBuildSnapshot(),
          { activate: true },
        )
        if (!result) return null
        set((cur) => ({
          activeProfileId: result.profile.id,
          savedBuildsVersion: cur.savedBuildsVersion + 1,
        }))
        return result.profile.id
      },
    ),

  duplicateActiveProfile: (profileId) =>
    guardStorage<string | null>(
      (m) => set({ storageError: m }),
      null,
      () => {
        // Duplicates a profile within the active SavedBuild, auto-committing the outgoing profile first so the duplicate captures the user's current edits, and switches the live state to the copy. Returns the new profile id, or null on failure. Used by ProfileSwitcher's duplicate action.
        const s = get()
        if (!s.activeBuildId) return null
        if (s.activeProfileId) {
          storeCommitProfile(
            s.activeBuildId,
            s.activeProfileId,
            s.exportBuildSnapshot(),
          )
        }
        const result = storeDuplicateProfile(s.activeBuildId, profileId)
        if (!result) return null
        const snap = loadProfileSnapshot(s.activeBuildId, result.profile.id)
        if (snap) {
          set((cur) => ({
            ...snapshotPatch(snap),
            activeProfileId: result.profile.id,
            savedBuildsVersion: cur.savedBuildsVersion + 1,
          }))
        } else {
          bumpSavedBuilds(set)
        }
        return result.profile.id
      },
    ),

  renameActiveProfile: (profileId, name) =>
    guardStorage(
      (m) => set({ storageError: m }),
      false,
      () => {
        // Renames a profile inside the active SavedBuild on disk. Used by ProfileSwitcher's inline rename.
        const s = get()
        if (!s.activeBuildId) return false
        const ok = storeRenameProfile(s.activeBuildId, profileId, name) !== null
        if (ok) bumpSavedBuilds(set)
        return ok
      },
    ),

  removeActiveProfile: (profileId) =>
    guardStorage(
      (m) => set({ storageError: m }),
      false,
      () => {
        // Removes a profile from the active SavedBuild and, if the deleted profile was active, hydrates the live state from the new active profile. Used by ProfileSwitcher's delete action.
        const s = get()
        if (!s.activeBuildId) return false
        const isActive = s.activeProfileId === profileId
        const updated = storeRemoveProfile(s.activeBuildId, profileId)
        if (!updated) return false
        if (isActive) {
          const snap = loadProfileSnapshot(
            s.activeBuildId,
            updated.activeProfileId,
          )
          if (snap) {
            set((cur) => ({
              ...snapshotPatch(snap),
              activeProfileId: updated.activeProfileId,
              savedBuildsVersion: cur.savedBuildsVersion + 1,
            }))
          } else {
            bumpSavedBuilds(set)
          }
        } else {
          bumpSavedBuilds(set)
        }
        return true
      },
    ),

  applyRuneword: (slot, runewordId) => {
    // Applies a runeword to a common-rarity item by inserting its rune sequence into the sockets, validating the base type and socket cap. Used by GearView's runeword picker.
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      const base = getItem(cur.baseId)
      const rw = getRuneword(runewordId)
      if (!base || !rw) return s
      if (base.rarity !== 'common') return s
      if (!rw.allowedBaseTypes.includes(base.baseType)) return s
      const cap = maxSocketsFor(cur.baseId)
      if (rw.runes.length > cap) return s
      const socketed: (string | null)[] = [...rw.runes]
      const socketTypes = cur.socketTypes.slice(0, rw.runes.length)
      while (socketTypes.length < rw.runes.length) socketTypes.push('normal')
      return {
        inventory: {
          ...s.inventory,
          [slot]: {
            ...cur,
            socketCount: rw.runes.length,
            socketed,
            socketTypes,
          },
        },
      }
    })
  },

  setAugment: (augmentId) => {
    // Sets or clears the angelic augment on the body-armor slot, defaulting newly-applied augments to level 1 while preserving the level when the same augment is reapplied. Used by GearView's augment picker.
    set((s) => {
      const slot: SlotKey = 'armor'
      const cur = s.inventory[slot]
      if (!cur) return s
      if (augmentId === null) {
        const { augment: _drop, ...rest } = cur
        void _drop
        return { inventory: { ...s.inventory, [slot]: rest } }
      }
      if (!getAugment(augmentId)) return s
      const level = cur.augment?.id === augmentId ? cur.augment.level : 1
      return {
        inventory: {
          ...s.inventory,
          [slot]: { ...cur, augment: { id: augmentId, level } },
        },
      }
    })
  },

  setAugmentLevel: (level) => {
    // Sets the augment's level on the body-armor slot, clamped to [1, AUGMENT_MAX_LEVEL]. No-op when nothing is equipped or no augment is set. Used by GearView's augment level slider.
    set((s) => {
      const slot: SlotKey = 'armor'
      const cur = s.inventory[slot]
      if (!cur || !cur.augment) return s
      const clamped = Math.max(1, Math.min(AUGMENT_MAX_LEVEL, Math.round(level)))
      if (clamped === cur.augment.level) return s
      return {
        inventory: {
          ...s.inventory,
          [slot]: { ...cur, augment: { ...cur.augment, level: clamped } },
        },
      }
    })
  },

  addAffix: (slot, affixId, tier) => {
    // Appends a new affix entry to the equipped item, refusing to add when the item already holds its `maxAffixes` cap. Used by GearView's affix picker.
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      const base = getItem(cur.baseId)
      if (base?.maxAffixes !== undefined && cur.affixes.length >= base.maxAffixes)
        return s
      const next: EquippedItem = {
        ...cur,
        affixes: [...cur.affixes, { affixId, tier, roll: 1 }],
      }
      return { inventory: { ...s.inventory, [slot]: next } }
    })
  },

  removeAffix: (slot, index) => {
    // Removes the affix at `index` from the equipped item. Used by GearView's per-affix delete button.
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur || index < 0 || index >= cur.affixes.length) return s
      const affixes = cur.affixes.filter((_, i) => i !== index)
      return { inventory: { ...s.inventory, [slot]: { ...cur, affixes } } }
    })
  },

  setAffixRoll: (slot, index, roll) => {
    // Sets the 0-1 roll position of an affix on the equipped item, clamping out-of-range values. Used by GearView's affix roll slider.
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur || index < 0 || index >= cur.affixes.length) return s
      const clamped = Math.max(0, Math.min(1, roll))
      const affixes = cur.affixes.map((a, i) =>
        i === index ? { ...a, roll: clamped } : a,
      )
      return { inventory: { ...s.inventory, [slot]: { ...cur, affixes } } }
    })
  },

  addForgedMod: (slot, modId, tier) => {
    // Sets the (single) forged crystal mod on the equipped item, replacing any existing one, and resyncs the socket arrays against the (possibly new) max socket count. Used by GearView's forge picker.
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      const forgedMods = [{ affixId: modId, tier, roll: 1 }]
      const newMax = maxSocketsFor(cur.baseId, forgedMods)
      const socketCount = Math.min(cur.socketCount, newMax)
      const socketed = cur.socketed.slice(0, socketCount)
      const socketTypes = cur.socketTypes.slice(0, socketCount)
      return {
        inventory: {
          ...s.inventory,
          [slot]: { ...cur, forgedMods, socketCount, socketed, socketTypes },
        },
      }
    })
  },

  removeForgedMod: (slot, index) => {
    // Removes the forged mod at `index` from the equipped item and re-clamps socket arrays in case the bonus-socket mod has been removed. Used by GearView's forge clear action.
    set((s) => {
      const cur = s.inventory[slot]
      const list = cur?.forgedMods ?? []
      if (!cur || index < 0 || index >= list.length) return s
      const forgedMods = list.filter((_, i) => i !== index)
      const newMax = maxSocketsFor(cur.baseId, forgedMods)
      const socketCount = Math.min(cur.socketCount, newMax)
      const socketed = cur.socketed.slice(0, socketCount)
      const socketTypes = cur.socketTypes.slice(0, socketCount)
      return {
        inventory: {
          ...s.inventory,
          [slot]: { ...cur, forgedMods, socketCount, socketed, socketTypes },
        },
      }
    })
  },

  moveItem: (fromSlot, toSlot) => {
    // Swaps the items in two inventory slots (no-op when the source equals the destination). Used by GearView drag-and-drop between slots.
    set((s) => {
      if (fromSlot === toSlot) return s
      const fromItem = s.inventory[fromSlot]
      const toItem = s.inventory[toSlot]
      const next = { ...s.inventory }

      if (fromItem) {
        next[toSlot] = fromItem
      } else {
        delete next[toSlot]
      }

      if (toItem) {
        next[fromSlot] = toItem
      } else {
        delete next[fromSlot]
      }

      return { inventory: next }
    })
  },

  setSkillRank: (skillId, rank, maxRank) => {
    // Sets a skill's rank, clamped against the unallocated skill-points budget at the current level, the per-skill maxRank, and zero. Refuses to allocate a rank when the skill has a prerequisite that is not yet learned, and cascades-removes dependants when the skill is reduced to 0. Used by SkillsView's per-skill rank controls.
    const { skillRanks, level } = get()
    const total = skillPointsFor(level)
    const currentSkillRank = skillRanks[skillId] ?? 0
    const otherSpent =
      Object.entries(skillRanks).reduce(
        (s, [id, r]) => (id === skillId ? s : s + r),
        0,
      )
    const clamped = Math.max(0, Math.min(maxRank, rank, total - otherSpent))
    if (clamped === currentSkillRank) return

    const skillDef = ALL_SKILLS.find((s) => s.id === skillId)
    if (clamped > 0 && skillDef?.requiresSkill) {
      const reqRank = skillRanks[skillDef.requiresSkill] ?? 0
      if (reqRank === 0) return
    }

    const next = { ...skillRanks }
    if (clamped === 0) delete next[skillId]
    else next[skillId] = clamped

    if (clamped === 0) {
      let changed = true
      while (changed) {
        changed = false
        for (const dep of ALL_SKILLS) {
          if (
            dep.requiresSkill &&
            (next[dep.id] ?? 0) > 0 &&
            !next[dep.requiresSkill]
          ) {
            delete next[dep.id]
            changed = true
          }
        }
      }
    }

    set({ skillRanks: next })
  },

  incSkillRank: (skillId, maxRank) => {
    // Convenience helper that adds one rank to a skill through `setSkillRank` so the same clamping/cascade rules apply. Used by SkillsView's "+" button.
    const { skillRanks } = get()
    const cur = skillRanks[skillId] ?? 0
    get().setSkillRank(skillId, cur + 1, maxRank)
  },

  decSkillRank: (skillId) => {
    // Decrements a skill's rank by one (no-op at zero). Does not handle prerequisite cascades because reducing rank above zero never invalidates any dependant. Used by SkillsView's "-" button.
    const { skillRanks } = get()
    const cur = skillRanks[skillId] ?? 0
    if (cur <= 0) return
    const next = { ...skillRanks }
    if (cur - 1 === 0) delete next[skillId]
    else next[skillId] = cur - 1
    set({ skillRanks: next })
  },

  resetSkillRanks: () => set({ skillRanks: {} }),
  // Clears every allocated skill rank. Used by SkillsView's reset button.

  dismissStorageError: () => set({ storageError: null }),
  // Clears the recorded storage-write error, e.g. after the user dismisses the storage-full banner.

  saveCurrentAsNewBuild: (name, notes = '', folderId = null) =>
    guardStorage<SavedBuild | null>(
      (m) => set({ storageError: m }),
      null,
      () => {
        // Saves the live in-memory state as a brand-new SavedBuild with a single profile and binds the store to it, optionally filing it under `folderId`. Returns the new record, or null (with storageError set) when localStorage rejects the write. Used by the "Save as new" flow.
        const snapshot = get().exportBuildSnapshot()
        const record = storeCreateBuild(
          name,
          snapshot,
          undefined,
          notes,
          folderId,
        )
        set((cur) => ({
          activeBuildId: record.id,
          activeProfileId: record.activeProfileId,
          savedBuildsVersion: cur.savedBuildsVersion + 1,
        }))
        return record
      },
    ),

  duplicateSavedBuild: (buildId) =>
    guardStorage<SavedBuild | null>(
      (m) => set({ storageError: m }),
      null,
      () => {
        // Deep-clones a SavedBuild on disk (new ids, "(copy)" name) and bumps the savedBuilds version. Returns the new record, or null when the source build is missing. Used by the build library's "Copy" action.
        const record = storeDuplicateBuild(buildId)
        if (record) bumpSavedBuilds(set)
        return record
      },
    ),

  setSavedBuildFavorite: (buildId, favorite) =>
    guardStorage(
      (m) => set({ storageError: m }),
      false,
      () => {
        // Toggles the `favorite` flag on a SavedBuild and bumps the savedBuilds version. Used by the build library's star toggle.
        const ok = storeSetBuildFavorite(buildId, favorite) !== null
        if (ok) bumpSavedBuilds(set)
        return ok
      },
    ),

  setSavedBuildTags: (buildId, tags) =>
    guardStorage(
      (m) => set({ storageError: m }),
      false,
      () => {
        // Replaces a SavedBuild's tag list on disk and bumps the savedBuilds version. Used by the build library's tag editor.
        const ok = storeSetBuildTags(buildId, tags) !== null
        if (ok) bumpSavedBuilds(set)
        return ok
      },
    ),

  moveSavedBuildToFolder: (buildId, folderId) =>
    guardStorage(
      (m) => set({ storageError: m }),
      false,
      () => {
        // Moves a SavedBuild into a folder (or unfiles it) on disk and bumps the savedBuilds version. Used by the build library's "Move to folder" action.
        const ok = storeMoveBuildToFolder(buildId, folderId) !== null
        if (ok) bumpSavedBuilds(set)
        return ok
      },
    ),

  switchSavedBuildProfile: (buildId, profileId) =>
    guardStorage(
      (m) => set({ storageError: m }),
      false,
      () => {
        // Switches a SavedBuild's active profile from the Build Select library. When the build is the one currently loaded in the planner this delegates to switchActiveProfile so the live editor re-hydrates and stays in sync; otherwise it just updates the stored activeProfileId.
        const s = get()
        if (buildId === s.activeBuildId) return s.switchActiveProfile(profileId)
        const ok = storeSetActiveProfile(buildId, profileId) !== null
        if (ok) bumpSavedBuilds(set)
        return ok
      },
    ),

  addSavedBuildProfile: (buildId, name) =>
    guardStorage<string | null>(
      (m) => set({ storageError: m }),
      null,
      () => {
        // Appends a new profile to a SavedBuild from the Build Select library, seeded from the build's current active profile. When the build is loaded in the planner its live profile is committed first so the seed is fresh. The new profile is NOT activated — the build keeps its current active profile. Returns the new profile id, or null when the build is missing or its code cannot be decoded.
        const s = get()
        if (buildId === s.activeBuildId && s.activeProfileId) {
          storeCommitProfile(buildId, s.activeProfileId, s.exportBuildSnapshot())
        }
        const build = getSavedBuild(buildId)
        if (!build) return null
        const seed = loadProfileSnapshot(buildId, build.activeProfileId)
        if (!seed) return null
        const result = storeAddProfile(buildId, name, seed, {
          activate: false,
        })
        if (!result) return null
        bumpSavedBuilds(set)
        return result.profile.id
      },
    ),

  renameSavedBuildProfile: (buildId, profileId, name) =>
    guardStorage(
      (m) => set({ storageError: m }),
      false,
      () => {
        // Renames a profile inside any SavedBuild from the Build Select library. Rename never touches live editor state, so no planner-sync delegation is needed.
        const ok = storeRenameProfile(buildId, profileId, name) !== null
        if (ok) bumpSavedBuilds(set)
        return ok
      },
    ),

  duplicateSavedBuildProfile: (buildId, profileId) =>
    guardStorage<string | null>(
      (m) => set({ storageError: m }),
      null,
      () => {
        // Duplicates a profile inside any SavedBuild from the Build Select library without changing the build's active profile. Returns the new profile id, or null when the source is missing.
        const result = storeDuplicateProfile(buildId, profileId, {
          activate: false,
        })
        if (!result) return null
        bumpSavedBuilds(set)
        return result.profile.id
      },
    ),

  removeSavedBuildProfile: (buildId, profileId) =>
    guardStorage(
      (m) => set({ storageError: m }),
      false,
      () => {
        // Removes a profile from a SavedBuild via the Build Select library. When the build is loaded in the planner this delegates to removeActiveProfile so the live editor re-hydrates from the surviving active profile; otherwise it removes the profile directly. Refuses to remove the last surviving profile.
        const s = get()
        if (buildId === s.activeBuildId) return s.removeActiveProfile(profileId)
        const ok = storeRemoveProfile(buildId, profileId) !== null
        if (ok) bumpSavedBuilds(set)
        return ok
      },
    ),

  createSavedFolder: (name, parentId) =>
    guardStorage<Folder | null>(
      (m) => set({ storageError: m }),
      null,
      () => {
        // Creates a new folder on disk and bumps the savedBuilds version. Used by the build library's "New folder" action.
        const folder = storeCreateFolder(name, parentId)
        bumpSavedBuilds(set)
        return folder
      },
    ),

  renameSavedFolder: (folderId, name) =>
    guardStorage(
      (m) => set({ storageError: m }),
      false,
      () => {
        // Renames a folder on disk and bumps the savedBuilds version. Used by the build library's folder rename action.
        const ok = storeRenameFolder(folderId, name) !== null
        if (ok) bumpSavedBuilds(set)
        return ok
      },
    ),

  deleteSavedFolder: (folderId, cascade) =>
    guardStorage(
      (m) => set({ storageError: m }),
      false,
      () => {
        // Deletes a folder on disk (cascade removes its subtree + builds, otherwise reparents children and unfiles builds) and bumps the savedBuilds version. Used by the build library's folder delete action.
        storeDeleteFolder(folderId, { cascade })
        bumpSavedBuilds(set)
        return true
      },
    ),
}))

export function skillPointsFor(level: number): number {
  // Returns the total number of skill points the character has earned at the supplied level. Used by setSkillRank's budget check and by the SkillsView header.
  return level * gameConfig.skillPointsPerLevel
}

export function subskillPointsFor(level: number): number {
  // Returns the total number of subskill points available at the supplied level (one per five levels). Used by setSubskillRank's clamping and by the subtree UI.
  return Math.floor(level / 5)
}

export function subskillKey(skillId: string, subskillId: string): string {
  // Returns the composite key (`skillId:subskillId`) used to address a single subskill node inside the flat `subskillRanks` map. Used everywhere subskill ranks are read or written.
  return `${skillId}:${subskillId}`
}

export function attrPointsFor(level: number): number {
  // Returns the total number of attribute points the character has earned at the supplied level. Used by incAttr's budget check and by CharacterView's header.
  return level * gameConfig.attributePointsPerLevel
}

export function finalAttributes(
  classId: string | null,
  allocated: AttrMap,
): AttrMap {
  // Computes the final attribute totals (default base + class base + allocated points) for every game attribute. Used by CharacterView and any caller that needs the player's actual attribute values.
  const cls = classId ? getClass(classId) : undefined
  const out = emptyAllocation()
  for (const attr of gameConfig.attributes) {
    const defaultBase = gameConfig.defaultBaseAttributes?.[attr.key] ?? 0
    const classBase = cls?.baseAttributes[attr.key] ?? 0
    const spent = allocated[attr.key] ?? 0
    out[attr.key] = defaultBase + classBase + spent
  }
  return out
}
