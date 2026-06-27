import { useMemo } from 'react'
import { SkillIconImage } from '../../components/SkillIconImage'
import { resolveSkillIcon, skills } from '../../data'
import { useBuild } from '../../store/build'
import { CountBadge, Panel } from './primitives'

export default function ActiveBuffsPanel() {
  const classId = useBuild((s) => s.classId)
  const skillRanks = useBuild((s) => s.skillRanks)
  const activeBuffs = useBuild((s) => s.activeBuffs)
  const setBuffActive = useBuild((s) => s.setBuffActive)

  const buffSkills = useMemo(() => {
    if (!classId) return []
    return skills.filter(
      (s) =>
        s.classId === classId &&
        (s.kind === 'buff' || (s.tags?.includes('Buff') ?? false)),
    )
  }, [classId])

  const activeBuffCount = buffSkills.filter((s) => !!activeBuffs[s.id]).length

  return (
    <Panel
      title="Active Buffs"
      subtitle="Enable buffs you have cast and are currently active."
      trailing={
        <CountBadge
          value={activeBuffCount}
          total={buffSkills.length}
          highlight={activeBuffCount > 0}
        />
      }
    >
      {buffSkills.length === 0 ? (
        <p className="font-mono text-[12px] tracking-[0.04em] text-muted italic">
          No buffs available for this class.
        </p>
      ) : (
        <ul className="space-y-2">
          {buffSkills.map((s) => {
            const rank = skillRanks[s.id] ?? 0
            const ready = rank > 0
            const checked = !!activeBuffs[s.id]
            return (
              <li key={s.id}>
                <label
                  className={`flex items-center justify-between gap-3 rounded-[3px] border px-3 py-2 transition-colors ${
                    ready
                      ? checked
                        ? 'cursor-pointer border-accent-deep'
                        : 'cursor-pointer border-border-2 hover:border-accent-deep'
                      : 'border-border opacity-60'
                  }`}
                  style={{
                    background: checked
                      ? 'linear-gradient(180deg, rgba(58,46,24,0.5), rgba(28,29,36,0.5))'
                      : 'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
                    boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.4)',
                  }}
                >
                  <span className="flex min-w-0 items-center gap-2">
                    <input
                      type="checkbox"
                      checked={checked}
                      onChange={(e) =>
                        setBuffActive(s.id, e.target.checked)
                      }
                      disabled={!ready}
                    />
                    <SkillIconImage
                      icon={resolveSkillIcon(s)}
                      size={32}
                      className="text-2xl"
                    />
                    <span className="min-w-0">
                      <div
                        className={`truncate text-sm font-medium ${checked ? 'text-accent-hot' : 'text-text'}`}
                      >
                        {s.name}
                      </div>
                    </span>
                  </span>
                  {s.effectDuration !== undefined && (
                    <span className="shrink-0 font-mono text-[10px] uppercase tracking-[0.14em] text-faint tabular-nums">
                      <span className="text-text">{s.effectDuration}</span>
                      s duration
                    </span>
                  )}
                </label>
              </li>
            )
          })}
        </ul>
      )}
    </Panel>
  )
}
