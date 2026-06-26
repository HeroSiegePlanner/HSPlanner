import { useEffect, useMemo, useState } from 'react'
import ItemTextEditorModal from '../../components/ItemTextEditorModal'
import type { PickerRow } from '../../components/PickerModal'
import { activeSeasonId, canStarForge, detectRuneword, forgeKindFor, getItem, getItemSet } from '../../data'
import { maxSocketsFor, useBuild } from '../../store/build'
import { useBuildPerformanceDeps } from '../../hooks/useBuildPerformanceDeps'
import type { EquippedItem, Inventory, SlotKey } from '../../types'
import { useSetHoverPreview } from '../../contexts/HoverContext'
import { type BuildSummaryDeps } from './lib/diff'
import { Modal } from '../../components/Modal'
import { SectionCard } from './SectionCard'
import { CompareColumn } from './CompareColumn'
import { ItemListRail } from './ItemListRail'
import { AffixesSection } from './sections/AffixesSection'
import { AugmentSection } from './sections/AugmentSection'
import { ForgedModsSection } from './sections/ForgedModsSection'
import { RunewordPresets } from './sections/RunewordPresets'
import { SocketsSection } from './sections/SocketsSection'
import { StarsSection } from './sections/StarsSection'
import { RARITY_LABEL, RARITY_TEXT } from './lib/rarity'
import { useGearDraft } from './lib/useGearDraft'

const GHOST_BTN =
  'rounded-md border border-border bg-transparent px-3 py-1.5 text-[12px] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot'
const GHOST_BTN_RED =
  'rounded-md border border-border bg-transparent px-3 py-1.5 text-[12px] text-muted transition-colors hover:border-stat-red hover:text-stat-red'
const PRIMARY_BTN =
  'rounded-md border border-accent-deep bg-accent-hot/10 px-4 py-1.5 text-[12px] font-medium text-accent-hot transition-colors hover:border-accent-hot hover:bg-accent-hot/15 disabled:cursor-not-allowed disabled:border-border disabled:bg-transparent disabled:text-faint'

interface GearSlotModalProps {
  slot: SlotKey
  slotName: string
  equipped: EquippedItem | undefined
  offhandLocked: boolean
  socketPickerRows: PickerRow[]
  onCommit: (item: EquippedItem | null) => string | null
  onClose: () => void
}

