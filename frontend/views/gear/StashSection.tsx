import { useMemo, useState } from 'react'
import ItemTooltip from '../../components/ItemTooltip'
import Dropdown, { type DropdownOption } from '../../components/ui/Dropdown'
import {
  detectRuneword,
  effectiveStars,
  gameConfig,
  getItem,
  getItemImage,
} from '@data'
import { useBuild } from '../../store/build'
import type { SlotKey, StashEntry } from '../../types'
import { RARITY_TEXT } from './lib/rarity'
import { equipTargets } from './lib/stashEquip'
import { GearPanel } from './SlotRail'

function slotGroupOf(entry: StashEntry): string | null {
  const base = getItem(entry.item.baseId)
  return base ? base.slot.replace(/_\d+$/, '') : null
}

function slotName(key: SlotKey): string {
  return gameConfig.slots.find((s) => s.key === key)?.name ?? key
}

function StashRow({ entry }: { entry: StashEntry }) {
  const inventory = useBuild((s) => s.inventory)
  const allocatedTreeNodes = useBuild((s) => s.allocatedTreeNodes)
  const commitEquippedItem = useBuild((s) => s.commitEquippedItem)
  const removeEntry = useBuild((s) => s.removeStashItem)

  const base = getItem(entry.item.baseId)
  if (!base) return null
  const runeword = detectRuneword(base, entry.item.socketed)
  const sprite = getItemImage(base.id)
  const rarityText = runeword ? 'text-accent-hot' : RARITY_TEXT[base.rarity]

  const stars = effectiveStars(base.slot, entry.item.stars) ?? 0
  const badges: string[] = []
  if (stars > 0) badges.push('★'.repeat(Math.min(stars, 5)))
  if (entry.item.socketCount > 0)
    badges.push(
      `${entry.item.socketed.filter(Boolean).length}/${entry.item.socketCount}◇`,
    )
  if (entry.item.affixes.length > 0) badges.push(`${entry.item.affixes.length} aff`)

  const targets = equipTargets(entry.item, inventory, allocatedTreeNodes)
  const equipTo = (slot: SlotKey) =>
    commitEquippedItem(slot, JSON.parse(JSON.stringify(entry.item)))

  return (
    <li className="flex items-center gap-2 rounded-[3px] border border-border bg-panel-2/40 px-2 py-1.5">
      <ItemTooltip equipped={entry.item} placement="right" className="flex min-w-0 flex-1 items-center gap-2">
        <span
          className="flex h-[34px] w-[34px] shrink-0 items-center justify-center overflow-hidden rounded-[3px] border border-border-2"
          style={{
            background: 'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
          }}
        >
          {sprite ? (
            <img
              src={sprite}
              alt=""
              draggable={false}
              className="h-full w-full object-contain select-none"
              style={{ imageRendering: 'pixelated' }}
            />
          ) : (
            <span className={`text-sm leading-none ${rarityText}`}>◆</span>
          )}
        </span>
        <span className="min-w-0 flex-1">
          <span className={`block truncate text-[12px] font-semibold ${rarityText}`}>
            {runeword ? runeword.name : base.name}
          </span>
          <span className="block truncate text-[10px] text-muted">
            {base.baseType}
            {badges.length > 0 && (
              <span className="ml-1.5 font-mono text-[9px] uppercase tracking-[0.1em] text-faint">
                {badges.join(' · ')}
              </span>
            )}
          </span>
        </span>
      </ItemTooltip>
      {targets.length > 1 ? (
        <Dropdown
          compact
          searchable={false}
          value={null}
          placeholder="Equip"
          options={targets.map((k) => ({ id: k, label: slotName(k) }))}
          onChange={(id) => id && equipTo(id as SlotKey)}
        />
      ) : (
        <button
          type="button"
          disabled={targets.length === 0}
          title={
            targets.length === 0
              ? 'No slot can take this item right now'
              : undefined
          }
          onClick={() => targets[0] && equipTo(targets[0])}
          className="shrink-0 rounded-[3px] border border-accent-deep px-2 py-1 font-mono text-[9px] font-semibold uppercase tracking-[0.14em] text-accent-hot transition-colors hover:bg-accent-hot/10 disabled:cursor-not-allowed disabled:border-border disabled:text-faint disabled:hover:bg-transparent"
        >
          Equip
        </button>
      )}
      <button
        type="button"
        onClick={() => removeEntry(entry.id)}
        aria-label={`Remove ${base.name} from stash`}
        className="shrink-0 rounded-[3px] border border-border px-1.5 py-1 text-[11px] leading-none text-faint transition-colors hover:border-stat-red/60 hover:text-stat-red"
      >
        ×
      </button>
    </li>
  )
}

export function StashSection() {
  const entries = useBuild((s) => s.stash)
  const [group, setGroup] = useState('all')

  const groups = useMemo(() => {
    const gs = new Set<string>()
    for (const e of entries) {
      const g = slotGroupOf(e)
      if (g) gs.add(g)
    }
    return [...gs].sort()
  }, [entries])

  const filtered =
    group === 'all' ? entries : entries.filter((e) => slotGroupOf(e) === group)

  const groupOptions = useMemo<DropdownOption[]>(
    () => [
      { id: 'all', label: 'All slots' },
      ...groups.map((g) => ({
        id: g,
        label: g.charAt(0).toUpperCase() + g.slice(1),
      })),
    ],
    [groups],
  )

  return (
    <GearPanel
      title="Stash"
      trailing={
        <span className="flex items-center gap-2">
          {groups.length > 1 && (
            <Dropdown
              compact
              searchable={false}
              value={group}
              options={groupOptions}
              onChange={(id) => setGroup(id ?? 'all')}
            />
          )}
          <span className="font-mono text-[10px] uppercase tracking-[0.14em] whitespace-nowrap text-faint">
            <span className={entries.length > 0 ? 'text-accent-hot' : 'text-muted'}>
              {filtered.length}
            </span>{' '}
            saved
          </span>
        </span>
      }
    >
      {filtered.length === 0 ? (
        <p className="m-0 text-[11px] text-faint italic">
          Items you equip are snapshotted here — re-equip a saved configuration
          anytime to compare setups without rebuilding them.
        </p>
      ) : (
        <ul
          className="grid list-none gap-1.5 p-0"
          style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(260px, 1fr))' }}
        >
          {filtered.map((e) => (
            <StashRow key={e.id} entry={e} />
          ))}
        </ul>
      )}
    </GearPanel>
  )
}
