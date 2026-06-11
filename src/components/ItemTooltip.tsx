import type { ReactNode } from 'react'
import { useCalcResult } from '../hooks/useCalcResult'
import { RARITY_LABEL } from '../views/gear/lib/rarity'
import {
  detectRuneword,
  FORGE_KIND_LABEL,
  forgeKindFor,
  getAffix,
  getCrystalMod,
  getGem,
  getItem,
  getItemGrantedSkillByName,
  getItemImage,
  getItemSet,
  getRune,
  isGearSlot,
} from '../data'
import { BONUS_SOCKET_MOD_ID, RAINBOW_MULTIPLIER, useBuild } from '../store/build'
import type {
  EquippedItem,
  ItemBase,
  ItemGrantedSkill,
  ItemRarity,
  RangedValue,
  StatMap,
} from '../types'
import {
  formatAffixRangeFromValues,
  formatValue,
  isZero,
  rangedMax,
  rangedMin,
  shouldScaleImplicit,
  statName,
} from '../utils/item/stats'
import { displayValuesNative } from '../lib/calc/bridge'
import type { AffixValueOutput } from '../lib/calc/bridge'
import Tooltip, {
  TooltipFooter,
  TooltipHeader,
  TooltipSection,
  TooltipSectionHeader,
} from './Tooltip'
import { TONE_BORDER, TONE_GLOW, TONE_RGB, TONE_TEXT } from './tooltip-tones'
import type { CSSProperties } from 'react'
import type { TooltipTone } from './tooltip-tones'

const RARITY_TONE: Record<ItemRarity, TooltipTone> = {
  common: 'common',
  uncommon: 'uncommon',
  rare: 'rare',
  mythic: 'mythic',
  satanic: 'satanic',
  heroic: 'heroic',
  angelic: 'angelic',
  satanic_set: 'satanic_set',
  unholy: 'unholy',
  relic: 'relic',
}

const TRIGGER_LABEL: Record<string, string> = {
  on_hit: 'on Hit',
  when_struck: 'when Struck',
  on_kill: 'on Kill',
  on_cast: 'on Cast',
  on_block: 'on Block',
  on_death: 'on Death',
  aura: 'Aura:',
  passive: '',
}

interface Props {
  equipped: EquippedItem
  children: ReactNode
  placement?: 'right' | 'left' | 'top' | 'bottom'
  className?: string
  // Forwarded to the underlying Tooltip so triggers inside higher-stacked modals can layer above the backdrop.
  zIndex?: number
}

export default function ItemTooltip({
  equipped,
  children,
  placement = 'right',
  className,
  zIndex,
}: Props) {
  const base = getItem(equipped.baseId)
  if (!base) return <>{children}</>

  const runeword = detectRuneword(base, equipped.socketed)
  const tone: TooltipTone = runeword ? 'rare' : RARITY_TONE[base.rarity]

  return (
    <Tooltip
      tone={tone}
      placement={placement}
      className={className}
      zIndex={zIndex}
      content={<ItemTooltipBody equipped={equipped} base={base} />}
    >
      {children}
    </Tooltip>
  )
}

interface TooltipDisplayValues {
  implicitScaled: Record<string, [number, number]>
  skillRankScaled: Record<string, [number, number]>
  affixRanges: (AffixValueOutput | null)[]
}

const EMPTY_DISPLAY: TooltipDisplayValues = {
  implicitScaled: {},
  skillRankScaled: {},
  affixRanges: [],
}

