import { useMemo } from 'react'
import { conditionalItemGrantedSkills, getItem } from '../../data'
import { useBuild } from '../../store/build'
import { CountBadge, Panel } from './primitives'

export default function ItemBlessingsPanel() {
  const playerConditions = useBuild((s) => s.playerConditions)
  const setPlayerCondition = useBuild((s) => s.setPlayerCondition)
  const inventory = useBuild((s) => s.inventory)

  // Only surface blessings granted by a currently-equipped item.
  const blessings = useMemo(() => {
    const granted = new Set<string>()
    for (const equipped of Object.values(inventory)) {
      const base = equipped && getItem(equipped.baseId)
      if (!base?.skillBonuses) continue
      for (const name of Object.keys(base.skillBonuses)) {
        granted.add(name.trim().toLowerCase())
      }
    }
    return conditionalItemGrantedSkills().filter((b) =>
      granted.has(b.name.trim().toLowerCase()),
    )
  }, [inventory])

  if (blessings.length === 0) return null

  const activeCount = blessings.filter(
    (b) => !!playerConditions[b.condition!],
  ).length

  return (
    <Panel
      title="Item Blessings"
      subtitle="Conditional item-granted effects"
      trailing={
        <CountBadge
          value={activeCount}
          total={blessings.length}
          highlight={activeCount > 0}
        />
      }
    >
      <div className="grid grid-cols-1 gap-2">
        {blessings.map((b) => {
          const checked = !!playerConditions[b.condition!]
          return (
            <label
              key={b.id}
              className={`flex cursor-pointer items-start gap-2 rounded-[3px] border px-3 py-2 text-sm transition-colors ${checked ? 'border-accent-deep' : 'border-border-2 hover:border-accent-deep'}`}
              style={{
                background: checked
                  ? 'linear-gradient(180deg, rgba(58,46,24,0.5), rgba(28,29,36,0.5))'
                  : 'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
                boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.4)',
              }}
            >
              <input
                type="checkbox"
                className="mt-0.5"
                checked={checked}
                onChange={(e) =>
                  setPlayerCondition(b.condition!, e.target.checked)
                }
              />
              <span className="min-w-0">
                <span className={checked ? 'text-accent-hot' : 'text-text'}>
                  {b.name}
                </span>
                {b.description && (
                  <span className="block text-[10px] leading-snug text-faint">
                    {b.description}
                  </span>
                )}
              </span>
            </label>
          )
        })}
      </div>
    </Panel>
  )
}
