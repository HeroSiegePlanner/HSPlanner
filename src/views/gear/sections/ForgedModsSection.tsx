import { useMemo, useState } from 'react'
import PickerModal, { type PickerRow } from '../../../components/PickerModal'
import { crystalMods, FORGE_KIND_LABEL, getCrystalMod } from '../../../data'
import type { ForgeKind } from '../../../data'
import { formatValue } from '../../../utils/item/stats'
import type { EquippedItem } from '../../../types'
import { affixAverageStats, buildCrystalModTooltip } from '../tooltips'
import { SectionCard } from '../SectionCard'
import { formatAffixRange } from './AffixesSection'

export function ForgedModsSection({
  forgeKind,
  equipped,
  onAdd,
  onRemove,
}: {
  forgeKind: ForgeKind
  equipped: EquippedItem
  onAdd: (modId: string, tier: number) => void
  onRemove: (index: number) => void
}) {
  // Editor for the (single) crystal-forge mod on a gear item, exposing a "+ Add" button that opens the satanic-crystal PickerModal and a per-mod remove control. The picker's row tooltips include a NetChangeBlock that diffs each candidate's averaged stats against whatever crystal is currently forged on the item, so users see the build delta without leaving the picker. Used inside the GearSlotModal right column for satanic-tier and above items.
  const [open, setOpen] = useState(false)
  const mods = useMemo(
    () => equipped.forgedMods ?? [],
    [equipped.forgedMods],
  )
  const sourceLabel = FORGE_KIND_LABEL[forgeKind]

  const previousStats = useMemo<Record<string, number>>(() => {
    const eq = mods[0]
    if (!eq) return {}
    const m = getCrystalMod(eq.affixId)
    if (!m) return {}
    return affixAverageStats(m)
  }, [mods])

  const pickerRows = useMemo<PickerRow[]>(
    () =>
      crystalMods
        .slice()
        .sort(
          (a, b) =>
            a.name.localeCompare(b.name) || a.tier - b.tier,
        )
        .map((m) => ({
          id: m.id,
          name: m.name,
          tier: m.tier,
          kindLabel: 'CRYSTAL',
          meta: m.description,
          iconColor: 'var(--color-stat-red)',
          tooltip: buildCrystalModTooltip(m, { previousStats }),
          tooltipTone: 'satanic' as const,
        })),
    [previousStats],
  )

  const canAdd = mods.length === 0

  return (
    <SectionCard
      label={`${sourceLabel} · Forged`}
      tone="satanic"
      rightSlot={
        canAdd ? (
          <button
            onClick={() => setOpen(true)}
            className="rounded-xs border border-red-500/40 px-2 py-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-red-300 transition-all hover:border-red-400 hover:text-red-200 hover:shadow-[0_0_10px_rgba(239,68,68,0.25)]"
            style={{
              background: 'linear-gradient(180deg, #3a1a1a, #2a1818)',
            }}
          >
            + Add
          </button>
        ) : undefined
      }
      bodyClassName={mods.length > 0 ? 'p-2 space-y-1.5' : 'px-3 py-2'}
    >
      {mods.length === 0 ? (
        <div className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint italic">
          No crystal forged
        </div>
      ) : (
        mods.map((eq, idx) => {
          const mod = getCrystalMod(eq.affixId)
          if (!mod) return null
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
                  {eq.customValue !== undefined && mod.statKey
                    ? formatValue(eq.customValue, mod.statKey)
                    : formatAffixRange(mod)}
                </span>
                <span className="truncate text-text/85">{mod.name}</span>
                <span className="rounded-xs border border-accent-deep/40 px-1 py-px font-mono text-[9px] tabular-nums text-accent-hot/75">
                  T{mod.tier}
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
                aria-label="Remove forged mod"
              >
                ×
              </button>
            </div>
          )
        })
      )}

      {open && (
        <PickerModal
          title={`Forge ${sourceLabel}`}
          sectionLabel={sourceLabel}
          rows={pickerRows}
          searchPlaceholder={`Search ${sourceLabel} mods…`}
          emptyMessage={`No matching ${sourceLabel} mods`}
          width={680}
          onSelect={(id) => {
            const m = crystalMods.find((x) => x.id === id)
            if (!m) return
            onAdd(m.id, m.tier)
          }}
          onClose={() => setOpen(false)}
        />
      )}
    </SectionCard>
  )
}
