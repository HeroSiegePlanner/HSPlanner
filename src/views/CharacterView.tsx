import { useMemo, type ReactNode } from 'react'
import { CornerMarks } from '../components/CornerMarks'
import { SkillIconImage } from '../components/SkillIconImage'
import {
  gameConfig,
  getClass,
  getClassIcon,
  getSkillsByClass,
  resolveSkillIcon,
} from '../data'
import {
  attrPointsFor,
  skillPointsFor,
  subskillKey,
  useBuild,
} from '../store/build'
import { getSavedBuild } from '../utils/build/savedBuilds'
import { computeBuildPerformanceAsync } from '../lib/calc/bridge'
import { useBuildPerformanceDeps } from '../hooks/useBuildPerformanceDeps'
import { useCalcResult } from '../hooks/useCalcResult'
import type { BuildPerformance } from '../utils/build/buildPerformance'
import { effectiveCap, formatValue, isZero, rangedMax } from '../utils/item/stats'
import { isImageUrl } from '../utils/imageUrl'
import type { RangedValue } from '../types'

const ATTRIBUTE_ORDER = [
  'strength',
  'dexterity',
  'intelligence',
  'energy',
  'vitality',
  'armor',
] as const

const ATTR_COLOR: Record<string, string> = {
  strength: 'text-stat-orange',
  dexterity: 'text-stat-green',
  intelligence: 'text-stat-purple',
  energy: 'text-stat-blue',
  vitality: 'text-stat-red',
  armor: 'text-muted',
}

const ATTR_BAR: Record<string, string> = {
  strength: 'var(--color-stat-orange)',
  dexterity: 'var(--color-stat-green)',
  intelligence: 'var(--color-stat-purple)',
  energy: 'var(--color-stat-blue)',
  vitality: 'var(--color-stat-red)',
  armor: 'var(--color-muted)',
}

const RESISTANCES: { key: string; label: string; cls: string }[] = [
  { key: 'fire_resistance', label: 'Fire', cls: 'text-stat-red' },
  { key: 'cold_resistance', label: 'Cold', cls: 'text-stat-blue' },
  { key: 'lightning_resistance', label: 'Lightning', cls: 'text-stat-orange' },
  { key: 'poison_resistance', label: 'Poison', cls: 'text-stat-green' },
  { key: 'arcane_resistance', label: 'Arcane', cls: 'text-stat-purple' },
]

function fmtInt(n: number): string {
  return Math.round(n).toLocaleString('en-US')
}

function intRange(
  min: number | undefined | null,
  max: number | undefined | null,
): string {
  if (min == null || max == null) return '—'
  if (Math.abs(min - max) < 0.5) return fmtInt(max)
  return `${fmtInt(min)}–${fmtInt(max)}`
}

function fmtRate(n: number): string {
  return (Math.round(n * 100) / 100).toString()
}

interface LoadoutEntry {
  key: string
  icon: ReturnType<typeof resolveSkillIcon>
  name: string
  sub: string
  detail: string
}

