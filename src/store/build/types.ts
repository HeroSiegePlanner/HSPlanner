import type {
  AttributeKey,
  CustomStat,
  EquippedItem,
  Inventory,
  SlotKey,
  SocketType,
  TreeSocketContent,
} from '../../types'
import type { Folder, SavedBuild } from '../../utils/build/savedBuilds'
import type { BuildSnapshot } from '../../utils/build/shareBuild'

export type AttrMap = Record<AttributeKey, number>

export interface BuildState {
  classId: string | null
  level: number
  allocated: AttrMap
  inventory: Inventory
  skillRanks: Record<string, number>
  allocatedTreeNodes: Set<number>
  treeSocketed: Record<number, TreeSocketContent | null>
  activeSkillIds: string[]
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

export interface BuildActions {
  setClass: (id: string) => void
  setLevel: (lvl: number) => void
  incAttr: (key: AttributeKey, amount?: number) => void
  decAttr: (key: AttributeKey, amount?: number) => void
  resetAttrs: () => void
  equipItem: (slot: SlotKey, baseId: string) => void
  unequipItem: (slot: SlotKey) => void
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
  toggleActiveSkill: (skillId: string) => void
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

export type BuildStore = BuildState & BuildActions
