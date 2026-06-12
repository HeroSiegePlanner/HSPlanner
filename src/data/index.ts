import type {
  GameConfig,
  CharacterClass,
  Skill,
  TalentTree,
  ItemBase,
  ItemGrantedSkill,
  Affix,
  Rune,
  Gem,
  Runeword,
  ItemSet,
  Relic,
  AngelicAugment,
} from '../types'
import affixesJson from './affixes.json'
import augmentsJson from './augments.json'
import crystalsJson from './crystals.json'
import gameConfigJson from './game-config.json'
import itemGrantedSkillsJson from './item-granted-skills.json'
import runewordsJson from './runewords.json'
import setsJson from './sets.json'
import treeNodesJson from './tree-nodes.json'
import heroSiegeTreeJson from './hero-siege-tree.json'
import nodeIconsJson from './node-icons.json'
import { resolveActiveSeasonId } from './seasons/registry'
import { loadSeasonPatchSet } from './seasons/load'
import {
  applyGameConfigPatch,
  applyListPatch,
  applyRecordMergePatch,
  applyRecordReplacePatch,
  applyTreePatch,
  type PatchResult,
} from './seasons/resolve'
import type {
  HeroSiegeTree,
  RecordPatch,
  TreeNodeInfo,
} from './seasons/patchTypes'

export const activeSeasonId = resolveActiveSeasonId()

const seasonErrors: string[] = []
const seasonLoad = loadSeasonPatchSet(activeSeasonId)
seasonErrors.push(...seasonLoad.errors)
const seasonPatches = seasonLoad.patches

// All-or-nothing per collection: on any patch error the base data is kept so
// a broken overlay can never half-apply (spec §6).
function patched<T>(base: T, result: PatchResult<T>): T {
  if (result.errors.length > 0) {
    seasonErrors.push(...result.errors)
    return base
  }
  return result.data
}

export const gameConfig = patched(
  gameConfigJson as GameConfig,
  applyGameConfigPatch(
    gameConfigJson as GameConfig & Record<string, unknown>,
    seasonPatches.gameConfig,
    'game-config',
  ) as PatchResult<GameConfig>,
)

const classModules = import.meta.glob<{ default: CharacterClass }>(
  './classes/*.json',
  { eager: true },
)
const skillModules = import.meta.glob<{ default: Skill[] }>(
  './skills/*.json',
  { eager: true },
)
const talentModules = import.meta.glob<{ default: TalentTree }>(
  './talents/*.json',
  { eager: true },
)
const itemModules = import.meta.glob<{ default: ItemBase[] }>(
  './items/*.json',
  { eager: true },
)
const gemModules = import.meta.glob<{ default: Gem[] }>(
  './gems/*.json',
  { eager: true },
)
const runeModules = import.meta.glob<{ default: Rune[] }>(
  './runes/*.json',
  { eager: true },
)
const itemImageModules = import.meta.glob<string>(
  '../assets/items/*.{png,webp,jpg,jpeg}',
  { eager: true, query: '?url', import: 'default' },
)

const itemImageIndex = new Map<string, string>()
for (const [path, url] of Object.entries(itemImageModules)) {
  const file = path.split('/').pop() ?? ''
  const id = file.replace(/\.(png|webp|jpe?g)$/i, '')
  if (id) itemImageIndex.set(id, url)
}

const skillImageModules = import.meta.glob<string>(
  '../assets/skills/**/*.{png,webp,jpg,jpeg}',
  { eager: true, query: '?url', import: 'default' },
)

const skillImageIndex = new Map<string, string>()
for (const [path, url] of Object.entries(skillImageModules)) {
  const parts = path.split('/')
  const file = parts.pop() ?? ''
  const classDir = parts.pop() ?? ''
  const skillId = file.replace(/\.(png|webp|jpe?g)$/i, '')
  if (classDir && skillId) {
    skillImageIndex.set(`${classDir}/${skillId}`, url)
  }
}

const classImageModules = import.meta.glob<string>(
  '../assets/classes/*.{png,webp,jpg,jpeg}',
  { eager: true, query: '?url', import: 'default' },
)

const classImageIndex = new Map<string, string>()
for (const [path, url] of Object.entries(classImageModules)) {
  const file = path.split('/').pop() ?? ''
  const id = file.replace(/\.(png|webp|jpe?g)$/i, '')
  if (id) classImageIndex.set(id, url)
}

