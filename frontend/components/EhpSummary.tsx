import { useMemo } from 'react'
import type { DefenseInsight } from '../utils/build/ehp'
import { rangedMax } from '../utils/item/stats'
import type { RangedValue } from '../types'

interface EhpSummaryProps {
  stats: Record<string, RangedValue>
  statsCombined: Record<string, RangedValue>
  insights: DefenseInsight[] | undefined
}

function fmtPct(value: number): number {
  return Math.round(value * 10) / 10
}

export function EhpSummary({ stats, statsCombined, insights = [] }: EhpSummaryProps) {
  const merged = useMemo(
    () => ({ ...stats, ...statsCombined }),
    [stats, statsCombined],
  )

  const avoidance: string[] = []
  const block = rangedMax(merged['block_chance'] ?? 0)
  const dodge = rangedMax(merged['dodge_chance'] ?? 0)
  const spellDodge = rangedMax(merged['dodge_spell_hits'] ?? 0)
  if (block > 0) avoidance.push(`block ${fmtPct(block)}%`)
  if (dodge > 0) avoidance.push(`dodge ${fmtPct(dodge)}%`)
  if (spellDodge > 0) avoidance.push(`spell dodge ${fmtPct(spellDodge)}%`)

  return (
    <div className="mb-3 border-b border-dashed border-accent-deep/25 pb-3">
      <div className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
        Avoidance:{' '}
        <span className="text-muted">
          {avoidance.length > 0 ? avoidance.join(' · ') : '—'}
        </span>
      </div>

      {insights.length > 0 && (
        <div className="mt-2 space-y-0.5">
          {insights.map((insight) => (
            <div
              key={insight.text}
              className="font-mono text-[10px] tracking-[0.06em] text-accent-hot/80"
            >
              ▸ {insight.text}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
