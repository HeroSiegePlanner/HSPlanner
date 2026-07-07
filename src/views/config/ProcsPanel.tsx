import { useMemo } from 'react'
import { SkillIconImage } from '../../components/SkillIconImage'
import { resolveSkillIcon, skills } from '../../data'
import { subskillKey, useBuild } from '../../store/build'
import { normalizeSkillName } from '../../utils/item/stats'
import type { Skill, SubskillNode } from '../../types'
import { CountBadge, Panel } from './primitives'

export default function ProcsPanel() {
  const classId = useBuild((s) => s.classId)
  const skillRanks = useBuild((s) => s.skillRanks)
  const procToggles = useBuild((s) => s.procToggles)
  const setProcToggle = useBuild((s) => s.setProcToggle)
  const killsPerSec = useBuild((s) => s.killsPerSec)
  const setKillsPerSec = useBuild((s) => s.setKillsPerSec)
  const subskillRanks = useBuild((s) => s.subskillRanks)

  const procSkills = useMemo(() => {
    if (!classId) return []
    return skills.filter(
      (s) =>
        s.classId === classId && !!s.proc && (skillRanks[s.id] ?? 0) > 0,
    )
  }, [classId, skillRanks])

  const skillsByNormalizedName = useMemo(() => {
    if (!classId) return {} as Record<string, Skill>
    const out: Record<string, Skill> = {}
    for (const s of skills) {
      if (s.classId === classId) out[normalizeSkillName(s.name)] = s
    }
    return out
  }, [classId])

  const subskillProcs = useMemo(() => {
    type SubskillProcRow = {
      ownerSkill: Skill
      sub: SubskillNode
      toggleKey: string
      rank: number
      chance: number
      trigger: string
      target: string
    }
    if (!classId) return [] as SubskillProcRow[]
    const out: SubskillProcRow[] = []
    for (const ownerSkill of skills) {
      if (ownerSkill.classId !== classId) continue
      for (const sub of ownerSkill.subskills ?? []) {
        if (!sub.proc?.target) continue
        const toggleKey = subskillKey(ownerSkill.id, sub.id)
        const rank = subskillRanks[toggleKey] ?? 0
        if (rank === 0) continue
        const chance =
          (sub.proc.chance.base ?? 0) + (sub.proc.chance.perRank ?? 0) * rank
        out.push({
          ownerSkill,
          sub,
          toggleKey,
          rank,
          chance,
          trigger: sub.proc.trigger,
          target: sub.proc.target,
        })
      }
    }
    return out
  }, [classId, subskillRanks])

  const activeProcCount =
    procSkills.filter((p) => !!procToggles[p.id]).length +
    subskillProcs.filter((s) => !!procToggles[s.toggleKey]).length
  const totalProcCount = procSkills.length + subskillProcs.length

  return (
    <Panel
      title="Procs"
      subtitle="Skills (and subtree nodes) that trigger another skill on hit / kill / cast."
      trailing={
        <CountBadge
          value={activeProcCount}
          total={totalProcCount}
          highlight={activeProcCount > 0}
        />
      }
    >
      {totalProcCount === 0 ? (
        <p className="font-mono text-[12px] tracking-[0.04em] text-muted italic">
          {classId
            ? 'No proc skills or subtree nodes allocated for this class.'
            : 'Pick a class first.'}
        </p>
      ) : (
        <>
          <div className="mb-3 flex items-center justify-between gap-2">
            <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
              Kills / sec
            </span>
            <div
              className="inline-flex w-20 shrink-0 items-center rounded-[3px] border border-border-2 px-2 py-1 transition-colors focus-within:border-accent-hot"
              style={{
                background:
                  'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
                boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
              }}
            >
              <input
                type="number"
                min={0}
                step={0.5}
                value={killsPerSec}
                onChange={(e) => {
                  const raw = e.target.value
                  if (raw === '') {
                    setKillsPerSec(0)
                    return
                  }
                  const n = Number(raw)
                  if (!Number.isFinite(n)) return
                  setKillsPerSec(Math.max(0, n))
                }}
                className="w-full bg-transparent text-right font-mono text-[12px] tabular-nums text-accent-hot outline-none"
              />
            </div>
          </div>
          <ul className="space-y-2">
            {procSkills.map((p) => {
              const targetName = normalizeSkillName(p.proc!.target)
              const target = skillsByNormalizedName[targetName]
              const targetRank = target ? (skillRanks[target.id] ?? 0) : 0
              const ready = !!target && targetRank > 0
              const checked = !!procToggles[p.id]
              return (
                <li key={p.id}>
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
                        onChange={(e) => {
                          setProcToggle(p.id, e.target.checked)
                        }}
                        disabled={!ready}
                      />
                      <SkillIconImage
                        icon={resolveSkillIcon(p)}
                        size={32}
                        className="text-2xl"
                      />
                      <span className="min-w-0">
                        <div
                          className={`truncate text-sm font-medium ${checked ? 'text-accent-hot' : 'text-text'}`}
                        >
                          {p.name}
                        </div>
                        <div className="truncate font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
                          → {p.proc!.target}
                          {!ready && ' · target not allocated'}
                        </div>
                      </span>
                    </span>
                    <span className="shrink-0 font-mono text-[10px] uppercase tracking-[0.14em] text-faint tabular-nums">
                      <span className="text-text">{p.proc!.chance}%</span>
                      {' · '}
                      {p.proc!.trigger.replace('on_', '')}
                    </span>
                  </label>
                </li>
              )
            })}
          </ul>
          {subskillProcs.length > 0 && (
            <>
              <div className="mt-4 mb-2 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
                From Subtree Nodes
              </div>
              <ul className="space-y-2">
                {subskillProcs.map((entry) => {
                  const targetName = normalizeSkillName(entry.target)
                  const target = skillsByNormalizedName[targetName]
                  const targetRank = target
                    ? (skillRanks[target.id] ?? 0)
                    : 0
                  const ready = !!target && targetRank > 0
                  const checked = !!procToggles[entry.toggleKey]
                  return (
                    <li key={entry.toggleKey}>
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
                            onChange={(e) => {
                              setProcToggle(
                                entry.toggleKey,
                                e.target.checked,
                              )
                            }}
                            disabled={!ready}
                          />
                          <SkillIconImage
                            icon={resolveSkillIcon(entry.ownerSkill)}
                            size={32}
                            className="text-2xl"
                          />
                          <span className="min-w-0">
                            <div
                              className={`truncate text-sm font-medium ${checked ? 'text-accent-hot' : 'text-text'}`}
                            >
                              {entry.sub.name}
                              <span className="ml-1 text-[10px] font-normal uppercase tracking-[0.14em] text-faint">
                                · {entry.ownerSkill.name}
                              </span>
                            </div>
                            <div className="truncate font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
                              → {entry.target}
                              {!ready && ' · target not allocated'}
                            </div>
                          </span>
                        </span>
                        <span className="shrink-0 font-mono text-[10px] uppercase tracking-[0.14em] text-faint tabular-nums">
                          <span className="text-text">{entry.chance}%</span>
                          {' · '}
                          {entry.trigger.replace('on_', '')}
                        </span>
                      </label>
                    </li>
                  )
                })}
              </ul>
            </>
          )}
        </>
      )}
    </Panel>
  )
}
