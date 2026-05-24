import { useCallback, useEffect, useMemo, useState } from 'react'
import SourceTooltip from '../components/SourceTooltip'
import { classes, gameConfig, skills } from '../data'
import { useBuildPerformanceDeps } from '../hooks/useBuildPerformanceDeps'
import {
  computeBuildStatsAsync,
  computeStatBreakdownAsync,
} from '../lib/calc/bridge'
import type { StatBreakdown, StatBreakdownKind } from '../lib/calc/bridge'
import { useBuild } from '../store/build'
import {
  aggregateItemSkillBonuses,
  combineAdditiveAndMore,
  computeSkillDamage,
  computeWeaponDamage,
  effectiveCap,
  effectiveRankRangeFor,
  formatValue,
  isZero,
  normalizeSkillName,
  rangedMax,
  rangedMin,
  statDef,
  statName,
} from '../utils/stats'
import type {
  ComputedStats,
  SkillDamageBreakdown,
  SourceContribution,
  WeaponDamageBreakdown,
} from '../utils/stats'
import type {
  AttributeKey,
  DamageType,
  RangedStatMap,
  RangedValue,
  Skill,
} from '../types'

type FilterTab = 'all' | 'damage' | 'stats' | 'skills'

const FILTER_TABS: Array<{ id: FilterTab; label: string }> = [
  { id: 'all', label: 'All' },
  { id: 'damage', label: 'Damage' },
  { id: 'stats', label: 'All stats' },
  { id: 'skills', label: 'Skills' },
]

const DAMAGE_TYPE_TEXT: Record<DamageType, string> = {
  physical: 'text-text',
  lightning: 'text-yellow-300',
  cold: 'text-sky-300',
  fire: 'text-red-400',
  poison: 'text-green-400',
  arcane: 'text-purple-300',
  explosion: 'text-orange-300',
  magic: 'text-pink-300',
}

const DAMAGE_TYPE_PILL: Record<DamageType, string> = {
  physical: 'text-text/90 border-white/25 bg-white/[0.04]',
  lightning: 'text-yellow-300 border-yellow-500/40 bg-yellow-500/[0.08]',
  cold: 'text-sky-300 border-sky-500/40 bg-sky-500/[0.08]',
  fire: 'text-red-400 border-red-500/40 bg-red-500/[0.08]',
  poison: 'text-green-400 border-green-500/40 bg-green-500/[0.08]',
  arcane: 'text-purple-300 border-purple-500/40 bg-purple-500/[0.08]',
  explosion: 'text-orange-300 border-orange-500/40 bg-orange-500/[0.08]',
  magic: 'text-pink-300 border-pink-500/40 bg-pink-500/[0.08]',
}

const RESISTANCE_KEYS = [
  'fire_resistance',
  'cold_resistance',
  'lightning_resistance',
  'poison_resistance',
  'arcane_resistance',
  'all_resistances',
  'max_fire_resistance',
  'max_cold_resistance',
  'max_lightning_resistance',
  'max_poison_resistance',
  'max_arcane_resistance',
  'max_all_resistances',
  'fire_absorption',
  'cold_absorption',
  'lightning_absorption',
  'poison_absorption',
  'arcane_absorption',
]

// Project's gameConfig keeps life/mana under category="base", so we hard-list the keys that conceptually belong in the Resources panel. Also covers leech / regen / interplay (mana<->life) stats so they don't end up duplicated under defense or hidden.
const RESOURCE_KEYS = [
  'life',
  'mana',
  'increased_life',
  'increased_mana',
  'life_replenish',
  'life_replenish_pct',
  'mana_replenish',
  'mana_replenish_pct',
  'life_steal',
  'life_steal_rate',
  'life_steal_instant',
  'mana_steal',
  'mana_steal_rate',
  'life_per_kill',
  'mana_per_kill',
  'mana_cost_reduction',
  'mana_cost_paid_in_life',
  'damage_recouped_as_life',
  'damage_recouped_as_mana',
  'overflow_mana_recouped_as_life',
  'damage_drained_from_mana',
  'max_life_to_mana',
  'max_mana_to_life',
  'overflow_res_to_life',
  'life_replenish_flask',
  'damage_mitigated_flask',
]

// Defensive mitigation — explicit list (resistances handled separately).
const MITIGATION_KEYS = [
  'defense',
  'enhanced_defense',
  'defense_vs_missiles',
  'damage_reduced',
  'all_damage_taken_reduced_pct',
  'physical_damage_reduction',
  'magic_damage_reduction',
  'magic_absorption',
  'damage_taken_reduced',
  'damage_mitigation',
  'damage_return',
  'block_chance',
  'dodge_chance',
  'dodge_spell_hits',
  'suppress_spell_hits',
  'faster_hit_recovery',
  'immune_duration',
  'cc_immune_no_dodge',
  'poison_length_reduced',
  'max_colossus_stacks',
  'max_combat_mitigation_stacks',
]

// Skill-bonus keys (live in category="base" but conceptually belong with skills, not life/mana).
const SKILL_BONUS_KEYS = [
  'all_skills',
  'physical_skills',
  'arcane_skills',
  'cold_skills',
  'fire_skills',
  'poison_skills',
  'lightning_skills',
  'explosion_skills',
  'summon_skills',
]

// World / loot / movement (also from category="base") — magic find, gold find, movement speed, etc.
const WORLD_LOOT_KEYS = [
  'movement_speed',
  'jumping_power',
  'light_radius',
  'experience_gain',
  'magic_find',
  'gold_find',
  'merchant_prices',
  'increased_all_attributes',
]

