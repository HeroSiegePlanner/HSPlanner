import { type CSSProperties, useMemo } from 'react'
import { useSkillDamage } from '../../hooks/useSkillDamage'
import { DAMAGE_COLORS } from '../../utils/damageColors'
import { rangedMax, rangedMin } from '../../utils/item/stats'
import type { AttributeKey, RangedStatMap, RangedValue, Skill } from '../../types'
import type { NativeSkillDamageInput } from '../../utils/nativeDamage'
import { formatDecimal, formatRange, formatRangeInt } from './format'
import { DamageBreakdown } from './DamageBreakdown'

export function SkillCard({
  skill,
  fcrRange,
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
  isMain,
}: {
  skill: Skill
  fcrRange: RangedValue
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
  isMain: boolean
}) {
  const rank1 = skill.ranks[0]
  const baseMana = rank1?.manaCost
  const mcrMin = rangedMin(mcrRange)
  const mcrMax = rangedMax(mcrRange)
  const effectiveManaMin =
    baseMana !== undefined
      ? Math.max(0, baseMana * (1 - mcrMax / 100))
      : undefined
  const effectiveManaMax =
    baseMana !== undefined
      ? Math.max(0, baseMana * (1 - mcrMin / 100))
      : undefined
  const fcrMin = rangedMin(fcrRange)
  const fcrMax = rangedMax(fcrRange)
  const effectiveCastRateMin =
    skill.baseCastRate !== undefined
      ? skill.baseCastRate * (1 + fcrMin / 100)
      : undefined
  const effectiveCastRateMax =
    skill.baseCastRate !== undefined
      ? skill.baseCastRate * (1 + fcrMax / 100)
      : undefined

  const hasDamage =
    !!skill.damageFormula ||
    (!!skill.damagePerRank && skill.damagePerRank.length > 0)
  const skillInput = useMemo<NativeSkillDamageInput | null>(() => {
    if (currentRank <= 0 || !hasDamage) return null
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
      projectileCount: skillProjectiles[skill.id],
    }
  }, [
    currentRank,
    hasDamage,
    skill,
    attributes,
    stats,
    skillRanksByName,
    itemSkillBonuses,
    enemyConditions,
    enemyResistances,
    skillsByNormalizedName,
    skillProjectiles,
  ])
  const damageBreakdown = useSkillDamage(skillInput)
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
              {currentRank}
            </span>
            /{skill.maxRank}
          </span>
        </div>
        {hasDamage && damageBreakdown ? (
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
        {skill.tags?.map((tag) => (
          <span
            key={tag}
            className="rounded-xs border border-accent-deep/50 px-1.5 py-0.5 font-semibold text-accent-hot"
            style={{
              background:
                'linear-gradient(180deg, rgba(58,46,24,0.4), rgba(42,36,24,0.2))',
            }}
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
              {baseMana !== undefined && mcrMax > 0 && (
                <span className="text-faint">
                  {' '}
                  (base {baseMana})
                </span>
              )}
            </span>
          )}
          {effectiveCastRateMin !== undefined &&
            effectiveCastRateMax !== undefined && (
              <span>
                <span className="font-mono font-medium text-text">
                  {formatRange(effectiveCastRateMin, effectiveCastRateMax)}
                </span>{' '}
                casts/s
                {fcrMax > 0 && (
                  <span className="text-faint">
                    {' '}
                    (base {formatDecimal(skill.baseCastRate!)})
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
    </li>
  )
}
