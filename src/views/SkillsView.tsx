import { useEffect, useMemo, useState, type ReactNode } from 'react'
import { CornerMarks } from '../components/CornerMarks'
import { motion } from 'motion/react'
import FlashOnChange from '../components/FlashOnChange'
import { SkillIconImage } from '../components/SkillIconImage'
import SubtreeOverlay from '../components/SubtreeOverlay'
import { listContainerVariants, skillIconVariants } from '../lib/motion'
import { classes, getClass, resolveSkillIcon, skills } from '../data'
import { useBuildPerformanceDeps } from '../hooks/useBuildPerformanceDeps'
import { useSkillRankInfo } from '../hooks/useSkillRankInfo'
import {
  computeBuildStatsAsync,
  subskillAggregationNative,
} from '../lib/calc/bridge'
import type { SubtreeAggregation } from '../lib/calc/bridge'
import { skillPointsFor, subskillKey, useBuild } from '../store/build'
import { DAMAGE_COLORS } from '../utils/damageColors'
import {
  formatValue,
  normalizeSkillName,
  rangedMax,
  rangedMin,
  statName,
} from '../utils/item/stats'
import type { ComputedStats } from '../utils/item/stats'
import type {
  AttributeKey,
  DamageType,
  RangedValue,
  Skill,
  SubskillNode,
} from '../types'

const CELL = 84
const GAP = 18

