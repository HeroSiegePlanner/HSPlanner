import { useEffect, useMemo, useRef, useState } from 'react'
import type { PickerRow } from '../../components/PickerModal'
import { gameConfig, gems, getItem, items, runes } from '../../data'
import { useBuild } from '../../store/build'
import { fmtStats } from '../../utils/item/stats'
import type { SlotKey } from '../../types'
import { packCharms } from './lib/charmPacking'
import { gemColorForName, socketableIconForName } from './lib/icons'
import { buildSocketableTooltip } from './tooltips'
import { CharmSection } from './CharmSection'
import { GearPanel, SlotRow } from './SlotRail'
import { GearSlotModal } from './GearSlotModal'

export default function GearView() {
  // Gear management view: shows every equipment slot, the relic/charm grid (with backtracking-based packing) and the per-slot edit panel for picking items, applying runewords, choosing sockets/gems/runes, setting stars, editing affix rolls, applying forge mods, and equipping angelic augments. The main inventory editor.
  const inventory = useBuild((s) => s.inventory)
  const equipItem = useBuild((s) => s.equipItem)
  const unequipItem = useBuild((s) => s.unequipItem)
  const setSocketCount = useBuild((s) => s.setSocketCount)
  const setSocketed = useBuild((s) => s.setSocketed)
  const setSocketType = useBuild((s) => s.setSocketType)
  const setStars = useBuild((s) => s.setStars)
  const addAffix = useBuild((s) => s.addAffix)
  const removeAffix = useBuild((s) => s.removeAffix)
  const addForgedMod = useBuild((s) => s.addForgedMod)
  const removeForgedMod = useBuild((s) => s.removeForgedMod)
  const applyRuneword = useBuild((s) => s.applyRuneword)

  const [activeSlot, setActiveSlot] = useState<SlotKey | null>(null)
  const [charmFitError, setCharmFitError] = useState<string | null>(null)
  const charmErrorTimer = useRef<number | null>(null)
  useEffect(
    () => () => {
      if (charmErrorTimer.current !== null)
        window.clearTimeout(charmErrorTimer.current)
    },
    [],
  )

  const tryEquipCharm = (slot: SlotKey, baseId: string): boolean => {
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
      others.push({
        slotKey: cs.key,
        w: b?.width ?? 1,
        h: b?.height ?? 1,
      })
    }
    const candidate = { slotKey: slot, w: newW, h: newH }
    const before = packCharms(others)
    const after = packCharms([...others, candidate])
    const candidateOverflowed = after.overflow.includes(slot)
    const overflowGrew = after.overflow.length > before.overflow.length
    if (candidateOverflowed || overflowGrew) {
      const name = base.name ?? baseId
      setCharmFitError(
        `${name} (${newW}×${newH}) won't fit — free up space first.`,
      )
      if (charmErrorTimer.current !== null)
        window.clearTimeout(charmErrorTimer.current)
      charmErrorTimer.current = window.setTimeout(
        () => setCharmFitError(null),
        4000,
      )
      return false
    }
    setCharmFitError(null)
    return true
  }

  const socketPickerRows: PickerRow[] = useMemo(() => {
    const out: PickerRow[] = []
    for (const g of gems) {
      const isJewel = g.name.toLowerCase().includes('jewel')
      const kind = isJewel ? 'JEWEL' : 'GEM'
      out.push({
        id: g.id,
        name: g.name,
        tier: g.tier,
        kindLabel: kind,
        group: isJewel ? 'Jewels' : 'Gems',
        meta: fmtStats(g.stats) || '—',
        iconColor: gemColorForName(g.name),
        iconUrl: socketableIconForName(g.name),
        tooltip: buildSocketableTooltip(g, kind),
      })
    }
    for (const r of runes) {
      out.push({
        id: r.id,
        name: r.name,
        tier: r.tier,
        kindLabel: 'RUNE',
        group: 'Runes',
        meta: fmtStats(r.stats) || '—',
        iconColor: 'var(--color-accent)',
        iconUrl: socketableIconForName(r.name),
        tooltip: buildSocketableTooltip(r, 'RUNE'),
      })
    }
    return out
  }, [])

  const gearSlots = gameConfig.slots.filter((s) => !s.key.startsWith('charm_'))
  const charmSlots = gameConfig.slots.filter((s) => s.key.startsWith('charm_'))

  const weaponBase = inventory.weapon ? getItem(inventory.weapon.baseId) : undefined
  const offhandLocked = !!weaponBase?.twoHanded

  return (
    <div className="w-full">
      <header className="mb-4">
        <div className="mb-1 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
          <span
            aria-hidden
            className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
            style={{ boxShadow: '0 0 8px rgba(224,184,100,0.6)' }}
          />
          Config
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

      <div className="grid items-start gap-4 lg:grid-cols-[2fr_1fr]">
        <GearPanel
          title="Equipment"
          trailing={
            <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
              <span className="text-accent-hot">
                {gearSlots.filter((s) => inventory[s.key]).length}
              </span>
              <span className="text-faint">
                {' '}
                / {gearSlots.length}
              </span>{' '}
              equipped
            </span>
          }
        >
          <ul className="grid grid-cols-1 gap-1.5 sm:grid-cols-2">
            {gearSlots.map((slot) => (
              <li key={slot.key}>
                <SlotRow
                  slot={slot}
                  equipped={inventory[slot.key]}
                  active={activeSlot === slot.key}
                  locked={slot.key === 'offhand' && offhandLocked}
                  onSelect={() => setActiveSlot(slot.key)}
                />
              </li>
            ))}
          </ul>
        </GearPanel>

        <CharmSection
          charmSlots={charmSlots}
          activeSlot={activeSlot}
          onSelect={(s) => setActiveSlot(s)}
          fitError={charmFitError}
        />
      </div>

      {activeSlot && (
        <GearSlotModal
          slot={activeSlot}
          slotName={
            gameConfig.slots.find((s) => s.key === activeSlot)?.name ?? activeSlot
          }
          equipped={inventory[activeSlot]}
          offhandLocked={offhandLocked}
          socketPickerRows={socketPickerRows}
          onEquip={(id) => {
            if (activeSlot.startsWith('charm_')) {
              if (!tryEquipCharm(activeSlot, id)) return
            }
            equipItem(activeSlot, id)
          }}
          onUnequip={() => {
            unequipItem(activeSlot)
            setActiveSlot(null)
          }}
          onSocketCount={(n) => setSocketCount(activeSlot, n)}
          onSocketed={(idx, id) => setSocketed(activeSlot, idx, id)}
          onSocketType={(idx, t) => setSocketType(activeSlot, idx, t)}
          onSetStars={(n) => setStars(activeSlot, n)}
          onAddAffix={(affixId, tier) => addAffix(activeSlot, affixId, tier)}
          onRemoveAffix={(idx) => removeAffix(activeSlot, idx)}
          onAddForgedMod={(modId, tier) =>
            addForgedMod(activeSlot, modId, tier)
          }
          onRemoveForgedMod={(idx) => removeForgedMod(activeSlot, idx)}
          onApplyRuneword={(rwId) => applyRuneword(activeSlot, rwId)}
          onClose={() => setActiveSlot(null)}
        />
      )}
    </div>
  )
}