export default function CharacterView() {
  const classId = useBuild((s) => s.classId)
  const level = useBuild((s) => s.level)
  const allocated = useBuild((s) => s.allocated)
  const skillRanks = useBuild((s) => s.skillRanks)
  const activeSkillIds = useBuild((s) => s.activeSkillIds)
  const activeAuraId = useBuild((s) => s.activeAuraId)
  const activeBuffs = useBuild((s) => s.activeBuffs)
  const procToggles = useBuild((s) => s.procToggles)
  const subskillRanks = useBuild((s) => s.subskillRanks)
  const activeBuildId = useBuild((s) => s.activeBuildId)

  const buildDeps = useBuildPerformanceDeps()
  const performance = useCalcResult<BuildPerformance | null>(
    () => computeBuildPerformanceAsync(buildDeps),
    [buildDeps],
    null,
  )
  const stats = performance?.stats ?? {}
  const statsCombined = performance?.statsCombined ?? {}
  const attributes = performance?.attributes ?? {}
  const damage = performance?.damage ?? null
  const attackDamage = performance?.attackDamage ?? null

  const cls = classId ? getClass(classId) : undefined
  const classIcon = classId ? getClassIcon(classId) : undefined
  const buildName = activeBuildId
    ? (getSavedBuild(activeBuildId)?.name ?? null)
    : null

  const attrSpent = Object.values(allocated).reduce((s, v) => s + v, 0)
  const attrTotal = attrPointsFor(level)
  const skillSpent = Object.values(skillRanks).reduce((s, v) => s + v, 0)
  const skillTotal = skillPointsFor(level)
  const treeNodes = buildDeps.allocatedTreeNodes.size

  const allClassSkills = useMemo(() => getSkillsByClass(classId), [classId])
  const mainSkills = activeSkillIds
    .map((id) => allClassSkills.find((s) => s.id === id))
    .filter((s): s is NonNullable<typeof s> => !!s)
  const aura = activeAuraId
    ? allClassSkills.find((s) => s.id === activeAuraId)
    : undefined
  const buffList = useMemo(
    () => allClassSkills.filter((s) => !!activeBuffs[s.id]),
    [allClassSkills, activeBuffs],
  )

  const activeSkills: LoadoutEntry[] = [
    ...mainSkills.map((s, i) => ({
      key: `main-${s.id}`,
      icon: resolveSkillIcon(s),
      name: s.name,
      sub: i === 0 ? 'Main' : 'Active',
      detail: `Lv ${skillRanks[s.id] ?? 0}`,
    })),
    ...(aura
      ? [
          {
            key: `aura-${aura.id}`,
            icon: resolveSkillIcon(aura),
            name: aura.name,
            sub: 'Aura',
            detail: `Lv ${skillRanks[aura.id] ?? 0}`,
          },
        ]
      : []),
  ]

  const buffs: LoadoutEntry[] = buffList.map((s) => ({
    key: `buff-${s.id}`,
    icon: resolveSkillIcon(s),
    name: s.name,
    sub: 'Buff',
    detail: s.effectDuration !== undefined ? `${s.effectDuration}s` : '',
  }))

  const procs: LoadoutEntry[] = useMemo(() => {
    const out: LoadoutEntry[] = []
    for (const owner of allClassSkills) {
      const op = owner.proc
      if (op && (skillRanks[owner.id] ?? 0) > 0 && procToggles[owner.id]) {
        out.push({
          key: `proc-${owner.id}`,
          icon: resolveSkillIcon(owner),
          name: owner.name,
          sub: `→ ${op.target}`,
          detail: `${op.chance}% · ${op.trigger.replace('on_', '')}`,
        })
      }
      for (const subskill of owner.subskills ?? []) {
        const sp = subskill.proc
        if (!sp?.target) continue
        const toggleKey = subskillKey(owner.id, subskill.id)
        const rank = subskillRanks[toggleKey] ?? 0
        if (rank === 0 || !procToggles[toggleKey]) continue
        const chance =
          (sp.chance.base ?? 0) + (sp.chance.perRank ?? 0) * rank
        out.push({
          key: `proc-${toggleKey}`,
          icon: resolveSkillIcon(owner),
          name: subskill.name,
          sub: `→ ${sp.target}`,
          detail: `${chance}% · ${sp.trigger.replace('on_', '')}`,
        })
      }
    }
    return out
  }, [allClassSkills, skillRanks, subskillRanks, procToggles])

  const dpsText = intRange(
    performance?.combinedDpsMin,
    performance?.combinedDpsMax,
  )
  const avgHit = damage?.avgMax ?? attackDamage?.combinedAvgMax
  const hitDpsMax = performance?.hitDpsMax
  const rate =
    attackDamage?.attacksPerSecondMax ??
    (hitDpsMax != null && avgHit ? hitDpsMax / avgHit : undefined)
  const rateLabel = attackDamage ? 'Attack rate' : 'Cast rate'
  const hitDmg = damage
    ? intRange(damage.finalMin, damage.finalMax)
    : attackDamage
      ? intRange(attackDamage.combinedHitMin, attackDamage.combinedHitMax)
      : '—'
  const skillName = performance?.activeSkillName ?? mainSkills[0]?.name ?? null

  const stat = (key: string): RangedValue =>
    statsCombined[key] ?? stats[key] ?? 0
  const resistCap = effectiveCap('fire_resistance', stats) ?? 75

  return (
    <div className="mx-auto max-w-[1400px] space-y-4">
      <header>
        <div className="mb-1 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
          <span
            aria-hidden
            className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
            style={{ boxShadow: '0 0 8px rgba(224,184,100,0.6)' }}
          />
          Summary
        </div>
        <h2
          className="m-0 text-[22px] font-semibold tracking-[0.02em] text-accent-hot"
          style={{ textShadow: '0 0 16px rgba(224,184,100,0.18)' }}
        >
          Character
        </h2>
      </header>

      <Card className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex items-center gap-4">
          <div
            className="relative grid h-16 w-16 shrink-0 place-items-center overflow-hidden rounded-[4px] border border-accent-deep/50"
            style={{ background: 'linear-gradient(135deg, #1a1712, #0d0e12)' }}
          >
            {classIcon && isImageUrl(classIcon) ? (
              <img
                src={classIcon}
                alt=""
                className="h-11 w-11 object-contain"
                draggable={false}
              />
            ) : (
              <span
                aria-hidden
                className="inline-block h-5 w-5 rotate-45 bg-accent-hot/80"
                style={{ boxShadow: '0 0 14px rgba(224,184,100,0.4)' }}
              />
            )}
            <span className="absolute bottom-0.5 right-1 font-mono text-[10px] font-semibold tabular-nums text-accent-hot">
              {level}
            </span>
          </div>
          <div className="min-w-0">
            <div
              className="text-[22px] font-semibold leading-tight tracking-[0.01em] text-accent-hot"
              style={{ textShadow: '0 0 14px rgba(224,184,100,0.18)' }}
            >
              {buildName ?? 'Unsaved build'}
            </div>
            <div className="mt-1 flex flex-wrap items-center gap-2 font-mono text-[11px] uppercase tracking-[0.16em] text-muted">
              <span>{cls?.name ?? 'No class'}</span>
              <span className="text-faint">·</span>
              <span>Lv {level}</span>
              {cls?.primaryAttribute && (
                <span
                  className="inline-flex items-center gap-1.5 rounded-[3px] border border-accent-deep/40 px-2 py-0.5 text-[10px] text-accent-hot"
                  style={{
                    background:
                      'linear-gradient(180deg, rgba(58,46,24,0.6), rgba(42,36,24,0.4))',
                  }}
                >
                  <span
                    aria-hidden
                    className="inline-block h-1 w-1 rotate-45 bg-accent-hot"
                  />
                  {cls.primaryAttribute}
                </span>
              )}
            </div>
          </div>
        </div>

        <div className="flex items-stretch gap-2">
          <PointStat label="Attr used" value={attrSpent} total={attrTotal} />
          <PointStat label="Skill used" value={skillSpent} total={skillTotal} />
          <PointStat label="Tree nodes" value={treeNodes} />
        </div>
      </Card>

      <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
        {ATTRIBUTE_ORDER.map((key) => {
          const attr = gameConfig.attributes.find((a) => a.key === key)
          if (!attr) return null
          const base =
            (gameConfig.defaultBaseAttributes?.[key] ?? 0) +
            (cls?.baseAttributes[key] ?? 0) +
            (allocated[key] ?? 0)
          const finalNum = rangedMax(attributes[key] ?? 0)
          const delta = Math.round(finalNum - base)
          const color = ATTR_COLOR[key] ?? 'text-text'
          return (
            <div
              key={key}
              className="relative overflow-hidden rounded-md border border-border px-3 py-2.5"
              style={{
                background:
                  'linear-gradient(180deg, var(--color-panel), color-mix(in srgb, var(--color-bg) 70%, transparent))',
                boxShadow: 'inset 0 1px 0 rgba(255,255,255,0.02)',
              }}
            >
              <div className="flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
                <span
                  aria-hidden
                  className={`inline-block h-1 w-1 rotate-45 ${color}`}
                  style={{ backgroundColor: 'currentColor' }}
                />
                {attr.name}
              </div>
              <div
                className={`mt-1 font-mono text-[26px] font-semibold tabular-nums ${color}`}
              >
                {formatValue(attributes[key] ?? 0, key)}
              </div>
              <div className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
                {delta > 0 ? `+${delta} added` : 'base'}
              </div>
              <span
                aria-hidden
                className="absolute inset-x-0 bottom-0 h-[2px] opacity-70"
                style={{
                  background: `linear-gradient(90deg, transparent, ${ATTR_BAR[key]}, transparent)`,
                }}
              />
            </div>
          )
        })}
      </div>

      <div className="grid gap-4 lg:grid-cols-[1.4fr_1fr]">
        <Card>
          <SectionHead title={`Total DPS${skillName ? ` · ${skillName}` : ''}`} />
          <div
            className="font-mono text-[44px] font-semibold leading-none tracking-[0.01em] text-accent-hot tabular-nums"
            style={{ textShadow: '0 0 18px rgba(224,184,100,0.22)' }}
          >
            {dpsText}
          </div>
          <div className="mt-2 font-mono text-[11px] uppercase tracking-[0.14em] text-faint">
            {avgHit != null && rate != null ? (
              <>
                <span className="text-muted">{fmtInt(avgHit)}</span> avg hit ×{' '}
                <span className="text-muted">{fmtRate(rate)}</span> / sec
              </>
            ) : (
              'select a main skill to see the breakdown'
            )}
          </div>

          <div className="mt-4 grid grid-cols-2 gap-2.5 sm:grid-cols-3">
            <Metric
              label="Crit chance"
              value={formatValue(stat('crit_chance'), 'crit_chance')}
              valueClass="text-accent-hot"
            />
            <Metric
              label="Crit damage"
              value={formatValue(stat('crit_damage'), 'crit_damage')}
            />
            <Metric
              label={rateLabel}
              value={rate != null ? `${fmtRate(rate)}/s` : '—'}
            />
            <Metric label="Hit damage" value={hitDmg} />
            <Metric
              label="Attack speed"
              value={formatValue(
                stat('increased_attack_speed'),
                'increased_attack_speed',
              )}
            />
            <Metric
              label="Enhanced dmg"
              value={formatValue(stat('enhanced_damage'), 'enhanced_damage')}
            />
          </div>
        </Card>

        <Card>
          <SectionHead
            title="Resistances & Defense"
            trailing={
              <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
                Capped <span className="text-text">{resistCap}</span>
              </span>
            }
          />
          <div className="space-y-2">
            {RESISTANCES.map((r) => {
              const v = stat(r.key)
              const numeric = typeof v === 'number' ? v : rangedMax(v)
              const cap = effectiveCap(r.key, stats)
              const zero = isZero(v)
              const capped = cap !== undefined && numeric > cap
              const pct = Math.max(
                0,
                Math.min(100, (numeric / (cap ?? 100)) * 100),
              )
              return (
                <div key={r.key} className="flex items-center gap-3">
                  <span
                    className={`w-24 shrink-0 font-mono text-[10px] uppercase tracking-[0.12em] ${r.cls}`}
                  >
                    {r.label}
                  </span>
                  <span
                    className={`relative h-1.5 flex-1 overflow-hidden rounded-full bg-panel-2 ${r.cls}`}
                  >
                    <span
                      className="absolute inset-y-0 left-0 rounded-full opacity-80"
                      style={{
                        width: `${pct}%`,
                        backgroundColor: 'currentColor',
                      }}
                    />
                  </span>
                  <span
                    className={`shrink-0 text-right font-mono text-[12px] tabular-nums ${zero ? 'text-faint' : numeric < 0 ? 'text-stat-red' : r.cls}`}
                  >
                    {zero ? (
                      '—'
                    ) : capped ? (
                      <>
                        +{cap}%{' '}
                        <span className="text-[10px] text-faint">
                          ({numeric}%)
                        </span>
                      </>
                    ) : (
                      `${numeric >= 0 ? '+' : ''}${numeric}%`
                    )}
                  </span>
                </div>
              )
            })}
          </div>

          <div className="my-3 border-t border-dashed border-accent-deep/25" />

          <div className="space-y-px">
            <DefRow
              label="Life"
              value={stat('life')}
              statKey="life"
              accent="text-stat-red"
            />
            <DefRow
              label="Mana"
              value={stat('mana')}
              statKey="mana"
              accent="text-stat-blue"
            />
            <DefRow
              label="Block chance"
              value={stat('block_chance')}
              statKey="block_chance"
            />
            <DefRow
              label="Phys reduction"
              value={stat('physical_damage_reduction')}
              statKey="physical_damage_reduction"
              hint={
                !isZero(stat('defense'))
                  ? `armor ${formatValue(stat('defense'), 'defense')}`
                  : undefined
              }
            />
            <DefRow
              label="Movement speed"
              value={stat('movement_speed')}
              statKey="movement_speed"
            />
          </div>
        </Card>
      </div>

      <div className="grid gap-4 lg:grid-cols-3">
        <LoadoutCard
          title="Active Skills"
          entries={activeSkills}
          empty="No active skill selected."
        />
        <LoadoutCard title="Buffs" entries={buffs} empty="No buffs active." />
        <LoadoutCard title="Procs" entries={procs} empty="No procs active." />
      </div>
    </div>
  )
}

