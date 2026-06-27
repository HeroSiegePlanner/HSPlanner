import { useMemo, useState, type ReactNode } from 'react'
import PickerModal, { type PickerPanelState, type PickerRow } from '../../../components/PickerModal'
import { TooltipPanel } from '../../../components/Tooltip'
import { runewords } from '../../../data'
import type { ItemBase } from '../../../types'
import { buildRunewordTooltip, NetChangeBlock } from '../tooltips'
import { SectionCard } from '../SectionCard'

export function RunewordPresets({
  base,
  maxSockets,
  activeRunewordId,
  onApply,
}: {
  base: ItemBase
  maxSockets: number
  activeRunewordId?: string
  onApply: (runewordId: string) => void
}) {
  const compatible = useMemo(() => {
    if (base.rarity !== 'common') return []
    return runewords.filter(
      (rw) =>
        rw.allowedBaseTypes.includes(base.baseType) &&
        rw.runes.length <= maxSockets,
    )
  }, [base, maxSockets])

  const [open, setOpen] = useState(false)

  const pickerRows = useMemo<PickerRow[]>(
    () =>
      compatible.map((rw) => {
        const runeSeq = rw.runes
          .map((r) => r.replace(/^rune_/, ''))
          .join(' → ')
        const reqLabel = rw.requiresLevel ? ` · L${rw.requiresLevel}` : ''
        return {
          id: rw.id,
          name: rw.name,
          tier: rw.runes.length,
          kindLabel: 'RUNEWORD',
          meta: `${runeSeq}${reqLabel}`,
          iconColor: 'var(--color-accent-hot)',
          searchTerms: [
            rw.name,
            runeSeq,
            ...rw.runes,
            ...rw.allowedBaseTypes,
          ]
            .join(' ')
            .toLowerCase(),
          tooltip: buildRunewordTooltip(rw),
          tooltipTone: 'rare' as const,
        }
      }),
    [compatible],
  )

  if (compatible.length === 0) return null

  const activeRw = activeRunewordId
    ? compatible.find((rw) => rw.id === activeRunewordId)
    : undefined
  const activeRuneSeq = activeRw
    ? activeRw.runes.map((r) => r.replace(/^rune_/, '')).join(' → ')
    : null

  const renderSelectedPanel = (state: PickerPanelState): ReactNode => {
    if (!state.selectedId) return null
    const sel = compatible.find((rw) => rw.id === state.selectedId)
    if (!sel) return null
    const hovered =
      state.hoveredId && state.hoveredId !== state.selectedId
        ? compatible.find((rw) => rw.id === state.hoveredId)
        : undefined
    return (
      <TooltipPanel className="w-full" tone="rare">
        {buildRunewordTooltip(sel)}
        {hovered && (
          <NetChangeBlock previous={sel.stats} next={hovered.stats} />
        )}
      </TooltipPanel>
    )
  }

  return (
    <SectionCard
      label="Runeword Presets"
      rightSlot={
        <span className="font-mono text-[10px] tabular-nums text-faint">
          {compatible.length} compatible
        </span>
      }
    >
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="group flex w-full items-center justify-between gap-2 rounded-[3px] border border-accent-deep/30 px-3 py-2 text-left transition-all hover:border-accent/50 hover:shadow-[0_0_12px_rgba(224,184,100,0.15)]"
        style={{
          background:
            'linear-gradient(180deg, rgba(28,29,36,0.7), rgba(13,14,18,0.5))',
        }}
      >
        <span className="flex min-w-0 flex-col">
          <span
            className={`truncate text-[13px] font-medium ${
              activeRw ? 'text-accent-hot' : 'italic text-faint'
            }`}
          >
            {activeRw ? activeRw.name : 'Choose runeword…'}
          </span>
          {activeRuneSeq && (
            <span className="truncate font-mono text-[10px] tabular-nums tracking-[0.04em] text-muted/80">
              {activeRuneSeq}
            </span>
          )}
        </span>
        <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint group-hover:text-accent-hot">
          Browse →
        </span>
      </button>

      {open && (
        <PickerModal
          title="Pick Runeword"
          sectionLabel="Runeword"
          rows={pickerRows}
          selectedId={activeRunewordId ?? null}
          searchPlaceholder="Search runewords…"
          emptyMessage="No matching runewords"
          width={680}
          selectedPanel={renderSelectedPanel}
          onSelect={(id) => onApply(id)}
          onClose={() => setOpen(false)}
        />
      )}
    </SectionCard>
  )
}
