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
  // Pulls the `default` export out of every entry in a `import.meta.glob` module map and returns them as a flat array. Used to gather single-record JSON files such as classes and talent trees into runtime arrays.
  return Object.values(modules).map((m) => m.default)
}

function collectFlat<T>(modules: Record<string, { default: T[] }>): T[] {
  // Pulls the `default`-exported arrays out of every entry in a `import.meta.glob` module map and concatenates them. Used to gather array-typed JSON files (skills, items, gems, runes) into one runtime array.
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
const itemSets: ItemSet[] = setsJson as ItemSet[]
const itemGrantedSkills: ItemGrantedSkill[] =
  itemGrantedSkillsJson as ItemGrantedSkill[]

const itemGrantedSkillByName = new Map<string, ItemGrantedSkill>(
  itemGrantedSkills.map((s) => [s.name.trim().toLowerCase(), s]),
)

export function getItemGrantedSkillByName(
  name: string,
): ItemGrantedSkill | undefined {
  // Looks up an item-granted-skill entry by its display name using a normalised (trim+lowercase) key. Used by the stats pipeline to find the metadata for a skill rolled on an item affix.
  return itemGrantedSkillByName.get(name.trim().toLowerCase())
}
export const relics: Relic[] = []
export const augments: AngelicAugment[] = augmentsJson as AngelicAugment[]
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
  // Returns true when the supplied slot key refers to a wearable gear slot (rather than a special slot such as relic or augment). Used by the stats pipeline to decide whether to apply star scaling and gear-only logic.
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
  // Returns the forging kind available for an item of the supplied rarity (currently `satanic_crystal` for satanic-tier and above, null otherwise). Used by the gear UI to decide which forge picker to show.
  if (SATANIC_CRYSTAL_RARITIES.has(rarity)) return 'satanic_crystal'
  return null
}
export const FORGE_KIND_LABEL: Record<ForgeKind, string> = {
  satanic_crystal: 'Satanic Crystal',
}

function indexById<T extends { id: string }>(list: T[]): Map<string, T> {
  // Returns a Map keyed by each entry's `id` field for O(1) lookup. Used to build every per-data-type index in this module.
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
  // Returns every skill belonging to the supplied class, or an empty list when the class id is missing or unknown. Used by SkillsView and the stats pipeline.
  if (!classId) return EMPTY_SKILLS
  return skillsByClassId.get(classId) ?? EMPTY_SKILLS
}

export function getClass(id: string): CharacterClass | undefined {
  // Looks up a character class by id. Used by the stats pipeline and CharacterView.
  return classIndex.get(id)
}

export function getItem(id: string): ItemBase | undefined {
  // Looks up an item base by id. Used everywhere an equipped item's static metadata is needed.
  return itemIndex.get(id)
}

export function getItemImage(id: string): string | undefined {
  // Returns the bundled asset URL for the item icon matching the supplied item id, or undefined when no icon ships in `src/assets/items`. Used by GearView and tooltips for icon rendering.
  return itemImageIndex.get(id)
}

function getSkillImage(
  classId: string,
  skillId: string,
): string | undefined {
  // Returns the bundled asset URL for a skill icon matching `src/assets/skills/{classId}/{skillId}.{png,webp,jpg,jpeg}`, or undefined when no local sprite ships.
  return skillImageIndex.get(`${classId}/${skillId}`)
}

export function getClassIcon(classId: string): string | undefined {
  // Returns the bundled asset URL for a class avatar matching `src/assets/classes/{classId}.{png,webp,jpg,jpeg}`, or undefined when no local sprite ships. Used by the build library to show the class emblem in build cards and the "By class" sidebar.
  return classImageIndex.get(classId)
}

export function resolveSkillIcon(skill: {
  id: string
  classId: string
  icon?: string
}): string | undefined {
  // Returns the best available icon reference for a skill: prefers a bundled local sprite (src/assets/skills/{classId}/{id}.png), then falls back to the JSON-supplied `icon` (typically a wiki URL or an emoji glyph). Used by SkillsView and skill tooltips.
  return getSkillImage(skill.classId, skill.id) ?? skill.icon
}

export function getGem(id: string): Gem | undefined {
  // Looks up a gem by id. Used by socket pickers and the stats pipeline.
  return gemIndex.get(id)
}

export function getRune(id: string): Rune | undefined {
  // Looks up a rune by id. Used by socket pickers and the stats pipeline.
  return runeIndex.get(id)
}

export function getRuneword(id: string): Runeword | undefined {
  // Looks up a runeword definition by id. Used by GearView's runeword picker and by detectRuneword when validating a manual rune sequence.
  return runewordIndex.get(id)
}

export function getAffix(id: string): Affix | undefined {
  // Looks up an affix by id. Used by the stats pipeline and the affix picker.
  return affixIndex.get(id)
}

export function getCrystalMod(id: string): Affix | undefined {
  // Looks up a crystal forge mod by id. Used by the stats pipeline and the forge picker.
  return crystalModIndex.get(id)
}

export function getItemSet(id: string): ItemSet | undefined {
  // Looks up an item set by id. Used by the stats pipeline when applying set-bonus tiers.
  return itemSetIndex.get(id)
}

export function getAugment(id: string): AngelicAugment | undefined {
  // Looks up an angelic augment by id. Used by the augment picker and the stats pipeline.
  return augmentIndex.get(id)
}

export function detectRuneword(
  base: ItemBase,
  socketed: (string | null)[],
): Runeword | undefined {
  // Returns the runeword that exactly matches the runes currently socketed in the supplied common-rarity item, or undefined when nothing matches. Used by the stats pipeline to apply runeword stats and by GearView to highlight the active runeword.
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
