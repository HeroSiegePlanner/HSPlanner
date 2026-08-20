import { affixTags, subskillTags } from '@data'
import type { RangedValue, Skill } from '../../types'
import { rangedMax, rangedMin } from '../item/stats'

export function visibleSkillTags(
  skill: Pick<Skill, 'tags' | 'damageType'>,
): string[] {
  const tags = skill.tags ?? []
  if (!skill.damageType) return tags
  return tags.filter((tag) => tag.toLowerCase() !== skill.damageType)
}

// Mirror of the tag rewrite in engine calc/build.rs: subskill transmutations
// add/remove tags on the skill; removes win over adds.
export function effectiveSkillTags(
  skill: Pick<Skill, 'id' | 'tags'>,
  subskillRanks: Record<string, number>,
): string[] {
  const base = skill.tags ?? []
  const changes = subskillTags[skill.id]
  if (!changes) return base
  const adds: string[] = []
  const removes: string[] = []
  for (const [subId, ch] of Object.entries(changes)) {
    if ((subskillRanks[`${skill.id}:${subId}`] ?? 0) === 0) continue
    adds.push(...(ch.add ?? []))
    removes.push(...(ch.remove ?? []))
  }
  if (adds.length === 0 && removes.length === 0) return base
  const merged = [...base]
  for (const t of adds) {
    if (!merged.includes(t)) merged.push(t)
  }
  return merged.filter((t) => !removes.includes(t))
}

export interface EffectiveTagsView {
  tags: string[]
  added: Set<string>
  removed: string[]
}

export function visibleEffectiveSkillTags(
  skill: Pick<Skill, 'id' | 'tags' | 'damageType'>,
  subskillRanks: Record<string, number>,
): EffectiveTagsView {
  const base = visibleSkillTags(skill)
  const effective = visibleSkillTags({
    tags: effectiveSkillTags(skill, subskillRanks),
    damageType: skill.damageType,
  })
  return {
    tags: effective,
    added: new Set(effective.filter((t) => !base.includes(t))),
    removed: base.filter((t) => !effective.includes(t)),
  }
}

export interface TagSkillBonus {
  key: string
  label: string
  value: [number, number]
}

// "+X to <Tag> Skills" rows that reach this skill, per data/affix-tags.json.
export function tagSkillBonuses(
  tags: string[],
  stats: Record<string, RangedValue>,
): TagSkillBonus[] {
  return Object.entries(affixTags)
    .filter(
      ([, def]) =>
        def.effect === 'rank' && def.tags.every((t) => tags.includes(t)),
    )
    .map(([key, def]) => ({
      key,
      label: def.tags.join(' + '),
      value: [rangedMin(stats[key] ?? 0), rangedMax(stats[key] ?? 0)] as [
        number,
        number,
      ],
    }))
    .filter(({ value }) => value[0] !== 0 || value[1] !== 0)
}

// Sentry / Summon / Guardian — the tags whose affixes drive an entity's own
// attack rate, so the skill's DPS depends on the minion rate knob.
const ENTITY_TAGS = new Set(
  Object.values(affixTags)
    .filter((def) => def.effect === 'attack_speed')
    .flatMap((def) => def.tags),
)

export function entityTagOf(tags: string[]): string | undefined {
  return tags.find((t) => ENTITY_TAGS.has(t))
}
