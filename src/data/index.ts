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

export const gameConfig = gameConfigJson as GameConfig

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

export const classes: CharacterClass[] = collectScalar(classModules)
export const skills: Skill[] = collectFlat(skillModules)
export const talentTrees: TalentTree[] = collectScalar(talentModules)
export const items: ItemBase[] = collectFlat(itemModules)
export const gems: Gem[] = collectFlat(gemModules)
export const runes: Rune[] = collectFlat(runeModules)

export const affixes: Affix[] = affixesJson as Affix[]
export const crystalMods: Affix[] = crystalsJson as Affix[]
export const runewords: Runeword[] = runewordsJson as unknown as Runeword[]
export const itemSets: ItemSet[] = setsJson as ItemSet[]
export const itemGrantedSkills: ItemGrantedSkill[] =
  itemGrantedSkillsJson as ItemGrantedSkill[]

const itemGrantedSkillByName = new Map<string, ItemGrantedSkill>(
  itemGrantedSkills.map((s) => [s.name.trim().toLowerCase(), s]),
)

export function getItemGrantedSkillByName(
  name: string,
): ItemGrantedSkill | undefined {
  return itemGrantedSkillByName.get(name.trim().toLowerCase())
}
export const relics: Relic[] = []
export const augments: AngelicAugment[] = augmentsJson as AngelicAugment[]
export const GEAR_SLOTS = new Set<string>([
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

export function getSkillImage(
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

export function getSocketableById(
  id: string,
): { kind: 'gem'; data: Gem } | { kind: 'rune'; data: Rune } | undefined {
  const gem = getGem(id)
  if (gem) return { kind: 'gem', data: gem }
  const rune = getRune(id)
  if (rune) return { kind: 'rune', data: rune }
  return undefined
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
