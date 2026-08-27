import { RARITY_LABEL } from '../views/gear/lib/rarity'
import { describeAffixValue } from '../views/gear/lib/affixGroups'
import { formatAffixValue } from '../views/gear/lib/rollMath'
import {
  canStarForge,
  detectRuneword,
  effectiveStars,
  FORGE_KIND_LABEL,
  forgeKindFor,
  getAffix,
  getAugment,
  getCrystalMod,
  getGem,
  getItem,
  getItemGrantedSkillByName,
  getItemSet,
  skills,
} from '@data'
import { BONUS_SOCKET_MOD_ID } from '../store/itemRules'
import type {
  Affix,
  EquippedAffix,
  EquippedItem,
  ItemBase,
  ItemGrantedSkill,
  ItemRarity,
  RangedValue,
} from '../types'
import {
  formatValue,
  isZero,
  shouldScaleImplicit,
  statName,
} from '../utils/item/stats'
import { descriptionWithoutValue } from '../utils/item/itemTextShared'
import { collectSocketGroups } from '../utils/item/socketStats'
import type { AffixValueOutput } from '../utils/calc/bridge'
import type { TooltipTone } from './tooltipTones'

export type TooltipHeaderTone = 'gold' | 'orange' | 'red' | 'pink' | 'green' | 'muted'

// prefiks linii w 'set-items' — renderer po nim poznaje zalozona sztuke
export const EQUIPPED_MARK = '✓'

export type TooltipLineStyle =
  | 'implicit'
  | 'affix'
  | 'affix-missing'
  | 'unholy'
  | 'unholy-missing'
  | 'runeword'
  | 'forged'
  | 'socket'
  | 'set-active'
  | 'set-inactive'
  | 'set-items'
  | 'proc'
  | 'special'
  | 'unsupported'
  | 'muted'

export type TooltipLine =
  | { kind: 'row'; label: string; value: string }
  | {
      kind: 'text'
      text: string
      style: TooltipLineStyle
      italic?: boolean
      small?: boolean
      badge?: string
    }
  | {
      kind: 'entry'
      title: string
      style?: TooltipLineStyle
      suffix?: string
      desc?: string
      // Resolved to a sprite by the renderer.
      icon?: string
      lines: string[]
    }

export interface TooltipSectionModel {
  header?: { text: string; tone: TooltipHeaderTone; trailing?: string }
  lines: TooltipLine[]
  footnote?: string
}

export interface ItemTooltipModel {
  name: string
  tone: TooltipTone
  typeLine: string
  imageId: string
  sections: TooltipSectionModel[]
  footer?: string
}

export interface TooltipModelDeps {
  display: {
    implicitScaled: Record<string, [number, number]>
    skillRankScaled: Record<string, [number, number]>
    affixRanges: (AffixValueOutput | null)[]
  }
  inventory: Record<string, EquippedItem | null>
}

export const RARITY_TONE: Record<ItemRarity, TooltipTone> = {
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
  on_attack: 'when Attacking',
  when_struck: 'when Struck',
  on_kill: 'on Kill',
  on_cast: 'on Cast',
  on_block: 'on Block',
  on_death: 'on Death',
  aura: 'Aura:',
  passive: '',
}

const RECOGNIZED_EFFECTS = new Set([
  'attacks can hit multiple enemies',
  'cannot be frozen',
  'unholy',
  'movement phasing',
  'piercing attack',
  'half freeze duration',
  'double jump',
  'herobound',
  'all skills class',
])

const NOT_SUPPORTED_FOOTNOTE = 'These mods are not yet calculated by the planner.'

type TextLineExtra = { italic?: boolean; small?: boolean; badge?: string }

function textLine(
  text: string,
  style: TooltipLineStyle,
  extra?: TextLineExtra,
): TooltipLine {
  return { kind: 'text', text, style, ...extra }
}

interface GrantedSkillEntry {
  skill: ItemGrantedSkill
  displayRank: string
  lines: string[]
}

