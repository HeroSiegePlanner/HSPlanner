import { EMPTY_EHP, formatEhp, type DamageType, type EhpResult } from '../../utils/build/ehp'
import { rangedMax } from '../../utils/item/stats'
import type { RangedValue } from '../../types'
import { BDLine, BDSection } from './statPrimitives'

interface EhpBreakdownProps {
  ehp: EhpResult | undefined
  stats: Record<string, RangedValue>
  statsCombined: Record<string, RangedValue>
}

const TYPE_COLOR: Record<DamageType, string> = {
  physical: 'text-muted',
  fire: 'text-stat-red',
  cold: 'text-stat-blue',
  lightning: 'text-stat-orange',
  poison: 'text-stat-green',
  arcane: 'text-stat-purple',
}

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1)
}

export function EhpBreakdown({ ehp = EMPTY_EHP, stats, statsCombined }: EhpBreakdownProps) {
  const { entries, worst } = ehp

  if (entries.length === 0) {
    return (
      <div className="py-2 text-center text-xs text-muted italic">
        Add life and defenses to see effective HP.
      </div>
    )
  }

  const life = Math.round(rangedMax(statsCombined['life'] ?? stats['life'] ?? 0))

  return (
    <>
      <BDLine
        label="Life"
        value={
          <span className="text-stat-red">{life.toLocaleString('en-US')}</span>
        }
      />
      <div className="mt-1 md:columns-2" style={{ columnGap: '2rem' }}>
        {entries.map((entry) => {
          const weakest = worst === entry.type
          return (
            <div
              key={entry.type}
              className="break-inside-avoid"
              style={{ breakInside: 'avoid' }}
            >
              <BDSection
                titleClass={TYPE_COLOR[entry.type]}
                title={
                  weakest
                    ? `${capitalize(entry.type)} · weakest`
                    : capitalize(entry.type)
                }
              >
                <BDLine
                  label="eHP"
                  value={
                    <span
                      className={weakest ? 'text-stat-red' : 'text-accent-hot'}
                    >
                      {formatEhp(entry.ehp)}
                    </span>
                  }
                />
                {entry.layers.length === 0 ? (
                  <BDLine indent label="No mitigation" value="—" />
                ) : (
                  entry.layers.map((layer) => (
                    <BDLine
                      key={layer.label}
                      indent
                      label={layer.label}
                      value={`${layer.pct}%`}
                    />
                  ))
                )}
              </BDSection>
            </div>
          )
        })}
      </div>
    </>
  )
}
