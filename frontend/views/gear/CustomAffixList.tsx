import { useMemo, useState } from 'react'
import { gameConfig } from '@data'
import type { StatDef } from '../../types'

export type StatRow = Pick<StatDef, 'key' | 'name' | 'format'>

// Same name → key rule as the parser: last definition wins for duplicate names
const STAT_ROWS: StatRow[] = (() => {
  const byName = new Map<string, StatRow>()
  for (const s of gameConfig.stats) {
    byName.set(s.name.toLowerCase(), { key: s.key, name: s.name, format: s.format })
  }
  return [...byName.values()].sort((a, b) => a.name.localeCompare(b.name))
})()

interface Props {
  onPick: (stat: StatRow) => void
}

export function CustomAffixList({ onPick }: Props) {
  const [query, setQuery] = useState('')
  const rows = useMemo(() => {
    const q = query.trim().toLowerCase()
    if (!q) return STAT_ROWS
    return STAT_ROWS.filter(
      (s) => s.name.toLowerCase().includes(q) || s.key.includes(q),
    )
  }, [query])

  return (
    <div className="flex min-h-0 flex-1 flex-col border-t border-border">
      <div className="flex items-center gap-2 border-b border-border px-4 py-2 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
        <span
          className="inline-block h-1 w-1 rotate-45 bg-accent-deep"
          aria-hidden="true"
        />
        Custom affixes · click to insert
        <span className="ml-auto tabular-nums">{rows.length}</span>
      </div>
      <div className="border-b border-border px-3 py-2">
        <input
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search stat…"
          aria-label="Search custom affixes"
          spellCheck={false}
          className="w-full rounded-[3px] border border-border-2 bg-bg/60 px-2.5 py-1.5 font-mono text-[11px] text-text placeholder:text-faint focus:border-accent-deep focus:outline-none"
        />
      </div>
      <ul className="min-h-0 flex-1 overflow-y-auto py-1">
        {rows.length === 0 && (
          <li className="px-4 py-3 font-mono text-[11px] text-muted">
            No stat matches “{query.trim()}”.
          </li>
        )}
        {rows.map((s) => (
          <li key={s.key}>
            <button
              type="button"
              onClick={() => onPick(s)}
              className="group flex w-full items-baseline gap-2 px-4 py-1 text-left font-mono text-[11px] leading-[1.5] text-muted transition-colors hover:bg-accent-hot/10 hover:text-text focus-visible:bg-accent-hot/10 focus-visible:text-text focus-visible:outline-none"
            >
              <span className="shrink-0 tabular-nums text-accent-hot/70 group-hover:text-accent-hot">
                +1{s.format === 'percent' ? '%' : ''}
              </span>{' '}
              <span className="min-w-0 flex-1 truncate">{s.name}</span>{' '}
              <span className="shrink-0 text-[10px] text-faint">[custom]</span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  )
}
