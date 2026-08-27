/* eslint-disable react-refresh/only-export-components */
import { useMemo, useState } from 'react'
import { useCalcResult } from '../../../hooks/useCalcResult'
import PickerModal, { type PickerRow } from '../PickerModal'
import { affixes, effectiveStars, getAffix } from '@data'
import {
  formatAffixRangeFromValues,
  formatValue,
} from '../../../utils/item/stats'
import {
  formatAffixValue,
  rollFromValue,
  sliderPct,
  sliderStep,
  valueFromRoll,
} from '../lib/rollMath'
import {
  affixStatLabel,
  affixTierLabel,
  affixTiers,
  buildAffixGroups,
  groupBounds,
  isRandomPoolAffix,
  tierIndexForValue,
  type AffixGroup,
} from '../lib/affixGroups'
import {
  affixFitsPool,
  affixPoolLabel,
  affixPoolTypeFor,
} from '../../../utils/item/affixPools'
import { displayValuesNative } from '../../../utils/calc/bridge'
import type { AffixValueOutput } from '../../../utils/calc/bridge'
import type { Affix, EquippedItem, ItemBase } from '../../../types'
import { buildAffixGroupTooltip, buildAffixTooltip } from '../tooltips'
import { SectionCard } from '../SectionCard'
import { SectionIcon } from '../sectionIcons'

type AffixRangeDef = Parameters<typeof formatAffixRangeFromValues>[0] & {
  statKey: string | null
}

export interface AffixDisplayItem {
  def: AffixRangeDef | undefined
  roll?: number
}

export function useAffixDisplayRanges(
  items: AffixDisplayItem[],
  stars?: number | null,
): (AffixValueOutput | null)[] {
  return useCalcResult<(AffixValueOutput | null)[]>(
    () => {
      const present = items
        .map((item, i) => ({ item, i }))
        .filter((x) => !!x.item.def)
      if (present.length === 0) return items.map(() => null)
      return displayValuesNative({
        affixes: present.map((x) => ({
          affix: x.item.def,
          roll: x.item.roll ?? 0,
          stars: stars ?? null,
        })),
      }).then((res) => {
        const out: (AffixValueOutput | null)[] = items.map(() => null)
        present.forEach((x, j) => {
          out[x.i] = res.affixes[j] ?? null
        })
        return out
      })
    },
    [items, stars],
    [],
  )
}

function InvertedCrossIcon({ color = '#cf6db0' }: { color?: string }) {
  const dark = `color-mix(in srgb, ${color} 35%, #0d0b07)`
  return (
    <span
      className="flex h-7 w-7 items-center justify-center"
      aria-hidden="true"
    >
      <svg
        viewBox="0 0 24 24"
        width={20}
        height={20}
        style={{
          transform: 'rotate(180deg)',
          filter: `drop-shadow(0 0 6px ${color}80)`,
        }}
      >
        <defs>
          <linearGradient id="ic-cross" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={color} />
            <stop offset="100%" stopColor={dark} />
          </linearGradient>
        </defs>
        <path
          d="M10.5 2 H13.5 V8 H18 V11 H13.5 V22 H10.5 V11 H6 V8 H10.5 Z"
          fill="url(#ic-cross)"
          stroke={dark}
          strokeWidth="0.6"
          strokeLinejoin="miter"
        />
      </svg>
    </span>
  )
}

function kindOrder(a: Affix): number {
  return a.kind === 'prefix' ? 0 : a.kind === 'suffix' ? 1 : 2
}

function affixGroupToPickerRow(g: AffixGroup): PickerRow {
  const names = [...new Set(g.tiers.map((t) => t.name))]
  const kind = g.topTier.kind
  return {
    id: g.topTier.id,
    name: g.label,
    group: kind ? (kind === 'prefix' ? 'Prefixes' : 'Suffixes') : undefined,
    kindLabel: kind?.toUpperCase() ?? 'AFFIX',
    meta: names.join(' · '),
    searchTerms: `${names.join(' ')} ${g.tiers.map((t) => t.description).join(' ')}`,
    iconColor: 'var(--color-accent)',
    tooltip: buildAffixGroupTooltip(g.tiers, g.label),
  }
}

function affixToPickerRow(a: Affix, opts?: { useDescriptionAsName?: boolean }): PickerRow {
  const primary = opts?.useDescriptionAsName ? a.description : a.name
  const meta = opts?.useDescriptionAsName ? a.name : a.description
  const isUnholy = a.groupId === 'random_unholy'
  return {
    id: a.id,
    name: primary,
    tier: a.tier,
    group: a.kind ? (a.kind === 'prefix' ? 'Prefixes' : 'Suffixes') : undefined,
    kindLabel: a.kind?.toUpperCase() ?? 'AFFIX',
    meta,
    iconColor: isUnholy ? '#cf6db0' : 'var(--color-accent)',
    iconNode: isUnholy ? <InvertedCrossIcon color="#cf6db0" /> : undefined,
    tooltip: buildAffixTooltip(a),
  }
}

