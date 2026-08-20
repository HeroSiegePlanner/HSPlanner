import type { Skill, SubskillNode } from '../../types'

const SUBSKILL_SPRITE_FILES = import.meta.glob<string>(
  '../../assets/subskills/**/*.png',
  { eager: true, query: '?url', import: 'default' },
)
const SUBSKILL_SPRITE_BY_KEY: Record<string, string> = {}
for (const [p, url] of Object.entries(SUBSKILL_SPRITE_FILES)) {
  const file = p.split('/').pop() ?? ''
  const key = file.replace(/\.png$/i, '')
  SUBSKILL_SPRITE_BY_KEY[key] = url
}

// Nazwa pliku jest kluczem: <classId>_<skillId>_subskill_<positionIndex>.png
function subskillSpriteKey(skill: Skill, positionIndex: number): string {
  return `${skill.classId}_${skill.id}_subskill_${positionIndex}`
}

export function resolveSubskillIconUrl(
  skill: Skill,
  sub: SubskillNode,
): string | undefined {
  if (sub.icon && /^https?:\/\//i.test(sub.icon)) return sub.icon
  return SUBSKILL_SPRITE_BY_KEY[subskillSpriteKey(skill, sub.positionIndex)]
}

