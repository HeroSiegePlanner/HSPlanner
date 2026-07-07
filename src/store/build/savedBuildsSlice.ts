import type { StateCreator } from 'zustand'
import { activeSeasonId, classes } from '../../data'
import {
  PENDING_BUILD_KEY,
  PENDING_IMPORT_KEY,
  reloadIntoSeason,
} from '../../data/seasons/registry'
import { guardStorage } from '../storageError'
import { useSettings } from '../settings'
import { sanitizeHtml } from '../../utils/sanitizeHtml'
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
} from '../../utils/build/savedBuilds'
import type { Folder, SavedBuild } from '../../utils/build/savedBuilds'
import {
  createFolder as storeCreateFolder,
  deleteFolder as storeDeleteFolder,
  renameFolder as storeRenameFolder,
} from '../../utils/build/savedFolders'
import {
  defaultEnemyResistances,
  encodeBuildToShare,
} from '../../utils/build/shareBuild'
import { bumpSavedBuilds, emptyAllocation, snapshotPatch } from './helpers'
import type { BuildStore } from './types'

type SavedBuildsSlice = Pick<
  BuildStore,
  | 'activeBuildId'
  | 'activeProfileId'
  | 'savedBuildsVersion'
  | 'storageError'
  | 'notes'
  | 'customStats'
  | 'setNotes'
  | 'commitBuildNotes'
  | 'addCustomStat'
  | 'updateCustomStat'
  | 'removeCustomStat'
  | 'exportBuildSnapshot'
  | 'importBuildSnapshot'
  | 'detachFromBuild'
  | 'resetBuild'
  | 'loadSavedBuild'
  | 'changeActiveSeason'
  | 'switchActiveProfile'
  | 'commitActiveProfile'
  | 'saveBuildNow'
  | 'addProfileToActiveBuild'
  | 'duplicateActiveProfile'
  | 'renameActiveProfile'
  | 'removeActiveProfile'
  | 'deleteSavedBuild'
  | 'renameSavedBuild'
  | 'saveCurrentAsNewBuild'
  | 'duplicateSavedBuild'
  | 'setSavedBuildFavorite'
  | 'setSavedBuildTags'
  | 'moveSavedBuildToFolder'
  | 'switchSavedBuildProfile'
  | 'addSavedBuildProfile'
  | 'renameSavedBuildProfile'
  | 'duplicateSavedBuildProfile'
  | 'removeSavedBuildProfile'
  | 'createSavedFolder'
  | 'renameSavedFolder'
  | 'deleteSavedFolder'
  | 'dismissStorageError'
>

export const createSavedBuildsSlice: StateCreator<
  BuildStore,
  [],
  [],
  SavedBuildsSlice
> = (set, get) => ({
  activeBuildId: null,
  activeProfileId: null,
  savedBuildsVersion: 0,
  storageError: null,
  notes: '',
  customStats: [],

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
      activeSkillIds: s.activeSkillIds,
      activeAuraId: s.activeAuraId,
      activeBuffs: s.activeBuffs,
      enemyConditions: s.enemyConditions,
      playerConditions: s.playerConditions,
      skillProjectiles: s.skillProjectiles,
      enemyResistances: s.enemyResistances,
      procToggles: s.procToggles,
      disabledPotions: s.disabledPotions,
      killsPerSec: s.killsPerSec,
      customStats: s.customStats,
      allocatedEtherNodes: s.allocatedEtherNodes,
      mercClassId: s.mercClassId,
      mercSkillRanks: s.mercSkillRanks,
      mercInventory: s.mercInventory,
      mercDisabledAuras: s.mercDisabledAuras,
    }
  },

  importBuildSnapshot: (snapshot, notes = '') => {
    set(() => ({
      ...snapshotPatch(snapshot),
      notes,
      activeBuildId: null,
      activeProfileId: null,
    }))
  },

  detachFromBuild: () =>
    set(() => ({ activeBuildId: null, activeProfileId: null })),

  resetBuild: () =>
    set(() => ({
      classId: classes[0]?.id ?? null,
      level: 1,
      allocated: emptyAllocation(),
      inventory: {},
      skillRanks: {},
      allocatedTreeNodes: new Set<number>(),
      treeSocketed: {},
      activeSkillIds: [],
      activeAuraId: null,
      procToggles: {},
      disabledPotions: {},
      killsPerSec: 1,
      activeBuffs: {},
      enemyConditions: {},
      playerConditions: {},
      skillProjectiles: {},
      enemyResistances: defaultEnemyResistances(),
      subskillRanks: {},
      allocatedEtherNodes: new Set<number>(),
      mercClassId: null,
      mercSkillRanks: {},
      mercInventory: {},
      mercDisabledAuras: {},
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

  saveBuildNow: () => {
    const s = get()
    if (!s.activeBuildId || !s.activeProfileId) return false
    const ok = s.commitActiveProfile()
    s.commitBuildNotes()
    return ok
  },

  loadSavedBuild: (buildId, profileId) =>
    guardStorage(
      (m) => set({ storageError: m }),
      false,
      () => {
        const cur = get()
        if (
          useSettings.getState().autoSave &&
          cur.activeBuildId &&
          cur.activeProfileId
        ) {
          storeCommitProfile(
            cur.activeBuildId,
            cur.activeProfileId,
            cur.exportBuildSnapshot(),
          )
        }
        const build = getSavedBuild(buildId)
        if (!build) return false
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

  changeActiveSeason: (season) =>
    guardStorage(
      (m) => set({ storageError: m }),
      undefined,
      () => {
        const s = get()
        if (s.activeBuildId && s.activeProfileId) {
          if (useSettings.getState().autoSave) {
            storeCommitProfile(
              s.activeBuildId,
              s.activeProfileId,
              s.exportBuildSnapshot(),
            )
          }
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
        if (s.activeProfileId && useSettings.getState().autoSave) {
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
        if (s.activeProfileId && useSettings.getState().autoSave) {
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
        if (s.activeProfileId && useSettings.getState().autoSave) {
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
        const s = get()
        if (
          buildId === s.activeBuildId &&
          s.activeProfileId &&
          useSettings.getState().autoSave
        ) {
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
        storeDeleteFolder(folderId, { cascade })
        bumpSavedBuilds(set)
        return true
      },
    ),
})
