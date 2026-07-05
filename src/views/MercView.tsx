import { useCallback, useMemo, useState } from 'react'
import { motion } from 'motion/react'
import { hoverTap } from '../lib/motion'
import {
  gameConfig,
  getClassIcon,
  getItem,
  getMercClass,
  mercData,
  resolveSkillIcon,
} from '../data'
import { useBuild } from '../store/build'
import { useCalcResult } from '../hooks/useCalcResult'
import { computeBuildStatsAsync } from '../lib/calc/bridge'
import type { ComputedStats } from '../utils/item/stats'
import { formatValue, isZero } from '../utils/item/stats'
import {
  hasMercGear,
  mercGrantedAuras,
  mercOnlyDeps,
  mercSharedEffects,
} from '../utils/build/mercStats'
import type { MercSkill, SlotKey } from '../types'
import { GearPanel, SlotRow } from './gear/SlotRail'
import { GearSlotModal } from './gear/GearSlotModal'
import { getSocketPickerRows } from './gear/lib/socketPickerRows'

function SkillRow({
  skill,
  classId,
  rank,
  maxRank,
  onSetRank,
}: {
  skill: MercSkill
  classId: string
  rank: number
  maxRank: number
  onSetRank: (rank: number) => void
}) {
  const icon = resolveSkillIcon({ id: skill.id, classId })
  return (
    <li
      className={`flex items-start gap-2.5 rounded-[3px] border px-2.5 py-2 transition-colors ${
        rank > 0
          ? 'border-accent-deep/50 bg-accent-hot/4'
          : 'border-border bg-transparent'
      }`}
    >
      <span
        className="mt-0.5 flex h-[34px] w-[34px] shrink-0 items-center justify-center overflow-hidden rounded-[3px] border border-border-2"
        style={{
          background: 'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
          boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
        }}
      >
        {icon ? (
          <img
            src={icon}
            alt=""
            draggable={false}
            className="h-full w-full object-contain select-none"
            style={{ imageRendering: 'pixelated' }}
          />
        ) : (
          <span className="text-sm text-faint">◆</span>
        )}
      </span>
      <span className="min-w-0 flex-1">
        <span className="flex items-baseline gap-2">
          <span
            className={`truncate text-[12px] font-semibold ${
              rank > 0 ? 'text-accent-hot' : 'text-text'
            }`}
          >
            {skill.name}
          </span>
          <span className="shrink-0 font-mono text-[8.5px] uppercase tracking-[0.16em] text-faint">
            {skill.kind}
            {skill.damageType ? ` · ${skill.damageType}` : ''}
          </span>
          {skill.shared && (
            <span
              className="shrink-0 rounded-[2px] border border-stat-green/40 px-1 py-px font-mono text-[8.5px] uppercase tracking-[0.14em] text-stat-green"
              title="This skill also benefits your hero"
            >
              Hero
            </span>
          )}
        </span>
        <span className="mt-0.5 block text-[10.5px] leading-snug text-muted">
          {skill.description}
        </span>
      </span>
      <span className="flex shrink-0 items-center gap-1 self-center">
        <button
          type="button"
          onClick={() => onSetRank(rank - 1)}
          disabled={rank <= 0}
          className="h-6 w-6 rounded-[3px] border border-border-2 font-mono text-[12px] leading-none text-muted transition-colors hover:border-accent-deep hover:text-accent-hot disabled:cursor-not-allowed disabled:opacity-30"
          aria-label={`Decrease ${skill.name} rank`}
        >
          −
        </button>
        <span
          className={`w-12 text-center font-mono text-[11px] tabular-nums ${
            rank > 0 ? 'text-accent-hot' : 'text-faint'
          }`}
        >
          {rank}/{maxRank}
        </span>
        <button
          type="button"
          onClick={() => onSetRank(rank + 1)}
          disabled={rank >= maxRank}
          className="h-6 w-6 rounded-[3px] border border-border-2 font-mono text-[12px] leading-none text-muted transition-colors hover:border-accent-deep hover:text-accent-hot disabled:cursor-not-allowed disabled:opacity-30"
          aria-label={`Increase ${skill.name} rank`}
        >
          +
        </button>
      </span>
    </li>
  )
}