export default function StatsView() {
  // Stats tab matching the "Stats View Mockup": page head with filter chips, hero build summary, search box, attributes strip, weapon damage hero with breakdown, per-skill cards (main skill highlighted in gold), and a 2-column Defensive / Resources grid. All stat math reuses computeBuildStats / computeWeaponDamage / computeSkillDamage from utils.
  const classId = useBuild((s) => s.classId)
  const inventory = useBuild((s) => s.inventory)
  const skillRanks = useBuild((s) => s.skillRanks)
  const enemyConditions = useBuild((s) => s.enemyConditions)
  const skillProjectiles = useBuild((s) => s.skillProjectiles)
  const enemyResistances = useBuild((s) => s.enemyResistances)
  const storeMainSkillId = useBuild((s) => s.mainSkillId)
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState<FilterTab>('all')
  const normalizedQuery = query.trim().toLowerCase()
  const matches = (label: string) =>
    normalizedQuery.length > 0 &&
    label.toLowerCase().includes(normalizedQuery)
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

  // Per-stat breakdown cache. Keyed `${kind}:${statKey}` so attributes and
  // stats sharing a key (e.g. `strength`) don't collide. The state object
  // carries the buildDeps it was filled for, so when buildDeps swap we just
  // read an empty cache without a `setState`-in-effect — a fresh fetch will
  // commit a new bucket. `inFlight` lives in state too (not in a ref) so the
  // request callback doesn't read a ref during render.
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
  // Memoise the fallback so downstream useMemo deps don't see a fresh {} each render.
  const attributes = useMemo(() => computed?.attributes ?? {}, [computed])
  const stats = useMemo(() => computed?.stats ?? {}, [computed])
  const attributeSources = useMemo(
    () => computed?.attributeSources ?? {},
    [computed],
  )
  const statSources = useMemo(() => computed?.statSources ?? {}, [computed])
  const fcrRange = stats.faster_cast_rate ?? 0
  const mcrRange = stats.mana_cost_reduction ?? 0
  const itemSkillBonuses = useMemo(
    () => aggregateItemSkillBonuses(inventory),
    [inventory],
  )
  const weaponDamage = useMemo(
    () => computeWeaponDamage(inventory, stats, enemyConditions),
    [inventory, stats, enemyConditions],
  )

  // Map each gameConfig category into a list of base stat keys (filters out skill-scoped, item-only and pure-attribute modifiers).
  const grouped = useMemo(() => {
    const out: Record<string, string[]> = {
      base: [],
      offense: [],
      defense: [],
      resource: [],
      utility: [],
    }
    for (const def of gameConfig.stats) {
      if (def.modifiesAttribute) continue
      if (def.itemOnly) continue
      if (def.skillScoped) continue
      const bucket = out[def.category]
      if (bucket) bucket.push(def.key)
    }
    return out
  }, [])

  // Filter the canonical lists to whatever gameConfig actually exposes — this lets us keep stable ordering while staying robust to config changes.
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
  // Offense = whole offense category; subtract anything we already classified into Resources (e.g. life_steal lives under offense in gameConfig but reads better under Resources).
  const offenseKeys = useMemo(() => {
    const claimed = new Set<string>([...resourceKeys])
    return (grouped.offense ?? []).filter((k) => !claimed.has(k))
  }, [grouped, resourceKeys])
  // Catch-all: anything in known keys we haven't put into a section above. Keeps the panel exhaustive even if a key gets added without classifying it explicitly.
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

  // Effective "main skill": prefer the user's explicit pick from the build store; fall back to the highest-rank active skill so the headline panel still has something to show on a fresh build.
  const mainSkillId = useMemo(() => {
    if (
      storeMainSkillId &&
      skillsForClass.some((s) => s.id === storeMainSkillId)
    ) {
      return storeMainSkillId
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
  }, [storeMainSkillId, skillsForClass, skillRanks])
  const mainSkill = useMemo(
    () => skillsForClass.find((s) => s.id === mainSkillId) ?? null,
    [skillsForClass, mainSkillId],
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

  // Tab visibility: each tab whitelists which top-level panels and which sections inside the All-Stats panel show. Search applies *within* whatever panels are visible.
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

  return (
    <div className="w-full space-y-3.5">
      {/* PAGE HEAD: eyebrow + title + filter chips */}
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

      {/* SEARCH */}
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
          className="w-full rounded-[3px] border border-border-2 px-3 py-2 pl-9 pr-9 text-text placeholder:italic placeholder:text-faint focus:border-accent-deep focus:outline-none"
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

      {/* ATTRIBUTES STRIP */}
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

      {/* MAIN SKILL HERO — headline damage breakdown for the build store's `mainSkillId` (with a highest-rank fallback). Replaces the old Weapon Damage panel. */}
      {showMainSkill && (
        <MainSkillSection
          mainSkill={mainSkill}
          mainSkillRank={mainSkill ? (skillRanks[mainSkill.id] ?? 0) : 0}
          attributes={attributes}
          stats={stats}
          skillRanksByName={skillRanksByName}
          skillsByNormalizedName={skillsByNormalizedName}
          itemSkillBonuses={itemSkillBonuses}
          enemyConditions={enemyConditions}
          enemyResistances={enemyResistances}
          skillProjectiles={skillProjectiles}
          fcrRange={fcrRange}
          mcrRange={mcrRange}
          weaponDamage={weaponDamage}
        />
      )}

      {/* PER-SKILL DAMAGE */}
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
                {visibleSkills.map((skill) => (
                  <SkillCard
                    key={skill.id}
                    skill={skill}
                    fcrRange={fcrRange}
                    mcrRange={mcrRange}
                    attributes={attributes}
                    stats={stats}
                    skillRanksByName={skillRanksByName}
                    skillsByNormalizedName={skillsByNormalizedName}
                    itemSkillBonuses={itemSkillBonuses}
                    currentRank={skillRanks[skill.id] ?? 0}
                    enemyConditions={enemyConditions}
                    enemyResistances={enemyResistances}
                    skillProjectiles={skillProjectiles}
                    isMain={skill.id === mainSkillId}
                  />
                ))}
              </ul>
            )}
          </Panel>
        </>
      )}

      {/* ALL STATS — single big panel at the bottom of the page that lays out every base stat the build can have, grouped into 6 sub-sections so nothing slips through. The active filter chip narrows which sections render. */}
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
                  value={stats[key] ?? 0}
                  sources={statSources[key] ?? []}
                  moreValue={stats[`${key}_more`]}
                  moreSources={statSources[`${key}_more`]}
                  highlighted={matches(statName(key))}
                  stats={stats}
                  breakdown={getBreakdown(key, 'stat')}
                  onRequestBreakdown={() => requestBreakdown(key, 'stat')}
                />
              ))}
            </ul>
          )
          // Build the section list lazily so empty-after-filter sections collapse cleanly.
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
          // Two-column auto-balanced layout: long sections (Offense, Mitigation) tend to land on the left; shorter ones on the right. CSS columns gives near-balanced heights without manual splitting.
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

/* ===== SUB-COMPONENTS ===== */