// One display_values batch per item: star-scaled implicits, skill-rank
// bonuses, and affix roll ranges all come from the Rust engine.
function useItemDisplayValues(
  base: ItemBase,
  equipped: EquippedItem | undefined,
  scaleImplicit: boolean,
): TooltipDisplayValues | null {
  return useCalcResult<TooltipDisplayValues | null>(
    () => {
      const stars = equipped?.stars ?? null
      const toPair = (v: RangedValue): [number, number] => [
        rangedMin(v),
        rangedMax(v),
      ]
      const implicitEntries =
        scaleImplicit && base.implicit ? Object.entries(base.implicit) : []
      const skillEntries = base.skillBonuses
        ? Object.entries(base.skillBonuses)
        : []
      const equippedAffixes = equipped?.affixes ?? []
      const affixDefs = equippedAffixes.map((eq) => getAffix(eq.affixId))
      const affixReqs = equippedAffixes
        .map((eq, i) => ({ eq, def: affixDefs[i] }))
        .filter((x) => x.def)
        .map((x) => ({ affix: x.def, roll: x.eq.roll ?? 0, stars }))
      const scaled = [
        ...implicitEntries.map(([k, v]) => ({
          value: toPair(v),
          statKey: k,
          stars,
        })),
        ...skillEntries.map(([, v]) => ({
          value: toPair(v),
          statKey: 'item_granted_skill_rank',
          stars,
        })),
      ]
      if (affixReqs.length === 0 && scaled.length === 0) {
        return EMPTY_DISPLAY
      }
      return displayValuesNative({ affixes: affixReqs, scaled }).then((res) => {
        const implicitScaled: Record<string, [number, number]> = {}
        implicitEntries.forEach(([k], i) => {
          implicitScaled[k] = res.scaled[i]!
        })
        const skillRankScaled: Record<string, [number, number]> = {}
        skillEntries.forEach(([name], i) => {
          skillRankScaled[name] = res.scaled[implicitEntries.length + i]!
        })
        const affixRanges: (AffixValueOutput | null)[] = []
        let cursor = 0
        for (const def of affixDefs) {
          affixRanges.push(def ? (res.affixes[cursor++] ?? null) : null)
        }
        return { implicitScaled, skillRankScaled, affixRanges }
      })
    },
    [base, equipped, scaleImplicit],
    null,
  )
}

