import { useEffect, useMemo, useState } from 'react'
import { createPortal } from 'react-dom'
import ItemTextEditorModal from '../../components/ItemTextEditorModal'
import type { PickerRow } from '../../components/PickerModal'
import { detectRuneword, forgeKindFor, getItem, getItemSet, isGearSlot } from '../../data'
import { maxSocketsFor, useBuild } from '../../store/build'
import { useBuildPerformanceDeps } from '../../hooks/useBuildPerformanceDeps'
import type { EquippedItem, Inventory, SlotKey, SocketType } from '../../types'
import { useSetHoverPreview } from '../../contexts/HoverContext'
import { type BuildSummaryDeps } from './lib/diff'
import { pickerItemsForSlot } from './pickerItems'
import { ConfigEmptyState, ConfigSectionHeader, PickerCornerMarks, SectionCard } from './SectionCard'
import { CompareColumn } from './CompareColumn'
import { GearItemRow } from './GearItemRow'
import { AffixesSection } from './sections/AffixesSection'
import { AugmentSection } from './sections/AugmentSection'
import { ForgedModsSection } from './sections/ForgedModsSection'
import { RunewordPresets } from './sections/RunewordPresets'
import { SocketsSection } from './sections/SocketsSection'
import { StarsSection } from './sections/StarsSection'
import { RARITY_LABEL } from './lib/rarity'

interface GearSlotModalProps {
  slot: SlotKey
  slotName: string
  equipped: EquippedItem | undefined
  offhandLocked: boolean
  socketPickerRows: PickerRow[]
  onEquip: (id: string) => void
  onUnequip: () => void
  onSocketCount: (n: number) => void
  onSocketed: (idx: number, id: string | null) => void
  onSocketType: (idx: number, type: SocketType) => void
  onSetStars: (n: number) => void
  onAddAffix: (affixId: string, tier: number) => void
  onRemoveAffix: (idx: number) => void
  onAddForgedMod: (modId: string, tier: number) => void
  onRemoveForgedMod: (idx: number) => void
  onApplyRuneword: (rwId: string) => void
  onClose: () => void
}