export function buildItemTooltipModel(
  base: ItemBase,
  equipped: EquippedItem | undefined,
  deps: TooltipModelDeps,
): ItemTooltipModel {
  const { display, inventory } = deps

  const runeword = equipped ? detectRuneword(base, equipped.socketed) : undefined
  const tone: TooltipTone = runeword ? 'rare' : RARITY_TONE[base.rarity]

  const set = base.setId ? getItemSet(base.setId) : undefined
  const equippedPieceIds = new Set(
    Object.values(inventory).flatMap((eq) => (eq ? [eq.baseId] : [])),
  )
  const setEquippedCount = base.setId
    ? Object.values(inventory).reduce((acc, eq) => {
        if (!eq) return acc
        const b = getItem(eq.baseId)
        return b?.setId === base.setId ? acc + 1 : acc
      }, 0)
    : 0

  const typeLine = buildTypeLine(base, equipped, runeword)

  const scaleImplicit = shouldScaleImplicit(!!runeword)
  const implicitEntries = buildImplicitEntries(base, equipped, display, scaleImplicit)
  const skillBonusEntries = base.skillBonuses ? Object.entries(base.skillBonuses) : []

  const grantedSkillEntries = buildGrantedSkillEntries(base, equipped, display)
  const grantedSkillNames = new Set(
    grantedSkillEntries.map((e) => e.skill.name.trim().toLowerCase()),
  )
  const visibleSkillBonusEntries = skillBonusEntries
    .filter(([skill]) => !grantedSkillNames.has(skill.trim().toLowerCase()))
    .map(([skill, val]): [string, typeof val, boolean] => {
      const override = equipped?.skillBonusOverrides?.[skill]
      const label =
        skill === RANDOM_SKILL_NAME
          ? randomSkillLabel(equipped?.randomSkillId)
          : skill
      const shown =
        override ??
        (skill === RANDOM_SKILL_NAME
          ? (display.skillRankScaled[skill] ?? val)
          : val)
      return [label, shown, override !== undefined]
    })
  const runewordEntries = runeword
    ? Object.entries(runeword.stats).filter(([, v]) => v !== 0)
    : []

  const socketGroups = equipped ? collectSocketGroups(equipped, base) : []
  const displayName = buildDisplayName(base, equipped, runeword)
  const equippedForgedMods = equipped?.forgedMods ?? []
  const forgeKind = canStarForge(base.slot)
    ? forgeKindFor(base.rarity)
    : null

  const sections: TooltipSectionModel[] = []

  const baseStatRows = buildBaseStatRows(base)
  if (baseStatRows.length > 0) sections.push({ lines: baseStatRows })

  const implicitLines: TooltipLine[] = [
    ...implicitEntries.map(([key, value, isCustom]) =>
      textLine(
        key === RANDOM_ELEMENT_KEY
          ? `${formatValue(value, '')} ${randomElementLabel(equipped?.randomSkillElement)}`
          : `${formatValue(value, key)} ${statName(key)}`,
        'implicit',
        isCustom ? { badge: 'custom' } : undefined,
      ),
    ),
    ...visibleSkillBonusEntries.map(([skill, val, isCustom]) =>
      textLine(
        `${formatValue(val, '')} to ${skill}`,
        'implicit',
        isCustom ? { badge: 'custom' } : undefined,
      ),
    ),
  ]
  if (implicitLines.length > 0) {
    sections.push({ header: { text: 'Implicit', tone: 'gold' }, lines: implicitLines })
  }

  if (grantedSkillEntries.length > 0) {
    sections.push({
      header: { text: 'Granted Skill Effects', tone: 'orange' },
      lines: grantedSkillEntries.map((e): TooltipLine => ({
        kind: 'entry',
        title: e.skill.name,
        suffix: `rank ${e.displayRank}`,
        ...(e.skill.description ? { desc: e.skill.description } : {}),
        lines: e.lines,
      })),
    })
  }

  if (runewordEntries.length > 0) {
    sections.push({
      lines: runewordEntries.map(([key, val]) =>
        textLine(`${formatValue(val as number, key)} ${statName(key)}`, 'runeword'),
      ),
    })
  }

  const { supported, unsupported: unsupportedAffixes, unholy } = buildAffixLines(
    equipped?.affixes ?? [],
    display,
  )
  if (supported.length > 0) {
    sections.push({ header: { text: 'Affixes', tone: 'gold' }, lines: supported })
  }
  // uniqueEffects carries one "Unholy" per rollable slot; the picked ones are
  // already listed, so the remainder shows as slots still waiting for a roll.
  const unrolledUnholy = Math.max(0, countUnholySlots(base) - unholy.length)
  if (unholy.length > 0 || unrolledUnholy > 0) {
    sections.push({
      header: { text: 'Unholy Affixes', tone: 'pink' },
      lines: [
        ...unholy,
        ...Array.from({ length: unrolledUnholy }, () =>
          textLine('Unholy (not rolled)', 'unholy-missing'),
        ),
      ],
    })
  }

  if (equippedForgedMods.length > 0 && forgeKind) {
    const forgedLines = equippedForgedMods
      .map((eq) => getCrystalMod(eq.affixId))
      .filter((mod): mod is Affix => !!mod)
      .map((mod) => textLine(mod.description, 'forged'))
    if (forgedLines.length > 0) {
      sections.push({
        header: { text: `Forged · ${FORGE_KIND_LABEL[forgeKind]}`, tone: 'red' },
        lines: forgedLines,
      })
    }
  }

  if (socketGroups.length > 0) {
    sections.push({
      header: { text: 'From Sockets', tone: 'gold' },
      lines: socketGroups.map((group): TooltipLine => ({
        kind: 'entry',
        style: 'socket',
        title: group.count > 1 ? `${group.name} ×${group.count}` : group.name,
        icon: group.name,
        lines: group.stats.map(([k, v]) => `${formatValue(v, k)} ${statName(k)}`),
      })),
    })
  }

  const augment = equipped?.augment ? getAugment(equipped.augment.id) : undefined
  const augmentTier = augment
    ? augment.levels[
        Math.max(
          0,
          Math.min(augment.levels.length - 1, (equipped?.augment?.level ?? 1) - 1),
        )
      ]
    : undefined
  if (augment && augmentTier) {
    const augmentLines = Object.entries(augmentTier.stats)
      .filter(([, v]) => v !== 0)
      .map(([k, v]) => `${formatValue(v as number, k)} ${statName(k)}`)
    if (augmentTier.procChance !== undefined) {
      const duration = augmentTier.procDurationSec
      augmentLines.push(
        `${augmentTier.procChance}% ${augment.triggerNote.toLowerCase()}${
          duration !== undefined ? ` · ${duration}s` : ''
        }`,
      )
    }
    sections.push({
      header: { text: 'Angelic Augment', tone: 'gold' },
      lines: [
        {
          kind: 'entry',
          title: augment.name,
          icon: augment.id,
          suffix: `level ${equipped?.augment?.level ?? 1}`,
          desc: augment.description,
          lines: augmentLines,
        },
      ],
    })
  }

  if (set && set.bonuses.length > 0) {
    sections.push({
      header: {
        text: set.name,
        tone: 'green',
        trailing: `${setEquippedCount}/${set.items.length} pieces`,
      },
      lines: [
        ...set.bonuses.map((bonus): TooltipLine => {
          const active = setEquippedCount >= bonus.pieces
          return {
            kind: 'entry',
            title: active ? `${bonus.pieces}-Set (active)` : `${bonus.pieces}-Set`,
            style: active ? 'set-active' : 'set-inactive',
            lines: bonus.descriptions ?? [],
          }
        }),
        {
          kind: 'entry',
          title: 'Set items',
          style: 'set-items',
          lines: set.items.map(
            (piece) =>
              `${equippedPieceIds.has(piece.itemId) ? EQUIPPED_MARK : '·'} ${piece.name} (${piece.slot})`,
          ),
        },
      ],
    })
  }

  if (base.procs && base.procs.length > 0) {
    sections.push({
      lines: base.procs.map((p): TooltipLine => {
        const chancePart = p.chance !== undefined ? `${p.chance}% ` : ''
        const triggerPart = TRIGGER_LABEL[p.trigger]
          ? `Chance ${TRIGGER_LABEL[p.trigger]} to `
          : ''
        return {
          kind: 'entry',
          title: `${chancePart}${triggerPart}${p.description}`,
          style: 'proc',
          ...(p.details ? { desc: p.details } : {}),
          lines: [],
        }
      }),
    })
  }

  const effects = (base.uniqueEffects ?? []).filter((e) => e.trim() !== UNHOLY_EFFECT)
  const special = effects.filter((e) => RECOGNIZED_EFFECTS.has(e.trim().toLowerCase()))
  if (special.length > 0) {
    sections.push({
      header: { text: 'Special Effects', tone: 'gold' },
      lines: special.map((e) => textLine(e, 'special')),
    })
  }
  const notSupported = [
    ...unsupportedAffixes,
    ...effects
      .filter((e) => !RECOGNIZED_EFFECTS.has(e.trim().toLowerCase()))
      .map((e) => textLine(e, 'unsupported')),
  ]
  if (notSupported.length > 0) {
    sections.push({
      header: { text: 'Not Yet Supported', tone: 'muted' },
      lines: notSupported,
      footnote: NOT_SUPPORTED_FOOTNOTE,
    })
  }

  const descLines: TooltipLine[] = []
  if (base.description) descLines.push(textLine(base.description, 'muted', { italic: true }))
  if (base.flavor) descLines.push(textLine(base.flavor, 'muted', { italic: true }))
  if (descLines.length > 0) sections.push({ lines: descLines })

  const footer = buildFooter(base, runeword)

  return {
    name: displayName,
    tone,
    typeLine,
    imageId: base.id,
    sections,
    ...(footer ? { footer } : {}),
  }
}