export default function SkillsView() {
  const classId = useBuild((s) => s.classId)
  const level = useBuild((s) => s.level)
  const skillRanks = useBuild((s) => s.skillRanks)
  const subskillRanks = useBuild((s) => s.subskillRanks)
  const enemyConditions = useBuild((s) => s.enemyConditions)
  const incSkillRank = useBuild((s) => s.incSkillRank)
  const decSkillRank = useBuild((s) => s.decSkillRank)
  const resetSkillRanks = useBuild((s) => s.resetSkillRanks)
  const [hovered, setHovered] = useState<string | null>(null)
  const [pinned, setPinned] = useState<string | null>(null)
  const [openSubtree, setOpenSubtree] = useState<string | null>(null)

  // Clear stale skill selection when class changes (reset-state-on-prop-change pattern).
  const [prevClassId, setPrevClassId] = useState(classId)
  if (prevClassId !== classId) {
    setPrevClassId(classId)
    setHovered(null)
    setPinned(null)
    setOpenSubtree(null)
  }

  const handleHover = (id: string | null) => {
    setHovered(id)
    if (id !== null) setPinned(id)
  }

  const selected = pinned

  const cls = classId ? getClass(classId) : undefined
  const skillsForClass = useMemo(
    () => (classId ? skills.filter((s) => s.classId === classId) : []),
    [classId],
  )

  const buildDeps = useBuildPerformanceDeps()
  const [computed, setComputed] = useState<ComputedStats | null>(null)
  useEffect(() => {
    let cancelled = false
    computeBuildStatsAsync(buildDeps).then((c) => {
      if (!cancelled) setComputed(c)
    })
    return () => {
      cancelled = true
    }
  }, [buildDeps])
  const stats = computed?.stats ?? {}
  const attributes = computed?.attributes ?? {}
  const itemSkillBonuses = useMemo(
    () => computed?.itemSkillBonuses ?? {},
    [computed],
  )
  const rankBonuses = useMemo(() => computed?.rankBonuses ?? {}, [computed])

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
                canIncrement={remaining > 0}
                selected={hovered}
                onHover={handleHover}
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

function SkillTree({
  name,
  list,
  skillRanks,
  canIncrement,
  selected,
  onHover,
  onInc,
  onDec,
  onOpenSubtree,
}: {
  name: string
  list: Skill[]
  skillRanks: Record<string, number>
  canIncrement: boolean
  selected: string | null
  onHover: (id: string | null) => void
  onInc: (id: string, maxRank: number) => void
  onDec: (id: string) => void
  onOpenSubtree: (id: string | null) => void
}) {
  const maxRow = list.reduce((m, s) => Math.max(m, s.position?.row ?? 0), 0)
  const maxCol = list.reduce((m, s) => Math.max(m, s.position?.col ?? 0), 0)
  const cols = Math.max(maxCol + 1, 3)
  const rows = Math.max(maxRow + 1, 5)
  const width = cols * CELL + (cols - 1) * GAP
  const height = rows * CELL + (rows - 1) * GAP

  const byId = new Map(list.map((s) => [s.id, s]))
  const cellCenter = (pos: { row: number; col: number }) => ({
    x: pos.col * (CELL + GAP) + CELL / 2,
    y: pos.row * (CELL + GAP) + CELL / 2,
  })

  return (
    <section
      className="relative overflow-hidden rounded-md border border-border p-4"
      style={{
        width: width + 32,
        background:
          'linear-gradient(180deg, var(--color-panel), color-mix(in srgb, var(--color-bg) 70%, transparent))',
        boxShadow:
          'inset 0 1px 0 rgba(255,255,255,0.02), 0 8px 24px rgba(0,0,0,0.35)',
      }}
    >
      <CornerMarks size={8} opacity={0.45} />
      <div className="mb-3 flex items-center justify-center gap-2 border-b border-accent-deep/20 pb-2">
        <span
          aria-hidden
          className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
          style={{ boxShadow: '0 0 6px rgba(224,184,100,0.5)' }}
        />
        <h3 className="m-0 font-mono text-[11px] font-semibold uppercase tracking-[0.18em] text-accent-hot/80">
          {name}
        </h3>
      </div>
      <motion.div
        className="relative"
        style={{ width, height }}
        variants={listContainerVariants}
        initial="initial"
        animate="animate"
      >
        <svg
          className="pointer-events-none absolute inset-0"
          width={width}
          height={height}
        >
          {list.map((skill) => {
            if (!skill.requiresSkill || !skill.position) return null
            const parent = byId.get(skill.requiresSkill)
            if (!parent?.position) return null
            const a = cellCenter(parent.position)
            const b = cellCenter(skill.position)
            const parentRank = skillRanks[parent.id] ?? 0
            const satisfied = parentRank > 0
            return (
              <line
                key={`${parent.id}-${skill.id}`}
                x1={a.x}
                y1={a.y}
                x2={b.x}
                y2={b.y}
                style={{ stroke: satisfied ? 'var(--color-accent)' : 'var(--color-stat-red)' }}
                strokeWidth={2.5}
              />
            )
          })}
        </svg>
        {list.map((skill) => {
          if (!skill.position) return null
          const locked =
            !!skill.requiresSkill &&
            (skillRanks[skill.requiresSkill] ?? 0) === 0
          const hasSubtree =
            skill.kind === 'active' && skill.id !== 'gateway'
          return (
            <SkillIcon
              key={skill.id}
              skill={skill}
              rank={skillRanks[skill.id] ?? 0}
              locked={locked}
              canIncrement={canIncrement}
              hovered={selected === skill.id}
              hasSubtree={hasSubtree}
              style={{
                position: 'absolute',
                left: skill.position.col * (CELL + GAP),
                top: skill.position.row * (CELL + GAP),
              }}
              onMouseEnter={() => onHover(skill.id)}
              onMouseLeave={() => onHover(null)}
              onInc={() => onInc(skill.id, skill.maxRank)}
              onDec={() => onDec(skill.id)}
              onOpenSubtree={() => onOpenSubtree(skill.id)}
            />
          )
        })}
      </motion.div>
    </section>
  )
}

function SkillIcon({
  skill,
  rank,
  locked,
  canIncrement,
  hovered,
  hasSubtree,
  style,
  onMouseEnter,
  onMouseLeave,
  onInc,
  onDec,
  onOpenSubtree,
}: {
  skill: Skill
  rank: number
  locked: boolean
  canIncrement: boolean
  hovered: boolean
  hasSubtree: boolean
  style: React.CSSProperties
  onMouseEnter: () => void
  onMouseLeave: () => void
  onInc: () => void
  onDec: () => void
  onOpenSubtree: () => void
}) {
  const allocated = rank > 0
  const border = skill.damageType
    ? DAMAGE_COLORS[skill.damageType].border
    : 'border-accent'
  const glow =
    allocated && skill.damageType
      ? DAMAGE_COLORS[skill.damageType].glow
      : ''
  const canInc = canIncrement && rank < skill.maxRank && !locked

  return (
    <motion.div
      style={style}
      variants={skillIconVariants}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      onContextMenu={(e) => {
        e.preventDefault()
        if (rank > 0) onDec()
      }}
      className={`group relative flex items-center justify-center rounded-[3px] border-2 transition-all ${
        locked
          ? 'border-stat-red/40 bg-panel-2/30'
          : allocated
            ? `${border} bg-panel-2 ${glow}`
            : hovered
              ? 'border-accent-deep bg-panel-2'
              : 'border-border-2 bg-panel-2/50'
      }`}
      title={locked ? `Requires ${skill.requiresSkill}` : undefined}
    >
      <button
        onClick={canInc ? onInc : undefined}
        disabled={!canInc}
        className={`flex items-center justify-center transition-transform ${
          canInc ? 'cursor-pointer hover:scale-110' : ''
        } ${locked ? 'opacity-30 grayscale' : allocated ? '' : 'opacity-50'}`}
        style={{ width: CELL, height: CELL }}
      >
        <SkillIconImage icon={resolveSkillIcon(skill)} size={CELL} />
      </button>

      <div
        className={`absolute bottom-0.5 left-0.5 flex h-5 min-w-5 items-center justify-center rounded-xs border px-1 font-mono text-[11px] font-semibold tabular-nums ${
          allocated
            ? 'border-accent-deep text-accent-hot'
            : 'border-border text-faint'
        }`}
        style={{
          background: allocated
            ? 'linear-gradient(180deg, rgba(58,46,24,0.85), rgba(20,16,10,0.85))'
            : 'rgba(0,0,0,0.7)',
          boxShadow: allocated
            ? '0 0 6px rgba(224,184,100,0.25)'
            : undefined,
        }}
      >
        {rank}
      </div>

      {canInc && (
        <button
          onClick={onInc}
          onContextMenu={(e) => {
            e.preventDefault()
            e.stopPropagation()
          }}
          className="absolute -right-1.5 -top-1.5 flex h-5 w-5 items-center justify-center rounded-xs border border-accent-deep font-mono text-[12px] font-bold text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]"
          style={{
            background: 'linear-gradient(180deg, #3a2f1a, #2a2418)',
            boxShadow: '0 0 8px rgba(224,184,100,0.35)',
          }}
          aria-label={`Add point to ${skill.name}`}
        >
          +
        </button>
      )}
      {hasSubtree && (
        <button
          onClick={(e) => {
            e.stopPropagation()
            onOpenSubtree()
          }}
          onContextMenu={(e) => {
            e.preventDefault()
            e.stopPropagation()
          }}
          className="absolute -bottom-1.5 -right-1.5 flex h-5 w-5 items-center justify-center rounded-xs border border-border-2 bg-panel text-[10px] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
          style={{ boxShadow: '0 2px 4px rgba(0,0,0,0.5)' }}
          aria-label={`Open ${skill.name} subtree`}
          title="Open subtree"
        >
          <GearIcon className="h-3 w-3" />
        </button>
      )}
    </motion.div>
  )
}

function SkillDetailsPanel({
  skill,
  currentRank,
  allSkillsBonus,
  elementSkillsBonus,
  itemBonus,
  allClassSkills,
  skillRanks,
  attributes,
  subskillRanks,
  enemyConditions,
  rankBonuses,
}: {
  skill: Skill | null
  currentRank: number
  allSkillsBonus: [number, number]
  elementSkillsBonus: [number, number]
  itemBonus: [number, number]
  allClassSkills: Skill[]
  skillRanks: Record<string, number>
  attributes: Record<AttributeKey, RangedValue>
  subskillRanks: Record<string, number>
  enemyConditions: Record<string, boolean>
  rankBonuses: Record<string, [number, number]>
}) {
  if (!skill) {
    return (
      <aside
        className="flex h-full min-h-0 w-96 shrink-0 flex-col gap-4 overflow-y-auto border-l border-border p-5"
        style={{
          background:
            'linear-gradient(180deg, var(--color-panel-2), var(--color-panel) 40%, var(--color-bg))',
          boxShadow: 'inset 1px 0 0 rgba(201,165,90,0.05)',
        }}
      >
        <div>
          <div className="mb-2 flex items-center gap-2 border-b border-accent-deep/20 pb-1.5 font-mono text-[10px] uppercase tracking-[0.18em] text-accent-hot/70">
            <span
              aria-hidden
              className="inline-block h-1 w-1 rotate-45 bg-accent-hot"
              style={{ boxShadow: '0 0 6px rgba(224,184,100,0.5)' }}
            />
            Details
          </div>
          <p className="font-mono text-[11px] leading-relaxed tracking-[0.04em] text-muted">
            Hover a skill to inspect its damage, mana cost, synergies, and
            subtree bonuses.
          </p>
        </div>

        <div>
          <div className="mb-2 flex items-center gap-2 border-b border-accent-deep/20 pb-1.5 font-mono text-[10px] uppercase tracking-[0.18em] text-accent-hot/70">
            <span
              aria-hidden
              className="inline-block h-1 w-1 rotate-45 bg-accent-deep"
            />
            Controls
          </div>
          <ul className="space-y-1.5 font-mono text-[10px] uppercase tracking-[0.12em] text-faint">
            <ControlsRow keys="L-CLICK" label="Add a point" />
            <ControlsRow keys="R-CLICK" label="Remove a point" />
            <ControlsRow keys="HOVER" label="Pin details panel" />
            <ControlsRow keys={<GearIcon className="h-3 w-3" />} label="Open subtree" />
          </ul>
        </div>

        <div>
          <div className="mb-2 flex items-center gap-2 border-b border-accent-deep/20 pb-1.5 font-mono text-[10px] uppercase tracking-[0.18em] text-accent-hot/70">
            <span
              aria-hidden
              className="inline-block h-1 w-1 rotate-45 bg-accent-deep"
            />
            Damage Types
          </div>
          <ul className="grid grid-cols-2 gap-1.5 font-mono text-[10px] uppercase tracking-[0.12em] text-muted">
            <DamageLegend type="physical" />
            <DamageLegend type="lightning" />
            <DamageLegend type="cold" />
            <DamageLegend type="fire" />
            <DamageLegend type="poison" />
            <DamageLegend type="arcane" />
            <DamageLegend type="explosion" />
            <DamageLegend type="magic" />
          </ul>
        </div>
      </aside>
    )
  }
  const baseMana = skill.ranks[0]?.manaCost
  const allocated = currentRank > 0
  const totalBonusMin = allocated
    ? allSkillsBonus[0] + elementSkillsBonus[0] + itemBonus[0]
    : 0
  const totalBonusMax = allocated
    ? allSkillsBonus[1] + elementSkillsBonus[1] + itemBonus[1]
    : 0
  const effMin = currentRank + totalBonusMin
  const effMax = currentRank + totalBonusMax
  const effLabel =
    effMin === effMax ? String(effMin) : `${effMin}-${effMax}`
  const hasBonus = totalBonusMin !== 0 || totalBonusMax !== 0
  return (
    <aside
      className="h-full min-h-0 w-96 shrink-0 overflow-y-auto border-l border-border p-4"
      style={{
        background:
          'linear-gradient(180deg, var(--color-panel-2), var(--color-panel) 40%, var(--color-bg))',
        boxShadow: 'inset 1px 0 0 rgba(201,165,90,0.05)',
      }}
    >
      <div className="mb-1 flex items-center gap-2.5">
        <div
          className="flex shrink-0 items-center justify-center rounded-[3px] border border-border-2 p-1"
          style={{
            background: 'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
            boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
          }}
        >
          <SkillIconImage icon={resolveSkillIcon(skill)} size={48} className="text-3xl" />
        </div>
        <div className="min-w-0">
          <div
            className="truncate text-[15px] font-semibold tracking-[0.02em] text-accent-hot"
            style={{ textShadow: '0 0 12px rgba(224,184,100,0.18)' }}
          >
            {skill.name}
          </div>
          <div className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
            {skill.damageType ?? '—'} · {skill.kind}
          </div>
        </div>
      </div>
      <div className="mb-3 flex items-center gap-2 font-mono text-[11px] tabular-nums">
        <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
          Rank
        </span>
        <span className="text-accent-hot">
          {hasBonus ? effLabel : currentRank}
        </span>
        <span className="text-faint">/ {skill.maxRank}</span>
        {hasBonus && (
          <span className="text-muted">
            ({currentRank}
            {totalBonusMin === totalBonusMax
              ? totalBonusMin >= 0
                ? `+${totalBonusMin}`
                : totalBonusMin
              : ` +${totalBonusMin}-${totalBonusMax}`}
            )
          </span>
        )}
      </div>
      {hasBonus && (
        <DetailBlock title="Skill bonuses">
          <div className="space-y-0.5 text-xs tabular-nums">
            {(allSkillsBonus[0] !== 0 || allSkillsBonus[1] !== 0) && (
              <div className="flex justify-between">
                <span className="text-muted">+ to All Skills</span>
                <span className="text-accent-hot">
                  +{formatPair(allSkillsBonus)}
                </span>
              </div>
            )}
            {(elementSkillsBonus[0] !== 0 || elementSkillsBonus[1] !== 0) &&
              skill.damageType && (
                <div className="flex justify-between">
                  <span className="text-muted">
                    + to {skill.damageType[0]!.toUpperCase()}
                    {skill.damageType.slice(1)} Skills
                  </span>
                  <span className="text-accent-hot">
                    +{formatPair(elementSkillsBonus)}
                  </span>
                </div>
              )}
            {(itemBonus[0] !== 0 || itemBonus[1] !== 0) && (
              <div className="flex justify-between">
                <span className="text-muted">+ to {skill.name}</span>
                <span className="text-accent-hot">+{formatPair(itemBonus)}</span>
              </div>
            )}
          </div>
        </DetailBlock>
      )}
      <SkillEffectsBlock
        skill={skill}
        currentRank={currentRank}
        effRankMin={effMin}
        effRankMax={effMax}
        allClassSkills={allClassSkills}
        skillRanks={skillRanks}
        attributes={attributes}
        rankBonuses={rankBonuses}
      />
      <SubtreeBonusBlock
        skill={skill}
        subskillRanks={subskillRanks}
        enemyConditions={enemyConditions}
      />
      {skill.description && (
        <p className="mb-3 text-[13px] leading-relaxed text-muted">
          {skill.description}
        </p>
      )}
      {(baseMana !== undefined ||
        skill.baseCastRate !== undefined ||
        skill.movementDuringUse !== undefined ||
        skill.range !== undefined ||
        skill.baseCooldown !== undefined ||
        skill.effectDuration !== undefined ||
        skill.requiresLevel !== undefined ||
        skill.requiresSkill !== undefined) && (
        <div className="mb-3 space-y-0.75">
          {baseMana !== undefined && (
            <PropertyRow label="Mana" value={baseMana} valueClass="text-stat-blue" />
          )}
          {skill.baseCastRate !== undefined && (
            <PropertyRow
              label="Base cast rate"
              value={skill.baseCastRate}
              suffix="/s"
            />
          )}
          {skill.movementDuringUse !== undefined && (
            <PropertyRow
              label="Movement during use"
              value={skill.movementDuringUse}
              suffix="%"
            />
          )}
          {skill.range !== undefined && (
            <PropertyRow label="Range" value={skill.range} />
          )}
          {skill.baseCooldown !== undefined && (
            <PropertyRow
              label="Cooldown"
              value={skill.baseCooldown}
              suffix="s"
            />
          )}
          {skill.effectDuration !== undefined && (
            <PropertyRow
              label="Effect duration"
              value={skill.effectDuration}
              suffix="s"
            />
          )}
          {skill.requiresLevel && (
            <PropertyRow
              label="Requires level"
              value={skill.requiresLevel}
            />
          )}
          {skill.requiresSkill && (
            <div className="flex items-baseline justify-between gap-2 py-0.75 text-xs">
              <span className="text-muted">Requires skill</span>
              <span className="font-mono text-faint">
                «{skill.requiresSkill}»
              </span>
            </div>
          )}
        </div>
      )}
      {skill.tags && skill.tags.length > 0 && (
        <div className="mt-3 flex flex-wrap gap-1.5">
          {skill.tags.map((t) => (
            <span
              key={t}
              className="rounded-[3px] border border-accent-deep/40 px-2 py-0.5 font-mono text-[10px] uppercase tracking-[0.18em] text-accent-hot/80"
              style={{
                background:
                  'linear-gradient(180deg, rgba(58,46,24,0.5), rgba(42,36,24,0.3))',
              }}
            >
              {t}
            </span>
          ))}
        </div>
      )}
    </aside>
  )
}

function PropertyRow({
  label,
  value,
  suffix,
  valueClass,
}: {
  label: string
  value: string | number
  suffix?: string
  valueClass?: string
}) {
  return (
    <div className="flex items-baseline justify-between gap-2 py-0.75 text-xs">
      <span className="text-muted">{label}</span>
      <span className="font-mono tabular-nums">
        <span className={valueClass ?? 'text-text'}>{value}</span>
        {suffix && <span className="text-faint">{suffix}</span>}
      </span>
    </div>
  )
}

function DetailBlock({
  title,
  trailing,
  children,
}: {
  title: string
  trailing?: ReactNode
  children: ReactNode
}) {
  return (
    <div
      className="mb-3 rounded-[3px] border border-border-2 p-2.5"
      style={{
        background:
          'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
        boxShadow: 'inset 0 1px 0 rgba(255,255,255,0.02)',
      }}
    >
      <div className="mb-1.5 flex items-center justify-between gap-2 border-b border-accent-deep/20 pb-1">
        <div className="flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.18em] text-accent-hot/70">
          <span
            aria-hidden
            className="inline-block h-1 w-1 rotate-45 bg-accent-deep"
          />
          {title}
        </div>
        {trailing}
      </div>
      {children}
    </div>
  )
}

function GearIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} className={className} aria-hidden>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
    </svg>
  )
}

