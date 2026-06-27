import { useBuild } from '../../store/build'
import { CountBadge, Panel } from './primitives'
import { PLAYER_CONDITIONS } from './constants'

export default function PlayerConditionsPanel() {
  const playerConditions = useBuild((s) => s.playerConditions)
  const setPlayerCondition = useBuild((s) => s.setPlayerCondition)

  const activePlayerConditionCount = PLAYER_CONDITIONS.filter(
    (c) => !!playerConditions[c.key],
  ).length

  return (
    <Panel
      title="Player Conditions"
      subtitle='Self-state flags'
      trailing={
        <CountBadge
          value={activePlayerConditionCount}
          total={PLAYER_CONDITIONS.length}
          highlight={activePlayerConditionCount > 0}
        />
      }
    >
      <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
        {PLAYER_CONDITIONS.map((c) => {
          const checked = !!playerConditions[c.key]
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
                  setPlayerCondition(c.key, e.target.checked)
                }
              />
              <span className={checked ? 'text-accent-hot' : 'text-text'}>
                {c.label}
              </span>
            </label>
          )
        })}
      </div>
    </Panel>
  )
}