function MainSkillSection({
  mainSkill,
  mainSkillRank,
  attributes,
  stats,
  skillRanksByName,
  skillsByNormalizedName,
  itemSkillBonuses,
  enemyConditions,
  enemyResistances,
  skillProjectiles,
  fcrRange,
  mcrRange,
  weaponDamage,
}: {
  mainSkill: Skill | null
  mainSkillRank: number
  attributes: Record<AttributeKey, RangedValue>
  stats: RangedStatMap
  skillRanksByName: Record<string, number>
  skillsByNormalizedName: Record<string, Skill>
  itemSkillBonuses: Record<string, [number, number]>
  enemyConditions: Record<string, boolean>
  enemyResistances: Record<string, number>
  skillProjectiles: Record<string, number>
  fcrRange: RangedValue
  mcrRange: RangedValue
  weaponDamage: WeaponDamageBreakdown
}) {
  // Headline damage panel — always visible at the top of the Stats tab. Shows the *currently selected* main skill (from the build store, with a highest-rank fallback) so the player doesn't have to scroll down to check the headline number. If the main skill has no damage formula or hasn't been ranked, falls back to the weapon damage hero (or an "equip a weapon" message).
  const hasSkillDamage =
    !!mainSkill &&
    mainSkillRank > 0 &&
    (!!mainSkill.damageFormula ||
      (!!mainSkill.damagePerRank && mainSkill.damagePerRank.length > 0))
  const skillBreakdown = hasSkillDamage
    ? computeSkillDamage(
        mainSkill!,
        mainSkillRank,
        attributes,
        stats,
        skillRanksByName,
        itemSkillBonuses,
        enemyConditions,
        enemyResistances,
        skillsByNormalizedName,
        skillProjectiles[mainSkill!.id],
      )
    : null

  if (mainSkill && skillBreakdown) {
    return (
      <Panel
        title="Main Skill"
        meta={`${mainSkill.name}`}
      >
        <SkillDamageHero
          skill={mainSkill}
          breakdown={skillBreakdown}
          fcrRange={fcrRange}
          mcrRange={mcrRange}
        />
        <DamageBreakdown
          skill={mainSkill}
          breakdown={skillBreakdown}
          currentRank={mainSkillRank}
          attributes={attributes}
          skillRanksByName={skillRanksByName}
          skillsByNormalizedName={skillsByNormalizedName}
          stats={stats}
          itemSkillBonuses={itemSkillBonuses}
        />
      </Panel>
    )
  }

  // Fallbacks — main skill not picked / not ranked / has no damage. Still useful to show *something* so the panel slot doesn't go empty.
  if (weaponDamage.hasWeapon) {
    return (
      <Panel
        title="Main Skill"
        meta={
          mainSkill
            ? `${mainSkill.name} · no damage`
            : 'No main skill selected · weapon damage shown'
        }
      >
        <DamageHero breakdown={weaponDamage} />
        <DamageBuildup breakdown={weaponDamage} />
      </Panel>
    )
  }
  return (
    <Panel title="Main Skill" meta="No main skill or weapon">
      <div className="py-6 text-center text-xs text-muted italic">
        Pick a main skill in the Skills tab, or equip a weapon, to see your
        headline damage breakdown here.
      </div>
    </Panel>
  )
}

function SkillDamageHero({
  skill,
  breakdown,
  fcrRange,
  mcrRange,
}: {
  skill: Skill
  breakdown: SkillDamageBreakdown
  fcrRange: RangedValue
  mcrRange: RangedValue
}) {
  // Hero tile for the chosen main skill. Same 1.2fr / 1fr layout as the weapon DamageHero — big "Average hit" on the left + DPS-ish metadata, 2x2 cells on the right (Hit / Crit damage / Crit chance / Crit multi). Adds the skill's tag pills + cast/mana meta below the hero so the user doesn't need a separate row.
  const hasCrit = breakdown.critChance > 0
  const baseMana = skill.ranks[0]?.manaCost
  const mcrMin = rangedMin(mcrRange)
  const mcrMax = rangedMax(mcrRange)
  const effectiveManaMin =
    baseMana !== undefined
      ? Math.max(0, baseMana * (1 - mcrMax / 100))
      : undefined
  const effectiveManaMax =
    baseMana !== undefined
      ? Math.max(0, baseMana * (1 - mcrMin / 100))
      : undefined
  const fcrMin = rangedMin(fcrRange)
  const fcrMax = rangedMax(fcrRange)
  const effectiveCastRateMin =
    skill.baseCastRate !== undefined
      ? skill.baseCastRate * (1 + fcrMin / 100)
      : undefined
  const effectiveCastRateMax =
    skill.baseCastRate !== undefined
      ? skill.baseCastRate * (1 + fcrMax / 100)
      : undefined
  const accentBg = skill.damageType
    ? skillHeroBg(skill.damageType)
    : 'linear-gradient(135deg, rgba(224,184,100,0.06), transparent 60%)'
  return (
    <div
      className="mb-3 grid grid-cols-1 overflow-hidden rounded-[3px] border border-border md:grid-cols-[1.2fr_1fr]"
      style={{ background: accentBg }}
    >
      <div className="border-b border-border px-4 py-3.5 md:border-b-0 md:border-r">
        <div className="mb-1.5 flex items-center gap-1.5 font-mono text-[9px] uppercase tracking-[0.18em] text-faint">
          <span>{hasCrit ? 'Average Hit' : 'Hit Damage'}</span>
          {skill.damageType && (
            <span
              className={`rounded-xs border px-1.5 py-px text-[9px] font-semibold ${DAMAGE_TYPE_PILL[skill.damageType]}`}
            >
              {skill.damageType}
            </span>
          )}
        </div>
        <div
          className="font-mono text-[28px] font-semibold leading-none tabular-nums tracking-[0.01em] text-accent-hot"
          style={{ textShadow: '0 0 16px rgba(224,184,100,0.18)' }}
        >
          {hasCrit
            ? formatRangeInt(breakdown.avgMin, breakdown.avgMax)
            : formatRangeInt(breakdown.hitMin, breakdown.hitMax)}
        </div>
        <div className="mt-2 flex flex-wrap items-baseline gap-x-4 gap-y-1 font-mono text-[10px] tracking-widest text-muted">
          {effectiveManaMin !== undefined &&
            effectiveManaMax !== undefined && (
              <span>
                <span className="text-text/90">
                  {formatRange(effectiveManaMin, effectiveManaMax)}
                </span>{' '}
                mana
              </span>
            )}
          {effectiveCastRateMin !== undefined &&
            effectiveCastRateMax !== undefined && (
              <span>
                <span className="text-text/90">
                  {formatRange(effectiveCastRateMin, effectiveCastRateMax)}
                </span>{' '}
                casts/s
              </span>
            )}
          {skill.tags && skill.tags.length > 0 && (
            <div className="flex flex-wrap gap-1">
              {skill.tags.map((tag) => (
                <span
                  key={tag}
                  className="rounded-xs border border-accent-deep/50 px-1.5 py-px text-[9px] font-semibold uppercase tracking-[0.14em] text-accent-hot"
                  style={{
                    background:
                      'linear-gradient(180deg, rgba(58,46,24,0.4), rgba(42,36,24,0.2))',
                  }}
                >
                  {tag}
                </span>
              ))}
            </div>
          )}
        </div>
      </div>
      <div className="grid grid-cols-2 gap-2.5 px-4 py-3.5">
        <HeroStat
          k="Hit damage"
          v={formatRangeInt(breakdown.hitMin, breakdown.hitMax)}
        />
        <HeroStat
          k="Crit damage"
          v={
            hasCrit ? formatRangeInt(breakdown.critMin, breakdown.critMax) : '—'
          }
        />
        <HeroStat
          k="Crit chance"
          v={hasCrit ? `${formatDecimal(breakdown.critChance)}%` : '—'}
        />
        <HeroStat
          k="Crit multi"
          v={
            hasCrit
              ? `+${formatDecimal(breakdown.critDamagePct)}%`
              : '—'
          }
        />
      </div>
    </div>
  )
}