function LoadoutCard({
  title,
  entries,
  empty,
}: {
  title: string
  entries: LoadoutEntry[]
  empty: string
}) {
  return (
    <Card>
      <SectionHead
        title={title}
        trailing={
          <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
            {entries.length} active
          </span>
        }
      />
      {entries.length === 0 ? (
        <p className="font-mono text-[12px] tracking-[0.04em] text-muted italic">
          {empty}
        </p>
      ) : (
        <ul className="space-y-2">
          {entries.map((e) => (
            <li
              key={e.key}
              className="flex items-center gap-2.5 rounded-[3px] border border-border-2 px-3 py-2"
              style={{
                background:
                  'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
                boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.4)',
              }}
            >
              <SkillIconImage icon={e.icon} size={32} className="text-2xl" />
              <span className="min-w-0 flex-1">
                <span className="block truncate text-[13px] font-medium text-text">
                  {e.name}
                </span>
                <span className="block truncate font-mono text-[10px] uppercase tracking-[0.16em] text-accent-deep">
                  {e.sub}
                </span>
              </span>
              {e.detail && (
                <span className="shrink-0 font-mono text-[10px] uppercase tracking-[0.14em] text-faint tabular-nums">
                  {e.detail}
                </span>
              )}
            </li>
          ))}
        </ul>
      )}
    </Card>
  )
}