export function ItemTooltipBody({
  equipped,
  base,
}: {
  equipped?: EquippedItem
  base: ItemBase
}) {
  const inventory = useBuild((s) => s.inventory)

  const runeword = equipped ? detectRuneword(base, equipped.socketed) : undefined
  const display = useItemDisplayValues(
    base,
    equipped,
    shouldScaleImplicit(!!runeword),
  )
  if (!display) return null
  const tone: TooltipTone = runeword ? 'rare' : RARITY_TONE[base.rarity]
  const set = base.setId ? getItemSet(base.setId) : undefined
  const setEquippedCount = base.setId
    ? Object.values(inventory).reduce((acc, eq) => {
        if (!eq) return acc
        const b = getItem(eq.baseId)
        return b?.setId === base.setId ? acc + 1 : acc
      }, 0)
    : 0

  const stars = equipped?.stars ?? 0
  const starSuffix = stars > 0 ? ` · ${'★'.repeat(stars)}` : ''
  const handSuffix =
    base.slot === 'weapon'
      ? base.twoHanded
        ? ' · 2-Handed'
        : ' · 1-Handed'
      : ''
  const isTinkered = !!equipped?.forgedMods?.some(
    (m) => m.affixId === BONUS_SOCKET_MOD_ID,
  )
  const tinkeredSuffix = isTinkered ? ' · Tinkered' : ''
  const subtitle = runeword
    ? `Runeword · ${base.baseType}${handSuffix}${starSuffix}${tinkeredSuffix}`
    : `${RARITY_LABEL[base.rarity]} · ${base.baseType}${handSuffix}${starSuffix}${tinkeredSuffix}`

  const hasBaseStats =
    base.defenseMin !== undefined ||
    base.damageMin !== undefined ||
    base.blockChance !== undefined ||
    base.attackSpeed !== undefined

  const scaleImplicit = shouldScaleImplicit(!!runeword)
  const implicitOverrides = equipped?.implicitOverrides
  const baseImplicitEntries: Array<[string, RangedValue, boolean]> = base.implicit
    ? Object.entries(base.implicit)
        .map(([k, v]) => {
          const override = implicitOverrides?.[k]
          if (override !== undefined) {
            return [k, override, true] as [string, RangedValue, boolean]
          }
          const scaled = scaleImplicit ? (display.implicitScaled[k] ?? v) : v
          return [k, scaled, false] as [string, RangedValue, boolean]
        })
        .filter(([, v]) => !isZero(v))
    : []
  const extraImplicitEntries: Array<[string, RangedValue, boolean]> =
    implicitOverrides
      ? Object.entries(implicitOverrides)
          .filter(([k]) => !base.implicit || !(k in base.implicit))
          .map(([k, v]) => [k, v, true] as [string, RangedValue, boolean])
      : []
  const implicitEntries = [...baseImplicitEntries, ...extraImplicitEntries]
  const skillBonusEntries = base.skillBonuses
    ? Object.entries(base.skillBonuses)
    : []

  const grantedSkillEntries = (() => {
    if (!base.skillBonuses) return [] as Array<{
      skill: ItemGrantedSkill
      displayRank: string
      lines: string[]
    }>
    const out: Array<{
      skill: ItemGrantedSkill
      displayRank: string
      lines: string[]
    }> = []
    for (const skillName of Object.keys(base.skillBonuses)) {
      const skill = getItemGrantedSkillByName(skillName)
      if (!skill) continue
      const [sMin, sMax] = display.skillRankScaled[skillName] ?? [0, 0]
      const rMin = Math.round(sMin)
      const rMax = Math.round(sMax)
      if (rMax <= 0) continue
      const displayRank = rMin === rMax ? String(rMin) : `${rMin}-${rMax}`
      const lines: string[] = []
      if (skill.passiveConverts) {
        for (const c of skill.passiveConverts.perRank) {
          const pctMin = c.pct * rMin
          const pctMax = c.pct * rMax
          const pctText = pctMin === pctMax ? `${pctMin}%` : `${pctMin}–${pctMax}%`
          lines.push(
            `${pctText} of ${statName(c.from)} added as ${statName(c.to)}`,
          )
        }
      }
      if (skill.passiveStats) {
        const { base: baseStats, perRank } = skill.passiveStats
        const totals: Record<string, [number, number]> = {}
        if (baseStats) {
          for (const [k, v] of Object.entries(baseStats)) totals[k] = [v, v]
        }
        if (perRank) {
          for (const [k, v] of Object.entries(perRank)) {
            const cur = totals[k] ?? [0, 0]
            totals[k] = [cur[0] + v * rMin, cur[1] + v * rMax]
          }
        }
        for (const [k, [a, b]] of Object.entries(totals)) {
          if (a === 0 && b === 0) continue
          const txt = a === b ? `${a >= 0 ? '+' : ''}${a}` : `+${a}–${b}`
          lines.push(`${txt} ${statName(k)}`)
        }
      }
      out.push({ skill, displayRank, lines })
    }
    return out
  })()
  const runewordEntries = runeword
    ? Object.entries(runeword.stats).filter(([, v]) => v !== 0)
    : []

  const socketStats = equipped ? collectSocketStats(equipped, base) : []

  const activeTransformGemNames = (() => {
    if (!equipped || !base.socketTransforms) return [] as string[]
    const names: string[] = []
    for (const id of equipped.socketed) {
      if (id && base.socketTransforms[id]) {
        const gem = getGem(id)
        if (gem) names.push(gem.name)
      }
    }
    return names
  })()
  const displayName = runeword
    ? runeword.name
    : activeTransformGemNames.length > 0
      ? `${base.name} (${activeTransformGemNames.join(' + ')})`
      : base.name

  const requiresLevel = runeword?.requiresLevel ?? base.requiresLevel
  const footerBits: string[] = []
  if (requiresLevel !== undefined) footerBits.push(`Req Level ${requiresLevel}`)
  if (base.itemLevel) footerBits.push(`iLvl ${base.itemLevel}`)
  if (base.grade) footerBits.push(`Tier ${base.grade}`)

  const equippedAffixes = equipped?.affixes ?? []
  const equippedForgedMods = equipped?.forgedMods ?? []
  const forgeKind = isGearSlot(base.slot) ? forgeKindFor(base.rarity) : null
  const forgeAccent = 'text-stat-red'

  return (
    <>
      <TooltipHeader
        tone={tone}
        title={displayName}
        subtitle={subtitle}
        image={getItemImage(base.id)}
      />

      {hasBaseStats && (
        <TooltipSection>
          <BaseStats base={base} />
        </TooltipSection>
      )}

      {(implicitEntries.length > 0 || skillBonusEntries.length > 0) && (
        <TooltipSection>
          <TooltipSectionHeader tone="gold">Implicit</TooltipSectionHeader>
          <ul className="space-y-0.5 text-[12px]">
            {implicitEntries.map(([key, value, isCustom]) => (
              <li key={`impl-${key}`} className="text-accent-hot">
                {formatValue(value, key)} {statName(key)}
                {isCustom && (
                  <span
                    className="ml-1 font-mono text-[10px] uppercase tracking-[0.14em] text-accent-hot/70"
                    title="Custom value (overrides base implicit)"
                  >
                    custom
                  </span>
                )}
              </li>
            ))}
            {skillBonusEntries.map(([skill, val]) => (
              <li key={`skill-${skill}`} className="text-accent-hot">
                {formatValue(val, '')} to {skill}
              </li>
            ))}
          </ul>
        </TooltipSection>
      )}

      {grantedSkillEntries.length > 0 && (
        <TooltipSection>
          <TooltipSectionHeader tone="orange">
            Granted Skill Effects
          </TooltipSectionHeader>
          <ul className="space-y-1 text-[11px]">
            {grantedSkillEntries.map(({ skill, displayRank, lines }) => (
              <li key={skill.id}>
                <div className="text-accent-hot text-[12px]">
                  {skill.name}{' '}
                  <span className="text-muted text-[10px]">
                    rank {displayRank}
                  </span>
                </div>
                {skill.description && (
                  <div className="text-muted text-[10px] italic leading-snug">
                    {skill.description}
                  </div>
                )}
                {lines.length > 0 && (
                  <ul className="mt-0.5 ml-2 space-y-0.5 text-text/80">
                    {lines.map((line, i) => (
                      <li key={i} className="text-[11px]">
                        {line}
                      </li>
                    ))}
                  </ul>
                )}
              </li>
            ))}
          </ul>
        </TooltipSection>
      )}

      {runewordEntries.length > 0 && (
        <TooltipSection>
          <ul className="space-y-0.5 text-[12px]">
            {runewordEntries.map(([key, val]) => (
              <li key={`rw-${key}`} className="text-accent-hot">
                {formatValue(val as number, key)} {statName(key)}
              </li>
            ))}
          </ul>
        </TooltipSection>
      )}

      {(() => {
        const indexed = equippedAffixes.map((eq, idx) => ({
          eq,
          idx,
          affix: getAffix(eq.affixId),
        }))
        const unholy = indexed.filter(
          (x) => x.affix?.groupId === 'random_unholy',
        )
        const standard = indexed.filter(
          (x) => x.affix?.groupId !== 'random_unholy',
        )
        const renderItem = ({
          idx,
          eq,
          affix,
        }: (typeof indexed)[number]) => {
          if (!affix) return null
          const isUnholy = affix.groupId === 'random_unholy'
          const colorBase = isUnholy ? 'text-pink-300' : 'text-yellow-300'
          const colorMissing = isUnholy
            ? 'text-pink-300/90 italic'
            : 'text-yellow-300/90 italic'
          if (!affix.statKey) {
            return (
              <li key={idx} className={colorMissing}>
                {affix.description}
              </li>
            )
          }
          const descNoValue = affix.description
            .replace(/^[+-]?\[?[^\]]*]?%?\s*/, '')
            .replace(/\s+/g, ' ')
            .trim()
          const valueDisplay =
            eq.customValue !== undefined
              ? formatValue(eq.customValue, affix.statKey)
              : formatAffixRangeFromValues(
                  affix,
                  display.affixRanges[idx] ?? null,
                )
          const customMark = eq.customValue !== undefined ? ' ✦' : ''
          return (
            <li key={idx} className={colorBase}>
              {valueDisplay} {descNoValue}
              {customMark && (
                <span
                  className="ml-1 font-mono text-[10px] uppercase tracking-[0.14em] text-accent-hot/70"
                  title="Custom value (overrides tier+roll)"
                >
                  custom
                </span>
              )}
            </li>
          )
        }
        return (
          <>
            {standard.length > 0 && (
              <TooltipSection>
                <ul className="space-y-0.5 text-[12px]">
                  {standard.map(renderItem)}
                </ul>
              </TooltipSection>
            )}
            {unholy.length > 0 && (
              <TooltipSection>
                <TooltipSectionHeader tone="pink">
                  Unholy Affixes
                </TooltipSectionHeader>
                <ul className="space-y-0.5 text-[12px]">
                  {unholy.map(renderItem)}
                </ul>
              </TooltipSection>
            )}
          </>
        )
      })()}

      {equippedForgedMods.length > 0 && forgeKind && (
        <TooltipSection>
          <TooltipSectionHeader tone="red">
            Forged · {FORGE_KIND_LABEL[forgeKind]}
          </TooltipSectionHeader>
          <ul className="space-y-0.5 text-[12px]">
            {equippedForgedMods.map((eq, idx) => {
              const mod = getCrystalMod(eq.affixId)
              if (!mod) return null
              return (
                <li key={idx} className={forgeAccent}>
                  {mod.description}
                </li>
              )
            })}
          </ul>
        </TooltipSection>
      )}

      {socketStats.length > 0 && (
        <TooltipSection>
          <TooltipSectionHeader tone="gold">
            From Sockets
          </TooltipSectionHeader>
          <ul className="space-y-0.5 text-[12px]">
            {socketStats.map(([k, v]) => (
              <li key={k} className="text-accent">
                {formatValue(v, k)} {statName(k)}
              </li>
            ))}
          </ul>
        </TooltipSection>
      )}

      {set && set.bonuses.length > 0 && (
        <TooltipSection>
          <TooltipSectionHeader
            tone="green"
            trailing={`${setEquippedCount}/${set.items.length} pieces`}
          >
            {set.name}
          </TooltipSectionHeader>
          <ul className="space-y-1">
            {set.bonuses.map((bonus, idx) => {
              const active = setEquippedCount >= bonus.pieces
              return (
                <li
                  key={idx}
                  className={`text-[11px] ${active ? 'text-green-300' : 'text-muted/70'}`}
                >
                  <div className="text-[10px] uppercase tracking-[0.12em]">
                    {bonus.pieces}-Set {active ? '(active)' : ''}
                  </div>
                  <ul className="ml-1 space-y-0.5">
                    {(bonus.descriptions ?? []).map((d, i) => (
                      <li key={i}>{d}</li>
                    ))}
                  </ul>
                </li>
              )
            })}
          </ul>
        </TooltipSection>
      )}

      {base.procs && base.procs.length > 0 && (
        <TooltipSection>
          <ul className="space-y-1.5 text-[12px]">
            {base.procs.map((p, idx) => (
              <li key={idx} className="text-emerald-300">
                {p.chance !== undefined && (
                  <span className="font-medium">{p.chance}% </span>
                )}
                <span>
                  {TRIGGER_LABEL[p.trigger]
                    ? `Chance ${TRIGGER_LABEL[p.trigger]} to `
                    : ''}
                  {p.description}
                </span>
                {p.details && (
                  <div className="text-[10px] text-muted uppercase tracking-[0.12em] leading-relaxed mt-0.5">
                    {p.details}
                  </div>
                )}
              </li>
            ))}
          </ul>
        </TooltipSection>
      )}

      {base.uniqueEffects && base.uniqueEffects.length > 0 && (
        <TooltipSection>
          <TooltipSectionHeader tone="muted">
            Not Yet Supported
          </TooltipSectionHeader>
          <ul className="space-y-0.5 text-[12px]">
            {base.uniqueEffects.map((effect, idx) => (
              <li key={idx} className={`${TONE_TEXT.angelic} opacity-70`}>
                {effect}
              </li>
            ))}
          </ul>
          <p className="text-[10px] text-muted/70 italic mt-1">
            These mods are not yet calculated by the planner.
          </p>
        </TooltipSection>
      )}

      {(base.description || base.flavor) && (
        <TooltipSection>
          {base.description && (
            <p className="text-[11px] text-muted italic">
              {base.description}
            </p>
          )}
          {base.flavor && (
            <p className="text-[11px] italic text-muted/70 leading-relaxed mt-1">
              {base.flavor}
            </p>
          )}
        </TooltipSection>
      )}

      {footerBits.length > 0 && (
        <TooltipFooter>{footerBits.join(' · ')}</TooltipFooter>
      )}
    </>
  )
}