export default function MercView() {
  const mercClassId = useBuild((s) => s.mercClassId)
  const mercSkillRanks = useBuild((s) => s.mercSkillRanks)
  const mercInventory = useBuild((s) => s.mercInventory)
  const setMercClass = useBuild((s) => s.setMercClass)
  const setMercSkillRank = useBuild((s) => s.setMercSkillRank)
  const commitMercItem = useBuild((s) => s.commitMercItem)
  const resetMerc = useBuild((s) => s.resetMerc)

  const [activeSlot, setActiveSlot] = useState<SlotKey | null>(null)
  const handleModalClose = useCallback(() => setActiveSlot(null), [])

  const cls = getMercClass(mercClassId)
  const mercSlots = useMemo(
    () =>
      mercData.slots.map((key) => ({
        key,
        name: gameConfig.slots.find((s) => s.key === key)?.name ?? key,
      })),
    [],
  )

  const mercDeps = useMemo(() => mercOnlyDeps(mercInventory), [mercInventory])
  const gearEquipped = hasMercGear(mercInventory)
  const mercComputed = useCalcResult<ComputedStats | null>(
    () => (gearEquipped ? computeBuildStatsAsync(mercDeps) : null),
    [mercDeps, gearEquipped],
    null,
  )
  const mercMagicFind = mercComputed?.stats.magic_find ?? 0
  const sharedEffects = useMemo(() => {
    const auras = mercGrantedAuras(mercInventory).map((a) => ({
      itemName: a.itemName,
      effect: `Grants ${a.name} Level ${
        a.levelMin === a.levelMax
          ? a.levelMin
          : `[${a.levelMin}-${a.levelMax}]`
      }`,
    }))
    return [...auras, ...mercSharedEffects(mercInventory)]
  }, [mercInventory])
  const sharedSkills = useMemo(
    () =>
      (cls?.skills ?? []).filter(
        (s) => s.shared && (mercSkillRanks[s.id] ?? 0) > 0,
      ),
    [cls, mercSkillRanks],
  )
  const usedSlots = mercSlots.filter((s) => mercInventory[s.key]).length
  const pointsSpent = Object.values(mercSkillRanks).reduce((a, b) => a + b, 0)

  const weaponBase = mercInventory.weapon
    ? getItem(mercInventory.weapon.baseId)
    : undefined
  const offhandLocked = !!weaponBase?.twoHanded

  return (
    <div className="w-full">
      <header className="mb-4">
        <div className="mb-1 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
          <span
            aria-hidden
            className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
            style={{ boxShadow: '0 0 8px rgba(224,184,100,0.6)' }}
          />
          Config
        </div>
        <div className="flex items-end justify-between gap-3">
          <h2
            className="m-0 text-[22px] font-semibold tracking-[0.02em] text-accent-hot"
            style={{ textShadow: '0 0 16px rgba(224,184,100,0.18)' }}
          >
            Mercenary
          </h2>
          <div className="flex items-center gap-3 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
            {cls && (
              <>
                <span>
                  <span className="text-text">{usedSlots}</span> / {mercSlots.length} equipped
                </span>
                <span aria-hidden className="h-3 w-px bg-border" />
                <span>
                  <span className="text-text">{pointsSpent}</span> skill points
                </span>
                <span aria-hidden className="h-3 w-px bg-border" />
                <button
                  type="button"
                  onClick={resetMerc}
                  className="rounded-[3px] border border-border-2 bg-transparent px-2.5 py-1 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-stat-red hover:text-stat-red"
                >
                  Dismiss
                </button>
              </>
            )}
          </div>
        </div>
      </header>

      <div className="mb-4 grid gap-2.5 sm:grid-cols-3">
        {mercData.classes.map((c) => {
          const portrait = getClassIcon(c.id)
          const selected = mercClassId === c.id
          return (
            <motion.button
              key={c.id}
              type="button"
              onClick={() => setMercClass(c.id)}
              {...hoverTap}
              className={`relative flex items-center gap-3 rounded-[3px] border px-3 py-2.5 text-left transition-colors ${
                selected
                  ? 'border-accent-hot bg-accent-hot/8 ring-1 ring-accent-hot/40'
                  : 'border-border bg-panel hover:border-accent-deep'
              }`}
            >
              <span
                className="flex h-12 w-12 shrink-0 items-center justify-center overflow-hidden rounded-[3px] border border-border-2"
                style={{
                  background:
                    'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
                  boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
                }}
              >
                {portrait ? (
                  <img
                    src={portrait}
                    alt=""
                    draggable={false}
                    className="h-full w-full object-contain select-none"
                    style={{ imageRendering: 'pixelated' }}
                  />
                ) : (
                  <span className="text-lg text-faint">◆</span>
                )}
              </span>
              <span className="min-w-0">
                <span
                  className={`block text-[13px] font-semibold ${
                    selected ? 'text-accent-hot' : 'text-text'
                  }`}
                >
                  {c.name}
                </span>
                <span className="block font-mono text-[9px] uppercase tracking-[0.16em] text-muted">
                  {c.role}
                </span>
                <span className="block truncate text-[10px] text-faint">
                  {c.location}
                </span>
              </span>
              {selected && (
                <span
                  aria-hidden
                  className="absolute right-2.5 top-2.5 inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
                  style={{ boxShadow: '0 0 8px rgba(224,184,100,0.6)' }}
                />
              )}
            </motion.button>
          )
        })}
      </div>

      {!cls ? (
        <div
          className="rounded-[3px] border border-border px-4 py-10 text-center"
          style={{
            background:
              'linear-gradient(180deg, var(--color-panel), var(--color-bg))',
          }}
        >
          <p className="m-0 text-[13px] text-muted">
            No mercenary hired — pick a class above.
          </p>
          <p className="mx-auto mb-0 mt-1.5 max-w-md text-[11px] leading-relaxed text-faint">
            Mercenaries fight beside your hero. Their Magic Find counts for
            your drops, and buffs from their items are shared with you.
          </p>
        </div>
      ) : (
        <div className="grid items-start gap-4 lg:grid-cols-[2fr_1fr]">
          <div className="flex min-w-0 flex-col gap-4">
            <GearPanel
              title="Loadout"
              trailing={
                <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
                  <span className={usedSlots > 0 ? 'text-accent-hot' : 'text-muted'}>
                    {usedSlots}
                  </span>
                  <span className="text-faint"> / {mercSlots.length}</span> equipped
                </span>
              }
            >
              <ul className="grid grid-cols-1 gap-1.5 sm:grid-cols-2">
                {mercSlots.map((slot) => (
                  <li key={slot.key}>
                    <SlotRow
                      slot={slot}
                      equipped={mercInventory[slot.key]}
                      active={activeSlot === slot.key}
                      locked={slot.key === 'offhand' && offhandLocked}
                      onSelect={() => setActiveSlot(slot.key)}
                    />
                  </li>
                ))}
              </ul>
            </GearPanel>

            <GearPanel
              title={`${cls.name} Skills`}
              trailing={
                <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
                  <span className={pointsSpent > 0 ? 'text-accent-hot' : 'text-muted'}>
                    {pointsSpent}
                  </span>{' '}
                  points
                </span>
              }
            >
              <ul className="m-0 flex list-none flex-col gap-1.5 p-0">
                {cls.skills.map((skill) => (
                  <SkillRow
                    key={skill.id}
                    skill={skill}
                    classId={cls.id}
                    rank={mercSkillRanks[skill.id] ?? 0}
                    maxRank={mercData.maxSkillRank}
                    onSetRank={(rank) =>
                      setMercSkillRank(skill.id, rank, mercData.maxSkillRank)
                    }
                  />
                ))}
              </ul>
            </GearPanel>
          </div>

          <div
            className="rounded-[3px] border border-border"
            style={{
              background:
                'linear-gradient(180deg, var(--color-panel), var(--color-bg))',
            }}
          >
            <div className="flex items-center justify-between border-b border-border px-3 py-2">
              <span className="inline-flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.18em] text-stat-green">
                <span
                  aria-hidden
                  className="inline-block h-1.5 w-1.5 rotate-45 bg-stat-green"
                  style={{ boxShadow: '0 0 8px rgba(116,201,138,0.5)' }}
                />
                Shared with Hero
              </span>
            </div>

            <div
              className="flex items-baseline justify-between border-b border-border px-3 py-2"
              title="Magic Find from the mercenary's gear counts toward your hero — see the Stats tab"
            >
              <span className="text-[12px] font-medium text-text">
                Magic Find
              </span>
              <span className="font-mono text-[12px] text-accent-hot">
                {gearEquipped && mercComputed && !isZero(mercMagicFind)
                  ? formatValue(mercMagicFind, 'magic_find')
                  : '—'}
              </span>
            </div>

            <div className="px-3 py-2.5">
              <div className="mb-1.5 font-mono text-[9px] uppercase tracking-[0.16em] text-faint">
                Item buffs
              </div>
              {sharedEffects.length === 0 ? (
                <p className="m-0 pb-1 text-[11px] italic text-muted">
                  No shared item buffs — equip uniques with effects (e.g.
                  Pearlescent Dream).
                </p>
              ) : (
                <ul className="m-0 flex list-none flex-col gap-1.5 p-0 pb-1">
                  {sharedEffects.map((e, i) => (
                    <li key={`${e.itemName}-${e.effect}-${i}`} className="leading-snug">
                      <span className="block text-[11.5px] text-stat-green">
                        {e.effect}
                      </span>
                      <span className="block text-[10px] text-faint">
                        {e.itemName}
                      </span>
                    </li>
                  ))}
                </ul>
              )}

              <div className="mb-1.5 mt-2.5 border-t border-border pt-2 font-mono text-[9px] uppercase tracking-[0.16em] text-faint">
                Skill effects
              </div>
              {sharedSkills.length === 0 ? (
                <p className="m-0 pb-1 text-[11px] italic text-muted">
                  No hero-affecting skills leveled yet.
                </p>
              ) : (
                <ul className="m-0 flex list-none flex-col gap-1.5 p-0 pb-1">
                  {sharedSkills.map((s) => (
                    <li key={s.id} className="leading-snug">
                      <span className="block text-[11.5px] text-stat-green">
                        {s.name}{' '}
                        <span className="font-mono text-[10px] text-faint">
                          {mercSkillRanks[s.id]}/{mercData.maxSkillRank}
                        </span>
                      </span>
                      <span className="block text-[10px] text-faint">
                        {s.description}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>
        </div>
      )}

      {activeSlot && (
        <GearSlotModal
          slot={activeSlot}
          slotName={
            mercSlots.find((s) => s.key === activeSlot)?.name ?? activeSlot
          }
          equipped={mercInventory[activeSlot]}
          offhandLocked={offhandLocked}
          socketPickerRows={getSocketPickerRows()}
          inventory={mercInventory}
          hideCompare
          onCommit={(item) => {
            commitMercItem(activeSlot, item)
            return null
          }}
          onClose={handleModalClose}
        />
      )}
    </div>
  )
}
