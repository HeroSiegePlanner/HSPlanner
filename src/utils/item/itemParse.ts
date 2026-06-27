import {
  affixes,
  augments,
  crystalMods,
  detectRuneword,
  gameConfig,
  gems,
  getItem,
  runes,
} from '../../data'
import { MAX_STARS } from '../../store/build'
import { AUGMENT_MAX_LEVEL } from '../../types'
import type {
  Affix,
  EquippedAffix,
  EquippedItem,
  ItemBase,
  SocketType,
} from '../../types'
import { shouldScaleImplicit, statName } from './stats'
import {
  RARITY_LABELS,
  descriptionWithoutValue,
  nativeAffixMath,
  normalizeWhitespace,
  toPair,
} from './itemTextShared'
import type { AffixMathProvider } from './itemTextShared'

export interface ParseError {
  line: number
  message: string
  severity: 'error' | 'warning'
}

export interface ParseResult {
  equipped: EquippedItem | null
  errors: ParseError[]
}

interface AffixLineResult {
  equipped: EquippedAffix | null
  errors: ParseError[]
  customCheck?: { affix: Affix; roll: number; userValue: number }
}

function parseAffixLine(
  line: string,
  lineNum: number,
  source: 'affix' | 'crystal',
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
  let customCheck: AffixLineResult['customCheck']
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
      customCheck = { affix: matched, roll, userValue }
    }
  }

  return {
    equipped:
      customValue !== undefined
        ? { affixId: matched.id, tier, roll, customValue }
        : { affixId: matched.id, tier, roll },
    errors,
    customCheck,
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

function lookupImplicitKey(base: ItemBase, displayName: string): string | null {
  const norm = normalizeWhitespace(displayName).toLowerCase()
  if (base.implicit) {
    for (const k of Object.keys(base.implicit)) {
      if (statName(k).toLowerCase() === norm) return k
    }
  }
  return STAT_NAME_TO_KEY.get(norm) ?? null
}

export async function parseItemText(
  text: string,
  baseItemId: string,
  math: AffixMathProvider = nativeAffixMath,
): Promise<ParseResult> {
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
  const affixCustomChecks: {
    list: EquippedAffix[]
    index: number
    check: { affix: Affix; roll: number; userValue: number }
    checkStars: number
  }[] = []
  const implicitPendingChecks: {
    key: string
    value: [number, number]
    userValue: number
    checkStars: number
  }[] = []

  const socketedForRuneword: (string | null)[] = (() => {
    const sec = sections.find((s) => s[0]?.text.startsWith('Sockets:'))
    if (!sec) return []
    const mapText = sec[0]!.text
      .slice('Sockets:'.length)
      .trim()
      .replace(/\s+/g, '')
    if (!mapText) return []
    const out: (string | null)[] = new Array(mapText.split('-').length).fill(
      null,
    )
    for (let i = 1; i < sec.length; i++) {
      const m = sec[i]!.text.match(
        /^\[(\d+)]\s*\((?:Normal|Rainbow)\):\s*(.+)$/i,
      )
      if (!m) continue
      const idx = Number(m[1]) - 1
      if (idx < 0 || idx >= out.length) continue
      const nameRaw = m[2]!.trim()
      if (/^Rune of /i.test(nameRaw)) {
        const runeName = nameRaw.replace(/^Rune of /i, '').trim()
        const rune = runes.find(
          (r) => r.name.toLowerCase() === runeName.toLowerCase(),
        )
        if (rune) out[idx] = rune.id
      } else {
        const gem = gems.find(
          (g) => g.name.toLowerCase() === nameRaw.toLowerCase(),
        )
        if (gem) out[idx] = gem.id
      }
    }
    return out
  })()

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
      const runewordHere = detectRuneword(base, socketedForRuneword)
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
            if (scaleImplicitHere) {
              implicitPendingChecks.push({
                key,
                value: toPair(baseValue),
                userValue,
                checkStars: stars,
              })
              continue
            }
            if (typeof baseValue === 'number') {
              const epsilon = 0.005
              if (Math.abs(userValue - baseValue) <= epsilon) continue
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
        const result = parseAffixLine(text, lineNum, 'affix')
        errors.push(...result.errors)
        if (result.equipped) {
          if (result.customCheck) {
            affixCustomChecks.push({
              list: newAffixes,
              index: newAffixes.length,
              check: result.customCheck,
              checkStars: stars,
            })
          }
          newAffixes.push(result.equipped)
        }
      }
      continue
    }

    if (firstLine === 'Forged Mods:') {
      for (let i = 1; i < sec.length; i++) {
        const { text, lineNum } = sec[i]!
        const result = parseAffixLine(text, lineNum, 'crystal')
        errors.push(...result.errors)
        if (result.equipped) {
          if (result.customCheck) {
            affixCustomChecks.push({
              list: newForgedMods,
              index: newForgedMods.length,
              check: result.customCheck,
              checkStars: stars,
            })
          }
          newForgedMods.push(result.equipped)
        }
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

  if (affixCustomChecks.length > 0 || implicitPendingChecks.length > 0) {
    const res = await math.batch({
      affixes: affixCustomChecks.map((c) => ({
        affix: c.check.affix,
        roll: c.check.roll,
        stars: c.checkStars,
      })),
      scaled: implicitPendingChecks.map((c) => ({
        value: c.value,
        statKey: c.key,
        stars: c.checkStars,
      })),
    })
    const epsilon = 0.005
    affixCustomChecks.forEach((c, i) => {
      const computed = res.affixes[i]?.value
      if (computed === undefined) return
      if (Math.abs(c.check.userValue - computed) > epsilon) {
        const target = c.list[c.index]
        if (target) {
          c.list[c.index] = { ...target, customValue: c.check.userValue }
        }
      }
    })
    implicitPendingChecks.forEach((c, i) => {
      const [mn, mx] = res.scaled[i] ?? c.value
      if (mn === mx && Math.abs(c.userValue - mn) <= epsilon) return
      implicitOverrides[c.key] = c.userValue
    })
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
