import { useMemo } from 'react'
import { useSkillDamage } from '../../hooks/useSkillDamage'
import { DAMAGE_COLORS, skillHeroBg } from '../../utils/damageColors'
import { rangedMax, rangedMin } from '../../utils/item/stats'
import type {
  SkillDamageBreakdown,
  WeaponDamageBreakdown,
} from '../../utils/item/stats'
import type { NativeSkillDamageInput } from '../../utils/nativeDamage'
import type {
  AttributeKey,
  RangedStatMap,
  RangedValue,
  Skill,
} from '../../types'
import { formatDecimal, formatRange, useFormatRangeInt } from './format'
import { BDLine, BDSection, HeroStat, Panel } from './primitives'
import { DamageBreakdown } from './DamageBreakdown'

export function MainSkillSection({
  mainSkill,
  mainSkillRank,
  attributes,
  stats,
  skillRanksByName,
  skillsByNormalizedName,
  itemSkillBonuses,
  rankBonuses,
  enemyConditions,
  enemyResistances,
  skillProjectiles,
  fcrRange,
  mcrRange,
  weaponDamage,
}: {
  mainSkill: Skill | null
  mainSkillRank: number
  attributes: Record<AttributeKey, RangedValue>
  stats: RangedStatMap
  skillRanksByName: Record<string, number>
  skillsByNormalizedName: Record<string, Skill>
  itemSkillBonuses: Record<string, [number, number]>
  rankBonuses: Record<string, [number, number]>
  enemyConditions: Record<string, boolean>
  enemyResistances: Record<string, number>
  skillProjectiles: Record<string, number>
  fcrRange: RangedValue
  mcrRange: RangedValue
  weaponDamage: WeaponDamageBreakdown | null
}) {
  const hasSkillDamage =
    !!mainSkill &&
    mainSkillRank > 0 &&
    (!!mainSkill.damageFormula ||
      (!!mainSkill.damagePerRank && mainSkill.damagePerRank.length > 0))
  const skillInput = useMemo<NativeSkillDamageInput | null>(() => {
    if (!hasSkillDamage || !mainSkill) return null
    return {
      skill: mainSkill,
      allocatedRank: mainSkillRank,
      attributes,
      stats,
      skillRanksByName,
      itemSkillBonuses,
      enemyConditions,
      enemyResistances,
      skillsByName: skillsByNormalizedName,
      projectileCount: skillProjectiles[mainSkill.id],
    }
  }, [
    hasSkillDamage,
    mainSkill,
    mainSkillRank,
    attributes,
    stats,
    skillRanksByName,
    itemSkillBonuses,
    enemyConditions,
    enemyResistances,
    skillsByNormalizedName,
    skillProjectiles,
  ])
  const skillBreakdown = useSkillDamage(skillInput)

  if (mainSkill && skillBreakdown) {
    return (
      <Panel
        title="Main Skill"
        meta={`${mainSkill.name}`}
      >
        <SkillDamageHero
          skill={mainSkill}
          breakdown={skillBreakdown}
          fcrRange={fcrRange}
          mcrRange={mcrRange}
        />
        <DamageBreakdown
          skill={mainSkill}
          breakdown={skillBreakdown}
          currentRank={mainSkillRank}
          attributes={attributes}
          skillRanksByName={skillRanksByName}
          skillsByNormalizedName={skillsByNormalizedName}
          rankBonuses={rankBonuses}
        />
      </Panel>
    )
  }

  if (weaponDamage && weaponDamage.hasWeapon) {
    return (
      <Panel
        title="Main Skill"
        meta={
          mainSkill
            ? `${mainSkill.name} · no damage`
            : 'No main skill selected · weapon damage shown'
        }
      >
        <DamageHero breakdown={weaponDamage} />
        <DamageBuildup breakdown={weaponDamage} />
      </Panel>
    )
  }
  return (
    <Panel title="Main Skill" meta="No main skill or weapon">
      <div className="py-6 text-center text-xs text-muted italic">
        Pick a main skill in the Skills tab, or equip a weapon, to see your
        headline damage breakdown here.
      </div>
    </Panel>
  )
}

