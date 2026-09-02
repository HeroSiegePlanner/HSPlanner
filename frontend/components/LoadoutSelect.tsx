import { useEffect, useMemo, useRef, useState } from 'react'
import { useBuild } from '../store/build'
import Dropdown, { type DropdownOption } from './ui/Dropdown'
import { IconAction } from './ui/IconAction'
import {
  isSlotOccupied,
  LOADOUT_SLOT_COUNT,
  MAX_LOADOUT_NAME_LENGTH,
  type LoadoutTab,
} from '../utils/build/loadouts'

interface LoadoutSelectProps {
  tab: LoadoutTab
  /** Eyebrow label, e.g. "Progression". */
  title: string
  className?: string
}

/**
 * Named-stage picker, for tabs whose loadouts are progression steps the user
 * invents ("Early", "Mid Game", "Aspirational") rather than the game's numbered
 * profile row. Same 8 underlying slots as LoadoutBar — the stage name is what
 * identifies an entry here, so a slot with only a name is already a real stage.
 */
export default function LoadoutSelect({
  tab,
  title,
  className,
}: LoadoutSelectProps) {
  const slots = useBuild((s) => s.loadoutSlots[tab])
  const activeIndex = useBuild((s) => s.activeLoadouts[tab])
  const liveInventory = useBuild((s) => s.inventory)
  const switchLoadout = useBuild((s) => s.switchLoadout)
  const renameLoadout = useBuild((s) => s.renameLoadout)
  const clearLoadout = useBuild((s) => s.clearLoadout)

  // 'create' opens a blank name field, 'rename' edits the active stage's name.
  // Keyed to the slot so switching stages mid-edit drops it instead of applying
  // the text to the wrong one.
  const [editing, setEditing] = useState<'create' | 'rename' | null>(null)
  const [editingKey, setEditingKey] = useState<string | null>(null)
  const [draft, setDraft] = useState('')
  const inputRef = useRef<HTMLInputElement>(null)
  const slotKey = `${tab}:${activeIndex}`
  const open = editingKey === slotKey ? editing : null

  useEffect(() => {
    if (open) inputRef.current?.focus()
  }, [open])

  const firstFreeSlot = useMemo(() => {
    for (let i = 0; i < LOADOUT_SLOT_COUNT; i++) {
      if (!isSlotOccupied(slots, i, activeIndex)) return i
    }
    return null
  }, [slots, activeIndex])

  const stageName = (index: number): string =>
    slots[index]?.name?.trim() || `Stage ${index + 1}`

  const options: DropdownOption[] = useMemo(() => {
    const out: DropdownOption[] = []
    for (let i = 0; i < LOADOUT_SLOT_COUNT; i++) {
      if (!isSlotOccupied(slots, i, activeIndex)) continue
      const inventory = i === activeIndex ? liveInventory : slots[i]?.data?.inventory
      const count = Object.keys(inventory ?? {}).length
      out.push({
        id: String(i),
        label: slots[i]?.name?.trim() || `Stage ${i + 1}`,
        meta: count > 0 ? `${count} equipped` : 'empty',
      })
    }
    return out
  }, [slots, activeIndex, liveInventory])

  const startCreate = () => {
    setDraft('')
    setEditing('create')
    setEditingKey(slotKey)
  }

  const startRename = () => {
    setDraft(slots[activeIndex]?.name ?? '')
    setEditing('rename')
    setEditingKey(slotKey)
  }

  const cancel = () => {
    setEditing(null)
    setEditingKey(null)
  }

  const commit = () => {
    const name = draft.trim()
    if (open === 'create') {
      // A stage is identified by its name, so an unnamed one is not created.
      if (name !== '' && firstFreeSlot !== null) {
        renameLoadout(tab, firstFreeSlot, name)
        switchLoadout(tab, firstFreeSlot)
      }
    } else if (open === 'rename') {
      renameLoadout(tab, activeIndex, name === '' ? null : name)
    }
    cancel()
  }

  const onlyStage = options.length <= 1

  return (
    <div
      className={`inline-flex items-center gap-2 rounded-[3px] border border-border px-2.5 py-1 ${className ?? ''}`}
      style={{
        background: 'linear-gradient(180deg, rgba(13,14,18,0.92), rgba(28,29,36,0.92))',
      }}
    >
      <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
        {title}
      </span>

      {open ? (
        <input
          ref={inputRef}
          type="text"
          value={draft}
          maxLength={MAX_LOADOUT_NAME_LENGTH}
          placeholder={open === 'create' ? 'Early, Mid Game…' : stageName(activeIndex)}
          aria-label={open === 'create' ? 'New stage name' : 'Stage name'}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === 'Enter') commit()
            if (e.key === 'Escape') cancel()
          }}
          className="w-40 rounded-[2px] border border-accent-deep bg-panel-2 px-1.5 py-0.5 font-mono text-[11px] text-text placeholder:text-faint focus:outline-none focus:ring-2 focus:ring-accent-hot/15"
        />
      ) : (
        <div className="min-w-40">
          <Dropdown
            compact
            searchable={false}
            value={String(activeIndex)}
            options={options}
            onChange={(id) => {
              if (id !== null) switchLoadout(tab, Number(id))
            }}
          />
        </div>
      )}

      <div className="flex items-center gap-0.5">
        <IconAction
          tooltipPlacement="left"
          label={
            firstFreeSlot === null
              ? `All ${LOADOUT_SLOT_COUNT} stages used`
              : 'Add a stage'
          }
          disabled={firstFreeSlot === null}
          active={open === 'create'}
          onClick={startCreate}
        >
          <svg viewBox="0 0 24 24" className="h-3.5 w-3.5" fill="none" stroke="currentColor" strokeWidth={2}>
            <path d="M12 5v14" />
            <path d="M5 12h14" />
          </svg>
        </IconAction>
        <IconAction tooltipPlacement="left" label="Rename this stage" active={open === 'rename'} onClick={startRename}>
          <svg viewBox="0 0 24 24" className="h-3.5 w-3.5" fill="none" stroke="currentColor" strokeWidth={2}>
            <path d="M12 20h9" />
            <path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" />
          </svg>
        </IconAction>
        <IconAction
          tooltipPlacement="left"
          danger
          disabled={onlyStage}
          label={
            onlyStage
              ? 'The last stage cannot be deleted'
              : `Delete “${stageName(activeIndex)}”`
          }
          onClick={() => {
            // Deleting the active stage frees the slot, so move off it first.
            const fallback = options.find((o) => o.id !== String(activeIndex))
            if (!fallback) return
            const doomed = activeIndex
            switchLoadout(tab, Number(fallback.id))
            clearLoadout(tab, doomed)
          }}
        >
          <svg viewBox="0 0 24 24" className="h-3.5 w-3.5" fill="none" stroke="currentColor" strokeWidth={2}>
            <path d="M4 7h16" />
            <path d="M9 7V4h6v3" />
            <path d="M6 7l1 13h10l1-13" />
          </svg>
        </IconAction>
      </div>
    </div>
  )
}