function skillHeroBg(type: DamageType): string {
  // Picks a tinted hero gradient that matches the damage type — keeps the panel feeling alive rather than gold-on-gold for every skill.
  const map: Record<DamageType, string> = {
    physical:
      'linear-gradient(135deg, rgba(212,207,191,0.06), transparent 60%)',
    fire: 'linear-gradient(135deg, rgba(232,144,122,0.07), transparent 60%)',
    cold: 'linear-gradient(135deg, rgba(122,166,216,0.07), transparent 60%)',
    lightning:
      'linear-gradient(135deg, rgba(232,217,107,0.06), transparent 60%)',
    poison:
      'linear-gradient(135deg, rgba(136,212,154,0.07), transparent 60%)',
    arcane:
      'linear-gradient(135deg, rgba(201,122,204,0.07), transparent 60%)',
    explosion:
      'linear-gradient(135deg, rgba(217,154,90,0.07), transparent 60%)',
    magic:
      'linear-gradient(135deg, rgba(220,140,200,0.07), transparent 60%)',
  }
  return map[type]
}

function AttributesStrip({
  attrs,
  attributes,
  attributeSources,
  matches,
  getBreakdown,
  requestBreakdown,
}: {
  attrs: Array<{ key: string; name: string }>
  attributes: Record<AttributeKey, RangedValue>
  attributeSources: Record<string, SourceContribution[]>
  matches: (label: string) => boolean
  getBreakdown: (statKey: string, kind: StatBreakdownKind) => StatBreakdown | null
  requestBreakdown: (statKey: string, kind: StatBreakdownKind) => void
}) {
  // Compact 6-column attribute strip (collapses to 3 / 2 on smaller widths). Uses 1px gap on the parent so the inner cells share borders like a table. Each cell opens the Rust-fetched breakdown on left- or right-click.
  return (
    <div
      className="grid grid-cols-2 overflow-hidden rounded-[3px] border border-border sm:grid-cols-3 lg:grid-cols-6"
      style={{ gap: 1, background: 'var(--color-border)' }}
    >
      {attrs.map((attr) => {
        const final: RangedValue = attributes[attr.key as AttributeKey] ?? 0
        const sources = attributeSources[attr.key] ?? []
        const [fmin, fmax] =
          typeof final === 'number' ? [final, final] : final
        const highlighted = matches(attr.name)
        return (
          <SourceTooltip
            key={attr.key}
            statKey={attr.key}
            sources={sources}
            breakdown={getBreakdown(attr.key, 'attribute')}
            onRequestBreakdown={() => requestBreakdown(attr.key, 'attribute')}
            title={attr.name}
          >
            <div
              className={`flex flex-col gap-1 px-3 py-2.5 transition-colors ${
                highlighted ? 'bg-accent-hot/10' : 'hover:bg-panel-3/40'
              }`}
              style={{
                background: highlighted
                  ? undefined
                  : 'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
              }}
            >
              <span className="font-mono text-[9px] uppercase tracking-[0.16em] text-faint">
                {attr.name}
              </span>
              <span
                className={`font-mono text-[15px] font-semibold tabular-nums ${
                  highlighted ? 'text-accent-hot' : 'text-text'
                }`}
              >
                {displayRange(fmin, fmax)}
              </span>
            </div>
          </SourceTooltip>
        )
      })}
    </div>
  )
}

function DamageHero({ breakdown }: { breakdown: WeaponDamageBreakdown }) {
  // The "big number" weapon-damage hero panel: average hit on the left (large gold), DPS + APS underneath, then a 2x2 grid on the right with hit damage / crit damage / crit chance / crit multi.
  const b = breakdown
  return (
    <div
      className="mb-3 grid grid-cols-1 overflow-hidden rounded-[3px] border border-border md:grid-cols-[1.2fr_1fr]"
      style={{
        background:
          'linear-gradient(135deg, rgba(217,107,90,0.05), transparent 60%)',
      }}
    >
      <div className="border-b border-border px-4 py-3.5 md:border-b-0 md:border-r">
        <div className="mb-1.5 font-mono text-[9px] uppercase tracking-[0.18em] text-faint">
          Average Hit
        </div>
        <div
          className="font-mono text-[26px] font-semibold leading-none tabular-nums tracking-[0.01em] text-accent-hot"
          style={{ textShadow: '0 0 16px rgba(224,184,100,0.18)' }}
        >
          {formatRangeInt(Math.round(b.avgMin), Math.round(b.avgMax))}
        </div>
        <div className="mt-1.5 font-mono text-[10px] tracking-widest text-muted">
          DPS{' '}
          <span className="text-accent-hot">
            {formatRangeInt(Math.round(b.dpsMin), Math.round(b.dpsMax))}
          </span>{' '}
          ·{' '}
          <span className="text-text/90">
            {formatRange(b.attacksPerSecondMin, b.attacksPerSecondMax)}
          </span>{' '}
          attacks/sec
        </div>
      </div>
      <div className="grid grid-cols-2 gap-2.5 px-4 py-3.5">
        <HeroStat
          k="Hit damage"
          v={formatRangeInt(b.hitMin, b.hitMax)}
        />
        <HeroStat
          k="Crit damage"
          v={
            b.critChance > 0
              ? formatRangeInt(b.critMin, b.critMax)
              : '—'
          }
        />
        <HeroStat
          k="Crit chance"
          v={`${formatDecimal(b.critChance)}%`}
        />
        <HeroStat
          k="Crit multi"
          v={`×${b.critMultiplierAvg.toFixed(2)}`}
        />
      </div>
    </div>
  )
}

