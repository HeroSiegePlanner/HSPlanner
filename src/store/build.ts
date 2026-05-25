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
  duplicateProfile as storeDuplicateProfile,
  getSavedBuild,
  loadProfileSnapshot,
  removeProfile as storeRemoveProfile,
  renameProfile as storeRenameProfile,
  renameBuild as storeRenameBuild,
  setActiveProfile as storeSetActiveProfile,
  setBuildNotes as storeSetBuildNotes,
  type SavedBuild,
} from '../utils/savedBuilds'
import { guardStorage } from './storageError'
import { sanitizeHtml } from '../utils/sanitizeHtml'
import {
  defaultEnemyResistances,
  DEFAULT_ENEMY_RESISTANCE_PCT,
  type BuildSnapshot,
} from '../utils/shareBuild'

export { defaultEnemyResistances, DEFAULT_ENEMY_RESISTANCE_PCT }
import { ADJ, findPath, reachableFromAny, START_IDS } from '../utils/treeGraph'

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
  bindToBuild: (buildId: string, profileId: string) => void
  deleteSavedBuild: (buildId: string) => void
  renameSavedBuild: (buildId: string, name: string) => boolean
  saveCurrentAsNewBuild: (name: string, notes?: string) => SavedBuild | null
  dismissStorageError: () => void
}

export { START_IDS as TREE_START_IDS, START_SET as TREE_START_SET } from '../utils/treeGraph'
export { findPath as findTreePath } from '../utils/treeGraph'

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

const HARD_SOCKET_CAP = 6
export const BONUS_SOCKET_MOD_ID = 'crystal_add_socket'

export function hasBonusSocketMod(
  forgedMods?: { affixId: string }[] | null,
): boolean {
  return !!forgedMods?.some((m) => m.affixId === BONUS_SOCKET_MOD_ID)
}

export function maxSocketsFor(
  baseId: string,
  forgedMods?: { affixId: string }[] | null,
): number {
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

  // Enforces two-handed rule: offhand cannot coexist with a 2H weapon.
  equipItem: (slot, baseId) => {
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

  // Preserves the level when reapplying the same augment; new augments default to level 1.
  setAugment: (augmentId) => {
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
    set((s) => {
      const cur = s.inventory[slot]
      if (!cur || index < 0 || index >= cur.affixes.length) return s
      const affixes = cur.affixes.filter((_, i) => i !== index)
      return { inventory: { ...s.inventory, [slot]: { ...cur, affixes } } }
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

  // Resyncs the socket arrays against the (possibly new) max socket count.
  addForgedMod: (slot, modId, tier) => {
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

  saveCurrentAsNewBuild: (name, notes = '') =>
    guardStorage<SavedBuild | null>(
      (m) => set({ storageError: m }),
      null,
      () => {
        const snapshot = get().exportBuildSnapshot()
        const record = storeCreateBuild(name, snapshot, undefined, notes)
        set((cur) => ({
          activeBuildId: record.id,
          activeProfileId: record.activeProfileId,
          savedBuildsVersion: cur.savedBuildsVersion + 1,
        }))
        return record
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
