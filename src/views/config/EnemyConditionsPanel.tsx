import { useBuild } from '../../store/build'
import { CountBadge, Panel } from './primitives'
import { ENEMY_CONDITIONS } from './constants'

export default function EnemyConditionsPanel() {
  const enemyConditions = useBuild((s) => s.enemyConditions)
  const setEnemyCondition = useBuild((s) => s.setEnemyCondition)

  const activeConditionCount = ENEMY_CONDITIONS.filter(
    (c) => !!enemyConditions[c.key],
  ).length

  return (
    <Panel
      title="Enemy Conditions"
      subtitle='Conditions on the target'
      trailing={
        <CountBadge
          value={activeConditionCount}
          total={ENEMY_CONDITIONS.length}
          highlight={activeConditionCount > 0}
        />
      }
    >
      <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
        {ENEMY_CONDITIONS.map((c) => {
          const checked = !!enemyConditions[c.key]
          return (
            <label
              key={c.key}
              className={`flex cursor-pointer items-center gap-2 rounded-[3px] border px-3 py-2 text-sm transition-colors ${checked ? 'border-accent-deep' : 'border-border-2 hover:border-accent-deep'}`}
              style={{
                background: checked
                  ? 'linear-gradient(180deg, rgba(58,46,24,0.5), rgba(28,29,36,0.5))'
                  : 'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
                boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.4)',
              }}
            >
              <input
                type="checkbox"
                checked={checked}
                onChange={(e) =>
                  setEnemyCondition(c.key, e.target.checked)
                }
              />
              <span className={checked ? 'text-accent-hot' : 'text-text'}>
                {c.label}
              </span>
            </label>
          )
        })}
      </div>
      <p className="mt-3 font-mono text-[10px] uppercase tracking-[0.14em] text-faint italic">
        Future stat modifiers can reference these flags (currently UI-only).
      </p>
    </Panel>
  )
}
