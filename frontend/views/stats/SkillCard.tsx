import { type CSSProperties, useMemo } from 'react'
import { useAttackSkillDamage, useSkillDamage } from '../../hooks/useSkillDamage'
import { DAMAGE_COLORS } from '../../utils/damageColors'
import { normalizeSkillName, rangedMax, rangedMin } from '../../utils/item/stats'
import { effectiveSkillCost } from './effectiveSkillCost'
import {
  effectiveSkillTags,
  entityTagOf,
  visibleEffectiveSkillTags,
} from '../../utils/skills/skillTags'
import {
  entityAttackRate,
  entityAttackRateFixedKey,
  entityAttackSpeedKey,
  entityKindOfTag,
} from '../../utils/build/entityRates'
import { useBuild } from '../../store/build'
import type { AttributeKey, RangedStatMap, RangedValue, Skill } from '../../types'
import type {
  NativeAttackSkillDamageInput,
  NativeSkillDamageInput,
  NativeWeaponRef,
} from '../../utils/nativeDamage'
import { formatDecimal, formatRange, useFormatRangeInt } from './statFormat'
import { DamageBreakdown } from './DamageBreakdown'
import { AttackDamageBreakdown } from './AttackDamageBreakdown'

export function SkillCard({
  skill,
  mcrRange,
  attributes,
  stats,
  skillRanksByName,
  skillsByNormalizedName,
  itemSkillBonuses,
  rankBonuses,
  currentRank,
  enemyConditions,
  enemyResistances,
  skillProjectiles,
  subtreeScoped,
  isMain,
  weapon,
}: {
  skill: Skill
  mcrRange: RangedValue
  attributes: Record<AttributeKey, RangedValue>
  stats: RangedStatMap
  skillRanksByName: Record<string, number>
  skillsByNormalizedName: Record<string, Skill>
  itemSkillBonuses: Record<string, [number, number]>
  rankBonuses: Record<string, [number, number]>
  currentRank: number
  enemyConditions: Record<string, boolean>
  enemyResistances: Record<string, number>
  skillProjectiles: Record<string, number>
  subtreeScoped: Record<string, RangedValue>
  isMain: boolean
  weapon?: NativeWeaponRef
}) {
  const formatRangeInt = useFormatRangeInt()
  const subskillRanks = useBuild((s) => s.subskillRanks)
  const entityRates = useBuild((s) => s.entityRates)
  const tagView = visibleEffectiveSkillTags(skill, subskillRanks)
  const entityTag = entityTagOf(effectiveSkillTags(skill, subskillRanks))
  const entityKind = entityTag ? entityKindOfTag(entityTag) : undefined
  // Entities swing on their own cadence; the cast rate only spawns them.
  const entitySwing = entityKind
    ? entityAttackRate(
        entityKind,
        entityRates,
        [
          rangedMin(stats[entityAttackSpeedKey(entityKind)] ?? 0),
          rangedMax(stats[entityAttackSpeedKey(entityKind)] ?? 0),
        ],
        rangedMax(stats[entityAttackRateFixedKey(entityKind)] ?? 0),
      )
    : undefined
  // Item "+X to <tag> Skills" ranks count toward the rank the game reports,
  // so mana cost and the header rank both use the effective total.
  const [rankBonusMin, rankBonusMax] =
    rankBonuses[normalizeSkillName(skill.name)] ?? [0, 0]
  const effRankMin = currentRank > 0 ? currentRank + rankBonusMin : 0
  const effRankMax = currentRank > 0 ? currentRank + rankBonusMax : 0
  const {
    baseManaMin,
    baseManaMax,
    mcrMax,
    fcrMax,
    effectiveManaMin,
    effectiveManaMax,
    lifeCostMin,
    lifeCostMax,
    effectiveCastRateMin,
    effectiveCastRateMax,
  } = effectiveSkillCost(
    skill,
    mcrRange,
    stats.faster_cast_rate ?? 0,
    stats.mana_cost_paid_in_life ?? 0,
    Math.max(1, effRankMin),
    Math.max(1, effRankMax),
  )

  const isAttack = skill.attackKind === 'attack'
  const hasSpellDamage =
    !!skill.damageFormula ||
    (!!skill.damagePerRank && skill.damagePerRank.length > 0)
  const hasDamage = hasSpellDamage || isAttack
  const skillInput = useMemo<NativeSkillDamageInput | null>(() => {
    if (currentRank <= 0 || !hasSpellDamage || isAttack) return null
    return {
      skill,
      allocatedRank: currentRank,
      attributes,
      stats,
      skillRanksByName,
      itemSkillBonuses,
      enemyConditions,
      enemyResistances,
      skillsByName: skillsByNormalizedName,
      projectileCount:
        (skillProjectiles[skill.id] ?? 1) +
        rangedMax(subtreeScoped.projectile_count ?? 0),
      ofTotalDamage: rangedMax(subtreeScoped.of_total_damage ?? 0),
    }
  }, [
    currentRank,
    hasSpellDamage,
    isAttack,
    skill,
    attributes,
    stats,
    skillRanksByName,
    itemSkillBonuses,
    enemyConditions,
    enemyResistances,
    skillsByNormalizedName,
    skillProjectiles,
    subtreeScoped,
  ])
  const damageBreakdown = useSkillDamage(skillInput)
  const attackInput = useMemo<NativeAttackSkillDamageInput | null>(() => {
    if (currentRank <= 0 || !isAttack) return null
    return {
      skill,
      allocatedRank: currentRank,
      attributes,
      stats,
      skillRanksByName,
      itemSkillBonuses,
      enemyConditions,
      enemyResistances,
      skillsByName: skillsByNormalizedName,
      projectileCount:
        (skillProjectiles[skill.id] ?? 1) +
        rangedMax(subtreeScoped.projectile_count ?? 0),
      ofTotalDamage: rangedMax(subtreeScoped.of_total_damage ?? 0),
      weapon,
    }
  }, [
    currentRank,
    isAttack,
    skill,
    attributes,
    stats,
    skillRanksByName,
    itemSkillBonuses,
    enemyConditions,
    enemyResistances,
    skillsByNormalizedName,
    skillProjectiles,
    subtreeScoped,
    weapon,
  ])
  const attackBreakdown = useAttackSkillDamage(attackInput)
  const typeLabel = skill.damageType
    ? skill.damageType.charAt(0).toUpperCase() + skill.damageType.slice(1)
    : ''
  const dmgAccent = skill.damageType
    ? DAMAGE_COLORS[skill.damageType].text
    : 'text-text'
  const learned = currentRank > 0
  const containerStyle: CSSProperties = isMain
    ? {
        background:
          'linear-gradient(135deg, rgba(224,184,100,0.06), transparent 60%), linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
        boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.4)',
      }
    : {
        background:
          'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
        boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.4)',
      }

  return (
    <li
      className={`relative space-y-1.5 rounded-[3px] border px-3.5 py-2.5 ${
        isMain ? 'border-accent-deep' : 'border-border'
      } ${!learned && !hasDamage ? '' : !learned ? 'opacity-55' : ''}`}
      style={containerStyle}
    >
      {isMain && (
        <span
          aria-hidden
          className="pointer-events-none absolute bottom-2 left-0 top-2 w-0.5 bg-accent-hot"
          style={{ boxShadow: '0 0 10px rgba(224,184,100,0.5)' }}
        />
      )}
      <div className="flex items-baseline justify-between gap-2">
        <div className="font-medium text-text">
          {skill.name}{' '}
          <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
            rank{' '}
            <span className={learned ? 'text-accent-hot' : 'text-faint'}>
              {effRankMin === effRankMax
                ? effRankMin
                : `${effRankMin}-${effRankMax}`}
            </span>
            /{skill.maxRank}
            {learned && (rankBonusMin !== 0 || rankBonusMax !== 0) && (
              <span className="text-faint">
                {' '}
                ({currentRank}
                {rankBonusMin === rankBonusMax
                  ? `+${rankBonusMax}`
                  : ` +${rankBonusMin}-${rankBonusMax}`}
                )
              </span>
            )}
          </span>
        </div>
        {isAttack && attackBreakdown ? (
          <div
            className={`font-mono text-[13px] font-semibold tabular-nums ${dmgAccent}`}
          >
            {formatRangeInt(
              attackBreakdown.combinedHitMin,
              attackBreakdown.combinedHitMax,
            )}{' '}
            <span className="ml-0.5 font-mono text-[9px] font-normal uppercase tracking-[0.14em] text-faint">
              {typeLabel} damage
            </span>
            <span className="ml-2 font-mono text-[9px] font-normal uppercase tracking-[0.14em] text-faint">
              DPS{' '}
              <span className="text-accent-hot">
                {formatRangeInt(
                  Math.round(attackBreakdown.dpsMin),
                  Math.round(attackBreakdown.dpsMax),
                )}
              </span>
            </span>
          </div>
        ) : hasDamage && damageBreakdown ? (
          <div
            className={`font-mono text-[13px] font-semibold tabular-nums ${dmgAccent}`}
          >
            {formatRangeInt(damageBreakdown.finalMin, damageBreakdown.finalMax)}{' '}
            <span className="ml-0.5 font-mono text-[9px] font-normal uppercase tracking-[0.14em] text-faint">
              {typeLabel} damage
            </span>
          </div>
        ) : hasDamage ? (
          <div className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint italic">
            Not learned
          </div>
        ) : null}
      </div>
      <div className="flex flex-wrap items-center gap-1.5 font-mono text-[9px] uppercase tracking-[0.14em]">
        {skill.damageType && (
          <span
            className={`rounded-xs border px-1.5 py-0.5 font-semibold ${DAMAGE_COLORS[skill.damageType].pill}`}
          >
            {skill.damageType}
          </span>
        )}
        {tagView.tags.map((tag) => (
          <span
            key={tag}
            title={tagView.added.has(tag) ? 'Added by subskill' : undefined}
            className={`rounded-xs border px-1.5 py-0.5 font-semibold text-accent-hot ${
              tagView.added.has(tag)
                ? 'border-accent-hot/80'
                : 'border-accent-deep/50'
            }`}
            style={{
              background:
                'linear-gradient(180deg, rgba(58,46,24,0.4), rgba(42,36,24,0.2))',
            }}
          >
            {tag}
          </span>
        ))}
        {tagView.removed.map((tag) => (
          <span
            key={tag}
            title="Removed by subskill"
            className="rounded-xs border border-border px-1.5 py-0.5 font-semibold text-faint line-through opacity-60"
          >
            {tag}
          </span>
        ))}
      </div>
      {(effectiveManaMin !== undefined ||
        effectiveCastRateMin !== undefined ||
        skill.movementDuringUse !== undefined) && (
        <div className="flex flex-wrap gap-x-3.5 gap-y-0.5 text-[11px] text-muted">
          {effectiveManaMin !== undefined && effectiveManaMax !== undefined && (
            <span>
              <span className="font-mono font-medium text-text">
                {formatRange(effectiveManaMin, effectiveManaMax)}
              </span>{' '}
              mana
              {baseManaMin !== undefined &&
                baseManaMax !== undefined &&
                mcrMax > 0 && (
                  <span className="text-faint">
                    {' '}
                    (base {formatRange(baseManaMin, baseManaMax)})
                  </span>
                )}
            </span>
          )}
          {lifeCostMax !== undefined && lifeCostMax > 0 && (
            <span>
              <span className="font-mono font-medium text-stat-red">
                {formatRange(lifeCostMin ?? 0, lifeCostMax)}
              </span>{' '}
              life
            </span>
          )}
          {isAttack && attackBreakdown && (
            <span>
              <span className="font-mono font-medium text-text">
                {formatRange(
                  attackBreakdown.attacksPerSecondMin,
                  attackBreakdown.attacksPerSecondMax,
                )}
              </span>{' '}
              attacks/s
            </span>
          )}
          {entitySwing && (
            <span>
              <span className="font-mono font-medium text-text">
                {formatRange(entitySwing.min, entitySwing.max)}
              </span>{' '}
              attacks/s
              <span className="text-faint">
                {' '}
                (base {formatDecimal(entitySwing.base)})
              </span>
            </span>
          )}
          {effectiveCastRateMin !== undefined &&
            effectiveCastRateMax !== undefined && (
              <span>
                <span className="font-mono font-medium text-text">
                  {formatRange(effectiveCastRateMin, effectiveCastRateMax)}
                </span>{' '}
                {entityKind
                  ? 'spawns/s'
                  : skill.usesAttackSpeed
                    ? 'throws/s'
                    : 'casts/s'}
                {fcrMax > 0 && skill.baseCastRate !== undefined && (
                  <span className="text-faint">
                    {' '}
                    (base {formatDecimal(skill.baseCastRate)})
                  </span>
                )}
              </span>
            )}
          {skill.movementDuringUse !== undefined && (
            <span>
              Move{' '}
              <span className="font-mono font-medium text-text">
                {skill.movementDuringUse}%
              </span>
            </span>
          )}
          <span>
            max rank{' '}
            <span className="font-mono font-medium text-text">
              {skill.maxRank}
            </span>
          </span>
        </div>
      )}
      {hasDamage && damageBreakdown && (
        <DamageBreakdown
          skill={skill}
          breakdown={damageBreakdown}
          currentRank={currentRank}
          attributes={attributes}
          skillRanksByName={skillRanksByName}
          skillsByNormalizedName={skillsByNormalizedName}
          rankBonuses={rankBonuses}
        />
      )}
      {isAttack && attackBreakdown && (
        <AttackDamageBreakdown
          skill={skill}
          breakdown={attackBreakdown}
          currentRank={currentRank}
          attributes={attributes}
          skillRanksByName={skillRanksByName}
          skillsByNormalizedName={skillsByNormalizedName}
          rankBonuses={rankBonuses}
        />
      )}
    </li>
  )
}
