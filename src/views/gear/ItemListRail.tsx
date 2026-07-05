import { useMemo, useState } from 'react'
import Dropdown from '../../components/Dropdown'
import type { DropdownOption } from '../../components/Dropdown'
import type { PickerRow } from '../../components/PickerModal'
import type { SlotKey } from '../../types'
import { gameConfig } from '../../data'
import { statName } from '../../utils/item/stats'
import { useBuildPerformanceDeps } from '../../hooks/useBuildPerformanceDeps'
import { useCalcResult } from '../../hooks/useCalcResult'
import { rankSlotItemsNative } from '../../lib/calc/bridge'
import { pickerItemsForSlot } from './pickerItems'
import { GearItemRow } from './GearItemRow'

const SORT_LABELS: Record<string, string> = {
  defense: 'Defense',
  weapon_damage: 'Weapon Damage',
  block_chance: 'Block Chance',
  sockets: 'Sockets',
}

const PERCENT_STAT_KEYS = new Set(
  gameConfig.stats.filter((s) => s.format === 'percent').map((s) => s.key),
)

function sortLabel(key: string): string {
  return SORT_LABELS[key] ?? statName(key)
}

function fmtSortValue(v: number, key: string): string {
  const suffix = PERCENT_STAT_KEYS.has(key) ? '%' : ''
  if (Math.abs(v) >= 1000)
    return `${(v / 1000).toFixed(Math.abs(v) >= 10000 ? 0 : 1)}k${suffix}`
  return `${Math.round(v * 10) / 10}${suffix}`
}

interface ItemListRailProps {
  slot: SlotKey
  selectedBaseId?: string
  onSelect: (baseId: string) => void
  onHoverBase: (baseId: string | null) => void
  dpsRankingEnabled?: boolean
}

export function ItemListRail({
  slot,
  selectedBaseId,
  onSelect,
  onHoverBase,
  dpsRankingEnabled = true,
}: ItemListRailProps) {
  const [q, setQ] = useState('')
  const [sortKey, setSortKey] = useState('default')
  const rows = useMemo(() => pickerItemsForSlot(slot), [slot])

  const sortOptions = useMemo<DropdownOption[]>(() => {
    const counts = new Map<string, number>()
    for (const r of rows) {
      for (const key of Object.keys(r.sortValues ?? {})) {
        counts.set(key, (counts.get(key) ?? 0) + 1)
      }
    }
    const statOptions = [...counts.entries()]
      .map(([key, count]) => ({ key, count, label: sortLabel(key) }))
      .sort(
        (a, b) => b.count - a.count || a.label.localeCompare(b.label),
      )
      .map(({ key, label, count }) => ({
        id: key,
        label,
        meta: `${count}`,
      }))
    return [
      { id: 'default', label: 'Default' },
      ...(dpsRankingEnabled ? [{ id: 'dps', label: 'DPS' }] : []),
      ...statOptions,
    ]
  }, [rows, dpsRankingEnabled])

  const buildDeps = useBuildPerformanceDeps()
  const baseIds = useMemo(() => rows.map((r) => r.id), [rows])
  const wantDps = dpsRankingEnabled && sortKey === 'dps'
  const dpsValues = useCalcResult<Record<string, number> | null>(
    () => (wantDps ? rankSlotItemsNative(buildDeps, slot, baseIds) : null),
    [wantDps, buildDeps, slot, baseIds],
    null,
  )

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

  const sortedRows = useMemo(() => {
    if (sortKey === 'default') return filteredRows
    const valueOf = (r: PickerRow): number =>
      sortKey === 'dps'
        ? (dpsValues?.[r.id] ?? -1)
        : (r.sortValues?.[sortKey] ?? Number.NEGATIVE_INFINITY)
    return filteredRows.slice().sort((a, b) => valueOf(b) - valueOf(a))
  }, [filteredRows, sortKey, dpsValues])

  const groupedRows = useMemo(() => {
    if (sortKey !== 'default') return [{ group: null, rows: sortedRows }]
    const out: { group: string | null; rows: PickerRow[] }[] = []
    const idx = new Map<string, number>()
    for (const r of sortedRows) {
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
  }, [sortedRows, sortKey])

  const hasGroupHeaders = groupedRows.some((g) => g.group !== null)

  const badgeFor = (r: PickerRow): string | null => {
    if (sortKey === 'default') return null
    if (sortKey === 'dps') {
      const v = dpsValues?.[r.id]
      return v === undefined ? null : fmtSortValue(v, sortKey)
    }
    const v = r.sortValues?.[sortKey]
    return v === undefined ? null : fmtSortValue(v, sortKey)
  }

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
        <div className="mt-2 flex items-center gap-2">
          <span className="font-mono text-[9px] uppercase tracking-[0.18em] text-faint">
            Sort
          </span>
          <Dropdown
            compact
            value={sortKey}
            options={sortOptions}
            searchPlaceholder="Search stats…"
            onChange={(id) => setSortKey(id ?? 'default')}
          />
          {wantDps && !dpsValues && (
            <span className="font-mono text-[9px] uppercase tracking-[0.14em] text-faint">
              computing…
            </span>
          )}
        </div>
      </div>

      <div
        className="min-h-0 flex-1 overflow-y-auto"
        onMouseLeave={() => onHoverBase(null)}
      >
        {sortedRows.length === 0 ? (
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
                  sortBadge={badgeFor(r)}
                />
              ))}
            </div>
          ))
        )}
      </div>
    </div>
  )
}
