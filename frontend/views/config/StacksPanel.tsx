import { useMemo } from 'react'
import { gameConfig } from '@data'
import { useBuild } from '../../store/build'
import { useBuildPerformanceDeps } from '../../hooks/useBuildPerformanceDeps'
import { useCalcResult } from '../../hooks/useCalcResult'
import { computeBuildPerformanceAsync } from '../../utils/calc/bridge'
import type { BuildPerformance } from '../../utils/build/buildPerformance'
import { rangedMax, statDef, statName } from '../../utils/item/stats'
import { SkillIconImage } from '../../components/SkillIconImage'
import { CountBadge, Panel } from './configPrimitives'

// Sprite filename is the stack key: assets/stacks/<key>.png
const STACK_SPRITE_FILES = import.meta.glob<string>('../../assets/stacks/*.png', {
  eager: true,
  query: '?url',
  import: 'default',
})
const STACK_SPRITE_BY_KEY: Record<string, string> = {}
for (const [path, url] of Object.entries(STACK_SPRITE_FILES)) {
  const file = path.split('/').pop() ?? ''
  STACK_SPRITE_BY_KEY[file.replace(/\.png$/i, '')] = url
}

function effectLabel(statKey: string, amount: number): string {
  const unit = statDef(statKey)?.format === 'percent' ? '%' : ''
  return `+${amount}${unit} ${statName(statKey)}`
}

interface StackRow {
  key: string
  name: string
  icon?: string
  max: number
  count: number
  effects: string[]
}

export default function StacksPanel() {
  const stackCounts = useBuild((s) => s.stackCounts)
  const setStackCount = useBuild((s) => s.setStackCount)

  const deps = useBuildPerformanceDeps()
  const performance = useCalcResult<BuildPerformance | null>(
    () => computeBuildPerformanceAsync(deps),
    [deps],
    null,
  )

  const rows = useMemo<StackRow[]>(() => {
    const stats = performance?.stats
    if (!stats) return []
    return (gameConfig.stackTypes ?? []).flatMap((def) => {
      const max = Math.floor(rangedMax(stats[def.maxStat] ?? 0))
      if (max <= 0) return []
      const effects = [
        ...Object.entries(def.perStack ?? {}).map(([key, per]) =>
          effectLabel(key, per),
        ),
        ...Object.entries(def.perStackStats ?? {}).flatMap(
          ([rateKey, target]) => {
            const rate = rangedMax(stats[rateKey] ?? 0)
            return rate > 0 ? [effectLabel(target, rate)] : []
          },
        ),
      ]
      return [
        {
          key: def.key,
          name: def.name,
          icon: STACK_SPRITE_BY_KEY[def.key],
          max,
          count: Math.min(stackCounts[def.key] ?? max, max),
          effects,
        },
      ]
    })
  }, [performance, stackCounts])

  const loweredCount = rows.filter((r) => r.count < r.max).length

  if (rows.length === 0) return null

  return (
    <Panel
      title="Combat Stacks"
      subtitle="How many stacks to assume are up. Builds start at their cap; drop the count to see a colder rotation."
      trailing={
        <CountBadge value={loweredCount} highlight={loweredCount > 0} />
      }
    >
      <ul className="space-y-2">
        {rows.map((row) => {
          const lowered = row.count < row.max
          return (
            <li key={row.key}>
              <label
                className={`flex items-center justify-between gap-2 rounded-[3px] border px-2.5 py-2 text-sm transition-colors ${
                  lowered
                    ? 'border-accent-deep'
                    : 'border-border-2 hover:border-accent-deep'
                }`}
                style={{
                  background: lowered
                    ? 'linear-gradient(180deg, rgba(58,46,24,0.5), rgba(28,29,36,0.5))'
                    : 'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
                  boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.4)',
                }}
              >
                <span className="flex min-w-0 items-center gap-2">
                  {row.icon && (
                    <SkillIconImage icon={row.icon} size={28} className="text-2xl" />
                  )}
                  <span className="min-w-0">
                    <div
                      className={`truncate text-sm font-medium ${lowered ? 'text-accent-hot' : 'text-text'}`}
                    >
                      {row.name}
                    </div>
                    {row.effects.length > 0 && (
                      <div className="truncate text-[11px] text-muted">
                        {row.effects.join(' · ')} per stack
                      </div>
                    )}
                  </span>
                </span>
                <div
                  className="inline-flex shrink-0 items-center rounded-[3px] border border-border-2 px-1.5 transition-colors focus-within:border-accent-hot"
                  style={{
                    background:
                      'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
                    boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
                  }}
                >
                  <input
                    type="number"
                    min={0}
                    max={row.max}
                    value={row.count}
                    aria-label={`${row.name} stacks`}
                    onChange={(e) => {
                      const raw = e.target.value
                      if (raw === '') {
                        setStackCount(row.key, null)
                        return
                      }
                      const n = Number(raw)
                      if (!Number.isFinite(n)) return
                      setStackCount(
                        row.key,
                        Math.max(0, Math.min(row.max, Math.floor(n))),
                      )
                    }}
                    className="w-10 bg-transparent py-0.5 text-right font-mono text-[12px] tabular-nums text-accent-hot outline-none"
                  />
                  <span className="font-mono text-[10px] text-faint">
                    / {row.max}
                  </span>
                </div>
              </label>
            </li>
          )
        })}
      </ul>
    </Panel>
  )
}
