/* eslint-disable react-refresh/only-export-components */
import { useMemo, useState } from 'react'
import PickerModal, { type PickerRow } from '../../../components/PickerModal'
import { affixes, getAffix } from '../../../data'
import { formatValue, rolledAffixValueWithStars } from '../../../utils/item/stats'
import type { Affix, EquippedItem, ItemBase } from '../../../types'
import { buildAffixTooltip } from '../tooltips'
import { SectionCard } from '../SectionCard'

export function formatAffixRange(
  affix: {
    sign: '+' | '-'
    format: 'flat' | 'percent'
    valueMin: number | null
    valueMax: number | null
    statKey: string | null
  },
  stars?: number,
): string {
  // Formats an affix's full roll window as a signed bracketed range ("+[15-25]%", "+[12-18]", "+30") with star scaling applied to both endpoints. Single-value affixes (min === max) drop the brackets and dash. Used by AffixesSection so each row shows the *range* the affix can roll, not just its current rolled value.
  if (affix.valueMin === null || affix.valueMax === null) return affix.sign
  const minSigned = rolledAffixValueWithStars(affix, 0, stars)
  const maxSigned = rolledAffixValueWithStars(affix, 1, stars)
  const fmtAbs = (v: number) => {
    const abs = Math.abs(v)
    return Number.isInteger(abs) ? abs : Math.round(abs * 100) / 100
  }
  const lo = fmtAbs(minSigned)
  const hi = fmtAbs(maxSigned)
  const sign =
    affix.sign === '-' || minSigned < 0 || maxSigned < 0 ? '-' : '+'
  const suffix = affix.format === 'percent' ? '%' : ''
  if (lo === hi) return `${sign}${hi}${suffix}`
  return `${sign}[${lo}-${hi}]${suffix}`
}

function InvertedCrossIcon({ color = '#cf6db0' }: { color?: string }) {
  // Pixel-art rotated crucifix glyph used as the row icon for unholy-group affixes in the PickerModal — replaces the default rotated diamond so unholy entries read as occult/dark even on a quick glance. Renders at the same 28×28 footprint as `FallbackIcon` so the row grid stays aligned.
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
  // Maps an affix definition into a PickerRow for the AffixesSection picker. When `useDescriptionAsName` is set, the description is shown as the primary label (used by the random_unholy group whose names are opaque). Affixes from the `random_unholy` group get the inverted-cross icon (pink unholy tone) instead of the default rotated diamond. Attaches a rich tooltip so users can preview the full description, value range, and stat key on hover.
  const primary = opts?.useDescriptionAsName ? a.description : a.name
  const meta = opts?.useDescriptionAsName ? a.name : a.description
  const isUnholy = a.groupId === 'random_unholy'
  return {
    id: a.id,
    name: primary,
    tier: a.tier,
    kindLabel: a.kind?.toUpperCase() ?? 'AFFIX',
    meta,
    iconColor: isUnholy ? '#cf6db0' : '#c9a560',
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
  // Per-item affix editor that lists each rolled affix with a roll slider and remove button, and exposes an "Add" button that opens the affix PickerModal restricted to the affix groups available on the current item slot. Honours the item's `maxAffixes` cap and routes the special `random_unholy` group through the same modal with description-first labelling.
  const [open, setOpen] = useState(false)
  const atCap =
    maxAffixes !== undefined && equipped.affixes.length >= maxAffixes
  const modalOpen = open && !atCap

  const randomGroupId = base?.randomAffixGroupId ?? null
  const isUnholy = randomGroupId === 'random_unholy'

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
                <span className="font-mono font-semibold tabular-nums text-amber-300">
                  {eq.customValue !== undefined && affix.statKey
                    ? formatValue(eq.customValue, affix.statKey)
                    : formatAffixRange(affix, equipped.stars)}
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
