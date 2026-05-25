import {
  affixes,
  augments,
  crystalMods,
  detectRuneword,
  gameConfig,
  gems,
  getAffix,
  getAugment,
  getCrystalMod,
  getGem,
  getItem,
  getRune,
  runes,
} from '../data'
import { MAX_STARS } from '../store/build'
import { AUGMENT_MAX_LEVEL } from '../types'
import type {
  Affix,
  EquippedAffix,
  EquippedItem,
  ItemBase,
  ItemRarity,
  SocketType,
} from '../types'
import {
  applyStarsToRangedValue,
  formatAffixRange,
  formatValue,
  isZero,
  rolledAffixValueWithStars,
  shouldScaleImplicit,
  statName,
} from './stats'

export interface ParseError {
  line: number
  message: string
  severity: 'error' | 'warning'
}

export interface ParseResult {
  equipped: EquippedItem | null
  errors: ParseError[]
}

const RARITY_LABELS: Record<ItemRarity, string> = {
  common: 'COMMON',
  uncommon: 'UNCOMMON',
  rare: 'RARE',
  mythic: 'MYTHIC',
  satanic: 'SATANIC',
  heroic: 'HEROIC',
  angelic: 'ANGELIC',
  satanic_set: 'SATANIC_SET',
  unholy: 'UNHOLY',
  relic: 'RELIC',
}

const SEP = '--------'

// Strips ONLY a leading numeric prefix — leaving value-less affixes ("Cannot Be Frozen") untouched so the fallback matcher does not collapse them to "" and false-positive.
function descriptionWithoutValue(description: string): string {
  return description
    .replace(/^[+-]?(?:\[[^\]]*]|[0-9][0-9.]*)%?\s*/, '')
    .replace(/\s+/g, ' ')
    .trim()
}

function normalizeWhitespace(s: string): string {
  return s.replace(/\s+/g, ' ').trim()
}

