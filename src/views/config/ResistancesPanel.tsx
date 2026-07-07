import { useBuild } from '../../store/build'
import { CountBadge, Panel } from './primitives'
import { ENEMY_RESISTANCE_TYPES, RESIST_COLOR } from './constants'

export default function ResistancesPanel() {
  const enemyResistances = useBuild((s) => s.enemyResistances)
  const setEnemyResistance = useBuild((s) => s.setEnemyResistance)

  const activeResistanceCount = ENEMY_RESISTANCE_TYPES.filter(
    (r) => enemyResistances[r.key] !== undefined && enemyResistances[r.key] !== null,
  ).length

  return (
    <Panel
      title="Enemy Resistances"
      subtitle="Per-element resistance % the target has. Damage modifier = 1 − (Enemy Res × (1 − Ignore)). 100% Ignore fully bypasses resistance; lower values help proportionally even against immune targets."
      trailing={
        <CountBadge
          value={activeResistanceCount}
          total={ENEMY_RESISTANCE_TYPES.length}
          highlight={activeResistanceCount > 0}
        />
      }
    >
      <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
        {ENEMY_RESISTANCE_TYPES.map((r) => {
          const value = enemyResistances[r.key]
          const colorClass = RESIST_COLOR[r.key] ?? 'text-text'
          return (
            <label
              key={r.key}
              className="flex items-center justify-between gap-2 rounded-[3px] border border-border-2 px-3 py-2 text-sm transition-colors hover:border-accent-deep"
              style={{
                background:
                  'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
                boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.4)',
              }}
            >
              <span
                className={`font-mono text-[10px] font-semibold uppercase tracking-[0.18em] ${colorClass}`}
              >
                {r.label}
              </span>
              <div
                className="inline-flex items-center rounded-[3px] border border-border-2 px-1.5 transition-colors focus-within:border-accent-hot"
                style={{
                  background:
                    'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
                  boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
                }}
              >
                <input
                  type="number"
                  value={value ?? ''}
                  placeholder="0"
                  onChange={(e) => {
                    const raw = e.target.value
                    if (raw === '') {
                      setEnemyResistance(r.key, null)
                      return
                    }
                    const n = Number(raw)
                    if (!Number.isFinite(n)) return
                    setEnemyResistance(r.key, n)
                  }}
                  className="w-12 bg-transparent py-0.5 text-right font-mono text-[12px] tabular-nums text-accent-hot outline-none"
                />
                <span className="font-mono text-[11px] text-faint">%</span>
              </div>
            </label>
          )
        })}
      </div>
    </Panel>
  )
}
