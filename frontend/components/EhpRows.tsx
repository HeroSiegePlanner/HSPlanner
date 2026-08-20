import { groupEhpRows, formatEhp } from '../utils/build/ehp'
import type { RangedValue } from '../types'

interface EhpRowsProps {
  stats: Record<string, RangedValue>
  statsCombined: Record<string, RangedValue>
}

export function EhpRows({ stats, statsCombined }: EhpRowsProps) {
  const rows = groupEhpRows({ ...stats, ...statsCombined })
  if (rows.length === 0) return null
  return (
    <>
      {rows.map((r) => (
        <div
          key={r.key}
          className="flex items-baseline justify-between gap-2 py-0.75"
        >
          <span className="flex-1 text-muted">{r.label}</span>
          <span className="shrink-0 text-right font-mono tabular-nums text-accent-hot">
            {formatEhp(r.ehp)}
          </span>
        </div>
      ))}
    </>
  )
}
