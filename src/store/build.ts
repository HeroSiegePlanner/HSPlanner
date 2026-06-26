import { create } from 'zustand'
import {
  activeSeasonId,
  classes,
  gameConfig,
  getClass,
  getItem,
  skills as ALL_SKILLS,
} from '../data'
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
  setBuildSeason as storeSetBuildSeason,
  setBuildTags as storeSetBuildTags,
  type Folder,
  type SavedBuild,
} from '../utils/build/savedBuilds'
import {
  createFolder as storeCreateFolder,
  deleteFolder as storeDeleteFolder,
  renameFolder as storeRenameFolder,
} from '../utils/build/savedFolders'
import {
  PENDING_BUILD_KEY,
  PENDING_IMPORT_KEY,
  reloadIntoSeason,
} from '../data/seasons/registry'
import { guardStorage } from './storageError'
import { setBridgeErrorListener } from '../lib/calc/bridge'
import { sanitizeHtml } from '../utils/sanitizeHtml'
import {
  defaultEnemyResistances,
  encodeBuildToShare,
  type BuildSnapshot,
} from '../utils/build/shareBuild'
import { ADJ, findPath, reachableFromAny, START_IDS } from '../utils/tree/treeGraph'
import * as itemEdits from '../views/gear/lib/itemEdits'

export {
  MAX_STARS,
  maxSocketsFor,
  BONUS_SOCKET_MOD_ID,
} from './itemRules'

type AttrMap = Record<AttributeKey, number>

export const RAINBOW_MULTIPLIER = 1.5

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
  commitEquippedItem: (slot: SlotKey, item: EquippedItem | null) => void
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
  changeActiveSeason: (season: string) => void
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
  return gameConfig.attributes.reduce<AttrMap>((acc, a) => {
    acc[a.key] = 0
    return acc
  }, {})
}


function bumpSavedBuilds(
  set: (fn: (s: BuildState) => Partial<BuildState>) => void,
) {
  set((s) => ({ savedBuildsVersion: s.savedBuildsVersion + 1 }))
}

