import { useEffect, useMemo, useState } from 'react'
import type { Inventory, SavedLootFilter } from '../../types'
import { Modal, MODAL_BTN_PRIMARY_CLASS } from '../../components/ui/Modal'
import { FILTER_STAT_BY_ID } from '../../utils/lootfilter/constants'
import {
  buildFilterStats,
  lootFilterCodeForStats,
  type BuildFilterStats,
} from '../../utils/lootfilter/buildFilter'
import { createFilter } from '../../utils/lootfilter/savedFilters'

interface GenerateFromBuildModalProps {
  buildId: string
  buildName: string | null
  inventory: Inventory
  onClose: () => void
  onDone: (record: SavedLootFilter) => void
}

export function GenerateFromBuildModal({
  buildId,
  buildName,
  inventory,
  onClose,
  onDone,
}: GenerateFromBuildModalProps) {
  const [scan, setScan] = useState<BuildFilterStats | null>(null)
  const [selected, setSelected] = useState<ReadonlySet<number>>(() => new Set())
  const [hideRest, setHideRest] = useState(false)
  const [name, setName] = useState(`${buildName ?? 'Build'} · auto`)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    buildFilterStats(inventory)
      .then((result) => {
        if (cancelled) return
        setScan(result)
        setSelected(new Set(result.statIds))
      })
      .catch(() => {
        if (!cancelled) setError('Could not scan the gear for affixes.')
      })
    return () => {
      cancelled = true
    }
  }, [inventory])

  const statIds = useMemo(() => scan?.statIds ?? [], [scan])
  const affixes = useMemo(
    () =>
      statIds
        .map((id) => ({ id, name: FILTER_STAT_BY_ID.get(id)?.name }))
        .filter((a): a is { id: number; name: string } => Boolean(a.name))
        .sort((a, b) => a.name.localeCompare(b.name)),
    [statIds],
  )

  const toggle = (id: number) =>
    setSelected((prev) => {
      const next = new Set(prev)
      if (!next.delete(id)) next.add(id)
      return next
    })

  const unmatched = scan?.unmatched ?? 0

  const submit = async () => {
    if (selected.size === 0) return
    try {
      const code = await lootFilterCodeForStats([...selected], hideRest)
      onDone(createFilter(buildId, name.trim() || 'Build filter', code))
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Could not create the filter.')
    }
  }

  return (
    <Modal
      onClose={onClose}
      panelClassName="w-[min(560px,92vw)]"
      eyebrow="Loot filter"
      title="Generate from build"
    >
      <div className="flex flex-col gap-3 px-6 py-4">
        {scan === null ? (
          <p className="text-[12px] leading-relaxed text-muted">Scanning gear…</p>
        ) : statIds.length === 0 ? (
          <p className="text-[12px] leading-relaxed text-muted">
            This build has no affixes on its gear yet, so there is nothing to
            highlight. Equip a few items in the Gear tab first.
          </p>
        ) : (
          <>
            <label className="flex flex-col gap-1">
              <span className="font-mono text-[9.5px] uppercase tracking-[0.14em] text-faint">
                Name
              </span>
              <input
                value={name}
                onChange={(e) => setName(e.target.value)}
                autoFocus
                onFocus={(e) => e.target.select()}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') void submit()
                }}
                className="rounded-[3px] border border-border-2 bg-panel-2 px-2 py-1.5 text-[12px] text-text outline-none transition-colors focus:border-accent-deep"
              />
            </label>

            <div className="flex flex-col gap-1.5">
              <div className="flex items-center justify-between gap-2">
                <span className="font-mono text-[9.5px] uppercase tracking-[0.14em] text-faint">
                  Highlights {selected.size} of {affixes.length} affixes from your
                  gear
                </span>
                <div className="flex items-center gap-1">
                  <button
                    type="button"
                    onClick={() => setSelected(new Set(statIds))}
                    className="font-mono text-[9.5px] uppercase tracking-[0.12em] text-faint transition-colors hover:text-accent-hot"
                  >
                    All
                  </button>
                  <span className="text-[9.5px] text-faint">·</span>
                  <button
                    type="button"
                    onClick={() => setSelected(new Set())}
                    className="font-mono text-[9.5px] uppercase tracking-[0.12em] text-faint transition-colors hover:text-accent-hot"
                  >
                    None
                  </button>
                </div>
              </div>
              <div className="flex max-h-[160px] flex-wrap gap-1 overflow-y-auto rounded-[3px] border border-border-2 bg-panel-2 p-2">
                {affixes.map((a) => {
                  const on = selected.has(a.id)
                  return (
                    <button
                      key={a.id}
                      type="button"
                      onClick={() => toggle(a.id)}
                      aria-pressed={on}
                      className={`rounded-[2px] border px-1.5 py-[1px] font-mono text-[9px] uppercase tracking-[0.12em] transition-colors ${
                        on
                          ? 'border-accent-deep bg-accent-hot/10 text-accent-hot'
                          : 'border-border-2 text-faint hover:text-muted'
                      }`}
                    >
                      {a.name}
                    </button>
                  )
                })}
              </div>
              {unmatched > 0 && (
                <span className="text-[11px] text-faint">
                  {unmatched} stat{unmatched === 1 ? '' : 's'} on your gear{' '}
                  {unmatched === 1 ? 'has' : 'have'} no counterpart in the game's
                  filter and {unmatched === 1 ? 'was' : 'were'} skipped.
                </span>
              )}
            </div>

            <label className="flex cursor-pointer items-start gap-2">
              <input
                type="checkbox"
                checked={hideRest}
                onChange={(e) => setHideRest(e.target.checked)}
                className="mt-[3px] accent-[var(--color-accent-deep)]"
              />
              <span className="text-[12px] leading-relaxed text-muted">
                Hide every other affix
                <span className="block text-[11px] text-faint">
                  Aggressive — drops with affixes you don't use yet stop showing up.
                </span>
              </span>
            </label>
          </>
        )}
        {error && <span className="text-[11px] text-stat-red">{error}</span>}
      </div>
      <div className="flex items-center justify-end gap-2 border-t border-border px-6 py-3">
        <button
          type="button"
          onClick={() => void submit()}
          disabled={selected.size === 0}
          className={`${MODAL_BTN_PRIMARY_CLASS} disabled:cursor-not-allowed disabled:opacity-40`}
        >
          Create filter
        </button>
      </div>
    </Modal>
  )
}