function HeroStat({ k, v }: { k: string; v: string }) {
  // One key/value cell of the DamageHero side grid.
  return (
    <div className="flex flex-col gap-0.5">
      <span className="font-mono text-[9px] uppercase tracking-[0.14em] text-faint">
        {k}
      </span>
      <span className="font-mono text-[13px] font-semibold tabular-nums text-text">
        {v}
      </span>
    </div>
  )
}

function DamageBuildup({ breakdown }: { breakdown: WeaponDamageBreakdown }) {
  // Build-up section under DamageHero: weapon damage → enhanced damage → additive physical → attack damage % → extra damage → Hit damage total.
  const b = breakdown
  const edRange =
    b.enhancedDamageMinPct === b.enhancedDamageMaxPct
      ? `${formatDecimal(b.enhancedDamageMinPct)}`
      : `${formatDecimal(b.enhancedDamageMinPct)}-${formatDecimal(b.enhancedDamageMaxPct)}`
  const atkDmgRange =
    b.attackDamageMinPct === b.attackDamageMaxPct
      ? `${formatDecimal(b.attackDamageMinPct)}`
      : `${formatDecimal(b.attackDamageMinPct)}-${formatDecimal(b.attackDamageMaxPct)}`
  return (
    <div className="mt-2 border-t border-dashed border-border pt-2.5">
      <BDSection title="Build-up">
        <BDLine
          label="Weapon damage"
          value={
            <span className="text-text">
              {formatRangeInt(b.weaponDamageMin, b.weaponDamageMax)}
            </span>
          }
        />
        {b.enhancedDamageMaxPct > 0 && (
          <BDLine
            label="Enhanced damage"
            value={<span className="text-accent-hot">+{edRange}%</span>}
          />
        )}
        {b.additivePhysicalMax > 0 && (
          <BDLine
            label="Additive physical"
            value={
              <span className="text-text">
                +{formatRangeInt(b.additivePhysicalMin, b.additivePhysicalMax)}
              </span>
            }
          />
        )}
        {b.attackDamageMaxPct > 0 && (
          <BDLine
            label="Attack damage %"
            value={<span className="text-accent-hot">+{atkDmgRange}%</span>}
          />
        )}
      </BDSection>
      {b.extraDamageSources.length > 0 && (
        <BDSection title="Extra damage">
          {b.extraDamageSources.map((s, i) => (
            <BDLine
              key={i}
              indent
              label={s.label}
              value={
                <span className="text-orange-300">
                  +{formatDecimal(s.pct)}%
                </span>
              }
            />
          ))}
        </BDSection>
      )}
      <div className="mt-1.5 flex items-baseline justify-between gap-3 border-t border-border pt-1.5">
        <span className="font-mono text-[10px] font-medium uppercase tracking-[0.14em] text-muted">
          Hit damage
        </span>
        <span className="font-mono text-[14px] font-semibold tabular-nums text-accent-hot">
          {formatRangeInt(b.hitMin, b.hitMax)}
        </span>
      </div>
    </div>
  )
}