function ControlsRow({ keys, label }: { keys: ReactNode; label: string }) {
  return (
    <li className="flex items-center justify-between gap-2">
      <span
        className="rounded-xs border border-border-2 px-1.5 py-0.5 font-mono text-[9px] tracking-[0.14em] text-muted"
        style={{ background: 'var(--color-panel-2)' }}
      >
        {keys}
      </span>
      <span>{label}</span>
    </li>
  )
}

function DamageLegend({ type }: { type: DamageType }) {
  return (
    <li className="flex items-center gap-1.5">
      <span
        aria-hidden
        className={`inline-block h-1.5 w-1.5 shrink-0 rotate-45 border ${DAMAGE_COLORS[type].border}`}
      />
      {type}
    </li>
  )
}

function formatPair(pair: [number, number]): string {
  return pair[0] === pair[1] ? String(pair[0]) : `${pair[0]}-${pair[1]}`
}

const EMPTY_SUBTREE_AGGREGATION: SubtreeAggregation = {
  stats: {},
  procStats: {},
  appliedStates: [],
}

function SubtreeBonusBlock({
  skill,
  subskillRanks,
  enemyConditions,
}: {
  skill: Skill
  subskillRanks: Record<string, number>
  enemyConditions: Record<string, boolean>
}) {
  const [agg, setAgg] = useState<SubtreeAggregation>(EMPTY_SUBTREE_AGGREGATION)
  useEffect(() => {
    let cancelled = false
    subskillAggregationNative(
      skill.classId,
      skill.id,
      subskillRanks,
      enemyConditions,
    ).then((a) => {
      if (!cancelled) setAgg(a)
    })
    return () => {
      cancelled = true
    }
  }, [skill, subskillRanks, enemyConditions])
  const statEntries = Object.entries(agg.stats)
    .filter(([, v]) => v !== 0)
    .sort(([a], [b]) => a.localeCompare(b))

  const activeProcs = useMemo(() => {
    const out: { sub: SubskillNode; rank: number }[] = []
    for (const sub of skill.subskills ?? []) {
      if (!sub.proc) continue
      const rank = subskillRanks[subskillKey(skill.id, sub.id)] ?? 0
      if (rank <= 0) continue
      out.push({ sub, rank })
    }
    return out
  }, [skill, subskillRanks])

  if (statEntries.length === 0 && activeProcs.length === 0) return null

  return (
    <DetailBlock title="Subtree bonuses">
      {statEntries.length > 0 && (
        <div className="space-y-0.5 text-xs tabular-nums">
          {statEntries.map(([k, v]) => (
            <div key={k} className="flex justify-between">
              <span className="text-muted">{statName(k)}</span>
              <span className="text-accent-hot">{formatValue(v, k)}</span>
            </div>
          ))}
        </div>
      )}
      {activeProcs.length > 0 && (
        <div
          className={`space-y-1.5 ${statEntries.length > 0 ? 'mt-2.5 border-t border-dashed border-accent-deep/30 pt-2' : ''}`}
        >
          {activeProcs.map(({ sub, rank }) => {
            const proc = sub.proc!
            const chance =
              (proc.chance.base ?? 0) + (proc.chance.perRank ?? 0) * rank
            const effectParts: string[] = []
            const baseMap: Record<string, number> = proc.effects?.base ?? {}
            const perRankMap: Record<string, number> = proc.effects?.perRank ?? {}
            for (const [k, base] of Object.entries(baseMap)) {
              const per = perRankMap[k] ?? 0
              const v = base + per * rank
              if (v === 0) continue
              effectParts.push(`${formatValue(v, k)} ${statName(k)}`)
            }
            for (const [k, per] of Object.entries(perRankMap)) {
              if (k in baseMap) continue
              const v = per * rank
              if (v === 0) continue
              effectParts.push(`${formatValue(v, k)} ${statName(k)}`)
            }
            for (const s of proc.appliesStates ?? []) {
              if (typeof s === 'string') {
                effectParts.push(`applies ${s.replace(/_/g, ' ')}`)
              } else {
                const amt =
                  (s.amount?.base ?? 0) + (s.amount?.perRank ?? 0) * rank
                effectParts.push(
                  amt
                    ? `applies ${s.state.replace(/_/g, ' ')} (${amt}%)`
                    : `applies ${s.state.replace(/_/g, ' ')}`,
                )
              }
            }
            return (
              <div key={sub.id} className="text-xs">
                <div className="flex items-baseline justify-between gap-2">
                  <span className="text-text">{sub.name}</span>
                  <span className="tabular-nums">
                    <span className="text-accent-hot">{chance}%</span>{' '}
                    <span className="font-mono text-[10px] uppercase tracking-[0.12em] text-faint">
                      {proc.trigger.replace('_', ' ')}
                    </span>
                  </span>
                </div>
                <div className="text-muted leading-snug">
                  {effectParts.join(', ')}
                </div>
              </div>
            )
          })}
        </div>
      )}
    </DetailBlock>
  )
}


