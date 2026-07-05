import { useMemo, type ReactNode } from 'react'
import { subskillAggregationNative } from '../../lib/calc/bridge'
import type { SubtreeAggregation } from '../../lib/calc/bridge'
import { useCalcResult } from '../../hooks/useCalcResult'
import { useSkillRankInfo } from '../../hooks/useSkillRankInfo'
import { subskillKey } from '../../store/build'
import {
  formatValue,
  normalizeSkillName,
  statName,
} from '../../utils/item/stats'
import type {
  AttributeKey,
  RangedValue,
  Skill,
  SubskillNode,
} from '../../types'
import { SYNERGY_RGB } from './treeConstants'
import {
  evalFormulaClamped,
  formatDmgRange,
  formatFlatPhys,
  formatPair,
  formatPctRange,
  formatStatPair,
} from './format'

export function DetailBlock({
  title,
  trailing,
  accentRgb,
  onMouseLeave,
  children,
}: {
  title: string
  trailing?: ReactNode
  accentRgb?: string
  onMouseLeave?: () => void
  children: ReactNode
}) {
  return (
    <div
      className="mb-3 rounded-[3px] border border-border-2 p-2.5"
      onMouseLeave={onMouseLeave}
      style={{
        background:
          'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
        boxShadow: 'inset 0 1px 0 rgba(255,255,255,0.02)',
      }}
    >
      <div
        className="mb-2 flex items-center justify-between gap-2 border-b pb-1.5"
        style={{
          borderColor: accentRgb
            ? `rgba(${accentRgb},0.25)`
            : 'rgba(138,111,58,0.2)',
        }}
      >
        <div
          className="flex items-center gap-1.5 font-mono text-[11px] uppercase tracking-[0.16em]"
          style={{ color: accentRgb ? `rgb(${accentRgb})` : undefined }}
        >
          <span
            aria-hidden
            className="inline-block h-1 w-1 rotate-45"
            style={{
              background: accentRgb
                ? `rgb(${accentRgb})`
                : 'var(--color-accent-deep)',
            }}
          />
          <span className={accentRgb ? '' : 'text-accent-hot/80'}>{title}</span>
        </div>
        {trailing}
      </div>
      {children}
    </div>
  )
}

const EMPTY_SUBTREE_AGGREGATION: SubtreeAggregation = {
  stats: {},
  procStats: {},
  appliedStates: [],
}

const PROVIDES_RGB = '217,154,90'

export function SubtreeBonusBlock({
  skill,
  subskillRanks,
  enemyConditions,
}: {
  skill: Skill
  subskillRanks: Record<string, number>
  enemyConditions: Record<string, boolean>
}) {
  const agg = useCalcResult<SubtreeAggregation>(
    () =>
      subskillAggregationNative(
        skill.classId,
        skill.id,
        subskillRanks,
        enemyConditions,
      ),
    [skill, subskillRanks, enemyConditions],
    EMPTY_SUBTREE_AGGREGATION,
  )
  const statEntries = Object.entries(agg.stats)
    .filter(([, v]) => v !== 0)
    .sort(([a], [b]) => a.localeCompare(b))

  const activeProcs = useMemo(() => {
    const out: { sub: SubskillNode; rank: number }[] = []
    for (const sub of skill.subskills ?? []) {
      if (!sub.proc) continue
      const rank = subskillRanks[subskillKey(skill.id, sub.id)] ?? 0
      if (rank <= 0) continue
      out.push({ sub, rank })
    }
    return out
  }, [skill, subskillRanks])

  if (statEntries.length === 0 && activeProcs.length === 0) return null

  return (
    <DetailBlock title="Subtree bonuses">
      {statEntries.length > 0 && (
        <div className="space-y-1 text-[12px] tabular-nums">
          {statEntries.map(([k, v]) => (
            <div key={k} className="flex justify-between">
              <span className="text-muted">{statName(k)}</span>
              <span className="text-accent-hot">{formatValue(v, k)}</span>
            </div>
          ))}
        </div>
      )}
      {activeProcs.length > 0 && (
        <div
          className={`space-y-1.5 ${statEntries.length > 0 ? 'mt-2.5 border-t border-dashed border-accent-deep/30 pt-2' : ''}`}
        >
          {activeProcs.map(({ sub, rank }) => {
            const proc = sub.proc!
            const chance =
              (proc.chance.base ?? 0) + (proc.chance.perRank ?? 0) * rank
            const effectParts: string[] = []
            const baseMap: Record<string, number> = proc.effects?.base ?? {}
            const perRankMap: Record<string, number> = proc.effects?.perRank ?? {}
            for (const [k, base] of Object.entries(baseMap)) {
              const per = perRankMap[k] ?? 0
              const v = base + per * rank
              if (v === 0) continue
              effectParts.push(`${formatValue(v, k)} ${statName(k)}`)
            }
            for (const [k, per] of Object.entries(perRankMap)) {
              if (k in baseMap) continue
              const v = per * rank
              if (v === 0) continue
              effectParts.push(`${formatValue(v, k)} ${statName(k)}`)
            }
            for (const s of proc.appliesStates ?? []) {
              if (typeof s === 'string') {
                effectParts.push(`applies ${s.replace(/_/g, ' ')}`)
              } else {
                const amt =
                  (s.amount?.base ?? 0) + (s.amount?.perRank ?? 0) * rank
                effectParts.push(
                  amt
                    ? `applies ${s.state.replace(/_/g, ' ')} (${amt}%)`
                    : `applies ${s.state.replace(/_/g, ' ')}`,
                )
              }
            }
            return (
              <div key={sub.id} className="text-[12px]">
                <div className="flex items-baseline justify-between gap-2">
                  <span className="text-text">{sub.name}</span>
                  <span className="tabular-nums">
                    <span className="text-accent-hot">{chance}%</span>{' '}
                    <span className="font-mono text-[11px] uppercase tracking-[0.12em] text-muted">
                      {proc.trigger.replace('_', ' ')}
                    </span>
                  </span>
                </div>
                <div className="text-muted leading-snug">
                  {effectParts.join(', ')}
                </div>
              </div>
            )
          })}
        </div>
      )}
    </DetailBlock>
  )
}


