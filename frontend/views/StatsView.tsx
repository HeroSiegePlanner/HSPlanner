import { useCallback, useMemo, useState } from 'react'
import { classes, gameConfig, getItem, skills } from '@data'
import { useBuildPerformanceDeps } from '../hooks/useBuildPerformanceDeps'
import { useCalcResult } from '../hooks/useCalcResult'
import { useWeaponDamage } from '../hooks/useWeaponDamage'
import {
  computeBuildPerformanceAsync,
  computeBuildStatsAsync,
  computeStatBreakdownAsync,
} from '../utils/calc/bridge'
import type { StatBreakdown, StatBreakdownKind } from '../utils/calc/bridge'
import type { BuildPerformance } from '../utils/build/buildPerformance'
import { useBuild } from '../store/build'
import {
  groupStatKeysByCategory,
  isZero,
  normalizeSkillName,
  rangedMax,
  rangedMin,
  statName,
} from '../utils/item/stats'
import type { ComputedStats, SourceContribution } from '../utils/item/stats'
import type { RangedStatMap, RangedValue } from '../types'
import { etherMagicFindTotal } from '../utils/build/etherSummary'
import { hasMercGear, mercOnlyDeps } from '../utils/build/mercStats'
import type { NativeWeaponDamageInput } from '../utils/nativeDamage'
import type { Skill } from '../types'
import {
  FILTER_TABS,
  MITIGATION_KEYS,
  RESISTANCE_KEYS,
  RESOURCE_KEYS,
  SKILL_BONUS_KEYS,
  WORLD_LOOT_KEYS,
  ehpMatchesQuery,
} from './stats/statKeys'
import type { FilterTab } from './stats/statKeys'
import { Panel, SectionHeading, SubSectionLabel } from './stats/statPrimitives'
import { MainSkillSection } from './stats/MainSkillPanel'
import { SkillCard } from './stats/SkillCard'
import { AttributesStrip, StatRow } from './stats/rows'
import { EhpBreakdown } from './stats/EhpBreakdown'

const NO_SUBTREE: Record<string, RangedValue> = {}