function Card({
  children,
  className,
}: {
  children: ReactNode
  className?: string
}) {
  return (
    <section
      className={`relative overflow-hidden rounded-md border border-border p-4 ${className ?? ''}`}
      style={{
        background:
          'linear-gradient(180deg, var(--color-panel), color-mix(in srgb, var(--color-bg) 70%, transparent))',
        boxShadow:
          'inset 0 1px 0 rgba(255,255,255,0.02), 0 8px 24px rgba(0,0,0,0.35)',
      }}
    >
      <CornerMarks size={8} opacity={0.45} />
      {children}
    </section>
  )
}

function SectionHead({
  title,
  trailing,
}: {
  title: string
  trailing?: ReactNode
}) {
  return (
    <div className="mb-3 flex items-center justify-between gap-3 border-b border-accent-deep/20 pb-2">
      <div className="flex items-center gap-2">
        <span
          aria-hidden
          className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
          style={{ boxShadow: '0 0 6px rgba(224,184,100,0.5)' }}
        />
        <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-accent-hot/70">
          {title}
        </span>
      </div>
      {trailing}
    </div>
  )
}

function PointStat({
  label,
  value,
  total,
}: {
  label: string
  value: number
  total?: number
}) {
  return (
    <div
      className="min-w-24 rounded-[3px] border border-border-2 px-3 py-2"
      style={{
        background: 'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
        boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
      }}
    >
      <div className="font-mono text-[9px] uppercase tracking-[0.18em] text-faint">
        {label}
      </div>
      <div className="mt-0.5 font-mono text-[18px] font-semibold tabular-nums text-accent-hot">
        {value}
        {total !== undefined && (
          <span className="text-[11px] font-normal text-faint"> / {total}</span>
        )}
      </div>
    </div>
  )
}