export function serializeEquippedItem(
  equipped: EquippedItem,
  base: ItemBase,
): string {
  const lines: string[] = []
  const stars = equipped.stars ?? 0

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

  const runeword = detectRuneword(base, equipped.socketed)
  const scaleImplicit = shouldScaleImplicit(!!runeword)
  const implicitEntries = base.implicit
    ? Object.entries(base.implicit)
        .map(
          ([k, v]) =>
            [k, scaleImplicit ? applyStarsToRangedValue(v, k, stars) : v] as const,
        )
        .filter(([, v]) => !isZero(v))
    : []
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
      const range = formatAffixRange(affix, stars)
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
        const range = formatAffixRange(mod, stars)
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

interface AffixLineResult {
  equipped: EquippedAffix | null
  errors: ParseError[]
}

function parseAffixLine(
  line: string,
  lineNum: number,
  source: 'affix' | 'crystal',
  stars: number,
): AffixLineResult {
  const errors: ParseError[] = []
  let work = line.trim()

  let wantUnholy = false
  if (work.startsWith('[Unholy]')) {
    wantUnholy = true
    work = work.slice('[Unholy]'.length).trim()
  }

  const tierMatch = work.match(
    /\s*\[T(\d+)(?:,\s*(roll|custom)(?:\s+([+-]?[0-9]*\.?[0-9]+))?)?]\s*$/i,
  )
  if (!tierMatch) {
    errors.push({
      line: lineNum,
      message: `Missing [T<tier>(, roll <r> | custom)?] suffix on line: "${line}"`,
      severity: 'error',
    })
    return { equipped: null, errors }
  }
  const tier = Number(tierMatch[1])
  const modeKeyword = tierMatch[2]?.toLowerCase()
  const numRaw = tierMatch[3]
  const explicitCustom = modeKeyword === 'custom'

  let roll = 1.0
  if (modeKeyword === 'roll' && numRaw !== undefined) {
    roll = Number(numRaw)
    if (!Number.isFinite(roll) || roll < 0 || roll > 1) {
      errors.push({
        line: lineNum,
        message: `Roll must be between 0 and 1 (got ${numRaw})`,
        severity: 'error',
      })
      return { equipped: null, errors }
    }
  }

  const content = work.slice(0, tierMatch.index).trim()
  const pool: Affix[] = source === 'affix' ? affixes : crystalMods

  // wantUnholy=true requires random_unholy; false rejects it.
  const candidates = pool.filter(
    (a) =>
      a.tier === tier && (a.groupId === 'random_unholy') === wantUnholy,
  )

  const normContent = normalizeWhitespace(content)
  let matches = candidates.filter(
    (a) => normalizeWhitespace(a.description) === normContent,
  )
  let matchedByFallback = false

  if (matches.length === 0) {
    matchedByFallback = true
    const statTextOnly = descriptionWithoutValue(content)
    matches = candidates.filter(
      (a) =>
        normalizeWhitespace(descriptionWithoutValue(a.description)) === statTextOnly,
    )
  }

  if (matches.length === 0) {
    matches = candidates.filter(
      (a) => normalizeWhitespace(a.name).toLowerCase() === normContent.toLowerCase(),
    )
  }

  if (matches.length === 0) {
    const kindLabel = source === 'affix' ? 'affix' : 'crystal mod'
    const unholyHint = wantUnholy ? ' (Unholy)' : ''
    errors.push({
      line: lineNum,
      message: `Unknown ${kindLabel}${unholyHint}: "${content}" [T${tier}]`,
      severity: 'error',
    })
    return { equipped: null, errors }
  }

  const matched = matches[0]!
  if (matches.length > 1) {
    errors.push({
      line: lineNum,
      message: `Ambiguous match for "${content}" — using first (${matched.id})`,
      severity: 'warning',
    })
  }

  let customValue: number | undefined = undefined
  if (matched.statKey) {
    const userValue = parseValuePrefix(content)
    if (explicitCustom) {
      if (userValue === null) {
        errors.push({
          line: lineNum,
          message: `[T${tier}, custom] requires a numeric value at the start of the line (e.g. "+10 ${descriptionWithoutValue(matched.description)} [T${tier}, custom]")`,
          severity: 'error',
        })
        return { equipped: null, errors }
      }
      customValue = userValue
    } else if (matchedByFallback && userValue !== null) {
      const computed = rolledAffixValueWithStars(matched, roll, stars)
      const epsilon = 0.005
      if (Math.abs(userValue - computed) > epsilon) {
        customValue = userValue
      }
    }
  }

  return {
    equipped:
      customValue !== undefined
        ? { affixId: matched.id, tier, roll, customValue }
        : { affixId: matched.id, tier, roll },
    errors,
  }
}

function parseValuePrefix(content: string): number | null {
  const m = content.match(/^([+-]?)\s*(?:\[\s*([+-]?[0-9.]+)\s*-\s*([+-]?[0-9.]+)\s*]|([+-]?[0-9.]+))\s*%?/)
  if (!m) return null
  const signChar = m[1]
  if (m[2] !== undefined && m[3] !== undefined) {
    const hi = Number(m[3])
    if (!Number.isFinite(hi)) return null
    return signChar === '-' ? -Math.abs(hi) : Math.abs(hi)
  }
  if (m[4] === undefined) return null
  const v = Number(m[4])
  if (!Number.isFinite(v)) return null
  if (signChar === '-') return -Math.abs(v)
  if (signChar === '+') return Math.abs(v)
  return v
}

// True for leading [min-max] ranges — used to detect untouched implicit lines and skip overriding.
function valueLooksLikeRange(content: string): boolean {
  return /^[+-]?\s*\[\s*[+-]?[0-9.]+\s*-\s*[+-]?[0-9.]+\s*]/.test(content)
}

const STAT_NAME_TO_KEY: Map<string, string> = (() => {
  const m = new Map<string, string>()
  for (const s of gameConfig.stats) {
    m.set(s.name.toLowerCase(), s.key)
  }
  return m
})()

// Falls back to the global stat registry so users can ADD a new implicit not on the base item.
function lookupImplicitKey(base: ItemBase, displayName: string): string | null {
  const norm = normalizeWhitespace(displayName).toLowerCase()
  if (base.implicit) {
    for (const k of Object.keys(base.implicit)) {
      if (statName(k).toLowerCase() === norm) return k
    }
  }
  return STAT_NAME_TO_KEY.get(norm) ?? null
}

export function parseItemText(text: string, baseItemId: string): ParseResult {
  const errors: ParseError[] = []
  const base = getItem(baseItemId)
  if (!base) {
    errors.push({
      line: 0,
      message: `Unknown base item id: ${baseItemId}`,
      severity: 'error',
    })
    return { equipped: null, errors }
  }

  const rawLines = text.split(/\r?\n/)

  type SectionLine = { text: string; lineNum: number }
  const sections: SectionLine[][] = []
  let current: SectionLine[] = []

  rawLines.forEach((rawLine, idx) => {
    const lineNum = idx + 1
    const line = rawLine.trim()
    if (/^-{4,}$/.test(line)) {
      sections.push(current)
      current = []
      return
    }
    if (!line || line.startsWith('#')) return
    current.push({ text: line, lineNum })
  })
  sections.push(current)

  let stars = 0
  for (const sec of sections) {
    for (const { text: line } of sec) {
      const m = line.match(/^Stars:\s*(\d+)\s*$/)
      if (m) {
        const s = Number(m[1])
        if (Number.isFinite(s) && s >= 0 && s <= MAX_STARS) {
          stars = s
        }
      }
    }
  }

  const newAffixes: EquippedAffix[] = []
  const newForgedMods: EquippedAffix[] = []
  let socketCount = 0
  let socketed: (string | null)[] = []
  let socketTypes: SocketType[] = []
  let augment: { id: string; level: number } | undefined = undefined
  const implicitOverrides: Record<string, number> = {}

  for (const sec of sections) {
    if (sec.length === 0) continue
    const head = sec[0]!
    const firstLine = head.text

    if (firstLine.startsWith('Rarity:')) {
      const expectedRarity = RARITY_LABELS[base.rarity]
      const actualRarity = firstLine.slice('Rarity:'.length).trim().toUpperCase()
      if (actualRarity !== expectedRarity) {
        errors.push({
          line: head.lineNum,
          message: `Rarity is read-only (expected ${expectedRarity}, got ${actualRarity})`,
          severity: 'warning',
        })
      }
      continue
    }

    if (
      firstLine.startsWith('Item Level:') ||
      firstLine.startsWith('Stars:') ||
      firstLine.startsWith('Requires Level:') ||
      firstLine.startsWith('Defense:') ||
      firstLine.startsWith('Damage:') ||
      firstLine.startsWith('Attack Speed:') ||
      firstLine.startsWith('Block:')
    ) {
      for (const { text: line, lineNum } of sec) {
        const starsMatch = line.match(/^Stars:\s*(\d+)\s*$/)
        if (starsMatch) {
          const s = Number(starsMatch[1])
          if (!Number.isFinite(s) || s < 0 || s > MAX_STARS) {
            errors.push({
              line: lineNum,
              message: `Stars must be 0..${MAX_STARS} (got ${starsMatch[1]})`,
              severity: 'error',
            })
          }
        }
      }
      continue
    }

    if (firstLine === 'Implicit:') {
      const runewordHere = detectRuneword(base, [])
      const scaleImplicitHere = shouldScaleImplicit(!!runewordHere)
      for (let i = 1; i < sec.length; i++) {
        const { text, lineNum } = sec[i]!
        const explicitCustom = /\[custom]\s*$/i.test(text)
        const body = text.replace(/\[custom]\s*$/i, '').trim()
        if (!body) continue
        if (!explicitCustom && valueLooksLikeRange(body)) continue

        const userValue = parseValuePrefix(body)
        if (userValue === null) {
          errors.push({
            line: lineNum,
            message: `Implicit line missing numeric value: "${text}"`,
            severity: 'error',
          })
          continue
        }
        const statText = descriptionWithoutValue(body)
        if (!statText) continue

        const key = lookupImplicitKey(base, statText)
        if (!key) {
          errors.push({
            line: lineNum,
            message: `Unknown implicit stat: "${statText}"`,
            severity: 'warning',
          })
          continue
        }

        if (!explicitCustom) {
          const baseValue = base.implicit?.[key]
          if (baseValue !== undefined) {
            const computed = scaleImplicitHere
              ? applyStarsToRangedValue(baseValue, key, stars)
              : baseValue
            if (typeof computed === 'number') {
              const epsilon = 0.005
              if (Math.abs(userValue - computed) <= epsilon) continue
            }
          }
        }

        implicitOverrides[key] = userValue
      }
      continue
    }

    if (firstLine === 'Affixes:') {
      for (let i = 1; i < sec.length; i++) {
        const { text, lineNum } = sec[i]!
        const result = parseAffixLine(text, lineNum, 'affix', stars)
        errors.push(...result.errors)
        if (result.equipped) newAffixes.push(result.equipped)
      }
      continue
    }

    if (firstLine === 'Forged Mods:') {
      for (let i = 1; i < sec.length; i++) {
        const { text, lineNum } = sec[i]!
        const result = parseAffixLine(text, lineNum, 'crystal', stars)
        errors.push(...result.errors)
        if (result.equipped) newForgedMods.push(result.equipped)
      }
      continue
    }

    if (firstLine.startsWith('Sockets:')) {
      const mapText = head.text
        .slice('Sockets:'.length)
        .trim()
        .replace(/\s+/g, '')
      if (!mapText) {
        socketCount = 0
      } else {
        const slots = mapText.split('-')
        if (slots.some((s) => !/^[NR_]$/.test(s))) {
          errors.push({
            line: head.lineNum,
            message: `Invalid socket map "${mapText}" — expected dash-separated N/R/_`,
            severity: 'error',
          })
          continue
        }
        socketCount = slots.length
        socketed = new Array(socketCount).fill(null)
        socketTypes = slots.map((s) => (s === 'R' ? 'rainbow' : 'normal'))
      }

      for (let i = 1; i < sec.length; i++) {
        const { text, lineNum } = sec[i]!
        const m = text.match(/^\[(\d+)]\s*\((Normal|Rainbow)\):\s*(.+)$/i)
        if (!m) {
          errors.push({
            line: lineNum,
            message: `Invalid socketed line: "${text}"`,
            severity: 'error',
          })
          continue
        }
        const idx = Number(m[1]) - 1
        const typeLabel: SocketType =
          m[2]!.toLowerCase() === 'rainbow' ? 'rainbow' : 'normal'
        const nameRaw = m[3]!.trim()
        if (idx < 0 || idx >= socketCount) {
          errors.push({
            line: lineNum,
            message: `Socket index ${idx + 1} out of range (1..${socketCount})`,
            severity: 'error',
          })
          continue
        }
        socketTypes[idx] = typeLabel
        if (/^Rune of /i.test(nameRaw)) {
          const runeName = nameRaw.replace(/^Rune of /i, '').trim()
          const rune = runes.find(
            (r) => r.name.toLowerCase() === runeName.toLowerCase(),
          )
          if (!rune) {
            errors.push({
              line: lineNum,
              message: `Unknown rune: "${runeName}"`,
              severity: 'error',
            })
            continue
          }
          socketed[idx] = rune.id
        } else {
          const gem = gems.find(
            (g) => g.name.toLowerCase() === nameRaw.toLowerCase(),
          )
          if (!gem) {
            errors.push({
              line: lineNum,
              message: `Unknown gem: "${nameRaw}"`,
              severity: 'error',
            })
            continue
          }
          socketed[idx] = gem.id
        }
      }
      continue
    }

    if (firstLine.startsWith('Runeword:')) continue

    if (firstLine.startsWith('Augment:')) {
      const m = firstLine.match(/^Augment:\s*(.+?)\s*·\s*Level\s*(\d+)\s*$/)
      if (!m) {
        errors.push({
          line: head.lineNum,
          message: `Invalid augment line: "${firstLine}" — expected "Augment: <name> · Level <n>"`,
          severity: 'error',
        })
        continue
      }
      const name = m[1]!.trim()
      const level = Number(m[2])
      const aug = augments.find((a) => a.name.toLowerCase() === name.toLowerCase())
      if (!aug) {
        errors.push({
          line: head.lineNum,
          message: `Unknown augment: "${name}"`,
          severity: 'error',
        })
        continue
      }
      if (level < 1 || level > AUGMENT_MAX_LEVEL) {
        errors.push({
          line: head.lineNum,
          message: `Augment level must be 1..${AUGMENT_MAX_LEVEL} (got ${level})`,
          severity: 'error',
        })
        continue
      }
      augment = { id: aug.id, level }
      continue
    }

    errors.push({
      line: head.lineNum,
      message: `Unknown section starting with: "${firstLine}"`,
      severity: 'warning',
    })
  }

  const hasErrors = errors.some((e) => e.severity === 'error')
  if (hasErrors) {
    return { equipped: null, errors }
  }

  const equipped: EquippedItem = {
    baseId: baseItemId,
    affixes: newAffixes,
    socketCount,
    socketed,
    socketTypes,
    stars,
    forgedMods: newForgedMods.length > 0 ? newForgedMods : undefined,
    augment,
    implicitOverrides:
      Object.keys(implicitOverrides).length > 0 ? implicitOverrides : undefined,
  }

  return { equipped, errors }
}
