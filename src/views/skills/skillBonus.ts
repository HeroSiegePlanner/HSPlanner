import type { RangedStatMap, Skill } from '../../types'
import { normalizeSkillName, rangedMax, rangedMin } from '../../utils/item/stats'

// Effective skill-level bonus for one skill, summed as [min, max]:
// +all skills, +<element> skills, and +named-skill from items.
// Only allocated skills (rank > 0) gain the bonus — mirrors SkillDetailsPanel
// and the in-game rule that +skills never activates an unallocated skill.
export function skillLevelBonus(
  skill: Skill,
  rank: number,
  stats: RangedStatMap,
  itemSkillBonuses: Record<string, [number, number]>,
): [number, number] {
  if (rank <= 0) return [0, 0]
  const item = itemSkillBonuses[normalizeSkillName(skill.name)] ?? [0, 0]
  const min =
    rangedMin(stats.all_skills ?? 0) +
    (skill.damageType ? rangedMin(stats[`${skill.damageType}_skills`] ?? 0) : 0) +
    item[0]
  const max =
    rangedMax(stats.all_skills ?? 0) +
    (skill.damageType ? rangedMax(stats[`${skill.damageType}_skills`] ?? 0) : 0) +
    item[1]
  return [min, max]
}