function SkillCard({
  skill,
  fcrRange,
  mcrRange,
  attributes,
  stats,
  skillRanksByName,
  skillsByNormalizedName,
  itemSkillBonuses,
  currentRank,
  enemyConditions,
  enemyResistances,
  skillProjectiles,
  isMain,
}: {
  skill: Skill
  fcrRange: RangedValue
  mcrRange: RangedValue
  attributes: Record<AttributeKey, RangedValue>
  stats: RangedStatMap
  skillRanksByName: Record<string, number>
  skillsByNormalizedName: Record<string, Skill>
  itemSkillBonuses: Record<string, [number, number]>
  currentRank: number
  enemyConditions: Record<string, boolean>
  enemyResistances: Record<string, number>
  skillProjectiles: Record<string, number>
  isMain: boolean
}) {
  // Per-skill row: name + rank, computed damage range with damage-type colour, a tag strip (damage type + skill tags), mana / cast-rate meta, and the full DamageBreakdown when the skill has damage. The "main" skill (highest current rank) gets the gold border + left-bar treatment from the mockup.
  const rank1 = skill.ranks[0]
  const baseMana = rank1?.manaCost
  const mcrMin = rangedMin(mcrRange)
  const mcrMax = rangedMax(mcrRange)
  const effectiveManaMin =
    baseMana !== undefined
      ? Math.max(0, baseMana * (1 - mcrMax / 100))
      : undefined
  const effectiveManaMax =
    baseMana !== undefined
      ? Math.max(0, baseMana * (1 - mcrMin / 100))
      : undefined
  const fcrMin = rangedMin(fcrRange)
  const fcrMax = rangedMax(fcrRange)
  const effectiveCastRateMin =
    skill.baseCastRate !== undefined
      ? skill.baseCastRate * (1 + fcrMin / 100)
      : undefined
  const effectiveCastRateMax =
    skill.baseCastRate !== undefined
      ? skill.baseCastRate * (1 + fcrMax / 100)
      : undefined

  const hasDamage =
    !!skill.damageFormula ||
    (!!skill.damagePerRank && skill.damagePerRank.length > 0)
  const damageBreakdown =
    currentRank > 0 && hasDamage
      ? computeSkillDamage(
          skill,
          currentRank,
          attributes,
          stats,
          skillRanksByName,
          itemSkillBonuses,
          enemyConditions,
          enemyResistances,
          skillsByNormalizedName,
          skillProjectiles[skill.id],
        )
      : null
  const typeLabel = skill.damageType
    ? skill.damageType.charAt(0).toUpperCase() + skill.damageType.slice(1)
    : ''
  const dmgAccent = skill.damageType
    ? DAMAGE_TYPE_TEXT[skill.damageType]
    : 'text-text'
  const learned = currentRank > 0
  const containerStyle: React.CSSProperties = isMain
    ? {
        background:
          'linear-gradient(135deg, rgba(224,184,100,0.06), transparent 60%), linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
        boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.4)',
      }
    : {
        background:
          'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
        boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.4)',
      }

  return (
    <li
      className={`relative space-y-1.5 rounded-[3px] border px-3.5 py-2.5 ${
        isMain ? 'border-accent-deep' : 'border-border'
      } ${!learned && !hasDamage ? '' : !learned ? 'opacity-55' : ''}`}
      style={containerStyle}
    >
      {isMain && (
        <span
          aria-hidden
          className="pointer-events-none absolute bottom-2 left-0 top-2 w-0.5 bg-accent-hot"
          style={{ boxShadow: '0 0 10px rgba(224,184,100,0.5)' }}
        />
      )}
      <div className="flex items-baseline justify-between gap-2">
        <div className="font-medium text-text">
          {skill.name}{' '}
          <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
            rank{' '}
            <span className={learned ? 'text-accent-hot' : 'text-faint'}>
              {currentRank}
            </span>
            /{skill.maxRank}
          </span>
        </div>
        {hasDamage && damageBreakdown ? (
          <div
            className={`font-mono text-[13px] font-semibold tabular-nums ${dmgAccent}`}
          >
            {formatRangeInt(damageBreakdown.finalMin, damageBreakdown.finalMax)}{' '}
            <span className="ml-0.5 font-mono text-[9px] font-normal uppercase tracking-[0.14em] text-faint">
              {typeLabel} damage
            </span>
          </div>
        ) : hasDamage ? (
          <div className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint italic">
            Not learned
          </div>
        ) : null}
      </div>
      <div className="flex flex-wrap items-center gap-1.5 font-mono text-[9px] uppercase tracking-[0.14em]">
        {skill.damageType && (
          <span
            className={`rounded-xs border px-1.5 py-0.5 font-semibold ${DAMAGE_TYPE_PILL[skill.damageType]}`}
          >
            {skill.damageType}
          </span>
        )}
        {skill.tags?.map((tag) => (
          <span
            key={tag}
            className="rounded-xs border border-accent-deep/50 px-1.5 py-0.5 font-semibold text-accent-hot"
            style={{
              background:
                'linear-gradient(180deg, rgba(58,46,24,0.4), rgba(42,36,24,0.2))',
            }}
          >
            {tag}
          </span>
        ))}
      </div>
      {(effectiveManaMin !== undefined ||
        effectiveCastRateMin !== undefined ||
        skill.movementDuringUse !== undefined) && (
        <div className="flex flex-wrap gap-x-3.5 gap-y-0.5 text-[11px] text-muted">
          {effectiveManaMin !== undefined && effectiveManaMax !== undefined && (
            <span>
              <span className="font-mono font-medium text-text">
                {formatRange(effectiveManaMin, effectiveManaMax)}
              </span>{' '}
              mana
              {baseMana !== undefined && mcrMax > 0 && (
                <span className="text-faint">
                  {' '}
                  (base {baseMana})
                </span>
              )}
            </span>
          )}
          {effectiveCastRateMin !== undefined &&
            effectiveCastRateMax !== undefined && (
              <span>
                <span className="font-mono font-medium text-text">
                  {formatRange(effectiveCastRateMin, effectiveCastRateMax)}
                </span>{' '}
                casts/s
                {fcrMax > 0 && (
                  <span className="text-faint">
                    {' '}
                    (base {formatDecimal(skill.baseCastRate!)})
                  </span>
                )}
              </span>
            )}
          {skill.movementDuringUse !== undefined && (
            <span>
              Move{' '}
              <span className="font-mono font-medium text-text">
                {skill.movementDuringUse}%
              </span>
            </span>
          )}
          <span>
            max rank{' '}
            <span className="font-mono font-medium text-text">
              {skill.maxRank}
            </span>
          </span>
        </div>
      )}
      {hasDamage && damageBreakdown && (
        <DamageBreakdown
          skill={skill}
          breakdown={damageBreakdown}
          currentRank={currentRank}
          attributes={attributes}
          skillRanksByName={skillRanksByName}
          skillsByNormalizedName={skillsByNormalizedName}
          stats={stats}
          itemSkillBonuses={itemSkillBonuses}
        />
      )}
    </li>
  )
}

function StatRow({
  statKey,
  value,
  sources,
  moreValue,
  moreSources,
  highlighted,
  stats,
  breakdown,
  onRequestBreakdown,
}: {
  statKey: string
  value: RangedValue
  sources: SourceContribution[]
  moreValue?: RangedValue
  moreSources?: SourceContribution[]
  highlighted: boolean
  stats: RangedStatMap
  breakdown: StatBreakdown | null
  onRequestBreakdown: () => void
}) {
  // Single label/value row used in Defensive and Resources panels (and in Offense / Utility extras). Hover surfaces the SourceTooltip with breakdown; left- or right-click pins it.
  const hasMore = !!moreSources && moreSources.length > 0
  const displayValue: RangedValue = hasMore
    ? combineAdditiveAndMore(value, moreValue)
    : value
  const zero = isZero(displayValue) && (!hasMore || isZero(moreValue ?? 0))
  const def = statDef(statKey)
  const cap = effectiveCap(statKey, stats)
  const overflow =
    cap !== undefined &&
    !zero &&
    typeof displayValue === 'number' &&
    displayValue > cap
  const suffix = def?.format === 'percent' ? '%' : ''
  return (
    <li>
      <SourceTooltip
        statKey={statKey}
        sources={sources}
        moreSources={moreSources}
        breakdown={breakdown}
        onRequestBreakdown={onRequestBreakdown}
      >
        <div
          className={`-mx-2 flex items-baseline justify-between gap-2 rounded-xs px-2 py-1 text-[13px] transition-colors ${
            highlighted
              ? 'bg-accent-hot/10 ring-1 ring-accent-deep/50'
              : 'hover:bg-accent-hot/4'
          } ${zero && !highlighted ? 'opacity-35' : ''}`}
        >
          <span className="text-text">{statName(statKey)}</span>
          <span
            className={`font-mono font-medium tabular-nums ${
              zero ? 'text-faint' : 'text-accent-hot'
            }`}
          >
            {zero ? (
              '—'
            ) : overflow ? (
              <>
                +{cap}
                {suffix}{' '}
                <span className="text-faint font-normal">
                  ({displayValue as number}
                  {suffix})
                </span>
              </>
            ) : (
              formatValue(displayValue, statKey)
            )}
          </span>
        </div>
      </SourceTooltip>
    </li>
  )
}