export type CompareState = 'equipped' | 'selected'

export function ItemCard({
  equipped,
  base,
  arcLabel,
  state,
  className,
}: {
  equipped?: EquippedItem
  base: ItemBase | undefined
  arcLabel?: ReactNode
  state?: CompareState
  className?: string
}) {
  const stateClass = state ? `compare-card is-${state}` : ''

  if (!base) {
    return (
      <div
        className={`relative bg-panel border border-dashed border-border rounded-sm ${stateClass} ${className ?? ''}`}
      >
        {state && arcLabel && (
          <div className="compare-arc" style={arcStyle('neutral')}>
            {arcLabel}
          </div>
        )}
        <div className="flex items-center justify-center text-center px-3 py-6">
          <p className="text-[11px] text-faint italic">empty slot</p>
        </div>
      </div>
    )
  }
  const runeword = equipped ? detectRuneword(base, equipped.socketed) : undefined
  const tone: TooltipTone = runeword ? 'rare' : RARITY_TONE[base.rarity]
  const overflow = state ? '' : 'overflow-hidden'

  return (
    <div
      className={`relative bg-panel border ${TONE_BORDER[tone]} ${TONE_GLOW[tone]} rounded-sm ${overflow} ${stateClass} ${className ?? ''}`}
    >
      {state && arcLabel && (
        <div className="compare-arc" style={arcStyle(tone)}>
          {arcLabel}
        </div>
      )}
      <ItemTooltipBody
        equipped={equipped}
        base={base}
      />
    </div>
  )
}