export function GearSlotModal({
  slot,
  slotName,
  equipped,
  offhandLocked,
  socketPickerRows,
  onCommit,
  onClose,
}: GearSlotModalProps) {
  const inv = useBuild((s) => s.inventory)
  const setHover = useSetHoverPreview()
  useEffect(() => () => setHover(null), [setHover])

  const d = useGearDraft(equipped)
  const { draft, baselineEquipped, dirty } = d

  const isOffhandLocked = slot === 'offhand' && offhandLocked && !draft

  // Two-step flow: pick an item, then configure it. Open straight into config
  // when the slot already holds an item.
  const [step, setStep] = useState<'select' | 'configure'>(() =>
    equipped ? 'configure' : 'select',
  )
  const [textEditorOpen, setTextEditorOpen] = useState(false)
  const [confirmingClose, setConfirmingClose] = useState(false)
  const [commitError, setCommitError] = useState<string | null>(null)

  const requestClose = () => {
    if (dirty) {
      setConfirmingClose(true)
      return
    }
    onClose()
  }

  const handleSave = () => {
    const err = onCommit(draft ?? null)
    if (err) {
      setCommitError(err)
      setConfirmingClose(false)
      return
    }
    onClose()
  }

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key !== 'Escape') return
      if (confirmingClose) {
        setConfirmingClose(false)
        return
      }
      if (dirty) {
        setConfirmingClose(true)
        return
      }
      onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [confirmingClose, dirty, onClose])

  const base = draft ? getItem(draft.baseId) : undefined
  const maxSockets = draft ? maxSocketsFor(draft.baseId, draft.forgedMods) : 0
  const set = base?.setId ? getItemSet(base.setId) : undefined
  const setEquippedCount = base?.setId
    ? Object.entries(inv).reduce((acc, [k, eq]) => {
        const item = k === slot ? draft : eq
        if (!item) return acc
        const b = getItem(item.baseId)
        return b?.setId === base.setId ? acc + 1 : acc
      }, 0)
    : 0
  const forgeKind = base && canStarForge(slot, activeSeasonId) ? forgeKindFor(base.rarity) : null

  const fullDeps = useBuildPerformanceDeps()
  const compareDeps = useMemo<BuildSummaryDeps>(() => {
    const { inventory: _drop, ...rest } = fullDeps
    void _drop
    return rest
  }, [fullDeps])
  const baselineInventory = useMemo<Inventory>(
    () => ({ ...inv, [slot]: baselineEquipped ?? undefined }),
    [inv, baselineEquipped, slot],
  )
  const currentInventory = useMemo<Inventory>(
    () => ({ ...inv, [slot]: draft ?? undefined }),
    [inv, draft, slot],
  )

  const configuring = step === 'configure' && !isOffhandLocked

  const footerLabel = isOffhandLocked
    ? 'Slot locked'
    : step === 'select'
      ? `Choose an item for ${slotName}`
      : draft && base
        ? `${base.name} · ${RARITY_LABEL[base.rarity]}`
        : 'Empty slot'

  let body
  if (isOffhandLocked) {
    body = (
      <div className="flex min-h-0 flex-1 items-center justify-center p-8">
        <div className="max-w-sm rounded-lg border border-amber-500/30 bg-amber-500/5 px-4 py-3 text-center text-[12px] leading-relaxed text-amber-200">
          This slot is locked while a Two-Handed weapon is equipped. Remove the
          weapon to free the offhand.
        </div>
      </div>
    )
  } else if (step === 'select') {
    body = (
      <ItemListRail
        slot={slot}
        selectedBaseId={draft?.baseId}
        isOffhandLocked={isOffhandLocked}
        onSelect={(id) => {
          d.pickBase(id)
          setCommitError(null)
          setStep('configure')
        }}
        onHoverBase={(baseId) =>
          setHover(baseId ? { kind: 'gear', slot, baseId } : null)
        }
      />
    )
  } else {
    body = (
      <div className="flex min-h-0 flex-1 flex-row">
        <div className="flex min-w-0 flex-1 flex-col overflow-hidden">
          {draft && base ? (
            <div className="flex min-h-0 flex-1 flex-col overflow-y-auto">
              <div
                className="sticky top-0 z-1 flex items-center justify-between gap-3 border-b border-border px-5 py-3"
                style={{ background: 'var(--color-panel-2)' }}
              >
                <div className="min-w-0">
                  <div
                    className={`truncate text-[14px] font-semibold ${RARITY_TEXT[base.rarity]}`}
                  >
                    {base.name}
                  </div>
                  <div className="text-[11px] text-muted">
                    {RARITY_LABEL[base.rarity]}
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => setStep('select')}
                  className={`shrink-0 ${GHOST_BTN}`}
                >
                  ← Change item
                </button>
              </div>

              <div className="space-y-4 p-5">
                {set && set.bonuses.length > 0 && (
                  <SetSummary set={set} count={setEquippedCount} />
                )}

                <SocketsSection
                  equipped={draft}
                  maxSockets={maxSockets}
                  base={base}
                  socketPickerRows={socketPickerRows}
                  onSocketCount={d.setSocketCount}
                  onSocketed={d.setSocketed}
                  onSocketType={d.setSocketType}
                />

                <div className="columns-2 gap-4 [&>*]:mb-4 [&>*]:break-inside-avoid">
                  <RunewordPresets
                    base={base}
                    maxSockets={maxSockets}
                    activeRunewordId={detectRuneword(base, draft.socketed)?.id}
                    onApply={d.applyRuneword}
                  />

                  {canStarForge(slot, activeSeasonId) && (
                    <StarsSection stars={draft.stars ?? 0} onChange={d.setStars} />
                  )}

                  {(base.rarity === 'common' || base.randomAffixGroupId) && (
                    <AffixesSection
                      equipped={draft}
                      base={base}
                      maxAffixes={base.maxAffixes}
                      onAdd={d.addAffix}
                      onRemove={d.removeAffix}
                    />
                  )}

                  {forgeKind && (
                    <ForgedModsSection
                      forgeKind={forgeKind}
                      equipped={draft}
                      onAdd={d.addForgedMod}
                      onRemove={d.removeForgedMod}
                    />
                  )}
                </div>

                {slot === 'armor' && (
                  <AugmentSection
                    equipped={draft}
                    onSetAugment={d.setAugment}
                    onSetAugmentLevel={d.setAugmentLevel}
                  />
                )}
              </div>
            </div>
          ) : (
            <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-5 p-8 text-center">
              <div>
                <div className="mb-1.5 text-[14px] font-semibold text-text">
                  No item selected
                </div>
                <div className="text-[12px] text-muted">
                  This slot will be emptied when you Save.
                </div>
              </div>
              <button
                type="button"
                onClick={() => setStep('select')}
                className="rounded-md border border-accent-deep bg-accent-hot/10 px-4 py-2 text-[12px] font-medium text-accent-hot transition-colors hover:border-accent-hot hover:bg-accent-hot/15"
              >
                Choose an item
              </button>
            </div>
          )}
        </div>

        <CompareColumn
          baselineInventory={baselineInventory}
          currentInventory={currentInventory}
          baselineEquipped={baselineEquipped}
          currentEquipped={draft ?? undefined}
          slot={slot}
          deps={compareDeps}
        />
      </div>
    )
  }

  return (
    <>
      <Modal
        onClose={requestClose}
        eyebrow="Gear Slot"
        title={slotName}
        panelClassName={`h-[88vh] max-w-[96vw] transition-[width] duration-300 ${
          configuring ? 'w-[1180px]' : 'w-[680px]'
        }`}
        headerActions={
          step === 'configure' && draft ? (
            <>
              {base && (
                <button
                  type="button"
                  onClick={() => setTextEditorOpen(true)}
                  className={GHOST_BTN}
                >
                  Edit Text
                </button>
              )}
              <button
                type="button"
                onClick={() => d.clearDraft()}
                className={GHOST_BTN_RED}
              >
                Remove
              </button>
            </>
          ) : null
        }
      >
        {body}

        {commitError && (
          <div className="border-t border-stat-red/30 bg-stat-red/8 px-5 py-2.5 text-[12px] text-stat-red">
            {commitError}
          </div>
        )}

        {confirmingClose && (
          <div className="flex items-center justify-between gap-3 border-t border-amber-500/30 bg-amber-500/8 px-5 py-3">
            <span className="text-[12px] text-amber-200">
              You have unsaved changes
            </span>
            <div className="flex shrink-0 gap-2">
              <button
                type="button"
                onClick={() => setConfirmingClose(false)}
                className={GHOST_BTN}
              >
                Keep Editing
              </button>
              <button type="button" onClick={onClose} className={GHOST_BTN_RED}>
                Discard
              </button>
              <button
                type="button"
                onClick={handleSave}
                className="rounded-md border border-accent-deep bg-accent-hot/10 px-3 py-1.5 text-[12px] font-medium text-accent-hot transition-colors hover:border-accent-hot hover:bg-accent-hot/15"
              >
                Save
              </button>
            </div>
          </div>
        )}

        <footer className="flex items-center justify-between gap-3 border-t border-border bg-black/20 px-5 py-3">
          <div
            className={`flex min-w-0 flex-1 items-center gap-2 text-[12px] ${
              configuring && draft ? 'text-text' : 'text-faint'
            }`}
          >
            <span
              className="h-1.5 w-1.5 shrink-0 rounded-full"
              style={{
                background: draft ? 'var(--color-accent)' : 'var(--color-faint)',
              }}
            />
            <span className="truncate">{footerLabel}</span>
            {dirty && (
              <span className="ml-2 rounded border border-amber-400/40 px-1.5 py-0.5 text-[10px] text-amber-300">
                Unsaved
              </span>
            )}
          </div>
          <div className="flex shrink-0 gap-2">
            {step === 'select' && draft && (
              <button
                type="button"
                onClick={() => setStep('configure')}
                className={GHOST_BTN}
              >
                ← Back
              </button>
            )}
            <button type="button" onClick={requestClose} className={GHOST_BTN}>
              Cancel
            </button>
            {step === 'configure' && (
              <button
                type="button"
                onClick={handleSave}
                disabled={!dirty}
                className={PRIMARY_BTN}
              >
                Save / Equip
              </button>
            )}
          </div>
        </footer>
      </Modal>
      {textEditorOpen && draft && base && (
        <ItemTextEditorModal
          slot={slot}
          slotName={slotName}
          equipped={draft}
          base={base}
          onSave={(next) => d.replaceDraft(next)}
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
                  <span className="font-mono text-[10px] text-green-300">✓</span>
                )}
              </div>
              {(bonus.descriptions ?? []).map((dsc, i) => (
                <div
                  key={i}
                  className={`ml-3 text-[10.5px] leading-snug ${
                    active ? 'text-green-200/90' : 'text-muted/55'
                  }`}
                >
                  {dsc}
                </div>
              ))}
            </li>
          )
        })}
      </ul>
    </SectionCard>
  )
}
