import { useMemo } from 'react'
import { SkillIconImage } from '../../components/SkillIconImage'
import { resolveSkillIcon, skills } from '../../data'
import { useBuild } from '../../store/build'
import { CountBadge, Panel } from './primitives'

export default function SkillProjectilesPanel() {
  const classId = useBuild((s) => s.classId)
  const skillRanks = useBuild((s) => s.skillRanks)
  const skillProjectiles = useBuild((s) => s.skillProjectiles)
  const setSkillProjectiles = useBuild((s) => s.setSkillProjectiles)
  const commitActiveProfile = useBuild((s) => s.commitActiveProfile)

  const damageSkills = useMemo(() => {
    if (!classId) return []
    return skills.filter(
      (s) =>
        s.classId === classId &&
        s.kind === 'active' &&
        (!!s.damageFormula ||
          (!!s.damagePerRank && s.damagePerRank.length > 0)),
    )
  }, [classId])

  const skillProjectileCount = Object.values(skillProjectiles).filter(
    (n) => n > 1,
  ).length

  return (
    <Panel
      title="Skill Projectile Counts"
      subtitle="Manual override of how many projectiles a skill fires per cast (e.g. Multi Shot = 5, Fan of Knives = 7). Multiplies that skill's per-cast damage and DPS. Leave empty / set to 1 for skills that fire a single projectile."
      trailing={
        <CountBadge
          value={skillProjectileCount}
          highlight={skillProjectileCount > 0}
        />
      }
    >
      {damageSkills.length === 0 ? (
        <p className="font-mono text-[12px] tracking-[0.04em] text-muted italic">
          {classId
            ? 'No damage skills available for this class.'
            : 'Pick a class first.'}
        </p>
      ) : (
        <ul className="grid grid-cols-1 gap-2 sm:grid-cols-2">
          {damageSkills.map((s) => {
            const rank = skillRanks[s.id] ?? 0
            const value = skillProjectiles[s.id] ?? 1
            const learned = rank > 0
            return (
              <li key={s.id}>
                <label
                  className={`flex items-center justify-between gap-2 rounded-[3px] border px-2.5 py-2 text-sm transition-colors ${
                    value > 1
                      ? 'border-accent-deep'
                      : learned
                        ? 'border-border-2 hover:border-accent-deep'
                        : 'border-border opacity-60'
                  }`}
                  style={{
                    background:
                      value > 1
                        ? 'linear-gradient(180deg, rgba(58,46,24,0.5), rgba(28,29,36,0.5))'
                        : 'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
                    boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.4)',
                  }}
                >
                  <span className="flex min-w-0 items-center gap-2">
                    <SkillIconImage
                      icon={resolveSkillIcon(s)}
                      size={28}
                      className="text-2xl"
                    />
                    <span className="min-w-0">
                      <div
                        className={`truncate text-sm font-medium ${value > 1 ? 'text-accent-hot' : 'text-text'}`}
                      >
                        {s.name}
                      </div>
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
                      min={1}
                      max={99}
                      value={value}
                      onChange={(e) => {
                        const raw = e.target.value
                        if (raw === '') {
                          setSkillProjectiles(s.id, null)
                          commitActiveProfile()
                          return
                        }
                        const n = Number(raw)
                        if (!Number.isFinite(n)) return
                        setSkillProjectiles(s.id, Math.max(1, Math.min(99, Math.floor(n))))
                        commitActiveProfile()
                      }}
                      className="w-10 bg-transparent py-0.5 text-right font-mono text-[12px] tabular-nums text-accent-hot outline-none"
                    />
                    <span className="font-mono text-[10px] text-faint">×</span>
                  </div>
                </label>
              </li>
            )
          })}
        </ul>
      )}
    </Panel>
  )
}