export default function StatsView() {
  const classId = useBuild((s) => s.classId)
  const inventory = useBuild((s) => s.inventory)
  const skillRanks = useBuild((s) => s.skillRanks)
  const enemyConditions = useBuild((s) => s.enemyConditions)
  const skillProjectiles = useBuild((s) => s.skillProjectiles)
  const enemyResistances = useBuild((s) => s.enemyResistances)
  const activeSkillIds = useBuild((s) => s.activeSkillIds)
  const allocatedEtherNodes = useBuild((s) => s.allocatedEtherNodes)
  const mercInventory = useBuild((s) => s.mercInventory)
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState<FilterTab>('all')
  const normalizedQuery = query.trim().toLowerCase()
  const matches = (label: string) =>
    normalizedQuery.length > 0 &&
    label.toLowerCase().includes(normalizedQuery)
  const buildDeps = useBuildPerformanceDeps()
  const computed = useCalcResult<ComputedStats | null>(
    () => computeBuildStatsAsync(buildDeps),
    [buildDeps],
    null,
  )

  const [breakdownState, setBreakdownState] = useState<{
    depsKey: unknown
    cache: Record<string, StatBreakdown>
    inFlight: Record<string, true>
  }>(() => ({ depsKey: null, cache: {}, inFlight: {} }))
  const stale = breakdownState.depsKey !== buildDeps
  const activeCache = useMemo(
    () => (stale ? {} : breakdownState.cache),
    [stale, breakdownState.cache],
  )
  const activeInFlight = useMemo(
    () => (stale ? {} : breakdownState.inFlight),
    [stale, breakdownState.inFlight],
  )
  const requestBreakdown = useCallback(
    (statKey: string, kind: StatBreakdownKind) => {
      const key = `${kind}:${statKey}`
      if (activeCache[key] || activeInFlight[key]) return
      const depsAtRequest = buildDeps
      setBreakdownState((prev) => {
        const carry = prev.depsKey === depsAtRequest ? prev : {
          depsKey: depsAtRequest,
          cache: {},
          inFlight: {},
        }
        return {
          ...carry,
          inFlight: { ...carry.inFlight, [key]: true },
        }
      })
      computeStatBreakdownAsync(buildDeps, statKey, kind)
        .then((result) => {
          setBreakdownState((prev) => {
            if (prev.depsKey !== depsAtRequest) return prev
            const { [key]: _drop, ...restInFlight } = prev.inFlight
            void _drop
            return {
              depsKey: prev.depsKey,
              cache: { ...prev.cache, [key]: result },
              inFlight: restInFlight,
            }
          })
        })
        .catch(() => {
          setBreakdownState((prev) => {
            if (prev.depsKey !== depsAtRequest) return prev
            const { [key]: _drop, ...restInFlight } = prev.inFlight
            void _drop
            return { ...prev, inFlight: restInFlight }
          })
        })
    },
    [buildDeps, activeCache, activeInFlight],
  )
  const getBreakdown = (
    statKey: string,
    kind: StatBreakdownKind,
  ): StatBreakdown | null =>
    activeCache[`${kind}:${statKey}`] ?? null
  const attributes = useMemo(() => computed?.attributes ?? {}, [computed])
  const stats = useMemo(() => computed?.stats ?? {}, [computed])
  const attributeSources = useMemo(
    () => computed?.attributeSources ?? {},
    [computed],
  )
  const statSources = useMemo(() => computed?.statSources ?? {}, [computed])

  const mercDeps = useMemo(() => mercOnlyDeps(mercInventory), [mercInventory])
  const mercGearOn = hasMercGear(mercInventory)
  const mercComputed = useCalcResult<ComputedStats | null>(
    () => (mercGearOn ? computeBuildStatsAsync(mercDeps) : null),
    [mercDeps, mercGearOn],
    null,
  )
  const mercMagicFind: RangedValue = mercComputed?.stats.magic_find ?? 0
  const etherMagicFind = useMemo(
    () => etherMagicFindTotal(allocatedEtherNodes),
    [allocatedEtherNodes],
  )

  const displayStats = useMemo(() => {
    if (etherMagicFind === 0 && isZero(mercMagicFind)) return stats
    const add = (a: RangedValue | undefined, b: RangedValue): RangedValue => {
      const base = a ?? 0
      if (typeof base === 'number' && typeof b === 'number') return base + b
      return [rangedMin(base) + rangedMin(b), rangedMax(base) + rangedMax(b)]
    }
    const next = { ...stats }
    if (!isZero(mercMagicFind)) {
      next.magic_find = add(next.magic_find, mercMagicFind)
    }
    if (etherMagicFind !== 0) {
      next.magic_find_more = add(next.magic_find_more, etherMagicFind)
    }
    return next
  }, [stats, etherMagicFind, mercMagicFind])

  const displayStatSources = useMemo(() => {
    if (etherMagicFind === 0 && isZero(mercMagicFind)) return statSources
    const next = { ...statSources }
    if (!isZero(mercMagicFind)) {
      const contribution: SourceContribution = {
        label: 'Mercenary',
        sourceType: 'item',
        value: mercMagicFind,
      }
      next.magic_find = [...(next.magic_find ?? []), contribution]
    }
    if (etherMagicFind !== 0) {
      const contribution: SourceContribution = {
        label: 'Ether Tree',
        sourceType: 'tree',
        value: etherMagicFind,
      }
      next.magic_find_more = [...(next.magic_find_more ?? []), contribution]
    }
    return next
  }, [statSources, etherMagicFind, mercMagicFind])
  const itemSkillBonuses = useMemo(
    () => computed?.itemSkillBonuses ?? {},
    [computed],
  )
  const rankBonuses = useMemo(() => computed?.rankBonuses ?? {}, [computed])
  const skillScoped = useMemo(() => computed?.skillScoped ?? {}, [computed])
  const skillStatOverrides = useMemo(
    () => computed?.skillStatOverrides ?? {},
    [computed],
  )
  // Identity must stay stable: SkillCard memoizes its damage input on the stat map,
  // so a fresh object every render would re-fire the calc in a loop.
  const statsBySkill = useMemo(() => {
    const out: Record<string, RangedStatMap> = {}
    for (const [skillId, overrides] of Object.entries(skillStatOverrides)) {
      out[skillId] = { ...stats, ...overrides }
    }
    return out
  }, [stats, skillStatOverrides])
  const statsForSkill = useCallback(
    (skillId: string): RangedStatMap => statsBySkill[skillId] ?? stats,
    [statsBySkill, stats],
  )
  const statsCombined = useMemo(() => computed?.statsCombined ?? {}, [computed])

  const displayStatsCombined = useMemo(() => {
    if (etherMagicFind === 0 && isZero(mercMagicFind)) return statsCombined
    const base = displayStats.magic_find ?? 0
    const more = displayStats.magic_find_more ?? 0
    const combine = (b: number, m: number) => b * (1 + m / 100)
    const combined: RangedValue =
      typeof base === 'number' && typeof more === 'number'
        ? combine(base, more)
        : [
            combine(rangedMin(base), rangedMin(more)),
            combine(rangedMax(base), rangedMax(more)),
          ]
    return { ...statsCombined, magic_find: combined }
  }, [statsCombined, displayStats, etherMagicFind, mercMagicFind])

  const weaponInput = useMemo<NativeWeaponDamageInput>(() => {
    const equipped = inventory.weapon
    const base = equipped ? getItem(equipped.baseId) : undefined
    const weapon =
      base && base.damageMin !== undefined && base.damageMax !== undefined
        ? {
            name: base.name,
            damageMin: base.damageMin,
            damageMax: base.damageMax,
          }
        : undefined
    return { weapon, stats, enemyConditions, enemyResistances }
  }, [inventory, stats, enemyConditions, enemyResistances])
  const weaponDamage = useWeaponDamage(weaponInput)

  const grouped = useMemo(
    () =>
      groupStatKeysByCategory(gameConfig.stats, [
        'base',
        'offense',
        'defense',
        'resource',
        'utility',
      ]),
    [],
  )

  const knownStatKeys = useMemo(() => {
    const out = new Set<string>()
    for (const def of gameConfig.stats) {
      if (def.modifiesAttribute) continue
      if (def.itemOnly) continue
      if (def.skillScoped) continue
      out.add(def.key)
    }
    return out
  }, [])
  const mitigationKeys = useMemo(
    () => MITIGATION_KEYS.filter((k) => knownStatKeys.has(k)),
    [knownStatKeys],
  )
  const resistanceKeys = useMemo(
    () => RESISTANCE_KEYS.filter((k) => knownStatKeys.has(k)),
    [knownStatKeys],
  )
  const resourceKeys = useMemo(
    () => RESOURCE_KEYS.filter((k) => knownStatKeys.has(k)),
    [knownStatKeys],
  )
  const skillBonusKeys = useMemo(
    () => SKILL_BONUS_KEYS.filter((k) => knownStatKeys.has(k)),
    [knownStatKeys],
  )
  const worldLootKeys = useMemo(
    () => WORLD_LOOT_KEYS.filter((k) => knownStatKeys.has(k)),
    [knownStatKeys],
  )
  const offenseKeys = useMemo(() => {
    const claimed = new Set<string>([...resourceKeys])
    return (grouped.offense ?? []).filter((k) => !claimed.has(k))
  }, [grouped, resourceKeys])
  const otherKeys = useMemo(() => {
    const claimed = new Set<string>([
      ...mitigationKeys,
      ...resistanceKeys,
      ...resourceKeys,
      ...skillBonusKeys,
      ...worldLootKeys,
      ...offenseKeys,
    ])
    const out: string[] = []
    for (const k of knownStatKeys) {
      if (!claimed.has(k)) out.push(k)
    }
    return out
  }, [
    knownStatKeys,
    mitigationKeys,
    resistanceKeys,
    resourceKeys,
    skillBonusKeys,
    worldLootKeys,
    offenseKeys,
  ])

  const allClassSkills = useMemo(
    () => (classId ? skills.filter((s) => s.classId === classId) : []),
    [classId],
  )
  const skillsForClass = useMemo(
    () => allClassSkills.filter((s) => s.kind === 'active'),
    [allClassSkills],
  )

  const mainSkillId = useMemo(() => {
    const primary = activeSkillIds[0] ?? null
    if (primary && skillsForClass.some((s) => s.id === primary)) {
      return primary
    }
    let bestId: string | null = null
    let bestRank = 0
    for (const s of skillsForClass) {
      const r = skillRanks[s.id] ?? 0
      if (r > bestRank) {
        bestRank = r
        bestId = s.id
      }
    }
    return bestId
  }, [activeSkillIds, skillsForClass, skillRanks])
  const mainSkill = useMemo(
    () => skillsForClass.find((s) => s.id === mainSkillId) ?? null,
    [skillsForClass, mainSkillId],
  )

  // The engine is the only place that folds in the skill-scoped subtree, conversions
  // and per-element break. Per-skill cards below still use compute_skill_damage.
  const mainSkillDeps = useMemo(
    () => ({
      ...buildDeps,
      activeSkillIds: mainSkillId ? [mainSkillId] : [],
    }),
    [buildDeps, mainSkillId],
  )
  const mainPerformance = useCalcResult<BuildPerformance | null>(
    () => computeBuildPerformanceAsync(mainSkillDeps),
    [mainSkillDeps],
    null,
  )

  const skillRanksByName: Record<string, number> = useMemo(() => {
    const out: Record<string, number> = {}
    for (const s of allClassSkills) {
      out[normalizeSkillName(s.name)] = skillRanks[s.id] ?? 0
    }
    return out
  }, [allClassSkills, skillRanks])

  const skillsByNormalizedName: Record<string, Skill> = useMemo(() => {
    const out: Record<string, Skill> = {}
    for (const s of allClassSkills) {
      out[normalizeSkillName(s.name)] = s
    }
    return out
  }, [allClassSkills])

  const visibleSkills =
    normalizedQuery.length === 0
      ? skillsForClass
      : skillsForClass.filter((s) => matches(s.name))

  const showAttributes = filter === 'all' || filter === 'stats'
  const showMainSkill = filter === 'all' || filter === 'damage'
  const showSkills = filter === 'all' || filter === 'damage' || filter === 'skills'
  const sectionVisible = {
    offense: filter === 'all' || filter === 'damage',
    mitigation: filter === 'all' || filter === 'stats',
    resistances: filter === 'all' || filter === 'stats',
    resources: filter === 'all' || filter === 'stats',
    skillBonus: filter === 'all' || filter === 'skills',
    worldLoot: filter === 'all',
    other: filter === 'all',
  }
  const showAllStats = Object.values(sectionVisible).some(Boolean)
  const showEhp =
    (filter === 'all' || filter === 'stats') &&
    (normalizedQuery.length === 0 || ehpMatchesQuery(normalizedQuery))

  return (
    <div className="w-full space-y-3.5">
      <header className="flex items-end justify-between gap-3">
        <div>
          <h2
            className="m-0 font-semibold tracking-[0.04em] text-accent-hot"
            style={{
              fontSize: '22px',
              textShadow: '0 0 16px rgba(224,184,100,0.18)',
            }}
          >
            Stats
          </h2>
        </div>
        <div className="flex items-center gap-1.5">
          {classes.length === 0 ? (
            <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
              Add a class JSON to begin
            </span>
          ) : (
            FILTER_TABS.map((tab) => (
              <button
                key={tab.id}
                type="button"
                onClick={() => setFilter(tab.id)}
                className={`rounded-xs border px-2.5 py-1 font-mono text-[9px] font-semibold uppercase tracking-[0.16em] transition-colors ${
                  filter === tab.id
                    ? 'border-accent-deep bg-accent/6 text-accent-hot'
                    : 'border-border-2 text-faint hover:border-accent-deep hover:text-accent-hot'
                }`}
              >
                {tab.label}
              </button>
            ))
          )}
        </div>
      </header>

      <div className="relative">
        <svg
          aria-hidden
          className="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-faint"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth={2}
        >
          <circle cx="11" cy="11" r="7" />
          <path d="m20 20-3.5-3.5" />
        </svg>
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search stats, attributes, or skills…"
          data-search-input
          data-tour="stats-search"
          className="w-full rounded-[3px] border border-border-2 px-3 py-2 pl-9 pr-9 text-text placeholder:text-faint focus:border-accent-deep focus:outline-none focus:ring-2 focus:ring-accent-hot/15"
          style={{
            background:
              'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
            boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
          }}
        />
        {query && (
          <button
            onClick={() => setQuery('')}
            className="absolute right-2 top-1/2 -translate-y-1/2 rounded-xs px-1.5 py-0.5 font-mono text-[12px] text-faint transition-colors hover:text-accent-hot"
            aria-label="Clear search"
          >
            ×
          </button>
        )}
      </div>

      {showAttributes &&
        (() => {
          const visibleAttrs = gameConfig.attributes.filter(
            (attr) => normalizedQuery.length === 0 || matches(attr.name),
          )
          if (visibleAttrs.length === 0) return null
          return (
            <Panel
              title="Attributes"
            >
              <AttributesStrip
                attrs={visibleAttrs}
                attributes={attributes}
                attributeSources={attributeSources}
                matches={matches}
                getBreakdown={getBreakdown}
                requestBreakdown={requestBreakdown}
              />
            </Panel>
          )
        })()}

      {showMainSkill && (
        <MainSkillSection
          mainSkill={mainSkill}
          mainSkillRank={mainSkill ? (skillRanks[mainSkill.id] ?? 0) : 0}
          attributes={attributes}
          skillRanksByName={skillRanksByName}
          skillsByNormalizedName={skillsByNormalizedName}
          rankBonuses={rankBonuses}
          skillBreakdown={mainPerformance?.damage ?? null}
          attackDamage={mainPerformance?.attackDamage ?? null}
          cost={mainSkill ? computed?.skillCosts[mainSkill.id] : undefined}
          weaponDamage={weaponDamage}
        />
      )}

      {showSkills && (
        <>
          <SectionHeading>Per-Skill Damage</SectionHeading>
          <Panel padded>
            {skillsForClass.length === 0 ? (
              <div className="py-2 text-center text-sm text-muted italic">
                No skills defined for this class yet.
                <br />
                Add JSON files in{' '}
                <code className="text-accent">src/data/skills/</code>.
              </div>
            ) : visibleSkills.length === 0 ? (
              <div className="py-2 text-center text-sm text-muted italic">
                No skills match “{query}”.
              </div>
            ) : (
              <ul className="m-0 flex list-none flex-col gap-2 p-0">
                {visibleSkills.map((skill) => {
                  const skillStats = statsForSkill(skill.id)
                  return (
                    <SkillCard
                      key={skill.id}
                      skill={skill}
                      cost={computed?.skillCosts[skill.id]}
                      attributes={attributes}
                      stats={skillStats}
                      skillRanksByName={skillRanksByName}
                      skillsByNormalizedName={skillsByNormalizedName}
                      itemSkillBonuses={itemSkillBonuses}
                      rankBonuses={rankBonuses}
                      currentRank={skillRanks[skill.id] ?? 0}
                      enemyConditions={enemyConditions}
                      enemyResistances={enemyResistances}
                      skillProjectiles={skillProjectiles}
                      subtreeScoped={skillScoped[skill.id] ?? NO_SUBTREE}
                      isMain={activeSkillIds.includes(skill.id)}
                      weapon={weaponInput.weapon}
                    />
                  )
                })}
              </ul>
            )}
          </Panel>
        </>
      )}

      {showEhp && (
        <Panel title="Effective HP">
          <EhpBreakdown ehp={computed?.ehp} stats={stats} statsCombined={statsCombined} />
        </Panel>
      )}

      {showAllStats &&
        (() => {
          const filterKeys = (keys: string[]) =>
            normalizedQuery.length === 0
              ? keys
              : keys.filter((k) => matches(statName(k)))
          const renderList = (keys: string[]) => (
            <ul className="m-0 list-none p-0">
              {keys.map((key) => (
                <StatRow
                  key={key}
                  statKey={key}
                  value={displayStats[key] ?? 0}
                  sources={displayStatSources[key] ?? []}
                  moreValue={displayStats[`${key}_more`]}
                  moreSources={displayStatSources[`${key}_more`]}
                  rawValue={computed?.diminishedRaw?.[key]}
                  highlighted={matches(statName(key))}
                  stats={displayStats}
                  statsCombined={displayStatsCombined}
                  breakdown={getBreakdown(key, 'stat')}
                  onRequestBreakdown={() => requestBreakdown(key, 'stat')}
                />
              ))}
            </ul>
          )
          type SectionId = keyof typeof sectionVisible
          type SectionDef = { id: SectionId; label: string; keys: string[] }
          const rawSections: SectionDef[] = [
            { id: 'offense', label: 'Offense', keys: filterKeys(offenseKeys) },
            {
              id: 'mitigation',
              label: 'Mitigation',
              keys: filterKeys(mitigationKeys),
            },
            {
              id: 'resistances',
              label: 'Resistances',
              keys: filterKeys(resistanceKeys),
            },
            {
              id: 'resources',
              label: 'Resources',
              keys: filterKeys(resourceKeys),
            },
            {
              id: 'skillBonus',
              label: 'Skill Bonuses',
              keys: filterKeys(skillBonusKeys),
            },
            {
              id: 'worldLoot',
              label: 'World & Loot',
              keys: filterKeys(worldLootKeys),
            },
            { id: 'other', label: 'Other', keys: filterKeys(otherKeys) },
          ]
          const allSections = rawSections.filter(
            (s) => sectionVisible[s.id] && s.keys.length > 0,
          )
          if (allSections.length === 0) {
            return (
              <Panel title="All Stats">
                <div className="py-2 text-center text-xs text-muted italic">
                  No matches.
                </div>
              </Panel>
            )
          }
          return (
            <Panel title="All Stats">
              <div
                className="md:columns-2"
                style={{ columnGap: '2rem', columnFill: 'balance' }}
              >
                {allSections.map((s, i) => (
                  <div
                    key={s.id}
                    className="break-inside-avoid"
                    style={{
                      breakInside: 'avoid',
                      marginTop: i === 0 ? 0 : 12,
                    }}
                  >
                    <SubSectionLabel first={i === 0}>
                      {s.label}
                    </SubSectionLabel>
                    {renderList(s.keys)}
                  </div>
                ))}
              </div>
            </Panel>
          )
        })()}
    </div>
  )
}