function SkillDamageHero({
  skill,
  breakdown,
  fcrRange,
  mcrRange,
}: {
  skill: Skill
  breakdown: SkillDamageBreakdown
  fcrRange: RangedValue
  mcrRange: RangedValue
}) {
  const formatRangeInt = useFormatRangeInt()
  const hasCrit = breakdown.critChance > 0
  const baseMana = skill.ranks[0]?.manaCost
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
  const accentBg = skill.damageType
    ? skillHeroBg(skill.damageType)
    : 'linear-gradient(135deg, rgba(224,184,100,0.06), transparent 60%)'
  return (
    <div
      className="mb-3 grid grid-cols-1 overflow-hidden rounded-[3px] border border-border md:grid-cols-[1.2fr_1fr]"
      style={{ background: accentBg }}
    >
      <div className="border-b border-border px-4 py-3.5 md:border-b-0 md:border-r">
        <div className="mb-1.5 flex items-center gap-1.5 font-mono text-[9px] uppercase tracking-[0.18em] text-faint">
          <span>{hasCrit ? 'Average Hit' : 'Hit Damage'}</span>
          {skill.damageType && (
            <span
              className={`rounded-xs border px-1.5 py-px text-[9px] font-semibold ${DAMAGE_COLORS[skill.damageType].pill}`}
            >
              {skill.damageType}
            </span>
          )}
        </div>
        <div
          className="font-mono text-[28px] font-semibold leading-none tabular-nums tracking-[0.01em] text-accent-hot"
          style={{ textShadow: '0 0 16px rgba(224,184,100,0.18)' }}
        >
          {hasCrit
            ? formatRangeInt(breakdown.avgMin, breakdown.avgMax)
            : formatRangeInt(breakdown.hitMin, breakdown.hitMax)}
        </div>
        <div className="mt-2 flex flex-wrap items-baseline gap-x-4 gap-y-1 font-mono text-[10px] tracking-widest text-muted">
          {effectiveManaMin !== undefined &&
            effectiveManaMax !== undefined && (
              <span>
                <span className="text-text/90">
                  {formatRange(effectiveManaMin, effectiveManaMax)}
                </span>{' '}
                mana
              </span>
            )}
          {effectiveCastRateMin !== undefined &&
            effectiveCastRateMax !== undefined && (
              <span>
                <span className="text-text/90">
                  {formatRange(effectiveCastRateMin, effectiveCastRateMax)}
                </span>{' '}
                casts/s
              </span>
            )}
          {skill.tags && skill.tags.length > 0 && (
            <div className="flex flex-wrap gap-1">
              {skill.tags.map((tag) => (
                <span
                  key={tag}
                  className="rounded-xs border border-accent-deep/50 px-1.5 py-px text-[9px] font-semibold uppercase tracking-[0.14em] text-accent-hot"
                  style={{
                    background:
                      'linear-gradient(180deg, rgba(58,46,24,0.4), rgba(42,36,24,0.2))',
                  }}
                >
                  {tag}
                </span>
              ))}
            </div>
          )}
        </div>
      </div>
      <div className="grid grid-cols-2 gap-2.5 px-4 py-3.5">
        <HeroStat
          k="Hit damage"
          v={formatRangeInt(breakdown.hitMin, breakdown.hitMax)}
        />
        <HeroStat
          k="Crit damage"
          v={
            hasCrit ? formatRangeInt(breakdown.critMin, breakdown.critMax) : '—'
          }
        />
        <HeroStat
          k="Crit chance"
          v={hasCrit ? `${formatDecimal(breakdown.critChance)}%` : '—'}
        />
        <HeroStat
          k="Crit multi"
          v={
            hasCrit
              ? `+${formatDecimal(breakdown.critDamagePct)}%`
              : '—'
          }
        />
      </div>
    </div>
  )
}

function DamageHero({ breakdown }: { breakdown: WeaponDamageBreakdown }) {
  const formatRangeInt = useFormatRangeInt()
  const b = breakdown
  return (
    <div
      className="mb-3 grid grid-cols-1 overflow-hidden rounded-[3px] border border-border md:grid-cols-[1.2fr_1fr]"
      style={{
        background:
          'linear-gradient(135deg, rgba(217,107,90,0.05), transparent 60%)',
      }}
    >
      <div className="border-b border-border px-4 py-3.5 md:border-b-0 md:border-r">
        <div className="mb-1.5 font-mono text-[9px] uppercase tracking-[0.18em] text-faint">
          Average Hit
        </div>
        <div
          className="font-mono text-[26px] font-semibold leading-none tabular-nums tracking-[0.01em] text-accent-hot"
          style={{ textShadow: '0 0 16px rgba(224,184,100,0.18)' }}
        >
          {formatRangeInt(Math.round(b.avgMin), Math.round(b.avgMax))}
        </div>
        <div className="mt-1.5 font-mono text-[10px] tracking-widest text-muted">
          DPS{' '}
          <span className="text-accent-hot">
            {formatRangeInt(Math.round(b.dpsMin), Math.round(b.dpsMax))}
          </span>{' '}
          ·{' '}
          <span className="text-text/90">
            {formatRange(b.attacksPerSecondMin, b.attacksPerSecondMax)}
          </span>{' '}
          attacks/sec
        </div>
      </div>
      <div className="grid grid-cols-2 gap-2.5 px-4 py-3.5">
        <HeroStat
          k="Hit damage"
          v={formatRangeInt(b.hitMin, b.hitMax)}
        />
        <HeroStat
          k="Crit damage"
          v={
            b.critChance > 0
              ? formatRangeInt(b.critMin, b.critMax)
              : '—'
          }
        />
        <HeroStat
          k="Crit chance"
          v={`${formatDecimal(b.critChance)}%`}
        />
        <HeroStat
          k="Crit multi"
          v={`×${b.critMultiplierAvg.toFixed(2)}`}
        />
      </div>
    </div>
  )
}