function Metric({
  label,
  value,
  hint,
  valueClass,
}: {
  label: string
  value: ReactNode
  hint?: string
  valueClass?: string
}) {
  return (
    <div
      className="rounded-[3px] border border-border-2 px-3 py-2.5"
      style={{
        background:
          'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
        boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.4)',
      }}
    >
      <div className="font-mono text-[9px] uppercase tracking-[0.18em] text-faint">
        {label}
      </div>
      <div
        className={`mt-1 font-mono text-[18px] font-semibold tabular-nums ${valueClass ?? 'text-text'}`}
      >
        {value}
      </div>
      {hint && (
        <div className="mt-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-faint">
          {hint}
        </div>
      )}
    </div>
  )
}

function DefRow({
  label,
  value,
  statKey,
  accent,
  hint,
}: {
  label: string
  value: RangedValue
  statKey: string
  accent?: string
  hint?: string
}) {
  const zero = isZero(value)
  return (
    <div className="flex items-baseline justify-between gap-2 py-0.75">
      <span className="flex-1 text-muted">{label}</span>
      {hint && (
        <span className="font-mono text-[10px] uppercase tracking-[0.12em] text-faint">
          {hint}
        </span>
      )}
      <span
        className={`shrink-0 text-right font-mono tabular-nums ${zero ? 'text-faint' : (accent ?? 'text-text')}`}
      >
        {zero ? '—' : formatValue(value, statKey)}
      </span>
    </div>
  )
}