export function GearSlotModal({
  slot,
  slotName,
  equipped,
  offhandLocked,
  socketPickerRows,
  onEquip,
  onUnequip,
  onSocketCount,
  onSocketed,
  onSocketType,
  onSetStars,
  onAddAffix,
  onRemoveAffix,
  onAddForgedMod,
  onRemoveForgedMod,
  onApplyRuneword,
  onClose,
}: GearSlotModalProps) {
  // Full-screen slot editor that replaces the old aside EditPanel. The left column lists every item that fits this slot (grouped by rarity, searchable by name AND affix/effect text); the right column shows the live configuration of the equipped item (sockets, stars, affixes, forge mods, augment, runeword presets) or — when hovering a different item — a side-by-side compare overlay.
  const [q, setQ] = useState('')
  const [showCompareCol, setShowCompareCol] = useState(true)
  const [textEditorOpen, setTextEditorOpen] = useState(false)
  const replaceEquippedItem = useBuild((s) => s.replaceEquippedItem)
  const rows = useMemo(() => pickerItemsForSlot(slot), [slot])
  const inv = useBuild((s) => s.inventory)
  // Phase A hover wiring: report which picker row the cursor is over so
  // the LeftStatsPanel (phase B) can paint a live delta against the build.
  // Clears on unmount so the sidebar doesn't stay stuck on the last hover
  // after the modal closes.
  const setHover = useSetHoverPreview()
  useEffect(() => () => setHover(null), [setHover])

  // Frozen at modal open so the compare column diffs live edits against the original equipped state.
  const [baselineEquipped] = useState<EquippedItem | null>(() =>
    equipped ? structuredClone(equipped) : null,
  )
  const baselineInventory = useMemo<Inventory>(
    () => ({ ...inv, [slot]: baselineEquipped ?? undefined }),
    [inv, baselineEquipped, slot],
  )
  const fullDeps = useBuildPerformanceDeps()
  const compareDeps = useMemo<BuildSummaryDeps>(
    () => {
      const { inventory: _, ...rest } = fullDeps
      return rest
    },
    [fullDeps],
  )

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])

  const filter = q.trim().toLowerCase()
  const filteredRows = useMemo(() => {
    if (!filter) return rows
    return rows.filter((r) => {
      if (r.name.toLowerCase().includes(filter)) return true
      if (typeof r.meta === 'string' && r.meta.toLowerCase().includes(filter))
        return true
      if (r.kindLabel?.toLowerCase().includes(filter)) return true
      if (r.group?.toLowerCase().includes(filter)) return true
      return r.searchTerms?.includes(filter) ?? false
    })
  }, [rows, filter])

  const groupedRows = useMemo(() => {
    const out: { group: string | null; rows: PickerRow[] }[] = []
    const idx = new Map<string, number>()
    for (const r of filteredRows) {
      const g = r.group ?? null
      const key = g ?? '__none__'
      let pos = idx.get(key)
      if (pos === undefined) {
        pos = out.length
        idx.set(key, pos)
        out.push({ group: g, rows: [] })
      }
      out[pos]!.rows.push(r)
    }
    return out
  }, [filteredRows])

  const hasGroupHeaders = groupedRows.some((g) => g.group !== null)

  const base = equipped ? getItem(equipped.baseId) : undefined
  const maxSockets = equipped
    ? maxSocketsFor(equipped.baseId, equipped.forgedMods)
    : 0
  const set = base?.setId ? getItemSet(base.setId) : undefined
  const setEquippedCount = base?.setId
    ? Object.values(inv).reduce((acc, eq) => {
        if (!eq) return acc
        const b = getItem(eq.baseId)
        return b?.setId === base.setId ? acc + 1 : acc
      }, 0)
    : 0
  const forgeKind =
    base && isGearSlot(slot) ? forgeKindFor(base.rarity) : null

  const isOffhandLocked = slot === 'offhand' && offhandLocked && !equipped

  return (
    <>
      {createPortal(
    <div
      role="presentation"
      className="fixed inset-0 z-100 flex items-center justify-center bg-black/70"
      onMouseDown={onClose}
      style={{
        background:
          'radial-gradient(ellipse at 50% 0%, rgba(201,165,90,0.06), rgba(0,0,0,0.78) 60%)',
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        onMouseDown={(e) => e.stopPropagation()}
        className={`relative flex h-[88vh] ${showCompareCol && !isOffhandLocked ? 'w-390' : 'w-295'} max-w-[96vw] flex-col overflow-hidden rounded-md border border-border`}
        style={{
          background:
            'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 80%, transparent))',
          boxShadow:
            'inset 0 1px 0 rgba(255,255,255,0.02), 0 24px 64px rgba(0,0,0,0.7)',
        }}
      >
        <PickerCornerMarks />

        <header
          className="flex items-start justify-between gap-3 border-b border-border px-5 py-4"
          style={{
            background:
              'linear-gradient(180deg, rgba(201,165,90,0.05), transparent)',
          }}
        >
          <div>
            <div className="mb-1 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
              <span
                className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
                style={{ boxShadow: '0 0 8px rgba(224,184,100,0.6)' }}
              />
              Gear Slot
            </div>
            <h2
              className="m-0 text-[18px] font-semibold tracking-[0.02em] text-accent-hot"
              style={{ textShadow: '0 0 16px rgba(224,184,100,0.15)' }}
            >
              {equipped && base ? (
                <>
                  {slotName}
                </>
              ) : (
                slotName
              )}
            </h2>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => setShowCompareCol((v) => !v)}
              aria-pressed={showCompareCol}
              className={`rounded-[3px] border px-3.5 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] transition-colors ${
                showCompareCol
                  ? 'border-accent-deep bg-accent-hot/8 text-accent-hot'
                  : 'border-border-2 bg-transparent text-muted hover:border-accent-deep hover:text-accent-hot'
              }`}
            >
              Compare
            </button>
            {equipped && base && (
              <button
                onClick={() => setTextEditorOpen(true)}
                className="rounded-[3px] border border-border-2 bg-transparent px-3.5 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
              >
                Edit Text
              </button>
            )}
            {equipped && (
              <button
                onClick={onUnequip}
                className="rounded-[3px] border border-border-2 bg-transparent px-3.5 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-stat-red hover:text-stat-red"
              >
                Remove
              </button>
            )}
            <button
              onClick={onClose}
              className="rounded-[3px] border border-border-2 bg-panel-2 px-3.5 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
            >
              Close
            </button>
          </div>
        </header>

        <div className="flex min-h-0 flex-1 flex-row">
          <div className="flex w-140 min-w-0 shrink-0 flex-col border-r border-border">
            <div className="border-b border-border px-4 py-3">
              <div className="relative">
                <svg
                  className="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-faint"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth={2}
                >
                  <circle cx="11" cy="11" r="7" />
                  <path d="m20 20-3.5-3.5" />
                </svg>
                <input
                  autoFocus
                  value={q}
                  onChange={(e) => setQ(e.target.value)}
                  placeholder="Search by name, affix, or effect…"
                  className="w-full rounded-[3px] border border-border-2 px-3 py-2 pl-9 text-text placeholder:text-faint focus:border-accent-deep focus:outline-none focus:ring-2 focus:ring-accent-hot/15"
                  style={{
                    background:
                      'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
                    boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
                  }}
                />
              </div>
            </div>

            <div
              className="min-h-0 flex-1 overflow-y-auto"
              onMouseLeave={() => setHover(null)}
            >
              {isOffhandLocked ? (
                <div className="m-4 rounded border border-amber-500/30 bg-amber-500/5 px-3 py-3 text-[11px] text-amber-200">
                  Offhand is locked while a Two-Handed weapon is in the main
                  hand. Remove the weapon to free this slot.
                </div>
              ) : filteredRows.length === 0 ? (
                <div className="p-8 text-center text-sm text-muted">
                  No items match
                </div>
              ) : (
                groupedRows.map((g, gi) => (
                  <div key={g.group ?? `__${gi}`}>
                    {hasGroupHeaders && g.group && (
                      <div
                        className="sticky top-0 z-1 flex items-center gap-2 border-b border-accent-deep/30 px-4 py-1 font-mono text-[10px] uppercase tracking-[0.18em] text-accent-hot/70"
                        style={{
                          background: 'var(--color-panel-2)',
                        }}
                      >
                        <span
                          className="inline-block h-1 w-1 rotate-45 bg-accent-deep"
                          aria-hidden="true"
                        />
                        {g.group}
                      </div>
                    )}
                    {g.rows.map((r) => (
                      <GearItemRow
                        key={r.id}
                        row={r}
                        selected={r.id === equipped?.baseId}
                        onSelect={() => onEquip(r.id)}
                        onHover={() =>
                          setHover({ kind: 'gear', slot, baseId: r.id })
                        }
                      />
                    ))}
                  </div>
                ))
              )}
            </div>
          </div>

          <div className="flex min-w-0 flex-1 flex-col overflow-hidden">
            {isOffhandLocked ? (
              <ConfigEmptyState
                title="Slot locked"
                hint="Remove the 2H weapon to enable this slot."
                tone="warning"
              />
            ) : equipped && base ? (
              <div className="flex min-h-0 flex-1 flex-col overflow-y-auto">
                <ConfigSectionHeader label="Item Configuration" />
                <div className="space-y-3 p-4">
                  {set && set.bonuses.length > 0 && (
                    <SetSummary set={set} count={setEquippedCount} />
                  )}

                  <RunewordPresets
                    base={base}
                    maxSockets={maxSockets}
                    activeRunewordId={
                      detectRuneword(base, equipped.socketed)?.id
                    }
                    onApply={onApplyRuneword}
                  />

                  <SocketsSection
                    equipped={equipped}
                    maxSockets={maxSockets}
                    base={base}
                    socketPickerRows={socketPickerRows}
                    onSocketCount={onSocketCount}
                    onSocketed={onSocketed}
                    onSocketType={onSocketType}
                  />

                  {isGearSlot(slot) && (
                    <StarsSection
                      stars={equipped.stars ?? 0}
                      onChange={onSetStars}
                    />
                  )}

                  {(base.rarity === 'common' || base.randomAffixGroupId) && (
                    <AffixesSection
                      equipped={equipped}
                      base={base}
                      maxAffixes={base.maxAffixes}
                      onAdd={onAddAffix}
                      onRemove={onRemoveAffix}
                    />
                  )}

                  {forgeKind && (
                    <ForgedModsSection
                      forgeKind={forgeKind}
                      equipped={equipped}
                      onAdd={onAddForgedMod}
                      onRemove={onRemoveForgedMod}
                    />
                  )}

                  {slot === 'armor' && <AugmentSection equipped={equipped} />}
                </div>
              </div>
            ) : (
              <ConfigEmptyState
                title="No item equipped"
                hint="Pick an item from the left to start configuring."
              />
            )}
          </div>

          {showCompareCol && !isOffhandLocked && (
            <CompareColumn
              baselineInventory={baselineInventory}
              currentInventory={inv}
              baselineEquipped={baselineEquipped}
              currentEquipped={equipped}
              slot={slot}
              deps={compareDeps}
            />
          )}
        </div>

        <footer className="flex items-center justify-between gap-3 border-t border-border bg-black/30 px-4 py-3">
          <div
            className={`flex min-w-0 flex-1 items-center gap-2 font-mono text-[11px] tracking-[0.06em] ${
              equipped ? 'text-accent-hot' : 'text-faint'
            }`}
          >
            <span
              className="h-1.5 w-1.5 shrink-0 rounded-full"
              style={{
                background: equipped
                  ? 'var(--color-accent-hot)'
                  : 'var(--color-faint)',
                boxShadow: equipped
                  ? '0 0 8px rgba(224,184,100,0.6)'
                  : '0 0 6px var(--color-faint)',
              }}
            />
            <span className="truncate">
              {equipped && base
                ? `${base.name} · ${RARITY_LABEL[base.rarity]}`
                : 'Empty slot'}
            </span>
          </div>
          <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
            {filteredRows.length} item{filteredRows.length === 1 ? '' : 's'}
          </span>
        </footer>
      </div>
    </div>,
        document.body,
      )}
      {textEditorOpen && equipped && base && (
        <ItemTextEditorModal
          slot={slot}
          slotName={slotName}
          equipped={equipped}
          base={base}
          onSave={(next) => replaceEquippedItem(slot, next)}
          onClose={() => setTextEditorOpen(false)}
        />
      )}
    </>
  )
}

