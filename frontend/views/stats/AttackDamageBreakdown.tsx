import {
  normalizeSkillName,
  rangedMax,
  rangedMin,
} from '../../utils/item/stats'
import type { AttackSkillDamageBreakdown } from '../../utils/item/stats'
import type { AttributeKey, RangedValue, Skill } from '../../types'
import { formatDecimal, formatRange, useFormatRangeInt } from './statFormat'
import { BDLine, BDSection } from './statPrimitives'

export function AttackDamageBreakdown({
  skill,
  breakdown,
  currentRank,
  attributes,
  skillRanksByName,
  skillsByNormalizedName,
  rankBonuses,
}: {
  skill: Skill
  breakdown: AttackSkillDamageBreakdown
  currentRank: number
  attributes: Record<AttributeKey, RangedValue>
  skillRanksByName: Record<string, number>
  skillsByNormalizedName: Record<string, Skill>
  rankBonuses: Record<string, [number, number]>
}) {
  const formatRangeInt = useFormatRangeInt()
  const b = breakdown
  const synergyLines: Array<{ label: string; pctMin: number; pctMax: number }> =
    []
  for (const bs of skill.bonusSources ?? []) {
    if (bs.stat !== 'attack_damage') continue
    if (bs.per === 'skill_level') {
      const sourceKey = normalizeSkillName(bs.source)
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
        label: `${bs.source} (${rankLabel})`,
        pctMin: effMin * bs.value,
        pctMax: effMax * bs.value,
      })
    } else if (bs.per === 'attribute_point') {
      const attrKey = Object.keys(attributes).find(
        (k) => k.toLowerCase() === bs.source.trim().toLowerCase(),
      )
      const attr = attrKey ? (attributes[attrKey] ?? 0) : 0
      const amin = rangedMin(attr)
      const amax = rangedMax(attr)
      if (amin === 0 && amax === 0) continue
      synergyLines.push({
        label: `${bs.source} (${amin === amax ? amin : `${amin}-${amax}`})`,
        pctMin: amin * bs.value,
        pctMax: amax * bs.value,
      })
    }
  }

  const effRankLabel =
    b.effectiveRankMin === b.effectiveRankMax
      ? String(b.effectiveRankMin)
      : `${b.effectiveRankMin}-${b.effectiveRankMax}`
  const rankBonusMin = b.effectiveRankMin - currentRank
  const rankBonusMax = b.effectiveRankMax - currentRank
  const hasRankBonus = rankBonusMin !== 0 || rankBonusMax !== 0
  const footerLabel =
    b.projectileCount > 1
      ? 'Average per swing'
      : b.critChance > 0
        ? 'Average hit'
        : 'Hit damage'
  const footerUsesAvg = b.projectileCount > 1 || b.critChance > 0

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
        {b.skillFlatPhysMax > 0 && (
          <BDLine
            label="Skill flat damage"
            value={
              <span className="text-text">
                +{formatRangeInt(b.skillFlatPhysMin, b.skillFlatPhysMax)}
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
                  +{formatRange(line.pctMin, line.pctMax)}%
                </span>
              }
            />
          ))}
          <BDLine
            label="Total synergy"
            value={
              <span className="text-yellow-300">
                +{formatRange(b.synergyMinPct, b.synergyMaxPct)}%
              </span>
            }
          />
        </BDSection>
      )}
      <BDSection title="Multipliers">
        <BDLine
          label="Attack damage (skill)"
          value={
            <span className="text-accent-hot">
              {formatRange(b.weaponDamagePctMin, b.weaponDamagePctMax)}%
            </span>
          }
        />
        {b.attackDamageMaxPct > 0 && (
          <BDLine
            label="Attack damage (gear)"
            value={
              <span className="text-accent-hot">
                +{formatRange(b.attackDamageMinPct, b.attackDamageMaxPct)}%
              </span>
            }
          />
        )}
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
        {b.critChance > 0 && (
          <BDLine
            label={`Crit (${formatDecimal(b.critChance)}%, +${formatDecimal(b.critDamagePct)}%)`}
            value={
              <span className="text-accent-hot">
                ×{b.critMultiplierAvg.toFixed(2)}
              </span>
            }
          />
        )}
        {b.projectileCount > 1 && (
          <BDLine
            label={`Projectiles (${b.projectileCount}×)`}
            value={
              <span className="text-accent-hot">
                ×{b.projectileCount.toFixed(2)}
              </span>
            }
          />
        )}
      </BDSection>
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
      {b.poisonHitMax > 0 && (
        <BDSection title="Elemental part">
          <BDLine
            label="Elemental hit"
            value={
              <span className="text-sky-300">
                {formatRangeInt(b.poisonHitMin, b.poisonHitMax)}
              </span>
            }
          />
        </BDSection>
      )}
      <div className="mt-1.5 flex items-baseline justify-between gap-3 border-t border-border pt-1.5">
        <span className="font-mono text-[10px] font-medium uppercase tracking-[0.14em] text-muted">
          {footerLabel}
        </span>
        <span className="font-mono text-[14px] font-semibold tabular-nums text-accent-hot">
          {footerUsesAvg
            ? formatRangeInt(b.combinedAvgMin, b.combinedAvgMax)
            : formatRangeInt(b.combinedHitMin, b.combinedHitMax)}
        </span>
      </div>
    </div>
  )
}