export function SkillEffectsBlock({
  skill,
  currentRank,
  effRankMin,
  effRankMax,
  allClassSkills,
  skillRanks,
  attributes,
  rankBonuses,
  onSynergyHover,
  buffingAuraEffectiveness,
}: {
  skill: Skill
  currentRank: number
  effRankMin: number
  effRankMax: number
  allClassSkills: Skill[]
  skillRanks: Record<string, number>
  attributes: Record<AttributeKey, RangedValue>
  rankBonuses: Record<string, [number, number]>
  onSynergyHover: (id: string | null) => void
  buffingAuraEffectiveness?: RangedValue
}) {
  const allocated = currentRank > 0
  const auraEff = buffingAuraEffectiveness ?? 0
  const auraEffMin = typeof auraEff === 'number' ? auraEff : auraEff[0]
  const auraEffMax = typeof auraEff === 'number' ? auraEff : auraEff[1]
  const applyAuraBoost =
    skill.kind === 'aura' && (auraEffMin !== 0 || auraEffMax !== 0)
  const boostMin = applyAuraBoost ? 1 + auraEffMin / 100 : 1
  const boostMax = applyAuraBoost ? 1 + auraEffMax / 100 : 1
  const curMin = allocated ? effRankMin : 1
  const curMax = allocated ? effRankMax : 1
  const canIncrement = allocated && currentRank < skill.maxRank
  const nextMin = canIncrement ? curMin + 1 : null
  const nextMax = canIncrement ? curMax + 1 : null

  const rankInfo = useSkillRankInfo(
    skill,
    [curMin, curMax, nextMin, nextMax].filter((r): r is number => r !== null),
  )
  const passiveCurMin = rankInfo.get(curMin)?.passive ?? {}
  const passiveCurMax = rankInfo.get(curMax)?.passive ?? {}
  const passiveNextMin =
    nextMin !== null ? (rankInfo.get(nextMin)?.passive ?? {}) : null
  const passiveNextMax =
    nextMax !== null ? (rankInfo.get(nextMax)?.passive ?? {}) : null

  const manaCurMin = rankInfo.get(curMin)?.mana
  const manaCurMax = rankInfo.get(curMax)?.mana
  const manaNextMin = nextMin !== null ? rankInfo.get(nextMin)?.mana : undefined
  const manaNextMax = nextMax !== null ? rankInfo.get(nextMax)?.mana : undefined

  const computeBaseDmg = (rank: number): [number, number] | null => {
    if (skill.damageFormula) {
      const v = Math.max(0, skill.damageFormula.base + skill.damageFormula.perLevel * rank)
      return [v, v]
    }
    if (skill.damagePerRank) {
      const t = skill.damagePerRank
      const n = t.length
      if (n === 0 || rank < 1) return null
      if (rank <= n) {
        const d = t[rank - 1]!
        return [Math.max(0, d.min), Math.max(0, d.max)]
      }
      const last = t[n - 1]!
      const prev = n >= 2 ? t[n - 2]! : last
      const over = rank - n
      return [
        Math.max(0, last.min + (last.min - prev.min) * over),
        Math.max(0, last.max + (last.max - prev.max) * over),
      ]
    }
    return null
  }
  const baseDmgCurMin = computeBaseDmg(curMin)
  const baseDmgCurMax = computeBaseDmg(curMax)
  const baseDmgNextMin = nextMin !== null ? computeBaseDmg(nextMin) : null
  const baseDmgNextMax = nextMax !== null ? computeBaseDmg(nextMax) : null

  const synergiesReceived: Array<{
    source: string
    perLabel: string
    sourceMin: number
    sourceMax: number
    pctMin: number
    pctMax: number
    stat: string
  }> = []
  if (allocated) {
    for (const bs of skill.bonusSources ?? []) {
      if (bs.per === 'skill_level') {
        const srcKey = normalizeSkillName(bs.source)
        const srcSkill = allClassSkills.find(
          (s) => normalizeSkillName(s.name) === srcKey,
        )
        if (!srcSkill) continue
        const baseRank = skillRanks[srcSkill.id] ?? 0
        if (baseRank === 0) continue
        const bonus: [number, number] = rankBonuses[srcKey] ?? [0, 0]
        const effMin = baseRank + bonus[0]
        const effMax = baseRank + bonus[1]
        const perLabel =
          effMin === effMax ? `rank ${effMin}` : `rank ${effMin}-${effMax}`
        synergiesReceived.push({
          source: bs.source,
          perLabel,
          sourceMin: effMin,
          sourceMax: effMax,
          pctMin: effMin * bs.value,
          pctMax: effMax * bs.value,
          stat: bs.stat,
        })
      } else if (bs.per === 'attribute_point') {
        const attrKey = (
          Object.keys(attributes) as AttributeKey[]
        ).find((k) => k.toLowerCase() === bs.source.trim().toLowerCase())
        if (!attrKey) continue
        const attr = attributes[attrKey] ?? 0
        const aMin = typeof attr === 'number' ? attr : attr[0]
        const aMax = typeof attr === 'number' ? attr : attr[1]
        if (aMin === 0 && aMax === 0) continue
        synergiesReceived.push({
          source: bs.source,
          perLabel: aMin === aMax ? `${aMin}` : `${aMin}-${aMax}`,
          sourceMin: aMin,
          sourceMax: aMax,
          pctMin: aMin * bs.value,
          pctMax: aMax * bs.value,
          stat: bs.stat,
        })
      }
    }
  }

  const synergiesProvided: Array<{
    target: string
    targetId: string
    perRank: number
    pctCurMin: number
    pctCurMax: number
    stat: string
  }> = []
  for (const other of allClassSkills) {
    for (const bs of other.bonusSources ?? []) {
      if (
        bs.per === 'skill_level' &&
        normalizeSkillName(bs.source) === normalizeSkillName(skill.name)
      ) {
        synergiesProvided.push({
          target: other.name,
          targetId: other.id,
          perRank: bs.value,
          pctCurMin: bs.value * curMin,
          pctCurMax: bs.value * curMax,
          stat: bs.stat,
        })
      }
    }
  }

  const passiveKeys = new Set([
    ...Object.keys(passiveCurMin),
    ...Object.keys(passiveCurMax),
  ])
  const hasAnything =
    passiveKeys.size > 0 ||
    baseDmgCurMin !== null ||
    manaCurMin !== undefined ||
    synergiesProvided.length > 0 ||
    synergiesReceived.length > 0 ||
    (skill.bonusSources?.length ?? 0) > 0

  if (!hasAnything) return null

  const hasOwnEffects =
    baseDmgCurMin !== null ||
    manaCurMin !== undefined ||
    passiveKeys.size > 0 ||
    !!skill.attackScaling

  const rankLabel =
    curMin === curMax ? String(curMin) : `${curMin}-${curMax}`
  const nextRankLabel =
    nextMin === null
      ? null
      : nextMin === nextMax
        ? String(nextMin)
        : `${nextMin}-${nextMax}`

  return (
    <>
    {hasOwnEffects && (
    <DetailBlock
      title={allocated ? 'Effects' : 'Preview (not learned)'}
      trailing={
        <span className="font-mono text-[11px] tabular-nums tracking-[0.06em] text-muted">
          rank {rankLabel}
          {nextRankLabel && (
            <>
              {' '}
              <span className="text-accent-deep">→</span> {nextRankLabel}
            </>
          )}
        </span>
      }
    >
      <div className="space-y-1 text-[12px] tabular-nums">
        {baseDmgCurMin && baseDmgCurMax && (
          <EffRow
            label={
              skill.attackKind === 'attack' && skill.damageType
                ? `${skill.damageType[0]!.toUpperCase()}${skill.damageType.slice(1)} damage`
                : 'Base damage'
            }
            cur={formatDmgRange(baseDmgCurMin, baseDmgCurMax)}
            next={
              baseDmgNextMin && baseDmgNextMax
                ? formatDmgRange(baseDmgNextMin, baseDmgNextMax)
                : undefined
            }
            color={allocated ? 'text-accent-hot' : 'text-muted'}
          />
        )}
        {(['weaponDamagePct', 'attackRatingPct'] as const).map((key) => {
          const f = skill.attackScaling?.[key]
          if (!f) return null
          const label = key === 'weaponDamagePct' ? 'Attack damage' : 'Attack rating'
          return (
            <EffRow
              key={key}
              label={label}
              cur={formatPctRange(evalFormulaClamped(f, curMin), evalFormulaClamped(f, curMax))}
              next={
                nextMin !== null && nextMax !== null
                  ? formatPctRange(
                      evalFormulaClamped(f, nextMin),
                      evalFormulaClamped(f, nextMax),
                    )
                  : undefined
              }
              color={allocated ? 'text-accent-hot' : 'text-muted'}
            />
          )
        })}
        {skill.attackScaling?.flatPhysicalMin &&
          skill.attackScaling.flatPhysicalMax && (
            <EffRow
              label="Physical damage"
              cur={formatFlatPhys(
                skill.attackScaling.flatPhysicalMin,
                skill.attackScaling.flatPhysicalMax,
                curMin,
                curMax,
              )}
              next={
                nextMin !== null && nextMax !== null
                  ? formatFlatPhys(
                      skill.attackScaling.flatPhysicalMin,
                      skill.attackScaling.flatPhysicalMax,
                      nextMin,
                      nextMax,
                    )
                  : undefined
              }
              color={allocated ? 'text-accent-hot' : 'text-muted'}
            />
          )}
        {manaCurMin !== undefined && manaCurMax !== undefined && (
          <EffRow
            label="Mana cost"
            cur={formatPair([manaCurMin, manaCurMax])}
            next={
              manaNextMin !== undefined && manaNextMax !== undefined
                ? formatPair([manaNextMin, manaNextMax])
                : undefined
            }
            color={allocated ? 'text-stat-blue' : 'text-muted'}
          />
        )}
        {[...passiveKeys].map((key) => {
          const vMin = (passiveCurMin[key] ?? 0) * boostMin
          const vMax = (passiveCurMax[key] ?? 0) * boostMax
          const rawNMin = passiveNextMin ? passiveNextMin[key] : undefined
          const rawNMax = passiveNextMax ? passiveNextMax[key] : undefined
          const nMin = rawNMin !== undefined ? rawNMin * boostMin : undefined
          const nMax = rawNMax !== undefined ? rawNMax * boostMax : undefined
          return (
            <EffRow
              key={key}
              label={statName(key)}
              cur={formatStatPair(key, vMin, vMax)}
              next={
                nMin !== undefined && nMax !== undefined
                  ? formatStatPair(key, nMin, nMax)
                  : undefined
              }
              color={allocated ? 'text-accent-hot' : 'text-muted'}
            />
          )
        })}
      </div>
    </DetailBlock>
    )}
    {synergiesProvided.length > 0 && (
      <DetailBlock
        title="Provides synergy to"
        accentRgb={PROVIDES_RGB}
        onMouseLeave={() => onSynergyHover(null)}
      >
        <div className="space-y-0.5 text-[12px]">
          {synergiesProvided.map((s, i) => (
            <div
              key={i}
              onMouseEnter={() => onSynergyHover(s.targetId)}
              className="-mx-1.5 rounded-[2px] px-1.5 py-1 tabular-nums transition-colors hover:bg-[rgba(217,154,90,0.12)]"
            >
              <div className="flex items-baseline justify-between gap-2">
                <span className="flex min-w-0 items-baseline gap-1.5">
                  <span
                    aria-hidden
                    className="inline-block h-1.5 w-1.5 shrink-0 translate-y-[-1px] rotate-45"
                    style={{
                      background: `rgb(${PROVIDES_RGB})`,
                      boxShadow: `0 0 5px rgba(${PROVIDES_RGB},0.6)`,
                    }}
                  />
                  <span className="truncate text-text/85">{s.target}</span>
                </span>
                <span
                  className={`shrink-0 ${allocated ? 'text-stat-orange' : 'text-muted'}`}
                >
                  {formatStatPair(s.stat, s.pctCurMin, s.pctCurMax)}
                </span>
              </div>
              <div className="pl-3 text-[10px] text-faint">
                {s.perRank}% per rank
              </div>
            </div>
          ))}
        </div>
      </DetailBlock>
    )}
    {(skill.bonusSources?.length ?? 0) > 0 && (
      <DetailBlock
        title="Receives synergy from"
        accentRgb={SYNERGY_RGB}
        onMouseLeave={() => onSynergyHover(null)}
      >
        <div className="space-y-0.5 text-[12px]">
          {(skill.bonusSources ?? []).map((bs, i) => {
            const matched = synergiesReceived.find(
              (sr) => sr.source === bs.source && sr.stat === bs.stat,
            )
            const srcSkill =
              bs.per === 'skill_level'
                ? allClassSkills.find(
                    (s) =>
                      normalizeSkillName(s.name) ===
                      normalizeSkillName(bs.source),
                  )
                : undefined
            const nodeId = srcSkill?.id ?? null
            return (
              <div
                key={i}
                onMouseEnter={() => onSynergyHover(nodeId)}
                className={`-mx-1.5 rounded-[2px] px-1.5 py-1 tabular-nums transition-colors ${
                  nodeId ? 'hover:bg-[rgba(167,139,250,0.12)]' : ''
                }`}
              >
                <div className="flex items-baseline justify-between gap-2">
                  <span className="flex min-w-0 items-baseline gap-1.5">
                    <span
                      aria-hidden
                      className="inline-block h-1.5 w-1.5 shrink-0 translate-y-[-1px] rotate-45"
                      style={{
                        background: nodeId
                          ? `rgb(${SYNERGY_RGB})`
                          : 'var(--color-faint)',
                        boxShadow: nodeId
                          ? `0 0 5px rgba(${SYNERGY_RGB},0.6)`
                          : undefined,
                      }}
                    />
                    <span className="truncate text-text/85">{bs.source}</span>
                  </span>
                  <span
                    className={`shrink-0 ${matched ? 'text-stat-orange' : 'text-faint'}`}
                  >
                    {matched
                      ? formatStatPair(bs.stat, matched.pctMin, matched.pctMax)
                      : '—'}
                  </span>
                </div>
                <div className="pl-3 text-[10px] text-faint">
                  {bs.value}% per {bs.per === 'skill_level' ? 'rank' : 'point'}
                </div>
              </div>
            )
          })}
        </div>
      </DetailBlock>
    )}
    </>
  )
}

function EffRow({
  label,
  cur,
  next,
  color,
}: {
  label: string
  cur: string
  next?: string
  color: string
}) {
  const hasNext = !!next && next !== cur
  if (hasNext) {
    return (
      <div className="min-w-0 leading-snug">
        <div className="truncate text-text/80" title={label}>
          {label}
        </div>
        <div className="whitespace-nowrap pl-4">
          <span className={color}>{cur}</span>
          <span className="px-1.5 text-muted">→</span>
          <span className={`${color} opacity-65`}>{next}</span>
        </div>
      </div>
    )
  }
  return (
    <div className="flex items-baseline gap-2 min-w-0">
      <span className="text-text/80 truncate" title={label}>
        {label}
      </span>
      <span
        aria-hidden
        className="mb-[3px] min-w-2 flex-1 self-end border-b border-dotted border-faint/40"
      />
      <span className="whitespace-nowrap shrink-0">
        <span className={color}>{cur}</span>
      </span>
    </div>
  )
}