function SetSummary({
  set,
  count,
}: {
  set: NonNullable<ReturnType<typeof getItemSet>>
  count: number
}) {
  // Renders a compact summary card for an item set inside the GearSlotModal right column showing the set name, equipped-count badge, and each tier with its description coloured by activation state. Wrapped in the shared SectionCard so its visual language matches the rest of the popup. Used when the equipped item belongs to a set.
  return (
    <SectionCard
      label={set.name}
      tone="set"
      rightSlot={
        <span className="font-mono text-[10px] tabular-nums text-green-300/80">
          {count}/{set.items.length} pieces
        </span>
      }
      bodyClassName="px-3.5 py-2.5"
    >
      <ul className="space-y-1.5">
        {set.bonuses.map((bonus, idx) => {
          const active = count >= bonus.pieces
          return (
            <li
              key={idx}
              className={`text-[11px] ${active ? 'text-green-200' : 'text-muted/60'}`}
            >
              <div className="flex items-baseline gap-2">
                <span
                  className={`font-mono text-[9px] uppercase tracking-[0.14em] ${
                    active ? 'text-green-300' : 'text-faint'
                  }`}
                >
                  {bonus.pieces}-Set
                </span>
                {active && (
                  <span className="font-mono text-[10px] text-green-300">
                    ✓
                  </span>
                )}
              </div>
              {(bonus.descriptions ?? []).map((d, i) => (
                <div
                  key={i}
                  className={`ml-3 text-[10.5px] leading-snug ${
                    active ? 'text-green-200/90' : 'text-muted/55'
                  }`}
                >
                  {d}
                </div>
              ))}
            </li>
          )
        })}
      </ul>
    </SectionCard>
  )
}