function SkillEffectsBlock({
  skill,
  currentRank,
  effRankMin,
  effRankMax,
  allClassSkills,
  skillRanks,
  attributes,
  rankBonuses,
}: {
  skill: Skill
  currentRank: number
  effRankMin: number
  effRankMax: number
  allClassSkills: Skill[]
  skillRanks: Record<string, number>
  attributes: Record<AttributeKey, RangedValue>
  rankBonuses: Record<string, [number, number]>
}) {
  const allocated = currentRank > 0
  const curMin = allocated ? effRankMin : 1
  const curMax = allocated ? effRankMax : 1
  // Cap on allocated rank, not effective rank (effective can exceed maxRank via +all_skills bonuses).
  const canIncrement = allocated && currentRank < skill.maxRank
  const nextMin = canIncrement ? curMin + 1 : null
  const nextMax = canIncrement ? curMax + 1 : null

  const rankInfo = useSkillRankInfo(
    skill,
    [curMin, curMax, nextMin, nextMax].filter((r): r is number => r !== null),
  )
  const passiveCurMin = rankInfo.get(curMin)?.passive ?? {}
  const passiveCurMax = rankInfo.get(curMax)?.passive ?? {}
  const passiveNextMin =
    nextMin !== null ? (rankInfo.get(nextMin)?.passive ?? {}) : null
  const passiveNextMax =
    nextMax !== null ? (rankInfo.get(nextMax)?.passive ?? {}) : null

  const manaCurMin = rankInfo.get(curMin)?.mana
  const manaCurMax = rankInfo.get(curMax)?.mana
  const manaNextMin = nextMin !== null ? rankInfo.get(nextMin)?.mana : undefined
  const manaNextMax = nextMax !== null ? rankInfo.get(nextMax)?.mana : undefined

  const computeBaseDmg = (rank: number): [number, number] | null => {
    if (skill.damageFormula) {
      const v = Math.max(0, skill.damageFormula.base + skill.damageFormula.perLevel * rank)
      return [v, v]
    }
    if (skill.damagePerRank) {
      const idx = Math.min(rank, skill.damagePerRank.length) - 1
      if (idx >= 0) {
        const d = skill.damagePerRank[idx]!
        return [Math.max(0, d.min), Math.max(0, d.max)]
      }
    }
    return null
  }
  const baseDmgCurMin = computeBaseDmg(curMin)
  const baseDmgCurMax = computeBaseDmg(curMax)
  const baseDmgNextMin = nextMin !== null ? computeBaseDmg(nextMin) : null
  const baseDmgNextMax = nextMax !== null ? computeBaseDmg(nextMax) : null

  const synergiesReceived: Array<{
    source: string
    perLabel: string
    sourceMin: number
    sourceMax: number
    pctMin: number
    pctMax: number
    stat: string
  }> = []
  if (allocated) {
    for (const bs of skill.bonusSources ?? []) {
      if (bs.per === 'skill_level') {
        const srcKey = normalizeSkillName(bs.source)
        const srcSkill = allClassSkills.find(
          (s) => normalizeSkillName(s.name) === srcKey,
        )
        if (!srcSkill) continue
        const baseRank = skillRanks[srcSkill.id] ?? 0
        if (baseRank === 0) continue
        const bonus: [number, number] = rankBonuses[srcKey] ?? [0, 0]
        const effMin = baseRank + bonus[0]
        const effMax = baseRank + bonus[1]
        const perLabel =
          effMin === effMax ? `rank ${effMin}` : `rank ${effMin}-${effMax}`
        synergiesReceived.push({
          source: bs.source,
          perLabel,
          sourceMin: effMin,
          sourceMax: effMax,
          pctMin: effMin * bs.value,
          pctMax: effMax * bs.value,
          stat: bs.stat,
        })
      } else if (bs.per === 'attribute_point') {
        const attrKey = (
          Object.keys(attributes) as AttributeKey[]
        ).find((k) => k.toLowerCase() === bs.source.trim().toLowerCase())
        if (!attrKey) continue
        const attr = attributes[attrKey] ?? 0
        const aMin = typeof attr === 'number' ? attr : attr[0]
        const aMax = typeof attr === 'number' ? attr : attr[1]
        if (aMin === 0 && aMax === 0) continue
        synergiesReceived.push({
          source: bs.source,
          perLabel: aMin === aMax ? `${aMin}` : `${aMin}-${aMax}`,
          sourceMin: aMin,
          sourceMax: aMax,
          pctMin: aMin * bs.value,
          pctMax: aMax * bs.value,
          stat: bs.stat,
        })
      }
    }
  }

  const synergiesProvided: Array<{
    target: string
    pctCurMin: number
    pctCurMax: number
    pctNextMin: number | null
    pctNextMax: number | null
    stat: string
  }> = []
  for (const other of allClassSkills) {
    for (const bs of other.bonusSources ?? []) {
      if (
        bs.per === 'skill_level' &&
        normalizeSkillName(bs.source) === normalizeSkillName(skill.name)
      ) {
        synergiesProvided.push({
          target: other.name,
          pctCurMin: bs.value * curMin,
          pctCurMax: bs.value * curMax,
          pctNextMin: nextMin !== null ? bs.value * nextMin : null,
          pctNextMax: nextMax !== null ? bs.value * nextMax : null,
          stat: bs.stat,
        })
      }
    }
  }

  const passiveKeys = new Set([
    ...Object.keys(passiveCurMin),
    ...Object.keys(passiveCurMax),
  ])
  const hasAnything =
    passiveKeys.size > 0 ||
    baseDmgCurMin !== null ||
    manaCurMin !== undefined ||
    synergiesProvided.length > 0 ||
    synergiesReceived.length > 0 ||
    (skill.bonusSources?.length ?? 0) > 0

  if (!hasAnything) return null

  const rankLabel =
    curMin === curMax ? String(curMin) : `${curMin}-${curMax}`
  const nextRankLabel =
    nextMin === null
      ? null
      : nextMin === nextMax
        ? String(nextMin)
        : `${nextMin}-${nextMax}`

  return (
    <DetailBlock
      title={allocated ? 'Effects' : 'Preview (not learned)'}
      trailing={
        <span className="font-mono text-[10px] tabular-nums tracking-[0.06em] text-faint">
          rank {rankLabel}
          {nextRankLabel && (
            <>
              {' '}
              <span className="text-accent-deep">→</span> {nextRankLabel}
            </>
          )}
        </span>
      }
    >
      <div className="space-y-0.5 text-xs tabular-nums">
        {baseDmgCurMin && baseDmgCurMax && (
          <EffRow
            label={
              skill.attackKind === 'attack' && skill.damageType
                ? `${skill.damageType[0]!.toUpperCase()}${skill.damageType.slice(1)} damage`
                : 'Base damage'
            }
            cur={formatDmgRange(baseDmgCurMin, baseDmgCurMax)}
            next={
              baseDmgNextMin && baseDmgNextMax
                ? formatDmgRange(baseDmgNextMin, baseDmgNextMax)
                : undefined
            }
            color={allocated ? 'text-accent-hot' : 'text-muted'}
          />
        )}
        {(['weaponDamagePct', 'attackRatingPct'] as const).map((key) => {
          const f = skill.attackScaling?.[key]
          if (!f) return null
          const label = key === 'weaponDamagePct' ? 'Attack damage' : 'Attack rating'
          return (
            <EffRow
              key={key}
              label={label}
              cur={formatPctRange(evalFormulaClamped(f, curMin), evalFormulaClamped(f, curMax))}
              next={
                nextMin !== null && nextMax !== null
                  ? formatPctRange(
                      evalFormulaClamped(f, nextMin),
                      evalFormulaClamped(f, nextMax),
                    )
                  : undefined
              }
              color={allocated ? 'text-accent-hot' : 'text-muted'}
            />
          )
        })}
        {skill.attackScaling?.flatPhysicalMin &&
          skill.attackScaling.flatPhysicalMax && (
            <EffRow
              label="Physical damage"
              cur={formatFlatPhys(
                skill.attackScaling.flatPhysicalMin,
                skill.attackScaling.flatPhysicalMax,
                curMin,
                curMax,
              )}
              next={
                nextMin !== null && nextMax !== null
                  ? formatFlatPhys(
                      skill.attackScaling.flatPhysicalMin,
                      skill.attackScaling.flatPhysicalMax,
                      nextMin,
                      nextMax,
                    )
                  : undefined
              }
              color={allocated ? 'text-accent-hot' : 'text-muted'}
            />
          )}
        {manaCurMin !== undefined && manaCurMax !== undefined && (
          <EffRow
            label="Mana cost"
            cur={formatPair([manaCurMin, manaCurMax])}
            next={
              manaNextMin !== undefined && manaNextMax !== undefined
                ? formatPair([manaNextMin, manaNextMax])
                : undefined
            }
            color={allocated ? 'text-stat-blue' : 'text-muted'}
          />
        )}
        {[...passiveKeys].map((key) => {
          const vMin = passiveCurMin[key] ?? 0
          const vMax = passiveCurMax[key] ?? 0
          const nMin = passiveNextMin ? passiveNextMin[key] : undefined
          const nMax = passiveNextMax ? passiveNextMax[key] : undefined
          return (
            <EffRow
              key={key}
              label={statName(key)}
              cur={formatStatPair(key, vMin, vMax)}
              next={
                nMin !== undefined && nMax !== undefined
                  ? formatStatPair(key, nMin, nMax)
                  : undefined
              }
              color={allocated ? 'text-accent-hot' : 'text-muted'}
            />
          )
        })}
        {synergiesProvided.map((s, i) => (
          <EffRow
            key={i}
            label={`→ ${s.target}`}
            cur={formatStatPair(s.stat, s.pctCurMin, s.pctCurMax)}
            next={
              s.pctNextMin !== null && s.pctNextMax !== null
                ? formatStatPair(s.stat, s.pctNextMin, s.pctNextMax)
                : undefined
            }
            color={allocated ? 'text-stat-orange' : 'text-muted'}
          />
        ))}
      </div>
      {(skill.bonusSources?.length ?? 0) > 0 && (
        <div className="mt-2.5 border-t border-dashed border-accent-deep/30 pt-2">
          <div className="mb-1.5 flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.18em] text-accent-hot/70">
            <span
              aria-hidden
              className="inline-block h-1 w-1 rotate-45 bg-accent-deep"
            />
            Receives synergy from
          </div>
          <div className="space-y-0.5 text-xs">
            {(skill.bonusSources ?? []).map((bs, i) => {
              const matched = synergiesReceived.find(
                (sr) => sr.source === bs.source && sr.stat === bs.stat,
              )
              return (
                <div
                  key={i}
                  className="flex items-baseline justify-between gap-2 tabular-nums"
                >
                  <span className="truncate text-muted">
                    {bs.source}{' '}
                    <span className="text-faint text-[10px]">
                      ({bs.value}% per{' '}
                      {bs.per === 'skill_level' ? 'rank' : 'point'})
                    </span>
                  </span>
                  <span
                    className={matched ? 'text-stat-orange' : 'text-faint'}
                  >
                    {matched
                      ? formatStatPair(bs.stat, matched.pctMin, matched.pctMax)
                      : '—'}
                  </span>
                </div>
              )
            })}
          </div>
        </div>
      )}
    </DetailBlock>
  )
}

