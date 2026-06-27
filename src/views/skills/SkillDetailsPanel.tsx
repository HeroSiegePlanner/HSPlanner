import { type ReactNode } from 'react'
import { SkillIconImage } from '../../components/SkillIconImage'
import { resolveSkillIcon } from '../../data'
import { DAMAGE_COLORS } from '../../utils/damageColors'
import type {
  AttributeKey,
  DamageType,
  RangedValue,
  Skill,
} from '../../types'
import { GearIcon } from './SkillTree'
import {
  DetailBlock,
  SkillEffectsBlock,
  SubtreeBonusBlock,
} from './SkillEffectsBlock'
import { formatPair } from './format'

export function SkillDetailsPanel({
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
  onSynergyHover,
  activeSkillIds,
  onToggleActive,
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
  onSynergyHover: (id: string | null) => void
  activeSkillIds: string[]
  onToggleActive: (id: string) => void
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
          <div className="mb-2 flex items-center gap-2 border-b border-accent-deep/20 pb-1.5 font-mono text-[11px] uppercase tracking-[0.16em] text-accent-hot/80">
            <span
              aria-hidden
              className="inline-block h-1 w-1 rotate-45 bg-accent-hot"
              style={{ boxShadow: '0 0 6px rgba(224,184,100,0.5)' }}
            />
            Details
          </div>
          <p className="font-mono text-[11px] leading-relaxed tracking-[0.04em] text-muted">
            Click a skill to inspect its damage, mana cost, synergies, and
            subtree bonuses.
          </p>
        </div>

        <div>
          <div className="mb-2 flex items-center gap-2 border-b border-accent-deep/20 pb-1.5 font-mono text-[11px] uppercase tracking-[0.16em] text-accent-hot/80">
            <span
              aria-hidden
              className="inline-block h-1 w-1 rotate-45 bg-accent-deep"
            />
            Controls
          </div>
          <ul className="space-y-2 font-mono text-[11px] uppercase tracking-[0.12em] text-muted">
            <ControlsRow keys="L-CLICK" label="Select skill" />
            <ControlsRow keys="+" label="Add a point" />
            <ControlsRow keys="R-CLICK" label="Remove a point" />
            <ControlsRow keys={<GearIcon className="h-3 w-3" />} label="Open subtree" />
          </ul>
        </div>

        <div>
          <div className="mb-2 flex items-center gap-2 border-b border-accent-deep/20 pb-1.5 font-mono text-[11px] uppercase tracking-[0.16em] text-accent-hot/80">
            <span
              aria-hidden
              className="inline-block h-1 w-1 rotate-45 bg-accent-deep"
            />
            Damage Types
          </div>
          <ul className="grid grid-cols-2 gap-2 font-mono text-[11px] uppercase tracking-[0.12em] text-muted">
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
  const isActive = skill.kind === 'active' && activeSkillIds.includes(skill.id)
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
          <div className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted">
            {skill.damageType ?? '—'} · {skill.kind}
          </div>
        </div>
        {skill.kind === 'active' && (
          <button
            onClick={() => onToggleActive(skill.id)}
            aria-pressed={isActive}
            className={`ml-auto shrink-0 self-start rounded-[3px] border px-2.5 py-1 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] transition-colors ${
              isActive
                ? 'border-accent-deep bg-accent-deep/20 text-accent-hot'
                : 'border-border-2 bg-transparent text-muted hover:border-accent-deep hover:text-accent-hot'
            }`}
            style={
              isActive
                ? { boxShadow: '0 0 10px rgba(224,184,100,0.25)' }
                : undefined
            }
          >
            {isActive ? '✓ Active' : '+ Active'}
          </button>
        )}
      </div>
      <div className="mb-3 flex items-center gap-2 font-mono text-[12px] tabular-nums">
        <span className="font-mono text-[11px] uppercase tracking-[0.14em] text-muted">
          Rank
        </span>
        <span className="text-accent-hot">
          {hasBonus ? effLabel : currentRank}
        </span>
        <span className="text-muted">/ {skill.maxRank}</span>
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
          <div className="space-y-1 text-[12px] tabular-nums">
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
        onSynergyHover={onSynergyHover}
      />
      <SubtreeBonusBlock
        skill={skill}
        subskillRanks={subskillRanks}
        enemyConditions={enemyConditions}
      />
      {skill.description && (
        <p className="mb-3 text-[12px] leading-relaxed text-muted">
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
            <div className="flex items-baseline justify-between gap-2 py-1 text-[12px]">
              <span className="text-muted">Requires skill</span>
              <span className="font-mono text-muted">
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
    <div className="flex items-baseline gap-2 py-1 text-[12px]">
      <span className="text-muted">{label}</span>
      <span
        aria-hidden
        className="mb-[3px] min-w-2 flex-1 self-end border-b border-dotted border-faint/40"
      />
      <span className="font-mono tabular-nums">
        <span className={valueClass ?? 'text-text'}>{value}</span>
        {suffix && <span className="text-muted">{suffix}</span>}
      </span>
    </div>
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

export function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex h-full items-center justify-center p-8 text-center text-sm text-muted">
      {message}
    </div>
  )
}