function collectScalar<T>(modules: Record<string, { default: T }>): T[] {
  return Object.values(modules).map((m) => m.default)
}

function collectFlat<T>(modules: Record<string, { default: T[] }>): T[] {
  return Object.values(modules).flatMap((m) => m.default)
}

export const classes: CharacterClass[] = patched(
  collectScalar(classModules),
  applyListPatch(
    collectScalar(classModules) as unknown as Record<string, unknown>[],
    seasonPatches.classes,
    'classes',
  ) as unknown as PatchResult<CharacterClass[]>,
)
export const skills: Skill[] = patched(
  collectFlat(skillModules),
  applyListPatch(
    collectFlat(skillModules) as unknown as Record<string, unknown>[],
    seasonPatches.skills,
    'skills',
  ) as unknown as PatchResult<Skill[]>,
)
export const talentTrees: TalentTree[] = collectScalar(talentModules)
export const items: ItemBase[] = patched(
  collectFlat(itemModules),
  applyListPatch(
    collectFlat(itemModules) as unknown as Record<string, unknown>[],
    seasonPatches.items,
    'items',
  ) as unknown as PatchResult<ItemBase[]>,
)
export const gems: Gem[] = patched(
  collectFlat(gemModules),
  applyListPatch(
    collectFlat(gemModules) as unknown as Record<string, unknown>[],
    seasonPatches.gems,
    'gems',
  ) as unknown as PatchResult<Gem[]>,
)
export const runes: Rune[] = patched(
  collectFlat(runeModules),
  applyListPatch(
    collectFlat(runeModules) as unknown as Record<string, unknown>[],
    seasonPatches.runes,
    'runes',
  ) as unknown as PatchResult<Rune[]>,
)

export const affixes: Affix[] = patched(
  affixesJson as Affix[],
  applyListPatch(
    affixesJson as unknown as Record<string, unknown>[],
    seasonPatches.affixes,
    'affixes',
  ) as unknown as PatchResult<Affix[]>,
)
export const crystalMods: Affix[] = patched(
  crystalsJson as Affix[],
  applyListPatch(
    crystalsJson as unknown as Record<string, unknown>[],
    seasonPatches.crystals,
    'crystals',
  ) as unknown as PatchResult<Affix[]>,
)
export const runewords: Runeword[] = patched(
  runewordsJson as unknown as Runeword[],
  applyListPatch(
    runewordsJson as unknown as Record<string, unknown>[],
    seasonPatches.runewords,
    'runewords',
  ) as unknown as PatchResult<Runeword[]>,
)
const itemSets: ItemSet[] = patched(
  setsJson as ItemSet[],
  applyListPatch(
    setsJson as unknown as Record<string, unknown>[],
    seasonPatches.sets,
    'sets',
  ) as unknown as PatchResult<ItemSet[]>,
)
const itemGrantedSkills: ItemGrantedSkill[] = patched(
  itemGrantedSkillsJson as ItemGrantedSkill[],
  applyListPatch(
    itemGrantedSkillsJson as unknown as Record<string, unknown>[],
    seasonPatches.itemGrantedSkills,
    'item-granted-skills',
    'name',
  ) as unknown as PatchResult<ItemGrantedSkill[]>,
)

const itemGrantedSkillByName = new Map<string, ItemGrantedSkill>(
  itemGrantedSkills.map((s) => [s.name.trim().toLowerCase(), s]),
)

export function getItemGrantedSkillByName(
  name: string,
): ItemGrantedSkill | undefined {
  return itemGrantedSkillByName.get(name.trim().toLowerCase())
}
export const relics: Relic[] = []
export const augments: AngelicAugment[] = patched(
  augmentsJson as AngelicAugment[],
  applyListPatch(
    augmentsJson as unknown as Record<string, unknown>[],
    seasonPatches.augments,
    'augments',
  ) as unknown as PatchResult<AngelicAugment[]>,
)
export const treeNodeInfo: Record<string, TreeNodeInfo> = patched(
  treeNodesJson as Record<string, TreeNodeInfo>,
  applyRecordMergePatch(
    treeNodesJson as Record<string, Record<string, unknown>>,
    seasonPatches.treeNodes as unknown as
      | RecordPatch<Record<string, unknown>>
      | undefined,
    'tree-nodes',
  ) as unknown as PatchResult<Record<string, TreeNodeInfo>>,
)
export const heroSiegeTree: HeroSiegeTree = patched(
  heroSiegeTreeJson as HeroSiegeTree,
  applyTreePatch(heroSiegeTreeJson as HeroSiegeTree, seasonPatches.heroSiegeTree, 'hero-siege-tree'),
)
export const nodeIcons: Record<string, string> = patched(
  nodeIconsJson as Record<string, string>,
  applyRecordReplacePatch(
    nodeIconsJson as Record<string, string>,
    seasonPatches.nodeIcons,
    'node-icons',
  ),
)
export const seasonDataErrors: ReadonlyArray<string> = seasonErrors
const GEAR_SLOTS = new Set<string>([
  'weapon',
  'offhand',
  'helmet',
  'armor',
  'gloves',
  'boots',
  'belt',
  'amulet',
  'ring_1',
  'ring_2',
])
export function isGearSlot(slot: string): boolean {
  return GEAR_SLOTS.has(slot)
}
export type ForgeKind = 'satanic_crystal'