function formatStatPair(key: string, min: number, max: number): string {
  if (min === max) return formatValue(min, key)
  return `${formatValue(min, key)}-${formatValue(max, key).replace(/^[+-]/, '')}`
}

// Linear formulas can extrapolate negative at low ranks; UI never shows negative scaling.
function evalFormulaClamped(f: { base: number; perLevel: number }, rank: number): number {
  return Math.max(0, f.base + f.perLevel * rank)
}

function formatPctRange(min: number, max: number): string {
  const m = Math.round(min * 100) / 100
  const mx = Math.round(max * 100) / 100
  if (m === mx) return `${m}%`
  return `${m}% - ${mx}%`
}

function formatFlatPhys(
  minF: { base: number; perLevel: number },
  maxF: { base: number; perLevel: number },
  curMin: number,
  curMax: number,
): string {
  const fmt = (rank: number) =>
    `[${Math.round(evalFormulaClamped(minF, rank) * 100) / 100} to ${
      Math.round(evalFormulaClamped(maxF, rank) * 100) / 100
    }]`
  if (curMin === curMax) return fmt(curMin)
  return `${fmt(curMin)} … ${fmt(curMax)}`
}

function formatDmgRange(min: [number, number], max: [number, number]): string {
  if (min[0] === max[0] && min[1] === max[1]) return formatRangeTuple(min)
  return `${formatRangeTuple(min)} … ${formatRangeTuple(max)}`
}

function formatRangeTuple([min, max]: [number, number]): string {
  const m = Math.round(min * 100) / 100
  const mx = Math.round(max * 100) / 100
  if (m === mx) return String(m)
  return `${m}-${mx}`
}

function EffRow({
  label,
  cur,
  next,
  color,
}: {
  label: string
  cur: string
  next?: string
  color: string
}) {
  return (
    <div className="flex items-baseline justify-between gap-2 min-w-0">
      <span className="text-text/80 truncate" title={label}>
        {label}
      </span>
      <span className="whitespace-nowrap shrink-0">
        <span className={color}>{cur}</span>
        {next && next !== cur && (
          <>
            <span className="text-muted"> → </span>
            <span className={color}>{next}</span>
          </>
        )}
      </span>
    </div>
  )
}

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex h-full items-center justify-center p-8 text-center text-sm text-muted">
      {message}
    </div>
  )
}