function buildTypeLine(
  base: ItemBase,
  equipped: EquippedItem | undefined,
  runeword: ReturnType<typeof detectRuneword>,
): string {
  const stars = effectiveStars(base.slot, equipped?.stars) ?? 0
  const starSuffix = stars > 0 ? ` · ${'★'.repeat(stars)}` : ''
  const handSuffix =
    base.slot === 'weapon' ? (base.twoHanded ? ' · 2-Handed' : ' · 1-Handed') : ''
  const isTinkered = !!equipped?.forgedMods?.some(
    (m) => m.affixId === BONUS_SOCKET_MOD_ID,
  )
  const tinkeredSuffix = isTinkered ? ' · Tinkered' : ''
  const rarityLabel = runeword ? 'Runeword' : RARITY_LABEL[base.rarity]
  return `${rarityLabel} · ${base.baseType}${handSuffix}${starSuffix}${tinkeredSuffix}`
}

function buildDisplayName(
  base: ItemBase,
  equipped: EquippedItem | undefined,
  runeword: ReturnType<typeof detectRuneword>,
): string {
  if (runeword) return runeword.name
  const gemNames: string[] = []
  if (equipped && base.socketTransforms) {
    for (const id of equipped.socketed) {
      if (id && base.socketTransforms[id]) {
        const gem = getGem(id)
        if (gem) gemNames.push(gem.name)
      }
    }
  }
  return gemNames.length > 0 ? `${base.name} (${gemNames.join(' + ')})` : base.name
}

