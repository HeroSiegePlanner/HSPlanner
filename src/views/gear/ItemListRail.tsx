import { useMemo, useState } from 'react'
import type { PickerRow } from '../../components/PickerModal'
import type { SlotKey } from '../../types'
import { pickerItemsForSlot } from './pickerItems'
import { GearItemRow } from './GearItemRow'

interface ItemListRailProps {
  slot: SlotKey
  selectedBaseId?: string
  onSelect: (baseId: string) => void
  onHoverBase: (baseId: string | null) => void
}

export function ItemListRail({
  slot,
  selectedBaseId,
  onSelect,
  onHoverBase,
}: ItemListRailProps) {
  const [q, setQ] = useState('')
  const rows = useMemo(() => pickerItemsForSlot(slot), [slot])

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

  return (
    <div className="flex min-h-0 flex-1 flex-col">
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
            className="w-full rounded-md border border-border bg-bg/60 px-3 py-2 pl-9 text-[13px] text-text placeholder:text-faint focus:border-accent-deep focus:outline-none focus:ring-2 focus:ring-accent-hot/15"
          />
        </div>
      </div>

      <div
        className="min-h-0 flex-1 overflow-y-auto"
        onMouseLeave={() => onHoverBase(null)}
      >
        {filteredRows.length === 0 ? (
          <div className="p-10 text-center text-[13px] text-muted">
            No items match
          </div>
        ) : (
          groupedRows.map((g, gi) => (
            <div key={g.group ?? `__${gi}`}>
              {hasGroupHeaders && g.group && (
                <div
                  className="sticky top-0 z-1 flex items-center gap-2 border-b border-border px-4 py-1.5 text-[11px] font-medium text-muted"
                  style={{ background: 'var(--color-panel-2)' }}
                >
                  <span
                    className="inline-block h-1 w-1 rounded-full bg-accent"
                    aria-hidden="true"
                  />
                  {g.group}
                </div>
              )}
              {g.rows.map((r) => (
                <GearItemRow
                  key={r.id}
                  row={r}
                  selected={r.id === selectedBaseId}
                  onSelect={() => onSelect(r.id)}
                  onHover={() => onHoverBase(r.id)}
                />
              ))}
            </div>
          ))
        )}
      </div>
    </div>
  )
}
