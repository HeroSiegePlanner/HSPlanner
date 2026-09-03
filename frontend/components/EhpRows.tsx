import { EMPTY_EHP, formatEhp, groupEhpRows, type EhpResult } from '../utils/build/ehp'

interface EhpRowsProps {
  ehp: EhpResult | undefined
}

export function EhpRows({ ehp }: EhpRowsProps) {
  const rows = groupEhpRows(ehp ?? EMPTY_EHP)
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
