import { useMemo } from 'react'
import SearchableSelect from '../../components/SearchableSelect'
import { gameConfig } from '../../data'
import { useBuild } from '../../store/build'
import { useCalcResult } from '../../hooks/useCalcResult'
import { parseCustomStatsNative } from '../../lib/calc/bridge'
import {
  dedupeStatDefsByKey,
  formatValue,
  statDef,
} from '../../utils/item/stats'
import { CountBadge, Panel } from './primitives'

export default function CustomStatsPanel() {
  const customStats = useBuild((s) => s.customStats)
  const addCustomStat = useBuild((s) => s.addCustomStat)
  const updateCustomStat = useBuild((s) => s.updateCustomStat)
  const removeCustomStat = useBuild((s) => s.removeCustomStat)
  const commitActiveProfile = useBuild((s) => s.commitActiveProfile)

  const parsedCustomValues = useCalcResult<([number, number] | null)[]>(
    () =>
      customStats.length === 0
        ? []
        : parseCustomStatsNative(customStats.map((cs) => cs.value)),
    [customStats],
    [],
  )

  const statOptions = useMemo(
    () =>
      dedupeStatDefsByKey(
        gameConfig.stats.filter((s) => !s.itemOnly && !s.skillScoped),
      ).map((s) => ({
        id: s.key,
        label: s.name,
        hint: s.format === 'percent' ? '%' : 'flat',
      })),
    [],
  )

  return (
    <Panel
      title="Custom Config"
      subtitle="Add stats the engine doesn't compute yet. Pick a stat, enter a value — it stacks with regular sources and shows up in tooltips. Per-profile."
      trailing={
        <CountBadge
          value={customStats.length}
          highlight={customStats.length > 0}
        />
      }
    >
      {customStats.length === 0 ? (
        <p className="font-mono text-[12px] tracking-[0.04em] text-muted italic">
          No custom stats yet. Click "Add stat" to override or supplement a
          calculated stat (e.g. Faster Cast Rate from a buff the engine
          doesn't know).
        </p>
      ) : (
        <ul className="space-y-2">
          {customStats.map((cs, i) => {
            const parsedPair = parsedCustomValues[i] ?? null
            const parsed =
              parsedPair === null
                ? null
                : parsedPair[0] === parsedPair[1]
                  ? parsedPair[0]
                  : parsedPair
            const def = cs.statKey ? statDef(cs.statKey) : undefined
            const willApply = !!cs.statKey && parsed !== null
            const previewText =
              willApply && parsed !== null
                ? `→ ${formatValue(parsed, cs.statKey)} ${def?.name ?? cs.statKey}`
                : cs.statKey
                  ? 'unparseable value'
                  : 'pick a stat to apply'
            return (
              <li
                key={i}
                className="space-y-1.5 rounded-[3px] border border-border-2 px-2.5 py-2"
                style={{
                  background:
                    'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
                  boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.4)',
                }}
              >
                <div className="flex items-center gap-2">
                  <div className="min-w-0 flex-1">
                    <SearchableSelect
                      value={cs.statKey || null}
                      options={statOptions}
                      placeholder="Pick stat…"
                      onChange={(id) => {
                        updateCustomStat(i, { statKey: id ?? '' })
                        commitActiveProfile()
                      }}
                    />
                  </div>
                  <div
                    className="inline-flex w-40 shrink-0 items-center rounded-[3px] border border-border-2 px-2 py-1 transition-colors focus-within:border-accent-hot"
                    style={{
                      background:
                        'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
                      boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
                    }}
                  >
                    <input
                      value={cs.value}
                      placeholder="Value (e.g. 100, 100%, 12-18)"
                      onChange={(e) =>
                        updateCustomStat(i, { value: e.target.value })
                      }
                      onBlur={() => commitActiveProfile()}
                      className="w-full bg-transparent text-right font-mono text-[12px] tabular-nums text-text outline-none"
                    />
                  </div>
                  <button
                    onClick={() => {
                      removeCustomStat(i)
                      commitActiveProfile()
                    }}
                    className="rounded-[3px] border border-border-2 bg-transparent px-2 py-1 font-mono text-[12px] text-muted transition-colors hover:border-stat-red hover:text-stat-red"
                    title="Remove"
                    aria-label="Remove"
                  >
                    ×
                  </button>
                </div>
                <div
                  className={`font-mono text-[10px] uppercase tracking-[0.14em] ${
                    willApply
                      ? 'text-stat-green'
                      : cs.statKey
                        ? 'text-stat-orange'
                        : 'text-faint italic'
                  }`}
                >
                  {previewText}
                </div>
              </li>
            )
          })}
        </ul>
      )}
      <button
        onClick={() => addCustomStat()}
        className="mt-3 rounded-[3px] border border-accent-deep px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]"
        style={{
          background: 'linear-gradient(180deg, #3a2f1a, #2a2418)',
        }}
      >
        + Add stat
      </button>
    </Panel>
  )
}
