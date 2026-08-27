import { useMemo } from 'react'
import { detectRuneword, effectiveStars } from '@data'
import { useCalcResult } from '../../../hooks/useCalcResult'
import { displayValuesNative } from '../../../utils/calc/bridge'
import {
  formatValue,
  rangedMax,
  rangedMin,
  statName,
} from '../../../utils/item/stats'
import type { EquippedItem, ItemBase, RangedValue } from '../../../types'
import { sliderPct, sliderStep } from '../lib/rollMath'
import { SectionCard } from '../SectionCard'
import { SectionIcon } from '../sectionIcons'

interface RollsSectionProps {
  equipped: EquippedItem
  base: ItemBase
  onSetOverride: (statKey: string, value: number | null) => void
  onSetSkillOverride: (skillName: string, value: number | null) => void
}

function RollRow({
  label,
  formatKey,
  lo,
  hi,
  override,
  onSet,
}: {
  label: string
  formatKey: string
  lo: number
  hi: number
  override: number | undefined
  onSet: (value: number | null) => void
}) {
  const pinned = override !== undefined
  const value = Math.min(hi, Math.max(lo, override ?? hi))
  const range = formatValue([lo, hi] as RangedValue, formatKey)
  return (
    <div className="rounded-[3px] border border-accent-deep/15 bg-bg/40 p-1.5">
      <div className="flex items-center gap-1.5">
        <span className="flex min-w-0 flex-1 items-baseline gap-1.5 truncate text-[12px] leading-snug">
          <span className="font-mono font-semibold tabular-nums text-accent-hot">
            {pinned ? formatValue(value, formatKey) : range}
          </span>
          <span className="truncate text-text/85">{label}</span>
          {pinned && (
            <span
              className="rounded-xs border border-accent-hot/60 px-1 py-px font-mono text-[9px] tabular-nums text-accent-hot"
              title="Pinned roll"
            >
              custom
            </span>
          )}
        </span>
        <button
          onClick={() => onSet(null)}
          aria-label={`Reset ${label} roll`}
          title="Reset to full range"
          className={`flex h-5 w-5 shrink-0 items-center justify-center rounded-xs border border-border-2 font-mono text-[12px] leading-none text-faint transition-colors hover:border-stat-red hover:text-stat-red ${
            pinned ? '' : 'invisible'
          }`}
        >
          ×
        </button>
      </div>
      <div className="mt-1 flex items-center gap-2 pr-0.5">
        <input
          type="range"
          min={lo}
          max={hi}
          step={sliderStep(lo, hi)}
          value={value}
          aria-label={`${label} roll`}
          onChange={(e) => onSet(Number(e.target.value))}
          style={{ ['--sl-pct' as never]: sliderPct(value, lo, hi) }}
          className="min-w-0 flex-1"
        />
        <span className="shrink-0 font-mono text-[9px] tabular-nums text-faint">
          {range}
        </span>
      </div>
    </div>
  )
}

export function RollsSection({
  equipped,
  base,
  onSetOverride,
  onSetSkillOverride,
}: RollsSectionProps) {
  const entries = useMemo(
    () =>
      Object.entries(base.implicit ?? {}).filter(
        ([key, v]) =>
          key !== 'random_skill_element' && Array.isArray(v) && v[0] !== v[1],
      ) as [string, [number, number]][],
    [base],
  )
  const skillEntries = useMemo(
    () =>
      Object.entries(base.skillBonuses ?? {}).filter(
        ([, v]) => Array.isArray(v) && v[0] !== v[1],
      ) as [string, [number, number]][],
    [base],
  )
  const isRuneword = !!detectRuneword(base, equipped.socketed)
  const stars = isRuneword ? null : effectiveStars(base.slot, equipped.stars)
  // Skill ranks star-scale regardless of runewords (mirrors calc/rank.rs).
  const skillStars = effectiveStars(base.slot, equipped.stars)
  const bounds = useCalcResult<[number, number][] | null>(
    () =>
      entries.length === 0 && skillEntries.length === 0
        ? null
        : displayValuesNative({
            scaled: [
              ...entries.map(([statKey, value]) => ({ value, statKey, stars })),
              ...skillEntries.map(([, value]) => ({
                value,
                statKey: 'item_granted_skill_rank',
                stars: skillStars,
              })),
            ],
          }).then((res) => res.scaled),
    [entries, skillEntries, stars, skillStars],
    null,
  )
  if (entries.length === 0 && skillEntries.length === 0) return null

  const overrides = equipped.implicitOverrides
  const skillOverrides = equipped.skillBonusOverrides
  const pinnedStats = entries.filter(([k]) => overrides?.[k] !== undefined)
  const pinnedSkills = skillEntries.filter(
    ([k]) => skillOverrides?.[k] !== undefined,
  )
  const pinnedCount = pinnedStats.length + pinnedSkills.length
  const totalCount = entries.length + skillEntries.length
  return (
    <SectionCard
      label="Stat Rolls"
      icon={<SectionIcon kind="rolls" />}
      collapsible
      defaultOpen={pinnedCount > 0}
      rightSlot={
        <>
          <span
            className={`font-mono text-[10px] tabular-nums tracking-[0.04em] ${
              pinnedCount > 0 ? 'text-accent-hot/80' : 'text-faint'
            }`}
          >
            {pinnedCount > 0
              ? `${pinnedCount}/${totalCount} pinned`
              : `${totalCount} rollable`}
          </span>
          {pinnedCount > 0 && (
            <button
              type="button"
              onClick={() => {
                pinnedStats.forEach(([k]) => onSetOverride(k, null))
                pinnedSkills.forEach(([k]) => onSetSkillOverride(k, null))
              }}
              className="rounded-xs border border-border-2 px-2 py-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-faint transition-colors hover:border-stat-red hover:text-stat-red"
            >
              Reset
            </button>
          )}
        </>
      }
      bodyClassName="p-2 space-y-1.5"
    >
      {entries.map(([key, raw], i) => (
        <RollRow
          key={key}
          label={statName(key)}
          formatKey={key}
          lo={bounds?.[i]?.[0] ?? rangedMin(raw)}
          hi={bounds?.[i]?.[1] ?? rangedMax(raw)}
          override={overrides?.[key]}
          onSet={(v) => onSetOverride(key, v)}
        />
      ))}
      {skillEntries.map(([skill, raw], i) => (
        <RollRow
          key={`skill:${skill}`}
          label={`to ${skill}`}
          formatKey=""
          lo={bounds?.[entries.length + i]?.[0] ?? rangedMin(raw)}
          hi={bounds?.[entries.length + i]?.[1] ?? rangedMax(raw)}
          override={skillOverrides?.[skill]}
          onSet={(v) => onSetSkillOverride(skill, v)}
        />
      ))}
      <p className="pt-0.5 font-mono text-[9px] uppercase tracking-[0.14em] leading-snug text-faint">
        Drag to pin a roll — unpinned stats count as their full range.
      </p>
    </SectionCard>
  )
}
