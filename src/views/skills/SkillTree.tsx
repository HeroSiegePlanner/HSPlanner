import { CornerMarks } from '../../components/CornerMarks'
import { motion } from 'motion/react'
import { SkillIconImage } from '../../components/SkillIconImage'
import { listContainerVariants, skillIconVariants } from '../../lib/motion'
import { resolveSkillIcon } from '../../data'
import { DAMAGE_COLORS } from '../../utils/damageColors'
import type { Skill } from '../../types'
import { formatPair } from './format'
import {
  ACCENT_HOT_RGB,
  CELL,
  GAP,
  SYNERGY_RGB,
  treeColorRgb,
} from './treeConstants'

export function GearIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} className={className} aria-hidden>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
    </svg>
  )
}

export function SkillTree({
  name,
  list,
  skillRanks,
  skillBonuses,
  canIncrement,
  hoveredId,
  selectedId,
  highlightId,
  onHover,
  onSelect,
  onInc,
  onDec,
  onOpenSubtree,
}: {
  name: string
  list: Skill[]
  skillRanks: Record<string, number>
  skillBonuses: Record<string, [number, number]>
  canIncrement: boolean
  hoveredId: string | null
  selectedId: string | null
  highlightId: string | null
  onHover: (id: string | null) => void
  onSelect: (id: string | null) => void
  onInc: (id: string, maxRank: number) => void
  onDec: (id: string) => void
  onOpenSubtree: (id: string | null) => void
}) {
  const rgb = treeColorRgb(list)
  const pts = list.reduce((a, s) => a + (skillRanks[s.id] ?? 0), 0)
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
      className="relative overflow-hidden rounded-md border p-4"
      style={{
        width: width + 32,
        borderColor: `rgba(${rgb},0.22)`,
        background: `linear-gradient(180deg, rgba(${rgb},0.06), var(--color-panel) 22%, color-mix(in srgb, var(--color-bg) 70%, transparent))`,
        boxShadow: `inset 0 1px 0 rgba(${rgb},0.18), 0 8px 24px rgba(0,0,0,0.35)`,
      }}
    >
      <CornerMarks size={8} opacity={0.45} />
      <div
        className="mb-3 flex items-center justify-between gap-2 border-b pb-2"
        style={{ borderColor: `rgba(${rgb},0.18)` }}
      >
        <div className="flex items-center gap-2">
          <span
            aria-hidden
            className="inline-block h-1.5 w-1.5 rotate-45"
            style={{
              background: `rgb(${rgb})`,
              boxShadow: `0 0 6px rgba(${rgb},0.6)`,
            }}
          />
          <h3
            className="m-0 font-mono text-[12px] font-semibold uppercase tracking-[0.18em]"
            style={{
              color: `rgb(${rgb})`,
              textShadow: `0 0 12px rgba(${rgb},0.3)`,
            }}
          >
            {name}
          </h3>
        </div>
        <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint tabular-nums">
          {pts} pts
        </span>
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
                style={{
                  stroke: satisfied
                    ? `rgba(${rgb},0.55)`
                    : 'rgba(120,110,95,0.35)',
                }}
                strokeWidth={2}
                strokeDasharray="4 5"
                strokeLinecap="round"
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
              bonus={skillBonuses[skill.id]}
              locked={locked}
              canIncrement={canIncrement}
              hovered={hoveredId === skill.id}
              isSelected={selectedId === skill.id}
              synergyHighlight={highlightId === skill.id}
              hasSubtree={hasSubtree}
              style={{
                position: 'absolute',
                left: skill.position.col * (CELL + GAP),
                top: skill.position.row * (CELL + GAP),
              }}
              onMouseEnter={() => onHover(skill.id)}
              onMouseLeave={() => onHover(null)}
              onSelect={() => onSelect(skill.id)}
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
  bonus,
  locked,
  canIncrement,
  hovered,
  isSelected,
  synergyHighlight,
  hasSubtree,
  style,
  onMouseEnter,
  onMouseLeave,
  onSelect,
  onInc,
  onDec,
  onOpenSubtree,
}: {
  skill: Skill
  rank: number
  bonus?: [number, number]
  locked: boolean
  canIncrement: boolean
  hovered: boolean
  isSelected: boolean
  synergyHighlight: boolean
  hasSubtree: boolean
  style: React.CSSProperties
  onMouseEnter: () => void
  onMouseLeave: () => void
  onSelect: () => void
  onInc: () => void
  onDec: () => void
  onOpenSubtree: () => void
}) {
  const allocated = rank > 0
  const canInc = canIncrement && rank < skill.maxRank && !locked
  const dmgRgb = skill.damageType
    ? DAMAGE_COLORS[skill.damageType].rgb
    : ACCENT_HOT_RGB
  const shadows: string[] = []
  if (synergyHighlight) {
    shadows.push(
      `0 0 0 2px rgba(${SYNERGY_RGB},0.9)`,
      `0 0 16px rgba(${SYNERGY_RGB},0.55)`,
    )
  } else if (isSelected) {
    shadows.push(
      `0 0 0 2px rgba(${ACCENT_HOT_RGB},0.9)`,
      `0 0 14px rgba(${ACCENT_HOT_RGB},0.45)`,
    )
  } else if (hovered) {
    shadows.push(`0 0 0 1.5px rgba(${dmgRgb},0.55)`)
  }
  if (allocated && !isSelected && !synergyHighlight) {
    shadows.push(`0 0 12px rgba(${dmgRgb},0.4)`)
  }
  const ringShadow = shadows.length ? shadows.join(', ') : undefined
  // Ranged bonuses get parentheses for readability: "20+(60-62)" vs flat "20+61".
  const bonusLabel = bonus
    ? `${bonus[0] >= 0 ? '+' : ''}${
        bonus[0] === bonus[1] ? formatPair(bonus) : `(${formatPair(bonus)})`
      }`
    : ''

  return (
    <motion.div
      variants={skillIconVariants}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      onContextMenu={(e) => {
        e.preventDefault()
        if (rank > 0) onDec()
      }}
      className="group relative flex items-center justify-center rounded-[3px] transition-all"
      style={{ ...style, boxShadow: ringShadow }}
      title={locked ? `Requires ${skill.requiresSkill}` : undefined}
    >
      <button
        onClick={onSelect}
        className={`flex cursor-pointer items-center justify-center transition-transform hover:scale-105 ${
          locked ? 'opacity-30 grayscale' : allocated ? '' : 'opacity-60'
        }`}
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
        {rank}{bonusLabel}
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
