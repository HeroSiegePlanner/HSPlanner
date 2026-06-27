import {
  normalizeSkillName,
  rangedMax,
  rangedMin,
} from '../../utils/item/stats'
import type { SkillDamageBreakdown } from '../../utils/item/stats'
import type { AttributeKey, RangedValue, Skill } from '../../types'
import { formatDecimal, formatRange, formatRangeInt } from './format'
import { BDLine, BDSection } from './primitives'

export function DamageBreakdown({
  skill,
  breakdown,
  currentRank,
  attributes,
  skillRanksByName,
  skillsByNormalizedName,
  rankBonuses,
}: {
  skill: Skill
  breakdown: SkillDamageBreakdown
  currentRank: number
  attributes: Record<AttributeKey, RangedValue>
  skillRanksByName: Record<string, number>
  skillsByNormalizedName: Record<string, Skill>
  rankBonuses: Record<string, [number, number]>
}) {
  const synergyLines: Array<{ label: string; pctMin: number; pctMax: number }> =
    []
  for (const b of skill.bonusSources ?? []) {
    if (b.per === 'skill_level') {
      const sourceKey = normalizeSkillName(b.source)
      const baseRank = skillRanksByName[sourceKey] ?? 0
      if (baseRank === 0) continue
      const srcSkill = skillsByNormalizedName[sourceKey]
      const [bonusMin, bonusMax] = srcSkill
        ? (rankBonuses[sourceKey] ?? [0, 0])
        : [0, 0]
      const [effMin, effMax] = [baseRank + bonusMin, baseRank + bonusMax]
      const rankLabel =
        effMin === effMax ? `rank ${effMin}` : `rank ${effMin}-${effMax}`
      synergyLines.push({
        label: `${b.source} (${rankLabel})`,
        pctMin: effMin * b.value,
        pctMax: effMax * b.value,
      })
    } else if (b.per === 'attribute_point') {
      const attrKey = Object.keys(attributes).find(
        (k) => k.toLowerCase() === b.source.trim().toLowerCase(),
      )
      const attr = attrKey ? (attributes[attrKey] ?? 0) : 0
      const amin = rangedMin(attr)
      const amax = rangedMax(attr)
      if (amin === 0 && amax === 0) continue
      synergyLines.push({
        label: `${b.source} (${amin === amax ? amin : `${amin}-${amax}`})`,
        pctMin: amin * b.value,
        pctMax: amax * b.value,
      })
    }
  }

  const effRankLabel =
    breakdown.effectiveRankMin === breakdown.effectiveRankMax
      ? String(breakdown.effectiveRankMin)
      : `${breakdown.effectiveRankMin}-${breakdown.effectiveRankMax}`
  const rankBonusMin = breakdown.effectiveRankMin - currentRank
  const rankBonusMax = breakdown.effectiveRankMax - currentRank
  const hasRankBonus = rankBonusMin !== 0 || rankBonusMax !== 0

  return (
    <div className="mt-2 border-t border-dashed border-border pt-2 text-[12px]">
      <BDSection title="Base">
        <BDLine
          label="Effective rank"
          value={
            <>
              <span className="text-text">{effRankLabel}</span>
              {hasRankBonus && (
                <span className="text-faint font-normal">
                  {' '}
                  ({currentRank}
                  {rankBonusMin === rankBonusMax
                    ? rankBonusMin >= 0
                      ? `+${rankBonusMin}`
                      : rankBonusMin
                    : ` +${rankBonusMin}-${rankBonusMax}`}
                  )
                </span>
              )}
            </>
          }
        />
        <BDLine
          label="Base damage"
          value={
            <span className="text-text">
              {formatRange(breakdown.baseMin, breakdown.baseMax)}
            </span>
          }
        />
        {(breakdown.flatMin > 0 || breakdown.flatMax > 0) && (
          <BDLine
            label="Flat added"
            value={
              <span className="text-text">
                {formatRange(breakdown.flatMin, breakdown.flatMax)}
              </span>
            }
          />
        )}
      </BDSection>
      {synergyLines.length > 0 && (
        <BDSection title="Synergies">
          {synergyLines.map((line, i) => (
            <BDLine
              key={i}
              label={line.label}
              indent
              value={
                <span className="text-yellow-300">
                  +
                  {formatRange(line.pctMin, line.pctMax)}
                  %
                </span>
              }
            />
          ))}
          <BDLine
            label="Total synergy"
            value={
              <span className="text-yellow-300">
                +
                {formatRange(breakdown.synergyMinPct, breakdown.synergyMaxPct)}
                %
              </span>
            }
          />
        </BDSection>
      )}
      {((breakdown.skillDamageMinPct > 0 || breakdown.skillDamageMaxPct > 0) ||
        breakdown.enemyResistancePct !== 0 ||
        breakdown.resistanceIgnoredPct !== 0 ||
        breakdown.multicastChancePct > 0 ||
        breakdown.elementalBreakPct > 0 ||
        breakdown.projectileCount > 1) && (
        <BDSection title="Multipliers">
          {(breakdown.skillDamageMinPct > 0 ||
            breakdown.skillDamageMaxPct > 0) && (
            <BDLine
              label="Skill damage %"
              value={
                <span className="text-accent-hot">
                  +
                  {formatRange(breakdown.skillDamageMinPct, breakdown.skillDamageMaxPct)}
                  %
                </span>
              }
            />
          )}
          {breakdown.multicastChancePct > 0 && (
            <BDLine
              label={`Multicast (${formatDecimal(breakdown.multicastChancePct)}%)`}
              value={
                <span className="text-accent-hot">
                  ×{breakdown.multicastMultiplier.toFixed(2)}
                </span>
              }
            />
          )}
          {breakdown.elementalBreakPct > 0 && (
            <BDLine
              label={`Elemental Break (${formatDecimal(breakdown.elementalBreakPct)}%)`}
              value={
                <span className="text-accent-hot">
                  ×{breakdown.elementalBreakMultiplier.toFixed(2)}
                </span>
              }
            />
          )}
          {breakdown.projectileCount > 1 && (
            <BDLine
              label={`Projectiles (${breakdown.projectileCount}×)`}
              value={
                <span className="text-accent-hot">
                  ×{breakdown.projectileCount.toFixed(2)}
                </span>
              }
            />
          )}
          {(breakdown.enemyResistancePct !== 0 ||
            breakdown.resistanceIgnoredPct !== 0) && (
            <BDLine
              label="Enemy resistance"
              value={
                <span
                  className={
                    breakdown.effectiveResistancePct > 0
                      ? 'text-stat-red'
                      : 'text-stat-green'
                  }
                >
                  ×{breakdown.resistanceMultiplier.toFixed(2)}
                  <span className="ml-1 text-faint font-normal">
                    ({formatDecimal(breakdown.effectiveResistancePct)}%)
                  </span>
                </span>
              }
            />
          )}
        </BDSection>
      )}
      {breakdown.extraDamageSources.length > 0 && (
        <BDSection title="Extra damage">
          {breakdown.extraDamageSources.map((s, i) => (
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
      <div className="mt-1.5 flex items-baseline justify-between gap-3 border-t border-border pt-1.5">
        <span className="font-mono text-[10px] font-medium uppercase tracking-[0.14em] text-muted">
          {breakdown.multicastChancePct > 0 || breakdown.projectileCount > 1
            ? 'Average per cast'
            : breakdown.critChance > 0
              ? 'Average hit'
              : 'Hit damage'}
        </span>
        <span className="font-mono text-[14px] font-semibold tabular-nums text-accent-hot">
          {breakdown.critChance > 0 ||
          breakdown.multicastChancePct > 0 ||
          breakdown.projectileCount > 1
            ? formatRangeInt(breakdown.avgMin, breakdown.avgMax)
            : formatRangeInt(breakdown.hitMin, breakdown.hitMax)}
        </span>
      </div>
    </div>
  )
}
