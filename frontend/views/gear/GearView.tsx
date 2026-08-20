import { useCallback, useMemo, useState } from 'react'
import type { PickerRow } from './PickerModal'
import { gameConfig, gems, getItem, items, runes } from '@data'
import { useBuild } from '../../store/build'
import { useSettings } from '../../store/settings'
import type { ItemBase, SlotKey } from '../../types'
import { canOffhand, isOffhandLocked } from '../../utils/tree/dualWield'
import { charmBlockedCells, packCharms } from './lib/charmPacking'
import { getSocketPickerRows } from './lib/socketPickerRows'
import { CharmSection } from './CharmSection'
import { EquipmentDoll } from './EquipmentDoll'
import { GearSlotModal } from './GearSlotModal'
import { StashSection } from './StashSection'
import { UpgradeAdvisor } from './UpgradeAdvisor'

export default function GearView() {
  const inventory = useBuild((s) => s.inventory)
  const commitEquippedItem = useBuild((s) => s.commitEquippedItem)
  const allocatedTreeNodes = useBuild((s) => s.allocatedTreeNodes)

  const [activeSlot, setActiveSlot] = useState<SlotKey | null>(null)
  const handleModalClose = useCallback(() => setActiveSlot(null), [])

  const charmFits = (slot: SlotKey, baseId: string): boolean => {
    const base = getItem(baseId)
    if (!base) return false
    const newW = base.width ?? 1
    const newH = base.height ?? 1
    const others: { slotKey: SlotKey; w: number; h: number }[] = []
    for (const cs of charmSlots) {
      if (cs.key === slot) continue
      const eq = inventory[cs.key]
      if (!eq) continue
      const b = getItem(eq.baseId)
      others.push({ slotKey: cs.key, w: b?.width ?? 1, h: b?.height ?? 1 })
    }
    const candidate = { slotKey: slot, w: newW, h: newH }
    const blockedCells = charmBlockedCells(useSettings.getState().extraCharmSlot)
    const before = packCharms(others, blockedCells)
    const after = packCharms([...others, candidate], blockedCells)
    const candidateOverflowed = after.overflow.includes(slot)
    const overflowGrew = after.overflow.length > before.overflow.length
    return !(candidateOverflowed || overflowGrew)
  }

  const socketPickerRows: PickerRow[] = getSocketPickerRows()

  const charmSlots = gameConfig.slots.filter((s) => s.key.startsWith('charm_'))

  const weaponBaseId = inventory.weapon?.baseId
  const weaponBase = weaponBaseId ? getItem(weaponBaseId) : undefined
  const offhandAccepts = useMemo(
    () => (base: ItemBase) => canOffhand(base, weaponBase, allocatedTreeNodes),
    [weaponBase, allocatedTreeNodes],
  )
  const offhandLocked = isOffhandLocked(weaponBase, allocatedTreeNodes)

  return (
    <div className="w-full">
      <header className="mb-4">
        <div className="mb-1 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
          <span
            aria-hidden
            className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
            style={{ boxShadow: '0 0 8px rgba(224,184,100,0.6)' }}
          />
          Loadout
        </div>
        <div className="flex items-end justify-between gap-3">
          <h2
            className="m-0 text-[22px] font-semibold tracking-[0.02em] text-accent-hot"
            style={{ textShadow: '0 0 16px rgba(224,184,100,0.18)' }}
          >
            Gear
          </h2>
          <div className="flex items-center gap-3 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
            <span>
              <span className="text-text">{items.length}</span> items
            </span>
            <span aria-hidden className="h-3 w-px bg-border" />
            <span>
              <span className="text-text">{gems.length}</span> gems
            </span>
            <span aria-hidden className="h-3 w-px bg-border" />
            <span>
              <span className="text-text">{runes.length}</span> runes
            </span>
          </div>
        </div>
      </header>

      <UpgradeAdvisor onPickSlot={(slot) => setActiveSlot(slot)} />

      <div className="flex flex-wrap items-start justify-center gap-4 lg:justify-start">
        <EquipmentDoll
          activeSlot={activeSlot}
          offhandLocked={offhandLocked}
          onSelect={(s) => setActiveSlot(s)}
        />

        <CharmSection
          charmSlots={charmSlots}
          activeSlot={activeSlot}
          onSelect={(s) => setActiveSlot(s)}
        />

        <div data-tour="gear-stash" className="min-w-[280px] flex-1">
          <StashSection />
        </div>
      </div>

      {activeSlot && (
        <GearSlotModal
          slot={activeSlot}
          slotName={
            gameConfig.slots.find((s) => s.key === activeSlot)?.name ?? activeSlot
          }
          equipped={inventory[activeSlot]}
          offhandLocked={offhandLocked}
          offhandAccepts={offhandAccepts}
          socketPickerRows={socketPickerRows}
          onCommit={(item) => {
            if (
              item &&
              activeSlot.startsWith('charm_') &&
              !charmFits(activeSlot, item.baseId)
            ) {
              const base = getItem(item.baseId)
              const w = base?.width ?? 1
              const h = base?.height ?? 1
              return `${base?.name ?? 'Item'} (${w}×${h}) won't fit — free up space first.`
            }
            commitEquippedItem(activeSlot, item)
            if (item) useBuild.getState().addStashItem(item)
            return null
          }}
          onClose={handleModalClose}
        />
      )}
    </div>
  )
}
