import { useMemo, useState } from 'react'
import FlashOnChange from '../components/FlashOnChange'
import SubtreeOverlay from '../components/SubtreeOverlay'
import { classes, getClass, skills } from '../data'
import { useBuildPerformanceDeps } from '../hooks/useBuildPerformanceDeps'
import { useCalcResult } from '../hooks/useCalcResult'
import { computeBuildStatsAsync } from '../lib/calc/bridge'
import { skillPointsFor, useBuild } from '../store/build'
import { normalizeSkillName, rangedMax, rangedMin } from '../utils/item/stats'
import type { ComputedStats } from '../utils/item/stats'
import type { Skill } from '../types'
import { SkillTree } from './skills/SkillTree'
import { skillLevelBonus } from './skills/skillBonus'
import { SkillDetailsPanel, EmptyState } from './skills/SkillDetailsPanel'

export default function SkillsView() {
  const classId = useBuild((s) => s.classId)
  const level = useBuild((s) => s.level)
  const skillRanks = useBuild((s) => s.skillRanks)
  const subskillRanks = useBuild((s) => s.subskillRanks)
  const enemyConditions = useBuild((s) => s.enemyConditions)
  const incSkillRank = useBuild((s) => s.incSkillRank)
  const decSkillRank = useBuild((s) => s.decSkillRank)
  const activeSkillIds = useBuild((s) => s.activeSkillIds)
  const toggleActiveSkill = useBuild((s) => s.toggleActiveSkill)
  const resetSkillRanks = useBuild((s) => s.resetSkillRanks)
  const [hovered, setHovered] = useState<string | null>(null)
  const [pinned, setPinned] = useState<string | null>(null)
  const [openSubtree, setOpenSubtree] = useState<string | null>(null)
  const [synergyNode, setSynergyNode] = useState<string | null>(null)

  const [prevClassId, setPrevClassId] = useState(classId)
  if (prevClassId !== classId) {
    setPrevClassId(classId)
    setHovered(null)
    setPinned(null)
    setOpenSubtree(null)
    setSynergyNode(null)
  }

  const selected = pinned

  const cls = classId ? getClass(classId) : undefined
  const skillsForClass = useMemo(
    () => (classId ? skills.filter((s) => s.classId === classId) : []),
    [classId],
  )

  const buildDeps = useBuildPerformanceDeps()
  const computed = useCalcResult<ComputedStats | null>(
    () => computeBuildStatsAsync(buildDeps),
    [buildDeps],
    null,
  )
  const stats = computed?.stats ?? {}
  const attributes = computed?.attributes ?? {}
  const itemSkillBonuses = useMemo(
    () => computed?.itemSkillBonuses ?? {},
    [computed],
  )
  const rankBonuses = useMemo(() => computed?.rankBonuses ?? {}, [computed])

  const skillBonuses = useMemo(() => {
    const s = computed?.stats ?? {}
    const map: Record<string, [number, number]> = {}
    for (const sk of skillsForClass) {
      const [min, max] = skillLevelBonus(
        sk,
        skillRanks[sk.id] ?? 0,
        s,
        itemSkillBonuses,
      )
      if (min !== 0 || max !== 0) map[sk.id] = [min, max]
    }
    return map
  }, [computed, skillsForClass, skillRanks, itemSkillBonuses])

  const trees = useMemo(() => {
    const byTree = new Map<string, Skill[]>()
    for (const s of skillsForClass) {
      const key = s.tree ?? 'Ungrouped'
      const list = byTree.get(key) ?? []
      list.push(s)
      byTree.set(key, list)
    }
    return [...byTree.entries()].map(([name, list]) => ({ name, list }))
  }, [skillsForClass])

  const totalPoints = skillPointsFor(level)
  const spent = Object.values(skillRanks).reduce((s, v) => s + v, 0)
  const remaining = totalPoints - spent
  const selectedSkill = selected
    ? skillsForClass.find((s) => s.id === selected) ?? null
    : null
  const openSubtreeSkill = openSubtree
    ? skillsForClass.find((s) => s.id === openSubtree) ?? null
    : null

  if (classes.length === 0) {
    return (
      <EmptyState message="No classes loaded. Add a file in src/data/classes/." />
    )
  }
  if (skillsForClass.length === 0) {
    return (
      <EmptyState
        message={`No skills defined for ${cls?.name ?? 'this class'}. Add JSON in src/data/skills/.`}
      />
    )
  }

  return (
    <div className="flex h-full flex-col">
      <header
        className="flex items-center justify-between gap-3 border-b border-border px-5 py-3"
        style={{
          background:
            'linear-gradient(180deg, rgba(201,165,90,0.05), transparent)',
        }}
      >
        <div className="flex items-center gap-3">
          <span
            aria-hidden
            className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
            style={{ boxShadow: '0 0 8px rgba(224,184,100,0.6)' }}
          />
          <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
            Skills
          </span>
          <span
            className="text-[15px] font-semibold tracking-[0.02em] text-accent-hot"
            style={{ textShadow: '0 0 14px rgba(224,184,100,0.18)' }}
          >
            {cls?.name}
          </span>
        </div>
        <div className="flex items-center gap-3 font-mono text-[10px] uppercase tracking-[0.14em]">
          <span className="text-faint">Points</span>
          <span
            className={`tabular-nums ${remaining > 0 ? 'text-accent-hot' : 'text-muted'}`}
            style={
              remaining > 0
                ? { textShadow: '0 0 8px rgba(224,184,100,0.25)' }
                : undefined
            }
          >
            <FlashOnChange value={spent}>{spent}</FlashOnChange>
          </span>
          <span className="text-faint">/ {totalPoints}</span>
          <span aria-hidden className="h-3 w-px bg-border" />
          <span className={remaining > 0 ? 'text-accent-hot' : 'text-faint'}>
            {remaining} available
          </span>
          <button
            onClick={resetSkillRanks}
            disabled={spent === 0}
            className="rounded-[3px] border border-border-2 bg-transparent px-3 py-1 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-muted transition-colors hover:border-stat-red hover:text-stat-red disabled:cursor-not-allowed disabled:opacity-40"
          >
            Reset
          </button>
        </div>
      </header>

      <div className="flex flex-1 overflow-hidden">
        <div className="flex-1 overflow-auto p-6">
          <div className="flex flex-wrap content-start justify-center gap-6">
            {trees.map((tree) => (
              <SkillTree
                key={tree.name}
                name={tree.name}
                list={tree.list}
                skillRanks={skillRanks}
                skillBonuses={skillBonuses}
                canIncrement={remaining > 0}
                hoveredId={hovered}
                selectedId={pinned}
                highlightId={synergyNode}
                onHover={setHovered}
                onSelect={setPinned}
                onInc={incSkillRank}
                onDec={decSkillRank}
                onOpenSubtree={setOpenSubtree}
              />
            ))}
          </div>
        </div>
        <SkillDetailsPanel
          skill={selectedSkill}
          currentRank={selectedSkill ? skillRanks[selectedSkill.id] ?? 0 : 0}
          allSkillsBonus={[
            rangedMin(stats.all_skills ?? 0),
            rangedMax(stats.all_skills ?? 0),
          ]}
          elementSkillsBonus={
            selectedSkill?.damageType
              ? [
                  rangedMin(stats[`${selectedSkill.damageType}_skills`] ?? 0),
                  rangedMax(stats[`${selectedSkill.damageType}_skills`] ?? 0),
                ]
              : [0, 0]
          }
          itemBonus={
            selectedSkill
              ? (itemSkillBonuses[normalizeSkillName(selectedSkill.name)] ?? [
                  0, 0,
                ])
              : [0, 0]
          }
          allClassSkills={skillsForClass}
          skillRanks={skillRanks}
          attributes={attributes}
          subskillRanks={subskillRanks}
          enemyConditions={enemyConditions}
          rankBonuses={rankBonuses}
          buffingAuraEffectiveness={stats.buffing_aura_effectiveness ?? 0}
          onSynergyHover={setSynergyNode}
          activeSkillIds={activeSkillIds}
          onToggleActive={toggleActiveSkill}
        />
      </div>
      {openSubtreeSkill && (
        <SubtreeOverlay
          skill={openSubtreeSkill}
          onClose={() => setOpenSubtree(null)}
        />
      )}
    </div>
  )
}