function DamageBreakdown({
  skill,
  breakdown,
  currentRank,
  attributes,
  skillRanksByName,
  skillsByNormalizedName,
  stats,
  itemSkillBonuses,
}: {
  skill: Skill
  breakdown: SkillDamageBreakdown
  currentRank: number
  attributes: Record<AttributeKey, RangedValue>
  skillRanksByName: Record<string, number>
  skillsByNormalizedName: Record<string, Skill>
  stats: RangedStatMap
  itemSkillBonuses: Record<string, [number, number]>
}) {
  // Renders the line-by-line skill damage breakdown: Base → Synergies → Multipliers → Hit / Crit / Average. Mirrors the .bd block in Stats View Mockup.
  const synergyLines: Array<{ label: string; pctMin: number; pctMax: number }> =
    []
  for (const b of skill.bonusSources ?? []) {
    if (b.per === 'skill_level') {
      const sourceKey = normalizeSkillName(b.source)
      const baseRank = skillRanksByName[sourceKey] ?? 0
      if (baseRank === 0) continue
      const srcSkill = skillsByNormalizedName[sourceKey]
      const [effMin, effMax] = srcSkill
        ? effectiveRankRangeFor(srcSkill, baseRank, stats, itemSkillBonuses)
        : [baseRank, baseRank]
      const rankLabel =
        effMin === effMax ? `rank ${effMin}` : `rank ${effMin}-${effMax}`
      synergyLines.push({
        label: `${b.source} (${rankLabel})`,
        pctMin: effMin * b.value,
        pctMax: effMax * b.value,
      })
    } else if (b.per === 'attribute_point') {
      const attrKey = Object.keys(attributes).find(
        (k) => k.toLowerCase() === b.source.trim().toLowerCase(),
      )
      const attr = attrKey ? (attributes[attrKey] ?? 0) : 0
      const amin = rangedMin(attr)
      const amax = rangedMax(attr)
      if (amin === 0 && amax === 0) continue
      synergyLines.push({
        label: `${b.source} (${amin === amax ? amin : `${amin}-${amax}`})`,
        pctMin: amin * b.value,
        pctMax: amax * b.value,
      })
    }
  }

  const effRankLabel =
    breakdown.effectiveRankMin === breakdown.effectiveRankMax
      ? String(breakdown.effectiveRankMin)
      : `${breakdown.effectiveRankMin}-${breakdown.effectiveRankMax}`
  const rankBonusMin = breakdown.effectiveRankMin - currentRank
  const rankBonusMax = breakdown.effectiveRankMax - currentRank
  const hasRankBonus = rankBonusMin !== 0 || rankBonusMax !== 0

  return (
    <div className="mt-2 border-t border-dashed border-border pt-2 text-[12px]">
      <BDSection title="Base">
        <BDLine
          label="Effective rank"
          value={
            <>
              <span className="text-text">{effRankLabel}</span>
              {hasRankBonus && (
                <span className="text-faint font-normal">
                  {' '}
                  ({currentRank}
                  {rankBonusMin === rankBonusMax
                    ? rankBonusMin >= 0
                      ? `+${rankBonusMin}`
                      : rankBonusMin
                    : ` +${rankBonusMin}-${rankBonusMax}`}
                  )
                </span>
              )}
            </>
          }
        />
        <BDLine
          label="Base damage"
          value={
            <span className="text-text">
              {formatRange(breakdown.baseMin, breakdown.baseMax)}
            </span>
          }
        />
        {(breakdown.flatMin > 0 || breakdown.flatMax > 0) && (
          <BDLine
            label="Flat added"
            value={
              <span className="text-text">
                {formatRange(breakdown.flatMin, breakdown.flatMax)}
              </span>
            }
          />
        )}
      </BDSection>
      {synergyLines.length > 0 && (
        <BDSection title="Synergies">
          {synergyLines.map((line, i) => (
            <BDLine
              key={i}
              label={line.label}
              indent
              value={
                <span className="text-yellow-300">
                  +
                  {line.pctMin === line.pctMax
                    ? formatDecimal(line.pctMin)
                    : `${formatDecimal(line.pctMin)}-${formatDecimal(line.pctMax)}`}
                  %
                </span>
              }
            />
          ))}
          <BDLine
            label="Total synergy"
            value={
              <span className="text-yellow-300">
                +
                {breakdown.synergyMinPct === breakdown.synergyMaxPct
                  ? formatDecimal(breakdown.synergyMinPct)
                  : `${formatDecimal(breakdown.synergyMinPct)}-${formatDecimal(breakdown.synergyMaxPct)}`}
                %
              </span>
            }
          />
        </BDSection>
      )}
      {((breakdown.skillDamageMinPct > 0 || breakdown.skillDamageMaxPct > 0) ||
        breakdown.enemyResistancePct !== 0 ||
        breakdown.resistanceIgnoredPct !== 0 ||
        breakdown.multicastChancePct > 0 ||
        breakdown.elementalBreakPct > 0 ||
        breakdown.projectileCount > 1) && (
        <BDSection title="Multipliers">
          {(breakdown.skillDamageMinPct > 0 ||
            breakdown.skillDamageMaxPct > 0) && (
            <BDLine
              label="Skill damage %"
              value={
                <span className="text-accent-hot">
                  +
                  {breakdown.skillDamageMinPct === breakdown.skillDamageMaxPct
                    ? formatDecimal(breakdown.skillDamageMinPct)
                    : `${formatDecimal(breakdown.skillDamageMinPct)}-${formatDecimal(breakdown.skillDamageMaxPct)}`}
                  %
                </span>
              }
            />
          )}
          {breakdown.multicastChancePct > 0 && (
            <BDLine
              label={`Multicast (${formatDecimal(breakdown.multicastChancePct)}%)`}
              value={
                <span className="text-accent-hot">
                  ×{breakdown.multicastMultiplier.toFixed(2)}
                </span>
              }
            />
          )}
          {breakdown.elementalBreakPct > 0 && (
            <BDLine
              label={`Elemental Break (${formatDecimal(breakdown.elementalBreakPct)}%)`}
              value={
                <span className="text-accent-hot">
                  ×{breakdown.elementalBreakMultiplier.toFixed(2)}
                </span>
              }
            />
          )}
          {breakdown.projectileCount > 1 && (
            <BDLine
              label={`Projectiles (${breakdown.projectileCount}×)`}
              value={
                <span className="text-accent-hot">
                  ×{breakdown.projectileCount.toFixed(2)}
                </span>
              }
            />
          )}
          {(breakdown.enemyResistancePct !== 0 ||
            breakdown.resistanceIgnoredPct !== 0) && (
            <BDLine
              label="Enemy resistance"
              value={
                <span
                  className={
                    breakdown.effectiveResistancePct > 0
                      ? 'text-stat-red'
                      : 'text-stat-green'
                  }
                >
                  ×{breakdown.resistanceMultiplier.toFixed(2)}
                  <span className="ml-1 text-faint font-normal">
                    ({formatDecimal(breakdown.effectiveResistancePct)}%)
                  </span>
                </span>
              }
            />
          )}
        </BDSection>
      )}
      {breakdown.extraDamageSources.length > 0 && (
        <BDSection title="Extra damage">
          {breakdown.extraDamageSources.map((s, i) => (
            <BDLine
              key={i}
              indent
              label={s.label}
              value={
                <span className="text-orange-300">
                  +{formatDecimal(s.pct)}%
                </span>
              }
            />
          ))}
        </BDSection>
      )}
      <div className="mt-1.5 flex items-baseline justify-between gap-3 border-t border-border pt-1.5">
        <span className="font-mono text-[10px] font-medium uppercase tracking-[0.14em] text-muted">
          {breakdown.multicastChancePct > 0 || breakdown.projectileCount > 1
            ? 'Average per cast'
            : breakdown.critChance > 0
              ? 'Average hit'
              : 'Hit damage'}
        </span>
        <span className="font-mono text-[14px] font-semibold tabular-nums text-accent-hot">
          {breakdown.critChance > 0 ||
          breakdown.multicastChancePct > 0 ||
          breakdown.projectileCount > 1
            ? formatRangeInt(breakdown.avgMin, breakdown.avgMax)
            : formatRangeInt(breakdown.hitMin, breakdown.hitMax)}
        </span>
      </div>
    </div>
  )
}

