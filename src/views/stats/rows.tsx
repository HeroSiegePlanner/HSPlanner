import SourceTooltip from '../../components/SourceTooltip'
import {
  effectiveCap,
  formatValue,
  isZero,
  statDef,
  statName,
} from '../../utils/item/stats'
import type { SourceContribution } from '../../utils/item/stats'
import type { StatBreakdown, StatBreakdownKind } from '../../lib/calc/bridge'
import type { AttributeKey, RangedStatMap, RangedValue } from '../../types'
import { displayRange } from './format'

export function AttributesStrip({
  attrs,
  attributes,
  attributeSources,
  matches,
  getBreakdown,
  requestBreakdown,
}: {
  attrs: Array<{ key: string; name: string }>
  attributes: Record<AttributeKey, RangedValue>
  attributeSources: Record<string, SourceContribution[]>
  matches: (label: string) => boolean
  getBreakdown: (statKey: string, kind: StatBreakdownKind) => StatBreakdown | null
  requestBreakdown: (statKey: string, kind: StatBreakdownKind) => void
}) {
  return (
    <div
      className="grid grid-cols-2 overflow-hidden rounded-[3px] border border-border sm:grid-cols-3 lg:grid-cols-6"
      style={{ gap: 1, background: 'var(--color-border)' }}
    >
      {attrs.map((attr) => {
        const final: RangedValue = attributes[attr.key as AttributeKey] ?? 0
        const sources = attributeSources[attr.key] ?? []
        const [fmin, fmax] =
          typeof final === 'number' ? [final, final] : final
        const highlighted = matches(attr.name)
        return (
          <SourceTooltip
            key={attr.key}
            statKey={attr.key}
            sources={sources}
            breakdown={getBreakdown(attr.key, 'attribute')}
            onRequestBreakdown={() => requestBreakdown(attr.key, 'attribute')}
            title={attr.name}
          >
            <div
              className={`flex flex-col gap-1 px-3 py-2.5 transition-colors ${
                highlighted ? 'bg-accent-hot/10' : 'hover:bg-panel-3/40'
              }`}
              style={{
                background: highlighted
                  ? undefined
                  : 'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
              }}
            >
              <span className="font-mono text-[9px] uppercase tracking-[0.16em] text-faint">
                {attr.name}
              </span>
              <span
                className={`font-mono text-[15px] font-semibold tabular-nums ${
                  highlighted ? 'text-accent-hot' : 'text-text'
                }`}
              >
                {displayRange(fmin, fmax)}
              </span>
            </div>
          </SourceTooltip>
        )
      })}
    </div>
  )
}

export function StatRow({
  statKey,
  value,
  sources,
  moreValue,
  moreSources,
  highlighted,
  stats,
  statsCombined,
  breakdown,
  onRequestBreakdown,
}: {
  statKey: string
  value: RangedValue
  sources: SourceContribution[]
  moreValue?: RangedValue
  moreSources?: SourceContribution[]
  highlighted: boolean
  stats: RangedStatMap
  statsCombined: Record<string, RangedValue>
  breakdown: StatBreakdown | null
  onRequestBreakdown: () => void
}) {
  const hasMore = !!moreSources && moreSources.length > 0
  const displayValue: RangedValue = hasMore
    ? (statsCombined[statKey] ?? value)
    : value
  const zero = isZero(displayValue) && (!hasMore || isZero(moreValue ?? 0))
  const def = statDef(statKey)
  const cap = effectiveCap(statKey, stats)
  const overflow =
    cap !== undefined &&
    !zero &&
    typeof displayValue === 'number' &&
    displayValue > cap
  const suffix = def?.format === 'percent' ? '%' : ''
  return (
    <li>
      <SourceTooltip
        statKey={statKey}
        sources={sources}
        moreSources={moreSources}
        breakdown={breakdown}
        onRequestBreakdown={onRequestBreakdown}
      >
        <div
          className={`-mx-2 flex items-baseline justify-between gap-2 rounded-xs px-2 py-1 text-[13px] transition-colors ${
            highlighted
              ? 'bg-accent-hot/10 ring-1 ring-accent-deep/50'
              : 'hover:bg-accent-hot/4'
          } ${zero && !highlighted ? 'opacity-35' : ''}`}
        >
          <span className="text-text">{statName(statKey)}</span>
          <span
            className={`font-mono font-medium tabular-nums ${
              zero ? 'text-faint' : 'text-accent-hot'
            }`}
          >
            {zero ? (
              '—'
            ) : overflow ? (
              <>
                +{cap}
                {suffix}{' '}
                <span className="text-faint font-normal">
                  ({displayValue as number}
                  {suffix})
                </span>
              </>
            ) : (
              formatValue(displayValue, statKey)
            )}
          </span>
        </div>
      </SourceTooltip>
    </li>
  )
}
