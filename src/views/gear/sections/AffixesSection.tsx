/* eslint-disable react-refresh/only-export-components */
import { useMemo, useState } from 'react'
import { useCalcResult } from '../../../hooks/useCalcResult'
import PickerModal, { type PickerRow } from '../../../components/PickerModal'
import { activeSeasonId, affixes, effectiveStars, getAffix } from '../../../data'
import {
  formatAffixRangeFromValues,
  formatValue,
} from '../../../utils/item/stats'
import { displayValuesNative } from '../../../lib/calc/bridge'
import type { AffixValueOutput } from '../../../lib/calc/bridge'
import type { Affix, EquippedItem, ItemBase } from '../../../types'
import { buildAffixTooltip } from '../tooltips'
import { SectionCard } from '../SectionCard'

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

function affixToPickerRow(a: Affix, opts?: { useDescriptionAsName?: boolean }): PickerRow {
  const primary = opts?.useDescriptionAsName ? a.description : a.name
  const meta = opts?.useDescriptionAsName ? a.name : a.description
  const isUnholy = a.groupId === 'random_unholy'
  return {
    id: a.id,
    name: primary,
    tier: a.tier,
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
}: {
  equipped: EquippedItem
  base?: ItemBase
  maxAffixes?: number
  onAdd: (affixId: string, tier: number) => void
  onRemove: (index: number) => void
}) {
  const [open, setOpen] = useState(false)
  const atCap =
    maxAffixes !== undefined && equipped.affixes.length >= maxAffixes
  const modalOpen = open && !atCap

  const randomGroupId = base?.randomAffixGroupId ?? null
  const isUnholy = randomGroupId === 'random_unholy'

  const equippedAffixItems = useMemo(
    () => equipped.affixes.map((eq) => ({ def: getAffix(eq.affixId) })),
    [equipped.affixes],
  )
  const affixRanges = useAffixDisplayRanges(
    equippedAffixItems,
    effectiveStars(base?.slot ?? '', activeSeasonId, equipped.stars),
  )

  const pickerRows = useMemo<PickerRow[]>(() => {
    const source = randomGroupId
      ? affixes.filter((a) => a.groupId === randomGroupId)
      : affixes
    return source
      .slice()
      .sort(
        (a, b) =>
          (isUnholy ? a.description : a.name).localeCompare(
            isUnholy ? b.description : b.name,
          ) || a.tier - b.tier,
      )
      .map((a) => affixToPickerRow(a, { useDescriptionAsName: isUnholy }))
  }, [randomGroupId, isUnholy])

  const sectionTitle = isUnholy ? 'Unholy Affixes' : 'Affixes'
  const modalTitle = isUnholy ? 'Pick Unholy Affix' : 'Add Affix'

  return (
    <SectionCard
      label={sectionTitle}
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
          return (
            <div
              key={idx}
              className="flex items-center gap-1.5 rounded-[3px] border border-accent-deep/15 bg-bg/40 p-1.5"
            >
              <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-xs border border-accent-deep/30 bg-panel-2/60 font-mono text-[10px] tabular-nums text-accent-hot/80">
                {idx + 1}
              </span>
              <span className="flex min-w-0 flex-1 items-baseline gap-1.5 truncate text-[12px] leading-snug">
                <span className="font-mono font-semibold tabular-nums text-accent-hot">
                  {eq.customValue !== undefined && affix.statKey
                    ? formatValue(eq.customValue, affix.statKey)
                    : formatAffixRangeFromValues(
                        affix,
                        affixRanges[idx] ?? null,
                      )}
                </span>
                <span className="truncate text-text/85">{affix.name}</span>
                <span className="rounded-xs border border-accent-deep/40 px-1 py-px font-mono text-[9px] tabular-nums text-accent-hot/75">
                  T{affix.tier}
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