function buildBaseStatRows(base: ItemBase): TooltipLine[] {
  const rows: TooltipLine[] = []
  if (base.defenseMin !== undefined && base.defenseMax !== undefined) {
    rows.push({ kind: 'row', label: 'Defense', value: `${base.defenseMin}–${base.defenseMax}` })
  }
  if (base.damageMin !== undefined && base.damageMax !== undefined) {
    rows.push({ kind: 'row', label: 'Damage', value: `${base.damageMin}–${base.damageMax}` })
  }
  if (base.blockChance !== undefined) {
    rows.push({ kind: 'row', label: 'Block', value: `${base.blockChance}%` })
  }
  if (base.attackSpeed !== undefined) {
    rows.push({ kind: 'row', label: 'Attacks / sec', value: `${base.attackSpeed}` })
  }
  return rows
}

function buildImplicitEntries(
  base: ItemBase,
  equipped: EquippedItem | undefined,
  display: TooltipModelDeps['display'],
  scaleImplicit: boolean,
): Array<[string, RangedValue, boolean]> {
  const implicitOverrides = equipped?.implicitOverrides
  const baseImplicitEntries: Array<[string, RangedValue, boolean]> = base.implicit
    ? Object.entries(base.implicit)
        .map(([k, v]): [string, RangedValue, boolean] => {
          const override = implicitOverrides?.[k]
          if (override !== undefined) return [k, override, true]
          const scaled = scaleImplicit ? (display.implicitScaled[k] ?? v) : v
          return [k, scaled, false]
        })
        .filter(([, v]) => !isZero(v))
    : []
  const extraImplicitEntries: Array<[string, RangedValue, boolean]> = implicitOverrides
    ? Object.entries(implicitOverrides)
        .filter(([k]) => !base.implicit || !(k in base.implicit))
        .map(([k, v]): [string, RangedValue, boolean] => [k, v, true])
    : []
  return [...baseImplicitEntries, ...extraImplicitEntries]
}

const round2 = (n: number): number => Math.round(n * 100) / 100

// Item data names the roll; the tooltip shows what the user says it landed on.
export const RANDOM_SKILL_NAME = 'Random Skill'

function randomSkillLabel(skillId: string | undefined): string {
  if (!skillId) return `${RANDOM_SKILL_NAME} (not rolled)`
  return skills.find((s) => s.id === skillId)?.name ?? RANDOM_SKILL_NAME
}

// Same idea for "+X to Random Skill Element": the element is the user's pick.
const RANDOM_ELEMENT_KEY = 'random_skill_element'

function randomElementLabel(element: string | undefined): string {
  if (!element) return 'to Random Element Skills (not rolled)'
  return `to ${element.charAt(0).toUpperCase()}${element.slice(1)} Skills (random element)`
}