function arcStyle(tone: TooltipTone): CSSProperties {
  const rgb = TONE_RGB[tone]
  return {
    '--arc-text': `rgb(${rgb})`,
    '--arc-border': `rgba(${rgb}, 0.6)`,
    '--arc-bg': `color-mix(in srgb, rgb(${rgb}) 18%, #14151b)`,
  } as CSSProperties
}

function BaseStats({ base }: { base: ItemBase }) {
  const rows: { label: string; value: string }[] = []
  if (base.defenseMin !== undefined && base.defenseMax !== undefined) {
    rows.push({
      label: 'Defense',
      value: `${base.defenseMin}–${base.defenseMax}`,
    })
  }
  if (base.damageMin !== undefined && base.damageMax !== undefined) {
    rows.push({
      label: 'Damage',
      value: `${base.damageMin}–${base.damageMax}`,
    })
  }
  if (base.blockChance !== undefined) {
    rows.push({ label: 'Block', value: `${base.blockChance}%` })
  }
  if (base.attackSpeed !== undefined) {
    rows.push({ label: 'Attacks / sec', value: `${base.attackSpeed}` })
  }
  return (
    <ul className="grid grid-cols-2 gap-x-4 gap-y-0.5 text-[12px] tabular-nums">
      {rows.map((r) => (
        <li key={r.label} className="contents">
          <span className="text-muted">{r.label}</span>
          <span className="text-text text-right font-medium">{r.value}</span>
        </li>
      ))}
    </ul>
  )
}

function collectSocketStats(
  equipped: EquippedItem,
  base?: ItemBase,
): [string, number][] {
  const stats: StatMap = {}
  for (let i = 0; i < equipped.socketed.length; i++) {
    const id = equipped.socketed[i]
    if (!id) continue
    const source = getGem(id) ?? getRune(id)
    if (!source) continue
    const mult = equipped.socketTypes[i] === 'rainbow' ? RAINBOW_MULTIPLIER : 1
    const transform = base?.socketTransforms?.[id]
    const src = transform ?? source.stats
    for (const [k, v] of Object.entries(src)) {
      stats[k] = (stats[k] ?? 0) + v * mult
    }
  }
  return Object.entries(stats).filter(([, v]) => v !== 0)
}

export { RARITY_TONE }
