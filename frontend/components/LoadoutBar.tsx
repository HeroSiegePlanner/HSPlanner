import { useEffect, useRef, useState } from 'react'
import { useBuild } from '../store/build'
import { IconAction } from './ui/IconAction'
import Tooltip from './ui/Tooltip'
import {
  isSlotOccupied,
  LOADOUT_SLOT_COUNT,
  MAX_LOADOUT_NAME_LENGTH,
  type LoadoutTab,
} from '../utils/build/loadouts'

interface LoadoutBarProps {
  tab: LoadoutTab
  /** What this tab's loadouts cover, shown in the slot tooltip. */
  scopeLabel: string
  className?: string
  dataTour?: string
}

const SLOT_BASE =
  'pointer-events-auto flex h-[22px] w-[22px] items-center justify-center rounded-[2px] border font-mono text-[10px] tabular-nums transition-all'

/**
 * The in-game profile row: slots 1..8 for one tab. Clicking a slot parks the
 * live state and loads that slot; an empty slot starts blank. Each tab keeps
 * its own row and its own active index.
 */
export default function LoadoutBar({
  tab,
  scopeLabel,
  className,
  dataTour,
}: LoadoutBarProps) {
  const slots = useBuild((s) => s.loadoutSlots[tab])
  const activeIndex = useBuild((s) => s.activeLoadouts[tab])
  const switchLoadout = useBuild((s) => s.switchLoadout)
  const renameLoadout = useBuild((s) => s.renameLoadout)
  const clearLoadout = useBuild((s) => s.clearLoadout)
  const duplicateLoadout = useBuild((s) => s.duplicateLoadout)

  // Keyed to the slot being renamed rather than a plain boolean, so switching
  // tab or slot mid-rename drops the edit instead of applying it to the wrong
  // slot — derived, no state-syncing effect.
  const slotKey = `${tab}:${activeIndex}`
  const [renamingKey, setRenamingKey] = useState<string | null>(null)
  const [draft, setDraft] = useState('')
  // While set, clicking a slot copies the active loadout into it instead of
  // switching to it. Keyed to the source slot so leaving that slot cancels.
  const [copyFromKey, setCopyFromKey] = useState<string | null>(null)
  const inputRef = useRef<HTMLInputElement>(null)
  const renaming = renamingKey === slotKey
  const copying = copyFromKey === slotKey

  useEffect(() => {
    if (renaming) inputRef.current?.select()
  }, [renaming])

  const activeName = slots[activeIndex]?.name ?? null

  const startRename = () => {
    setCopyFromKey(null)
    setDraft(activeName ?? '')
    setRenamingKey(slotKey)
  }

  const commitRename = () => {
    renameLoadout(tab, activeIndex, draft)
    setRenamingKey(null)
  }

  return (
    <div
      data-tour={dataTour}
      className={`pointer-events-none inline-flex items-center gap-2 rounded-[3px] border border-border px-2.5 py-1 ${className ?? ''}`}
      style={{
        background: 'linear-gradient(180deg, rgba(13,14,18,0.92), rgba(28,29,36,0.92))',
        backdropFilter: 'blur(4px)',
      }}
    >
      {renaming ? (
        <input
          ref={inputRef}
          type="text"
          value={draft}
          maxLength={MAX_LOADOUT_NAME_LENGTH}
          placeholder={`Loadout ${activeIndex + 1}`}
          aria-label="Loadout name"
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commitRename}
          onKeyDown={(e) => {
            if (e.key === 'Enter') commitRename()
            if (e.key === 'Escape') setRenamingKey(null)
          }}
          className="pointer-events-auto w-28 rounded-[2px] border border-accent-deep bg-panel-2 px-1.5 py-0.5 font-mono text-[10px] text-text placeholder:text-faint focus:outline-none focus:ring-2 focus:ring-accent-hot/15"
        />
      ) : copying ? (
        <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-accent-hot">
          Copy to…
        </span>
      ) : (
        activeName && (
          <span
            className="max-w-32 truncate font-mono text-[10px] text-accent-hot"
            title={activeName}
          >
            {activeName}
          </span>
        )
      )}

      <div className="flex items-center gap-1" role="group" aria-label={`${scopeLabel} loadouts`}>
        {Array.from({ length: LOADOUT_SLOT_COUNT }, (_, i) => {
          const isActive = i === activeIndex
          const occupied = isSlotOccupied(slots, i, activeIndex)
          const name = slots[i]?.name ?? null
          const isCopyTarget = copying && !isActive
          const tone = isCopyTarget
            ? 'border-accent-deep bg-panel-2 text-muted hover:border-accent-hot hover:text-accent-hot hover:shadow-[0_0_10px_rgba(224,184,100,0.28)]'
            : isActive
              ? 'border-accent-hot text-accent-hot shadow-[0_0_10px_rgba(224,184,100,0.28)]'
              : occupied
                ? 'border-border-2 bg-panel-2 text-text hover:border-accent-deep hover:text-accent-hot'
                : 'border-border bg-transparent text-faint hover:border-accent-deep hover:text-muted'
          return (
            <Tooltip
              key={i}
              placement="left"
              content={
                <div className="font-mono text-[11px]">
                  <div className="text-accent-hot">{name ?? `Loadout ${i + 1}`}</div>
                  <div className="text-faint">
                    {copying
                      ? isActive
                        ? 'Source of the copy'
                        : occupied
                          ? 'Click to overwrite with the current loadout'
                          : 'Click to copy the current loadout here'
                      : isActive
                        ? 'Active'
                        : occupied
                          ? 'Saved — click to load'
                          : 'Empty — click to start blank'}
                  </div>
                  <div className="mt-1 text-muted">{scopeLabel}</div>
                </div>
              }
            >
              <button
                type="button"
                aria-label={
                  copying && !isActive
                    ? `Copy into loadout ${i + 1}${name ? ` — ${name}` : ''}`
                    : `Loadout ${i + 1}${name ? ` — ${name}` : ''}`
                }
                aria-pressed={isActive}
                onClick={() => {
                  if (!copying) {
                    switchLoadout(tab, i)
                    return
                  }
                  duplicateLoadout(tab, activeIndex, i)
                  setCopyFromKey(null)
                }}
                className={`${SLOT_BASE} ${tone}`}
                style={isActive ? { background: 'linear-gradient(180deg, #2a2418, #1a1410)' } : undefined}
              >
                {i + 1}
              </button>
            </Tooltip>
          )
        })}
      </div>

      <div className="pointer-events-auto flex items-center gap-0.5">
        <IconAction tooltipPlacement="left" label="Rename this loadout" active={renaming} onClick={startRename}>
          <svg viewBox="0 0 24 24" className="h-3.5 w-3.5" fill="none" stroke="currentColor" strokeWidth={2}>
            <path d="M12 20h9" />
            <path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" />
          </svg>
        </IconAction>
        <IconAction
          tooltipPlacement="left"
          label={
            copying
              ? 'Pick a slot to copy into — click again to cancel'
              : 'Copy this loadout into a slot you pick'
          }
          active={copying}
          onClick={() => {
            setRenamingKey(null)
            setCopyFromKey(copying ? null : slotKey)
          }}
        >
          <svg viewBox="0 0 24 24" className="h-3.5 w-3.5" fill="none" stroke="currentColor" strokeWidth={2}>
            <rect x="9" y="9" width="11" height="11" rx="1.5" />
            <path d="M5 15V5a1.5 1.5 0 0 1 1.5-1.5H15" />
          </svg>
        </IconAction>
        <IconAction danger tooltipPlacement="left" label={`Clear this loadout (${scopeLabel})`} onClick={() => clearLoadout(tab, activeIndex)}>
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