function buildGrantedSkillEntries(
  base: ItemBase,
  equipped: EquippedItem | undefined,
  display: TooltipModelDeps['display'],
): GrantedSkillEntry[] {
  if (!base.skillBonuses) return []
  const out: GrantedSkillEntry[] = []
  for (const skillName of Object.keys(base.skillBonuses)) {
    // The catalog carries an empty "Random Skill" placeholder; the roll grants
    // ranks in a real skill, so it belongs on an implicit line, not here.
    if (skillName === RANDOM_SKILL_NAME) continue
    const skill = getItemGrantedSkillByName(skillName)
    if (!skill) continue
    const override = equipped?.skillBonusOverrides?.[skillName]
    const [sMin, sMax] =
      override !== undefined
        ? [override, override]
        : (display.skillRankScaled[skillName] ?? [0, 0])
    const rMin = Math.round(sMin)
    const rMax = Math.round(sMax)
    if (rMax <= 0) continue
    const displayRank = rMin === rMax ? String(rMin) : `${rMin}-${rMax}`
    const lines: string[] = []
    if (skill.passiveConverts) {
      for (const c of skill.passiveConverts.perRank) {
        const pctMin = round2((c.basePct ?? 0) + c.pct * rMin)
        const pctMax = round2((c.basePct ?? 0) + c.pct * rMax)
        const pctText = pctMin === pctMax ? `${pctMin}%` : `${pctMin}–${pctMax}%`
        const verb = c.replaces ? 'converted to' : 'added as'
        lines.push(`${pctText} of ${statName(c.from)} ${verb} ${statName(c.to)}`)
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
      for (const [k, pair] of Object.entries(totals)) {
        const [a, b] = pair
        if (a === 0 && b === 0) continue
        lines.push(`${formatValue(a === b ? a : pair, k)} ${statName(k)}`)
      }
    }
    out.push({ skill, displayRank, lines })
  }
  return out
}

// Item data spells one "Unholy" per rollable unholy-affix slot.
const UNHOLY_EFFECT = 'Unholy'

function countUnholySlots(base: ItemBase): number {
  return (base.uniqueEffects ?? []).filter((e) => e.trim() === UNHOLY_EFFECT).length
}

function buildAffixLines(
  equippedAffixes: EquippedAffix[],
  display: TooltipModelDeps['display'],
): {
  supported: TooltipLine[]
  unsupported: TooltipLine[]
  unholy: TooltipLine[]
} {
  const supported: TooltipLine[] = []
  const unsupported: TooltipLine[] = []
  const unholy: TooltipLine[] = []
  equippedAffixes.forEach((eq, idx) => {
    const affix = getAffix(eq.affixId)
    if (!affix) return
    const isUnholy = affix.groupId === 'random_unholy'
    const line = buildAffixLine(eq, affix, isUnholy, display.affixRanges[idx] ?? null)
    if (isUnholy) unholy.push(line)
    else if (affix.statKey) supported.push(line)
    else unsupported.push(line)
  })
  return { supported, unsupported, unholy }
}

function buildAffixLine(
  eq: EquippedAffix,
  affix: Affix,
  isUnholy: boolean,
  range: AffixValueOutput | null,
): TooltipLine {
  // Affixes the engine cannot calculate still show their roll, under "Not Yet Supported".
  const style: TooltipLineStyle = isUnholy
    ? 'unholy'
    : affix.statKey
      ? 'affix'
      : 'unsupported'
  // An equipped affix always carries a roll, so show what it landed on, not the range.
  const value = eq.customValue ?? range?.value
  if (value === undefined) return textLine(affix.description, style)
  const badge = eq.customValue !== undefined ? { badge: 'custom' } : undefined
  const described = describeAffixValue(affix, value)
  if (described) return textLine(described, style, badge)
  const shown = affix.statKey
    ? formatValue(value, affix.statKey)
    : formatAffixValue(affix, value)
  return textLine(
    `${shown} ${descriptionWithoutValue(affix.description)}`.trim(),
    style,
    badge,
  )
}

function buildFooter(
  base: ItemBase,
  runeword: ReturnType<typeof detectRuneword>,
): string | undefined {
  const requiresLevel = runeword?.requiresLevel ?? base.requiresLevel
  const footerBits: string[] = []
  if (requiresLevel !== undefined) footerBits.push(`Req Level ${requiresLevel}`)
  if (base.itemLevel) footerBits.push(`iLvl ${base.itemLevel}`)
  if (base.grade) footerBits.push(`Tier ${base.grade}`)
  return footerBits.length > 0 ? footerBits.join(' · ') : undefined
}