const SATANIC_CRYSTAL_RARITIES = new Set([
  'satanic',
  'satanic_set',
  'heroic',
  'angelic',
  'unholy',
  'relic',
])

export function forgeKindFor(rarity: string): ForgeKind | null {
  if (SATANIC_CRYSTAL_RARITIES.has(rarity)) return 'satanic_crystal'
  return null
}
export const FORGE_KIND_LABEL: Record<ForgeKind, string> = {
  satanic_crystal: 'Satanic Crystal',
}

function indexById<T extends { id: string }>(list: T[]): Map<string, T> {
  return new Map(list.map((entry) => [entry.id, entry]))
}

const classIndex = indexById(classes)
const itemIndex = indexById(items)
const gemIndex = indexById(gems)
const runeIndex = indexById(runes)
const runewordIndex = indexById(runewords)
const affixIndex = indexById(affixes)
const crystalModIndex = indexById(crystalMods)
const itemSetIndex = indexById(itemSets)
const augmentIndex = indexById(augments)

const skillsByClassId = new Map<string, Skill[]>()
for (const s of skills) {
  const list = skillsByClassId.get(s.classId)
  if (list) list.push(s)
  else skillsByClassId.set(s.classId, [s])
}

const EMPTY_SKILLS: Skill[] = []

export function getSkillsByClass(classId: string | null): Skill[] {
  if (!classId) return EMPTY_SKILLS
  return skillsByClassId.get(classId) ?? EMPTY_SKILLS
}

export function getClass(id: string): CharacterClass | undefined {
  return classIndex.get(id)
}

export function getItem(id: string): ItemBase | undefined {
  return itemIndex.get(id)
}

export function getItemImage(id: string): string | undefined {
  return itemImageIndex.get(id)
}

function getSkillImage(
  classId: string,
  skillId: string,
): string | undefined {
  return skillImageIndex.get(`${classId}/${skillId}`)
}

export function getClassIcon(classId: string): string | undefined {
  return classImageIndex.get(classId)
}

// Local sprite first, JSON-supplied `icon` (wiki URL or emoji) as fallback.
export function resolveSkillIcon(skill: {
  id: string
  classId: string
  icon?: string
}): string | undefined {
  return getSkillImage(skill.classId, skill.id) ?? skill.icon
}

export function getGem(id: string): Gem | undefined {
  return gemIndex.get(id)
}

export function getRune(id: string): Rune | undefined {
  return runeIndex.get(id)
}

export function getRuneword(id: string): Runeword | undefined {
  return runewordIndex.get(id)
}

export function getAffix(id: string): Affix | undefined {
  return affixIndex.get(id)
}

export function getCrystalMod(id: string): Affix | undefined {
  return crystalModIndex.get(id)
}

export function getItemSet(id: string): ItemSet | undefined {
  return itemSetIndex.get(id)
}

export function getAugment(id: string): AngelicAugment | undefined {
  return augmentIndex.get(id)
}

// Only common-rarity items can roll runewords; an exact rune-sequence match is required.
export function detectRuneword(
  base: ItemBase,
  socketed: (string | null)[],
): Runeword | undefined {
  if (base.rarity !== 'common') return undefined
  if (socketed.some((s) => !s)) return undefined
  for (const rw of runewords) {
    if (rw.runes.length !== socketed.length) continue
    if (!rw.allowedBaseTypes.includes(base.baseType)) continue
    let match = true
    for (let i = 0; i < rw.runes.length; i++) {
      if (rw.runes[i] !== socketed[i]) {
        match = false
        break
      }
    }
    if (match) return rw
  }
  return undefined
}