export function AffixesSection({
  equipped,
  base,
  maxAffixes,
  onAdd,
  onRemove,
  onSetRoll,
}: {
  equipped: EquippedItem
  base?: ItemBase
  maxAffixes?: number
  onAdd: (affixId: string, tier: number) => void
  onRemove: (index: number) => void
  onSetRoll?: (index: number, roll: number, affixId?: string) => void
}) {
  const [open, setOpen] = useState(false)
  const [showAllAffixes, setShowAllAffixes] = useState(false)
  const atCap =
    maxAffixes !== undefined && equipped.affixes.length >= maxAffixes
  const modalOpen = open && !atCap

  const randomGroupId = base?.randomAffixGroupId ?? null
  const isUnholy = randomGroupId === 'random_unholy'

  // Every tier of every equipped affix, so the slider can span the whole group.
  const equippedTiers = useMemo(
    () => equipped.affixes.map((eq) => affixTiers(eq.affixId)),
    [equipped.affixes],
  )
  const tierItems = useMemo(
    () => equippedTiers.flatMap((tiers) => tiers.map((def) => ({ def }))),
    [equippedTiers],
  )
  const tierRanges = useAffixDisplayRanges(
    tierItems,
    effectiveStars(base?.slot ?? '', equipped.stars),
  )
  const rangesByAffix = useMemo(() => {
    const out: (AffixValueOutput | null)[][] = []
    let at = 0
    for (const tiers of equippedTiers) {
      out.push(tierRanges.slice(at, at + tiers.length))
      at += tiers.length
    }
    return out
  }, [equippedTiers, tierRanges])

  const poolType = affixPoolTypeFor(base)

  const pickerRows = useMemo<PickerRow[]>(() => {
    // Unholy items roll one of many single-tier affixes, so those stay one row each.
    if (randomGroupId) {
      return affixes
        .filter((a) => a.groupId === randomGroupId)
        .slice()
        .sort(
          (a, b) =>
            (isUnholy ? a.description : a.name).localeCompare(
              isUnholy ? b.description : b.name,
            ) || a.tier - b.tier,
        )
        .map((a) => affixToPickerRow(a, { useDescriptionAsName: isUnholy }))
    }
    const rollable = affixes.filter((a) => !isRandomPoolAffix(a))
    const source =
      showAllAffixes || !poolType
        ? rollable
        : rollable.filter((a) => affixFitsPool(a, poolType))
    return buildAffixGroups(source)
      .sort(
        (a, b) =>
          kindOrder(a.topTier) - kindOrder(b.topTier) ||
          a.label.localeCompare(b.label),
      )
      .map(affixGroupToPickerRow)
  }, [randomGroupId, isUnholy, poolType, showAllAffixes])

  const sectionTitle = isUnholy ? 'Unholy Affixes' : 'Affixes'
  const modalTitle = isUnholy ? 'Pick Unholy Affix' : 'Add Affix'

  return (
    <SectionCard
      label={sectionTitle}
      icon={<SectionIcon kind="affixes" />}
      collapsible
      defaultOpen={equipped.affixes.length > 0}
      rightSlot={
        <>
          <span className="font-mono text-[10px] tabular-nums tracking-[0.04em] text-accent-hot/80">
            {equipped.affixes.length}
            {maxAffixes !== undefined ? ` / ${maxAffixes}` : ''}
          </span>
          <button
            onClick={() => setOpen(true)}
            disabled={atCap}
            className="rounded-xs border border-accent-deep px-2 py-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-accent-hot transition-all hover:border-accent-hot hover:shadow-[0_0_10px_rgba(224,184,100,0.25)] disabled:cursor-not-allowed disabled:border-border-2 disabled:text-faint disabled:shadow-none"
            style={{
              background: atCap
                ? 'transparent'
                : 'linear-gradient(180deg, #3a2f1a, #2a2418)',
            }}
          >
            + Add
          </button>
        </>
      }
      bodyClassName={
        equipped.affixes.length > 0 ? 'p-2 space-y-1.5' : 'px-3 py-2'
      }
    >
      {equipped.affixes.length === 0 ? (
        <div className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint italic">
          No affixes rolled
        </div>
      ) : (
        equipped.affixes.map((eq, idx) => {
          const affix = getAffix(eq.affixId)
          if (!affix) return null
          const tiers = equippedTiers[idx] ?? []
          const ranges = rangesByAffix[idx] ?? []
          const range = ranges[tiers.findIndex((t) => t.id === eq.affixId)] ?? null
          // Affixes the engine cannot calculate still roll a value worth pinning.
          const bounds = groupBounds(tiers, ranges)
          const sliderRange = bounds && bounds.lo !== bounds.hi ? bounds : null
          const lo = sliderRange?.lo ?? 0
          const hi = sliderRange?.hi ?? 0
          // The slider runs on magnitudes so dragging right always strengthens.
          const rolled =
            sliderRange && range
              ? Math.abs(
                  valueFromRoll(eq.roll, range.rangeMin, range.rangeMax, affix.format),
                )
              : 0
          const current = sliderRange
            ? Math.min(hi, Math.max(lo, Math.abs(eq.customValue ?? rolled)))
            : 0
          const statLabel = affixStatLabel(affix)
          return (
            <div
              key={idx}
              className="rounded-[3px] border border-accent-deep/15 bg-bg/40 p-1.5"
            >
              <div className="flex items-center gap-1.5">
                <span className="w-3 shrink-0 text-center font-mono text-[10px] tabular-nums text-faint">
                  {idx + 1}
                </span>
                <span className="flex min-w-0 flex-1 items-baseline gap-1.5 truncate text-[12px] leading-snug">
                  <span className="font-mono font-semibold tabular-nums text-accent-hot">
                    {eq.customValue !== undefined
                      ? affix.statKey
                        ? formatValue(eq.customValue, affix.statKey)
                        : formatAffixValue(affix, eq.customValue)
                      : sliderRange
                        ? formatAffixValue(affix, rolled)
                        : formatAffixRangeFromValues(affix, range)}
                  </span>
                  <span className="truncate text-text/85">
                    {statLabel}
                    {statLabel.toLowerCase().includes(affix.name.toLowerCase()) ? null : (
                      <span className="text-faint"> ({affix.name})</span>
                    )}
                  </span>
                  <span className="rounded-xs border border-accent-deep/40 px-1 py-px font-mono text-[9px] tabular-nums text-accent-hot/75">
                    {affixTierLabel(affix.tier)}
                  </span>
                  {eq.customValue !== undefined && (
                    <span
                      className="rounded-xs border border-accent-hot/60 px-1 py-px font-mono text-[9px] tabular-nums text-accent-hot"
                      title="Custom value override"
                    >
                      custom
                    </span>
                  )}
                </span>
                <button
                  onClick={() => onRemove(idx)}
                  className="flex h-5 w-5 shrink-0 items-center justify-center rounded-xs border border-border-2 font-mono text-[12px] leading-none text-faint transition-colors hover:border-stat-red hover:text-stat-red"
                  aria-label="Remove affix"
                >
                  ×
                </button>
              </div>
              {sliderRange && onSetRoll && (
                <div className="mt-1 flex items-center gap-2 pl-[26px] pr-0.5">
                  <input
                    type="range"
                    min={lo}
                    max={hi}
                    step={sliderStep(lo, hi)}
                    value={current}
                    aria-label={`${affix.name} roll`}
                    onChange={(e) => {
                      const value = Number(e.target.value)
                      const ti = tierIndexForValue(tiers, ranges, value)
                      const tier = tiers[ti]
                      const tierRange = ranges[ti]
                      if (!tier || !tierRange) return
                      onSetRoll(
                        idx,
                        rollFromValue(value, tierRange.rangeMin, tierRange.rangeMax),
                        tier.id,
                      )
                    }}
                    style={{ ['--sl-pct' as never]: sliderPct(current, lo, hi) }}
                    className="min-w-0 flex-1"
                  />
                  <span className="shrink-0 font-mono text-[9px] tabular-nums text-faint">
                    {formatAffixRangeFromValues(affix, {
                      rangeMin: lo,
                      rangeMax: hi,
                    })}
                  </span>
                </div>
              )}
            </div>
          )
        })
      )}

      {modalOpen && (
        <PickerModal
          title={modalTitle}
          sectionLabel="Affix"
          rows={pickerRows}
          searchPlaceholder="Search affixes…"
          emptyMessage="No matching affixes"
          width={680}
          footerActions={
            !randomGroupId && poolType ? (
              <label className="flex cursor-pointer items-center gap-1.5 font-mono text-[9px] uppercase tracking-[0.14em] text-faint transition-colors hover:text-accent-hot">
                <input
                  type="checkbox"
                  checked={showAllAffixes}
                  onChange={(e) => setShowAllAffixes(e.target.checked)}
                  className="accent-[var(--color-accent)]"
                />
                Show all affixes (outside the {affixPoolLabel(poolType)} pool)
              </label>
            ) : undefined
          }
          onSelect={(id) => {
            const a = affixes.find((x) => x.id === id)
            if (!a) return
            onAdd(a.id, a.tier)
          }}
          onClose={() => setOpen(false)}
        />
      )}
    </SectionCard>
  )
}
