import {
  detectRuneword,
  getAffix,
  getAugment,
  getCrystalMod,
  getGem,
  getRune,
} from '../../data'
import type { EquippedItem, ItemBase } from '../../types'
import {
  formatAffixRangeFromValues,
  formatValue,
  isZero,
  shouldScaleImplicit,
  statName,
} from './stats'
import type {
  AffixValueOutput,
  AffixValueRequest,
  DisplayValuesOutput,
  ScaledValueRequest,
} from '../../lib/calc/bridge'
import {
  RARITY_LABELS,
  SEP,
  descriptionWithoutValue,
  nativeAffixMath,
  toPair,
} from './itemTextShared'
import type { AffixMathProvider } from './itemTextShared'

const EMPTY_BATCH: DisplayValuesOutput = { affixes: [], scaled: [] }

export async function serializeEquippedItem(
  equipped: EquippedItem,
  base: ItemBase,
  math: AffixMathProvider = nativeAffixMath,
): Promise<string> {
  const lines: string[] = []
  const stars = equipped.stars ?? 0

  const runewordEarly = detectRuneword(base, equipped.socketed)
  const scaleImplicitEarly = shouldScaleImplicit(!!runewordEarly)
  const implicitBaseEntries = base.implicit ? Object.entries(base.implicit) : []
  const affixDefs = equipped.affixes.map((eq) => getAffix(eq.affixId))
  const forgedList = equipped.forgedMods ?? []
  const forgedDefs = forgedList.map((eq) => getCrystalMod(eq.affixId))
  const scaledReqs: ScaledValueRequest[] = scaleImplicitEarly
    ? implicitBaseEntries.map(([k, v]) => ({
        value: toPair(v),
        statKey: k,
        stars,
      }))
    : []
  const presentDefs = [...affixDefs, ...forgedDefs].filter(
    (d): d is NonNullable<typeof d> => !!d,
  )
  const affixReqs: AffixValueRequest[] = presentDefs.map((def) => ({
    affix: def,
    roll: 0,
    stars,
  }))
  const batch =
    scaledReqs.length > 0 || affixReqs.length > 0
      ? await math.batch({ affixes: affixReqs, scaled: scaledReqs })
      : EMPTY_BATCH
  const implicitScaled = new Map<string, [number, number]>()
  scaledReqs.forEach((r, i) => {
    const v = batch.scaled[i]
    if (v) implicitScaled.set(r.statKey, v)
  })
  const rangeByDef = new Map<object, AffixValueOutput>()
  presentDefs.forEach((def, i) => {
    const v = batch.affixes[i]
    if (v) rangeByDef.set(def, v)
  })

  lines.push(`Rarity: ${RARITY_LABELS[base.rarity]}`)
  lines.push(base.name)
  lines.push(base.baseType)
  lines.push(SEP)

  if (base.itemLevel !== undefined) {
    lines.push(`Item Level: ${base.itemLevel}`)
  }
  if (base.requiresLevel !== undefined) {
    lines.push(`Requires Level: ${base.requiresLevel}`)
  }
  lines.push(`Stars: ${stars}`)
  if (base.defenseMin !== undefined && base.defenseMax !== undefined) {
    lines.push(`Defense: ${base.defenseMin}-${base.defenseMax}`)
  }
  if (base.damageMin !== undefined && base.damageMax !== undefined) {
    lines.push(`Damage: ${base.damageMin}-${base.damageMax}`)
  }
  if (base.attackSpeed !== undefined) {
    lines.push(`Attack Speed: ${base.attackSpeed.toFixed(2)}`)
  }
  if (base.blockChance !== undefined) {
    lines.push(`Block: ${base.blockChance}%`)
  }

  const runeword = runewordEarly
  const scaleImplicit = scaleImplicitEarly
  const implicitEntries = implicitBaseEntries
    .map(
      ([k, v]) =>
        [k, scaleImplicit ? (implicitScaled.get(k) ?? v) : v] as const,
    )
    .filter(([, v]) => !isZero(v))
  const extraImplicits = equipped.implicitOverrides
    ? Object.entries(equipped.implicitOverrides).filter(
        ([k]) => !base.implicit || !(k in base.implicit),
      )
    : []
  if (implicitEntries.length > 0 || extraImplicits.length > 0) {
    lines.push(SEP)
    lines.push('Implicit:')
    for (const [k, v] of implicitEntries) {
      const override = equipped.implicitOverrides?.[k]
      if (override !== undefined) {
        lines.push(`${formatValue(override, k)} ${statName(k)} [custom]`)
      } else {
        lines.push(`${formatValue(v, k)} ${statName(k)}`)
      }
    }
    for (const [k, v] of extraImplicits) {
      lines.push(`${formatValue(v, k)} ${statName(k)} [custom]`)
    }
  }

  lines.push(SEP)
  lines.push('Affixes:')
  for (const eq of equipped.affixes) {
    const affix = getAffix(eq.affixId)
    if (!affix) {
      lines.push(
        `# unknown affix id: ${eq.affixId} [T${eq.tier}, roll ${eq.roll.toFixed(2)}]`,
      )
      continue
    }
    const isUnholy = affix.groupId === 'random_unholy'
    const prefix = isUnholy ? '[Unholy] ' : ''
    if (!affix.statKey) {
      lines.push(
        `${prefix}${affix.description} [T${eq.tier}, roll ${eq.roll.toFixed(2)}]`,
      )
      continue
    }
    const descNoValue = descriptionWithoutValue(affix.description)
    if (eq.customValue !== undefined) {
      const suffix = affix.format === 'percent' ? '%' : ''
      const signChar = eq.customValue >= 0 ? '+' : ''
      const num = Number.isInteger(eq.customValue)
        ? eq.customValue
        : Math.round(eq.customValue * 100) / 100
      lines.push(
        `${prefix}${signChar}${num}${suffix} ${descNoValue} [T${eq.tier}, custom]`,
      )
    } else {
      const range = formatAffixRangeFromValues(
        affix,
        rangeByDef.get(affix) ?? null,
      )
      lines.push(
        `${prefix}${range} ${descNoValue} [T${eq.tier}, roll ${eq.roll.toFixed(2)}]`,
      )
    }
  }

  if (equipped.forgedMods && equipped.forgedMods.length > 0) {
    lines.push(SEP)
    lines.push('Forged Mods:')
    for (const eq of equipped.forgedMods) {
      const mod = getCrystalMod(eq.affixId)
      if (!mod) {
        lines.push(`# unknown crystal mod id: ${eq.affixId} [T${eq.tier}]`)
        continue
      }
      if (!mod.statKey) {
        lines.push(`${mod.description} [T${eq.tier}]`)
        continue
      }
      const descNoValue = descriptionWithoutValue(mod.description)
      if (eq.customValue !== undefined) {
        const suffix = mod.format === 'percent' ? '%' : ''
        const signChar = eq.customValue >= 0 ? '+' : ''
        const num = Number.isInteger(eq.customValue)
          ? eq.customValue
          : Math.round(eq.customValue * 100) / 100
        lines.push(`${signChar}${num}${suffix} ${descNoValue} [T${eq.tier}, custom]`)
      } else {
        const range = formatAffixRangeFromValues(
          mod,
          rangeByDef.get(mod) ?? null,
        )
        lines.push(`${range} ${descNoValue} [T${eq.tier}]`)
      }
    }
  }

  if (equipped.socketCount > 0) {
    lines.push(SEP)
    const socketMap: string[] = []
    for (let i = 0; i < equipped.socketCount; i++) {
      const type = equipped.socketTypes[i] ?? 'normal'
      socketMap.push(type === 'rainbow' ? 'R' : 'N')
    }
    lines.push(`Sockets: ${socketMap.join('-')}`)
    for (let i = 0; i < equipped.socketCount; i++) {
      const filled = equipped.socketed[i]
      if (!filled) continue
      const type = equipped.socketTypes[i] === 'rainbow' ? 'Rainbow' : 'Normal'
      const gem = getGem(filled)
      const rune = getRune(filled)
      if (gem) {
        lines.push(`[${i + 1}] (${type}): ${gem.name}`)
      } else if (rune) {
        lines.push(`[${i + 1}] (${type}): Rune of ${rune.name}`)
      } else {
        lines.push(`[${i + 1}] (${type}): # unknown id ${filled}`)
      }
    }
  }

  if (runeword) {
    lines.push(SEP)
    lines.push(`Runeword: ${runeword.name} (computed, read-only)`)
  }

  if (equipped.augment) {
    const augment = getAugment(equipped.augment.id)
    if (augment) {
      lines.push(SEP)
      lines.push(`Augment: ${augment.name} · Level ${equipped.augment.level}`)
    }
  }

  return lines.join('\n')
}