// Excludes `notes` because notes live on the build, not the profile.
function snapshotPatch(snap: BuildSnapshot) {
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
      if (s.classId === id) return s
      // Drop state that references the old class's tree/skill IDs.
      return {
        classId: id,
        allocated: emptyAllocation(),
        skillRanks: {},
        mainSkillId: null,
        activeAuraId: null,
        procToggles: {},
        activeBuffs: {},
        subskillRanks: {},
        treeSocketed: {},
        skillProjectiles: {},
        activeBuildId: null,
        activeProfileId: null,
        notes: '',
        customStats: [],
      }
    }),

  setNotes: (html) =>
    set((s) => {
      const cleaned = sanitizeHtml(html)
      return s.notes === cleaned ? s : { notes: cleaned }
    }),

  commitBuildNotes: () =>
    guardStorage(
      (m) => set({ storageError: m }),
      false,
      () => {
        const s = get()
        if (!s.activeBuildId) return false
        const ok = storeSetBuildNotes(s.activeBuildId, s.notes) !== null
        if (ok) bumpSavedBuilds(set)
        return ok
      },
    ),

  addCustomStat: (init = {}) =>
    set((s) => ({
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
      if (index < 0 || index >= s.customStats.length) return s
      const next = s.customStats.map((cs, i) =>
        i === index ? { ...cs, ...patch } : cs,
      )
      return { customStats: next }
    }),

  removeCustomStat: (index) =>
    set((s) => {
      if (index < 0 || index >= s.customStats.length) return s
      return { customStats: s.customStats.filter((_, i) => i !== index) }
    }),

  // Deallocating prunes nodes unreachable from a START; allocating auto-pathfinds the cheapest connection to the existing allocation.
  toggleTreeNode: (nodeId) =>
    set((s) => {
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

  // Only pulls in START_IDS adjacent to the suggestion union — otherwise all 8 class entry points would pollute the allocation.
  applySuggestedNodes: (ids) =>
    set((s) => {
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

  setTreeSocketed: (nodeId, content) =>
    set((s) => {
      const next = { ...s.treeSocketed }
      if (content == null) delete next[nodeId]
      else next[nodeId] = content
      return { treeSocketed: next }
    }),

  setMainSkill: (skillId) => set({ mainSkillId: skillId }),

  setActiveAura: (skillId) => set({ activeAuraId: skillId }),

  setProcToggle: (skillId, enabled) =>
    set((s) => {
      const next = { ...s.procToggles }
      if (enabled) next[skillId] = true
      else delete next[skillId]
      return { procToggles: next }
    }),

  setKillsPerSec: (rate) =>
    set({ killsPerSec: Math.max(0, rate) }),

  setBuffActive: (skillId, enabled) =>
    set((s) => {
      const next = { ...s.activeBuffs }
      if (enabled) next[skillId] = true
      else delete next[skillId]
      return { activeBuffs: next }
    }),

  setEnemyCondition: (key, enabled) =>
    set((s) => {
      const next = { ...s.enemyConditions }
      if (enabled) next[key] = true
      else delete next[key]
      return { enemyConditions: next }
    }),

  setPlayerCondition: (key, enabled) =>
    set((s) => {
      const next = { ...s.playerConditions }
      if (enabled) next[key] = true
      else delete next[key]
      return { playerConditions: next }
    }),

  // Pass null/<=1 to clear the override (defaults to 1).
  setSkillProjectiles: (skillId, count) =>
    set((s) => {
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
      const next = { ...s.enemyResistances }
      if (value === null || !Number.isFinite(value)) {
        delete next[damageType]
      } else {
        next[damageType] = value
      }
      return { enemyResistances: next }
    }),

  setSubskillRank: (skillId, subskillId, rank, maxRank) => {
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
    const cur =
      get().subskillRanks[subskillKey(skillId, subskillId)] ?? 0
    get().setSubskillRank(skillId, subskillId, cur + 1, maxRank)
  },

  decSubskillRank: (skillId, subskillId) => {
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
      const next: Record<string, number> = {}
      for (const [k, v] of Object.entries(s.subskillRanks)) {
        if (!k.startsWith(`${skillId}:`)) next[k] = v
      }
      return { subskillRanks: next }
    }),

  setLevel: (lvl) => {
    const clamped = Math.max(1, Math.min(gameConfig.maxCharacterLevel, lvl))
    set({ level: clamped })
  },

  incAttr: (key, amount = 1) => {
    const { allocated, level } = get()
    const total = Object.values(allocated).reduce((s, v) => s + v, 0)
    const available = attrPointsFor(level) - total
    const step = Math.min(amount, available)
    if (step <= 0) return
    set({ allocated: { ...allocated, [key]: (allocated[key] ?? 0) + step } })
  },

  decAttr: (key, amount = 1) => {
    const { allocated } = get()
    const cur = allocated[key] ?? 0
    const step = Math.min(amount, cur)
    if (step <= 0) return
    set({ allocated: { ...allocated, [key]: cur - step } })
  },

  resetAttrs: () => set({ allocated: emptyAllocation() }),

  equipItem: (slot, baseId) => {
    const item = itemEdits.makeEquippedItem(baseId)
    if (!item) return
    get().commitEquippedItem(slot, item)
  },

  unequipItem: (slot) => {
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

  commitEquippedItem: (slot, item) => {
    set((s) => {
      if (item === null) {
        const next = { ...s.inventory }
        delete next[slot]
        return { inventory: next }
      }
      const base = getItem(item.baseId)
      if (!base) return s
      const next = { ...s.inventory, [slot]: item }
      if (slot === 'weapon' && base.twoHanded) {
        delete next.offhand
      }
      return { inventory: next }
    })
  },

  setSocketCount: (slot, count) => {
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      return { inventory: { ...s.inventory, [slot]: itemEdits.withSocketCount(cur, count) } }
    })
  },

  setSocketed: (slot, idx, socketableId) => {
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      return { inventory: { ...s.inventory, [slot]: itemEdits.withSocketed(cur, idx, socketableId) } }
    })
  },

  setSocketType: (slot, idx, type) => {
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      return { inventory: { ...s.inventory, [slot]: itemEdits.withSocketType(cur, idx, type) } }
    })
  },

  setStars: (slot, count) => {
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      return { inventory: { ...s.inventory, [slot]: itemEdits.withStars(cur, count) } }
    })
  },

  exportBuildSnapshot: () => {
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

  // Detaches from any saved build/profile so the import behaves as unsaved freeform.
  importBuildSnapshot: (snapshot, notes = '') => {
    set(() => ({
      ...snapshotPatch(snapshot),
      notes,
      activeBuildId: null,
      activeProfileId: null,
    }))
  },

  bindToBuild: (buildId, profileId) =>
    set((cur) => ({
      activeBuildId: buildId,
      activeProfileId: profileId,
      savedBuildsVersion: cur.savedBuildsVersion + 1,
    })),

  detachFromBuild: () =>
    set(() => ({ activeBuildId: null, activeProfileId: null })),

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
        // Auto-commits the previously-active profile so unsaved edits survive the load.
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
        // A build of another season reloads the app into its season, then reopens here.
        if (reloadIntoSeason(build.season, PENDING_BUILD_KEY, buildId, activeSeasonId)) {
          return true
        }
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

  // Moves the current build to another season and reloads; saved builds re-stamp, unsaved carry a snapshot.
  changeActiveSeason: (season) =>
    guardStorage(
      (m) => set({ storageError: m }),
      undefined,
      () => {
        const s = get()
        if (s.activeBuildId && s.activeProfileId) {
          storeCommitProfile(
            s.activeBuildId,
            s.activeProfileId,
            s.exportBuildSnapshot(),
          )
          storeSetBuildSeason(s.activeBuildId, season)
          reloadIntoSeason(
            season,
            PENDING_BUILD_KEY,
            s.activeBuildId,
            activeSeasonId,
          )
        } else {
          const code = encodeBuildToShare(s.exportBuildSnapshot(), s.notes)
          reloadIntoSeason(season, PENDING_IMPORT_KEY, code, activeSeasonId)
        }
      },
    ),

  switchActiveProfile: (profileId) =>
    guardStorage(
      (m) => set({ storageError: m }),
      false,
      () => {
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
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      return { inventory: { ...s.inventory, [slot]: itemEdits.withRuneword(cur, runewordId) } }
    })
  },

  setAugment: (augmentId) => {
    set((s) => {
      const cur = s.inventory.armor
      if (!cur) return s
      return { inventory: { ...s.inventory, armor: itemEdits.withAugment(cur, augmentId) } }
    })
  },

  setAugmentLevel: (level) => {
    set((s) => {
      const cur = s.inventory.armor
      if (!cur) return s
      return { inventory: { ...s.inventory, armor: itemEdits.withAugmentLevel(cur, level) } }
    })
  },

  addAffix: (slot, affixId, tier) => {
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      return { inventory: { ...s.inventory, [slot]: itemEdits.withAffixAdded(cur, affixId, tier) } }
    })
  },

  removeAffix: (slot, index) => {
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      return { inventory: { ...s.inventory, [slot]: itemEdits.withAffixRemoved(cur, index) } }
    })
  },

  setAffixRoll: (slot, index, roll) => {
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
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      return { inventory: { ...s.inventory, [slot]: itemEdits.withForgedModAdded(cur, modId, tier) } }
    })
  },

  removeForgedMod: (slot, index) => {
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur) return s
      return { inventory: { ...s.inventory, [slot]: itemEdits.withForgedModRemoved(cur, index) } }
    })
  },

  moveItem: (fromSlot, toSlot) => {
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

  // Cascades-removes dependants when a skill is reduced to 0 so we never leave orphaned ranks above a missing prereq.
  setSkillRank: (skillId, rank, maxRank) => {
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
    const { skillRanks } = get()
    const cur = skillRanks[skillId] ?? 0
    get().setSkillRank(skillId, cur + 1, maxRank)
  },

  decSkillRank: (skillId) => {
    const { skillRanks } = get()
    const cur = skillRanks[skillId] ?? 0
    if (cur <= 0) return
    const next = { ...skillRanks }
    if (cur - 1 === 0) delete next[skillId]
    else next[skillId] = cur - 1
    set({ skillRanks: next })
  },

  resetSkillRanks: () => set({ skillRanks: {} }),

  dismissStorageError: () => set({ storageError: null }),

  saveCurrentAsNewBuild: (name, notes = '', folderId = null) =>
    guardStorage<SavedBuild | null>(
      (m) => set({ storageError: m }),
      null,
      () => {
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
  return level * gameConfig.skillPointsPerLevel
}

export function subskillPointsFor(level: number): number {
  return Math.floor(level / 5)
}

export function subskillKey(skillId: string, subskillId: string): string {
  return `${skillId}:${subskillId}`
}

export function attrPointsFor(level: number): number {
  return level * gameConfig.attributePointsPerLevel
}

// Final attribute = default base + class base + allocated points.
export function finalAttributes(
  classId: string | null,
  allocated: AttrMap,
): AttrMap {
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

// Route Rust-side rejections through the existing storageError banner.
setBridgeErrorListener((err) => {
  useBuild.setState({
    storageError: `Calculation failed: ${err.message}`,
  })
})
