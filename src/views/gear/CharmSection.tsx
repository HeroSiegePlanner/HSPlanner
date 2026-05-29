import { useMemo } from 'react'
import ItemTooltip from '../../components/ItemTooltip'
import { getItem, getItemImage } from '../../data'
import { useBuild } from '../../store/build'
import type { SlotKey } from '../../types'
import { CHARM_BLOCKED_CELLS, CHARM_GRID_COLS, CHARM_GRID_ROWS, packCharms } from './lib/charmPacking'
import { RARITY_BG, RARITY_TEXT } from './lib/rarity'
import { GearPanel } from './SlotRail'

export function CharmSection({
  charmSlots,
  activeSlot,
  onSelect,
  fitError,
}: {
  charmSlots: { key: SlotKey; name: string }[]
  activeSlot: SlotKey | null
  onSelect: (s: SlotKey) => void
  fitError?: string | null
}) {
  // Renders the visual charm grid (with blocked cells, multi-cell charms, click-to-edit, and an "overflow" warning when the user tries to equip more area than the grid can hold). Used by GearView underneath the slot rail.
  const inventory = useBuild((s) => s.inventory)

  const { placed, overflow, occupancy, totalUsable, occupiedCells } =
    useMemo(() => {
      const charms = charmSlots
        .map((slot) => {
          const eq = inventory[slot.key]
          if (!eq) return null
          const base = getItem(eq.baseId)
          return {
            slotKey: slot.key,
            w: base?.width ?? 1,
            h: base?.height ?? 1,
          }
        })
        .filter((c): c is { slotKey: SlotKey; w: number; h: number } =>
          Boolean(c),
        )

      const result = packCharms(charms)
      const total =
        CHARM_GRID_ROWS * CHARM_GRID_COLS - CHARM_BLOCKED_CELLS.length
      const used = charms.reduce((acc, c) => acc + c.w * c.h, 0)
      return {
        ...result,
        totalUsable: total,
        occupiedCells: used,
      }
    }, [charmSlots, inventory])

  const blockedSet = useMemo(() => {
    const s = new Set<number>()
    for (const [r, c] of CHARM_BLOCKED_CELLS) s.add(r * CHARM_GRID_COLS + c)
    return s
  }, [])

  const nextEmptySlotKey = useMemo(
    () => charmSlots.find((s) => !inventory[s.key])?.key,
    [charmSlots, inventory],
  )

  const emptyCells: { row: number; col: number }[] = []
  for (let r = 0; r < CHARM_GRID_ROWS; r++) {
    for (let c = 0; c < CHARM_GRID_COLS; c++) {
      const idx = r * CHARM_GRID_COLS + c
      if (!occupancy[idx] && !blockedSet.has(idx)) {
        emptyCells.push({ row: r, col: c })
      }
    }
  }

  return (
    <GearPanel
      title="Charm Inventory"
      trailing={
        <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
          <span className="text-accent-hot">{occupiedCells}</span>
          <span className="text-faint"> / {totalUsable}</span>
          {overflow.length > 0 && (
            <span className="ml-2 text-stat-red">
              · {overflow.length} won't fit
            </span>
          )}
        </span>
      }
    >
      <div
        className="mx-auto grid w-fit gap-1 rounded-[3px] border border-border-2 p-3"
        style={{
          background: 'linear-gradient(180deg, #0c0804, #070302)',
          boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.7)',
          gridTemplateColumns: `repeat(${CHARM_GRID_COLS}, 4rem)`,
          gridTemplateRows: `repeat(${CHARM_GRID_ROWS}, 4rem)`,
        }}
      >
        {CHARM_BLOCKED_CELLS.map(([r, c]) => (
          <div
            key={`blocked-${r}-${c}`}
            style={{ gridColumn: c + 1, gridRow: r + 1 }}
            className="rounded border border-dashed border-[#3a2a18] bg-[#070403] opacity-60"
            aria-label="Blocked"
          />
        ))}

        {placed.map((p) => {
          const eq = inventory[p.slotKey]
          if (!eq) return null
          const base = getItem(eq.baseId)
          if (!base) return null
          const isActive = activeSlot === p.slotKey
          const rarityColor = RARITY_TEXT[base.rarity] ?? 'text-text'
          const rarityBg = RARITY_BG[base.rarity] ?? 'bg-panel-2'
          const sprite = getItemImage(base.id)

          const cell = (
            <button
              type="button"
              onClick={() => onSelect(p.slotKey)}
              aria-label={base.name}
              className={`relative w-full h-full rounded-[3px] text-[10px] flex flex-col items-center justify-center text-center transition-colors overflow-hidden ${
                isActive
                  ? 'border-2 border-accent bg-accent/10 ring-1 ring-accent'
                  : `border border-accent-deep ${rarityBg} hover:opacity-90 cursor-pointer`
              }`}
            >
              {sprite ? (
                <img
                  src={sprite}
                  alt=""
                  draggable={false}
                  className="pointer-events-none absolute inset-0 h-full w-full object-contain select-none"
                  style={{ imageRendering: 'pixelated' }}
                />
              ) : (
                <>
                  <div className={`text-2xl leading-none ${rarityColor}`}>◆</div>
                  <div
                    className={`mt-0.5 px-1 leading-tight ${rarityColor} font-medium text-[9px] pointer-events-none`}
                    style={{
                      display: '-webkit-box',
                      WebkitLineClamp: 2,
                      WebkitBoxOrient: 'vertical',
                      overflow: 'hidden',
                    }}
                  >
                    {base.name}
                  </div>
                </>
              )}
            </button>
          )

          return (
            <div
              key={p.slotKey}
              style={{
                gridColumn: `${p.col + 1} / span ${p.w}`,
                gridRow: `${p.row + 1} / span ${p.h}`,
              }}
              className="relative"
            >
              <ItemTooltip
                equipped={eq}
                placement="right"
                className="block w-full h-full"
              >
                {cell}
              </ItemTooltip>
            </div>
          )
        })}

        {emptyCells.map(({ row, col }) => (
          <button
            key={`empty-${row}-${col}`}
            type="button"
            onClick={() => {
              if (nextEmptySlotKey) onSelect(nextEmptySlotKey)
            }}
            disabled={!nextEmptySlotKey}
            style={{ gridColumn: col + 1, gridRow: row + 1 }}
            className="rounded-[3px] text-[10px] flex items-center justify-center border border-[#5a4528] bg-[#120c08] hover:border-accent-deep text-transparent hover:text-accent/40 cursor-pointer disabled:cursor-not-allowed"
            aria-label="Empty charm slot"
          >
            <span className="text-lg">+</span>
          </button>
        ))}
      </div>

      {overflow.length > 0 && (
        <div
          className="mt-2 rounded-[3px] border border-stat-red/40 px-2 py-1 font-mono text-[10px] uppercase tracking-[0.14em] text-stat-red"
          style={{
            background:
              'linear-gradient(180deg, rgba(60,30,28,0.4), rgba(44,22,20,0.25))',
          }}
        >
          {overflow.length} charm{overflow.length === 1 ? '' : 's'} could not be
          placed — remove some to free space.
        </div>
      )}
      {fitError && (
        <div
          className="mt-2 rounded-[3px] border border-stat-red/40 px-2 py-1 font-mono text-[10px] uppercase tracking-[0.14em] text-stat-red"
          style={{
            background:
              'linear-gradient(180deg, rgba(60,30,28,0.4), rgba(44,22,20,0.25))',
          }}
        >
          {fitError}
        </div>
      )}
    </GearPanel>
  )
}