function BDLine({
  label,
  value,
  indent,
}: {
  label: string
  value: React.ReactNode
  indent?: boolean
}) {
  // One label/value row inside a damage breakdown section. `indent` shifts label by 14px to nest sub-contributors.
  return (
    <div className="flex items-baseline justify-between gap-3 py-0.5 tabular-nums">
      <span className={`text-faint ${indent ? 'pl-3.5 text-muted' : ''}`}>
        {label}
      </span>
      <span className="text-right font-mono font-medium">{value}</span>
    </div>
  )
}

function BDSection({
  title,
  children,
}: {
  title: string
  children: React.ReactNode
}) {
  // Subsection inside a breakdown — gold-deep heading with a line that fills remaining width.
  return (
    <div className="mt-2 first:mt-0">
      <div className="mb-1 flex items-center gap-2">
        <span className="font-mono text-[9px] font-semibold uppercase tracking-[0.18em] text-accent-deep">
          {title}
        </span>
        <span className="h-px flex-1 bg-border" />
      </div>
      {children}
    </div>
  )
}

function SubSectionLabel({
  children,
  first,
}: {
  children: React.ReactNode
  first?: boolean
}) {
  // Small gold-deep section label inside a stat panel (e.g. "Mitigation", "Resistances"). Doubles as a divider line.
  return (
    <div className={`mb-1 flex items-center gap-2 ${first ? 'mt-0' : 'mt-3'}`}>
      <span className="font-mono text-[9px] font-semibold uppercase tracking-[0.18em] text-accent-deep">
        {children}
      </span>
      <span className="h-px flex-1 bg-border" />
    </div>
  )
}

function SectionHeading({ children }: { children: React.ReactNode }) {
  // The "ph" page-section heading from the mockup — small gold-deep dash + uppercase mono label. Plays nicely with the parent `space-y-*` so each heading gets the same vertical breathing room as the panels around it.
  return (
    <div className="flex items-center gap-2.5 font-mono text-[10px] font-semibold uppercase tracking-[0.2em] text-muted">
      <span className="h-px w-3.5 bg-accent-deep" />
      {children}
    </div>
  )
}

function Panel({
  title,
  meta,
  children,
  padded,
}: {
  title?: string
  meta?: string
  children: React.ReactNode
  padded?: boolean
}) {
  // Titled section card with PickerModal-style chrome (gradient, accent corners, JetBrains Mono header). When `title` is omitted (e.g. Per-Skill panel) the head is hidden but the chrome stays.
  return (
    <div
      className={`relative overflow-hidden rounded-md border border-border ${padded ? 'px-4 pb-3.5 pt-3.5' : 'px-4 pb-3.5 pt-4'}`}
      style={{
        background:
          'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
        boxShadow:
          'inset 0 1px 0 rgba(255,255,255,0.02), 0 8px 24px rgba(0,0,0,0.35)',
      }}
    >
      <CornerMark pos="tl" />
      <CornerMark pos="br" />
      {title && (
        <div className="mb-3 flex items-center gap-2 border-b border-accent-deep/20 pb-2">
          <span
            aria-hidden
            className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
            style={{ boxShadow: '0 0 6px rgba(224,184,100,0.5)' }}
          />
          <h3 className="m-0 font-mono text-[10px] font-semibold uppercase tracking-[0.18em] text-accent-hot/70">
            {title}
          </h3>
          {meta && (
            <span className="ml-auto font-mono text-[9px] uppercase tracking-[0.14em] text-faint">
              {meta}
            </span>
          )}
        </div>
      )}
      {children}
    </div>
  )
}

function CornerMark({ pos }: { pos: 'tl' | 'br' }) {
  // One of the two corner accent marks on a Panel — gold-deep L pinned to the top-left or bottom-right.
  const base: React.CSSProperties = {
    position: 'absolute',
    width: 8,
    height: 8,
    border: '1px solid var(--color-accent-deep)',
    opacity: 0.45,
    pointerEvents: 'none',
  }
  if (pos === 'tl') {
    return (
      <span
        style={{
          ...base,
          top: -1,
          left: -1,
          borderRight: 'none',
          borderBottom: 'none',
        }}
      />
    )
  }
  return (
    <span
      style={{
        ...base,
        bottom: -1,
        right: -1,
        borderLeft: 'none',
        borderTop: 'none',
      }}
    />
  )
}

function formatDecimal(v: number): string {
  // Renders a number as an integer when it is one, otherwise as a fixed-2 decimal.
  if (Number.isInteger(v)) return String(v)
  return v.toFixed(2)
}

function formatRange(min: number, max: number): string {
  // Renders a numeric range as either a single decimal or "min-max" with two decimals each.
  if (min === max) return formatDecimal(min)
  return `${formatDecimal(min)}-${formatDecimal(max)}`
}

function formatRangeInt(min: number, max: number): string {
  // Renders a numeric range as a hyphen-separated integer string, collapsing identical ends.
  if (min === max) return String(min)
  return `${min}-${max}`
}

function displayRange(min: number, max: number): string {
  // Renders a numeric range using an en-dash for visual variety. Used inside the AttributesStrip cell.
  if (min === max) return String(min)
  return `${min}–${max}`
}