function DamageBuildup({ breakdown }: { breakdown: WeaponDamageBreakdown }) {
  const formatRangeInt = useFormatRangeInt()
  const b = breakdown
  return (
    <div className="mt-2 border-t border-dashed border-border pt-2.5">
      <BDSection title="Build-up">
        <BDLine
          label="Weapon damage"
          value={
            <span className="text-text">
              {formatRangeInt(b.weaponDamageMin, b.weaponDamageMax)}
            </span>
          }
        />
        {b.enhancedDamageMaxPct > 0 && (
          <BDLine
            label="Enhanced damage"
            value={
              <span className="text-accent-hot">
                +{formatRange(b.enhancedDamageMinPct, b.enhancedDamageMaxPct)}%
              </span>
            }
          />
        )}
        {b.additivePhysicalMax > 0 && (
          <BDLine
            label="Additive physical"
            value={
              <span className="text-text">
                +{formatRangeInt(b.additivePhysicalMin, b.additivePhysicalMax)}
              </span>
            }
          />
        )}
        {b.attackDamageMaxPct > 0 && (
          <BDLine
            label="Attack damage %"
            value={
              <span className="text-accent-hot">
                +{formatRange(b.attackDamageMinPct, b.attackDamageMaxPct)}%
              </span>
            }
          />
        )}
      </BDSection>
      {b.additiveElementalBreakdown.length > 0 && (
        <BDSection title="Additive elemental">
          {b.additiveElementalBreakdown.map((s, i) => (
            <BDLine
              key={i}
              indent
              label={s.label}
              value={
                <span className="text-sky-300">+{formatDecimal(s.pct)}</span>
              }
            />
          ))}
        </BDSection>
      )}
      {b.extraDamageSources.length > 0 && (
        <BDSection title="Extra damage">
          {b.extraDamageSources.map((s, i) => (
            <BDLine
              key={i}
              indent
              label={s.label}
              value={
                <span className="text-orange-300">
                  +{formatDecimal(s.pct)}%
                </span>
              }
            />
          ))}
        </BDSection>
      )}
      {(b.crushingBlowModifier !== 1.5 ||
        b.armorBreakPct > 0 ||
        b.deadlyBlowChance > 0 ||
        b.hitChance !== 100 ||
        b.projectileCount > 1) && (
        <BDSection title="Combat modifiers">
          <BDLine
            label="Crushing blow"
            value={
              <span className="text-text">
                ×{b.crushingBlowModifier.toFixed(2)}
              </span>
            }
          />
          {b.armorBreakPct > 0 && (
            <BDLine
              label="Armor break"
              value={
                <span className="text-accent-hot">
                  +{formatDecimal(b.armorBreakPct)}%
                </span>
              }
            />
          )}
          {b.deadlyBlowChance > 0 && (
            <BDLine
              label="Deadly blow chance"
              value={
                <span className="text-accent-hot">
                  {formatDecimal(b.deadlyBlowChance)}%
                </span>
              }
            />
          )}
          {b.hitChance !== 100 && (
            <BDLine
              label="Hit chance"
              value={
                <span className="text-text">{formatDecimal(b.hitChance)}%</span>
              }
            />
          )}
          {b.projectileCount > 1 && (
            <BDLine
              label="Projectiles"
              value={<span className="text-text">×{b.projectileCount}</span>}
            />
          )}
        </BDSection>
      )}
      {b.openWoundsMax > 0 && (
        <BDSection title="Open wounds">
          <BDLine
            label="Damage per hit"
            value={
              <span className="text-red-400">
                {formatRangeInt(b.openWoundsMin, b.openWoundsMax)}
              </span>
            }
          />
        </BDSection>
      )}
      {(b.enemyPhysResPct > 0 || b.physResistanceIgnoredPct > 0) && (
        <BDSection title="Enemy resistance">
          <BDLine
            label="Physical resistance"
            value={
              <span className="text-text">
                {formatDecimal(b.enemyPhysResPct)}%
              </span>
            }
          />
          {b.physResistanceIgnoredPct > 0 && (
            <BDLine
              label="Ignored"
              value={
                <span className="text-accent-hot">
                  {formatDecimal(b.physResistanceIgnoredPct)}%
                </span>
              }
            />
          )}
        </BDSection>
      )}
      <div className="mt-1.5 flex items-baseline justify-between gap-3 border-t border-border pt-1.5">
        <span className="font-mono text-[10px] font-medium uppercase tracking-[0.14em] text-muted">
          Hit damage
        </span>
        <span className="font-mono text-[14px] font-semibold tabular-nums text-accent-hot">
          {formatRangeInt(b.hitMin, b.hitMax)}
        </span>
      </div>
    </div>
  )
}
