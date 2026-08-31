import {
  affixes,
  augments,
  classes,
  crystalMods,
  gems,
  getItem,
  items,
} from '@data'
import { AUGMENT_MAX_LEVEL } from '../../types'
import type { Affix, EquippedAffix, EquippedItem, ItemBase } from '../../types'
import { toPair } from './itemTextShared'
import { statName } from './stats'

export interface TooltipLine {
  text: string
  status: 'matched' | 'ignored' | 'warning'
  detail?: string
}

export interface TooltipParseResult {
  baseId: string | null
  equipped: EquippedItem | null
  lines: TooltipLine[]
  errors: string[]
}

const NAME_MATCH_THRESHOLD = 0.72
const PHRASE_MATCH_THRESHOLD = 0.8
const SKILL_NAME_THRESHOLD = 0.7
const CLASS_NAME_THRESHOLD = 0.55
const NAME_SCAN_LINES = 4

function levenshtein(a: string, b: string): number {
  if (a === b) return 0
  const m = a.length
  const n = b.length
  if (m === 0) return n
  if (n === 0) return m
  let prev = Array.from({ length: n + 1 }, (_, j) => j)
  for (let i = 1; i <= m; i++) {
    const cur = [i, ...new Array<number>(n).fill(0)]
    for (let j = 1; j <= n; j++) {
      const cost = a[i - 1] === b[j - 1] ? 0 : 1
      cur[j] = Math.min(cur[j - 1]! + 1, prev[j]! + 1, prev[j - 1]! + cost)
    }
    prev = cur
  }
  return prev[n]!
}

function similarity(a: string, b: string): number {
  const x = a.toLowerCase()
  const y = b.toLowerCase()
  const max = Math.max(x.length, y.length)
  if (max === 0) return 1
  return 1 - levenshtein(x, y) / max
}

/** Strip every numeric token, range and % so game/OCR phrasing collapses to one key. */
function canonPhrase(s: string): string {
  return s
    .replace(/[[\]|{}()]/g, ' ')
    .replace(/[+-]?\d+(?:\.\d+)?/g, ' ')
    .replace(/%/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .toLowerCase()
}

interface RangeExtract {
  candidates: [number, number][]
  rest: string
}

/** OCR mangles brackets into 1/|/l/I/), so try plausible (min,max) readings. */
function extractTrailingRange(line: string): RangeExtract {
  const m = line.match(/[\s[\]|({]([\d]{1,6})\s*[-–]\s*([\d]{1,6})[\]|)}\s]*$/)
  if (!m) return { candidates: [], rest: line }
  const rest = line.slice(0, m.index).trim()
  const rawA = m[1]!
  const rawB = m[2]!
  const seen = new Set<string>()
  const candidates: [number, number][] = []
  const push = (a: string, b: string) => {
    if (!a || !b) return
    const lo = Number(a)
    const hi = Number(b)
    if (!Number.isFinite(lo) || !Number.isFinite(hi)) return
    if (lo > hi) return
    const key = `${lo}-${hi}`
    if (seen.has(key)) return
    seen.add(key)
    candidates.push([lo, hi])
  }
  push(rawA, rawB)
  if (rawA.startsWith('1')) push(rawA.slice(1), rawB)
  if (rawB.endsWith('1')) push(rawA, rawB.slice(0, -1))
  if (rawA.startsWith('1') && rawB.endsWith('1')) push(rawA.slice(1), rawB.slice(0, -1))
  return { candidates, rest }
}

function extractValue(rest: string): number | null {
  const lead = rest.match(/^([+-]?)(\d+(?:\.\d+)?)/)
  if (lead) {
    const v = Number(lead[2])
    return lead[1] === '-' ? -v : v
  }
  const tail = rest.match(/(\d+(?:\.\d+)?)\s*%?\s*$/)
  if (tail) return Number(tail[1])
  return null
}

function rollFor(value: number, min: number, max: number): number {
  if (max <= min) return 1
  const r = (Math.abs(value) - min) / (max - min)
  return Math.min(1, Math.max(0, Math.round(r * 1000) / 1000))
}

interface AffixGroupIndex {
  canon: string
  entries: Affix[]
}

function buildGroupIndex(pool: Affix[]): AffixGroupIndex[] {
  const byCanon = new Map<string, Affix[]>()
  for (const a of pool) {
    const key = canonPhrase(a.description)
    if (!key) continue
    const list = byCanon.get(key)
    if (list) list.push(a)
    else byCanon.set(key, [a])
  }
  return [...byCanon.entries()].map(([canon, entries]) => ({ canon, entries }))
}

let affixIndex: AffixGroupIndex[] | null = null
let crystalIndex: AffixGroupIndex[] | null = null

function getAffixIndex(): AffixGroupIndex[] {
  affixIndex ??= buildGroupIndex(affixes)
  return affixIndex
}

function getCrystalIndex(): AffixGroupIndex[] {
  crystalIndex ??= buildGroupIndex(crystalMods)
  return crystalIndex
}

function bestGroup(
  index: AffixGroupIndex[],
  phrase: string,
): AffixGroupIndex | null {
  let best: AffixGroupIndex | null = null
  let bestScore = 0
  for (const g of index) {
    const score = similarity(g.canon, phrase)
    if (score > bestScore) {
      bestScore = score
      best = g
    }
  }
  return bestScore >= PHRASE_MATCH_THRESHOLD ? best : null
}

function rangesEqual(a: [number, number], b: [number, number]): boolean {
  return a[0] === b[0] && a[1] === b[1]
}

function findItemName(
  lines: string[],
): { baseId: string; nameEndIndex: number } | null {
  let best: { baseId: string; nameEndIndex: number; score: number } | null = null
  const scan = Math.min(lines.length, NAME_SCAN_LINES)
  for (let i = 0; i < scan; i++) {
    const single = lines[i]!.trim()
    const joined = i + 1 < lines.length ? `${single} ${lines[i + 1]!.trim()}` : null
    const candidates: [string, number][] = [[single, i]]
    if (joined) candidates.push([joined, i + 1])
    for (const [candidate, end] of candidates) {
      for (const item of items) {
        const score = similarity(candidate, item.name)
        if (score >= NAME_MATCH_THRESHOLD && (!best || score > best.score)) {
          best = { baseId: item.id, nameEndIndex: end, score }
        }
      }
    }
  }
  return best ? { baseId: best.baseId, nameEndIndex: best.nameEndIndex } : null
}

const IGNORED_PREFIXES = [
  /^defen/i,
  /^damage\s*[:.]/i,
  /^attack speed/i,
  /^block/i,
  /^\(gem/i,
  /^tier\s/i,
  /requires level/i,
  /^currently has/i,
  /^flask cooldown/i,
  /^effect duration/i,
  /^runeword/i,
]

interface ClassSuffix {
  classId: string | null
  rest: string
}

/** "(JÖTUNN)" survives OCR as e.g. "U?TUNN)" — fuzzy-match a trailing token vs class names. */
function extractClassSuffix(rest: string): ClassSuffix {
  const m = rest.match(/^(.*?)[\s(]+([\p{L}?]{3,20})\)\s*$/u)
  if (!m) return { classId: null, rest }
  let best: string | null = null
  let bestScore = 0
  for (const c of classes) {
    const score = similarity(m[2]!, c.name)
    if (score > bestScore) {
      bestScore = score
      best = c.id
    }
  }
  if (bestScore < CLASS_NAME_THRESHOLD) return { classId: null, rest }
  return { classId: best, rest: m[1]!.trim() }
}

interface StatMatch {
  status: 'matched' | 'warning'
  detail: string
  apply?: (out: ParseAccumulator) => void
  /** Set on fixed-value warning lines that may be socketed-gem contributions. */
  gemCandidate?: { statKeys: string[]; value: number }
}

interface ParseAccumulator {
  implicitOverrides: Record<string, number>
  skillBonusOverrides: Record<string, number>
  forgedMods: EquippedAffix[]
  affixList: EquippedAffix[]
  socketCount: number | null
  augment: { id: string; level: number } | undefined
  allSkillsClassId: string | undefined
}

function matchImplicit(
  base: ItemBase,
  phrase: string,
  value: number | null,
  ranges: [number, number][],
): StatMatch | null {
  const implicit = base.implicit ?? {}
  if (ranges.length > 0) {
    let best: { key: string; score: number } | null = null
    for (const [key, raw] of Object.entries(implicit)) {
      const pair = toPair(raw)
      if (pair[0] === pair[1]) continue
      if (!ranges.some((r) => rangesEqual(r, pair))) continue
      const score = similarity(canonPhrase(statName(key)), phrase)
      if (!best || score > best.score) best = { key, score }
    }
    if (best && value !== null) {
      const key = best.key
      const pinned = Math.abs(value)
      return {
        status: 'matched',
        detail: `implicit ${key} = ${pinned}`,
        apply: (out) => {
          out.implicitOverrides[key] = pinned
        },
      }
    }
    return null
  }
  if (value === null) return null
  let best: { key: string; score: number } | null = null
  for (const key of Object.keys(implicit)) {
    const score = similarity(canonPhrase(statName(key)), phrase)
    if (!best || score > best.score) best = { key, score }
  }
  if (!best || best.score < PHRASE_MATCH_THRESHOLD) return null
  const pair = toPair(implicit[best.key]!)
  const pinned = Math.abs(value)
  if (pair[0] === pair[1] && pair[0] === pinned) {
    return { status: 'matched', detail: `implicit ${best.key} (base value)` }
  }
  const key = best.key
  return {
    status: 'matched',
    detail: `implicit ${key} = ${pinned}`,
    apply: (out) => {
      out.implicitOverrides[key] = pinned
    },
  }
}

function matchSkillBonus(
  base: ItemBase,
  phrase: string,
  value: number | null,
): StatMatch | null {
  if (!base.skillBonuses || value === null) return null
  const target = phrase.replace(/^to\s+/, '')
  let best: { key: string; score: number } | null = null
  for (const key of Object.keys(base.skillBonuses)) {
    const score = similarity(canonPhrase(key), target)
    if (!best || score > best.score) best = { key, score }
  }
  if (!best || best.score < SKILL_NAME_THRESHOLD) return null
  const key = best.key
  const pinned = Math.abs(value)
  return {
    status: 'matched',
    detail: `skill bonus ${key} = ${pinned}`,
    apply: (out) => {
      out.skillBonusOverrides[key] = pinned
    },
  }
}

function matchPool(
  index: AffixGroupIndex[],
  kind: 'affix' | 'forged',
  phrase: string,
  value: number | null,
  ranges: [number, number][],
): StatMatch | null {
  const group = bestGroup(index, phrase)
  if (!group || value === null) return null
  const abs = Math.abs(value)
  const ranged = group.entries.filter(
    (a): a is Affix & { valueMin: number; valueMax: number } =>
      a.valueMin !== null && a.valueMax !== null,
  )
  let tier: (Affix & { valueMin: number; valueMax: number }) | null = null
  for (const a of ranged) {
    if (ranges.some((r) => rangesEqual(r, [a.valueMin, a.valueMax]))) {
      tier = a
      break
    }
  }
  if (!tier) {
    const containing = ranged
      .filter((a) => abs >= a.valueMin && abs <= a.valueMax)
      .sort((a, b) => a.valueMax - a.valueMin - (b.valueMax - b.valueMin))
    tier = containing[0] ?? null
  }
  if (!tier) return null
  const equippedAffix: EquippedAffix = {
    affixId: tier.id,
    tier: tier.tier,
    roll: rollFor(abs, tier.valueMin, tier.valueMax),
  }
  return {
    status: 'matched',
    detail: `${kind} ${tier.id}`,
    apply: (out) => {
      if (kind === 'affix') out.affixList.push(equippedAffix)
      else out.forgedMods.push(equippedAffix)
    },
  }
}

function matchProcLine(base: ItemBase, line: string): StatMatch | null {
  const level = line.match(/level\s*(\d+)\s*$/i)
  if (!level || !/chance\s+(when|on|while)/i.test(line)) return null
  if (!base.skillBonuses) {
    return { status: 'matched', detail: 'proc line (no granted-skill data)' }
  }
  const canonLine = canonPhrase(line)
  let best: { key: string; score: number } | null = null
  for (const key of Object.keys(base.skillBonuses)) {
    const canonKey = canonPhrase(key)
    const score = canonLine.includes(canonKey)
      ? 1
      : bestWindowSimilarity(canonLine, canonKey)
    if (!best || score > best.score) best = { key, score }
  }
  if (!best || best.score < SKILL_NAME_THRESHOLD) {
    return { status: 'matched', detail: 'proc line (granted skill not recognized)' }
  }
  const key = best.key
  const pinned = Number(level[1])
  return {
    status: 'matched',
    detail: `granted skill ${key} level ${pinned}`,
    apply: (out) => {
      out.skillBonusOverrides[key] = pinned
    },
  }
}

/** Best similarity of `needle` against any same-length word window of `haystack`. */
function bestWindowSimilarity(haystack: string, needle: string): number {
  const words = haystack.split(' ')
  const needleWords = needle.split(' ').length
  let best = 0
  for (let i = 0; i + needleWords <= words.length; i++) {
    const window = words.slice(i, i + needleWords).join(' ')
    best = Math.max(best, similarity(window, needle))
  }
  return best
}

function matchAugmentLine(line: string): StatMatch | null {
  const m = line.match(/^augment[:.]?\s*(.+?)[\s[|({l1]*level\s*(\d+)/i)
  if (!m) return null
  const name = m[1]!.trim()
  let best: { id: string; score: number } | null = null
  for (const a of augments) {
    const score = similarity(name, a.name)
    if (!best || score > best.score) best = { id: a.id, score }
  }
  if (!best || best.score < SKILL_NAME_THRESHOLD) {
    return { status: 'warning', detail: `unknown augment "${name}"` }
  }
  const id = best.id
  const level = Math.min(Math.max(Number(m[2]), 1), AUGMENT_MAX_LEVEL)
  return {
    status: 'matched',
    detail: `augment ${id} level ${level}`,
    apply: (out) => {
      out.augment = { id, level }
    },
  }
}

function matchStatLine(base: ItemBase, line: string): StatMatch | null {
  const { candidates, rest: noRange } = extractTrailingRange(line)
  const hasSign = /^[+-]/.test(line.trim())
  if (!hasSign && candidates.length === 0) return null

  const { classId, rest } = extractClassSuffix(noRange)
  const value = extractValue(rest)
  const phrase = canonPhrase(rest)
  if (!phrase) return null

  const match =
    matchImplicit(base, phrase, value, candidates) ??
    matchSkillBonus(base, phrase, value) ??
    matchPool(getAffixIndex(), 'affix', phrase, value, candidates) ??
    matchPool(getCrystalIndex(), 'forged', phrase, value, candidates)

  if (!match) {
    const group =
      bestGroup(getAffixIndex(), phrase) ?? bestGroup(getCrystalIndex(), phrase)
    const statKeys = [
      ...new Set(
        (group?.entries ?? [])
          .map((e) => e.statKey)
          .filter((k): k is string => !!k),
      ),
    ]
    const gemCandidate =
      candidates.length === 0 && value !== null && statKeys.length > 0
        ? { statKeys, value: Math.abs(value) }
        : undefined
    return {
      status: 'warning',
      detail: group
        ? `"${group.canon}" value outside known tiers — socketed gems or unsupported source`
        : 'unrecognized stat line — fix manually after import',
      gemCandidate,
    }
  }
  if (classId && match.status === 'matched') {
    const inner = match.apply
    return {
      ...match,
      apply: (out) => {
        inner?.(out)
        out.allSkillsClassId = classId
      },
    }
  }
  return match
}

interface PendingGemLine {
  lineIndex: number
  statKeys: string[]
  value: number
}

interface GemFill {
  gemIds: string[]
  lineDetails: Map<number, string>
}

/**
 * Leftover fixed stat lines are usually socketed gems rendered into the stat
 * block. Accept only a full explanation: every pending line consumed by a
 * consistent integer count of gems fitting the socket count.
 */
function resolveSocketedGems(
  pending: PendingGemLine[],
  socketCount: number,
): GemFill | null {
  if (pending.length === 0 || socketCount <= 0) return null
  const remaining = [...pending]
  const gemIds: string[] = []
  const lineDetails = new Map<number, string>()
  let slots = socketCount
  const sorted = [...gems].sort(
    (a, b) =>
      Object.keys(b.stats).length - Object.keys(a.stats).length ||
      b.tier - a.tier,
  )
  for (const gem of sorted) {
    const keys = Object.entries(gem.stats).filter(([, v]) => v !== 0)
    if (keys.length === 0) continue
    const picked: { line: PendingGemLine; per: number }[] = []
    for (const [key, per] of keys) {
      const line = remaining.find(
        (l) => l.statKeys.includes(key) && !picked.some((p) => p.line === l),
      )
      if (!line) break
      picked.push({ line, per })
    }
    if (picked.length !== keys.length) continue
    const counts = picked.map((p) => p.line.value / p.per)
    const count = counts[0]!
    if (!Number.isInteger(count) || count < 1 || count > slots) continue
    if (!counts.every((c) => c === count)) continue
    slots -= count
    for (let i = 0; i < count; i++) gemIds.push(gem.id)
    for (const p of picked) {
      lineDetails.set(p.line.lineIndex, `socketed gems: ${count}× ${gem.name}`)
      remaining.splice(remaining.indexOf(p.line), 1)
    }
  }
  return remaining.length === 0 ? { gemIds, lineDetails } : null
}

export function parseTooltipLines(rawLines: string[]): TooltipParseResult {
  const lines = rawLines.map((l) => l.replace(/\s+/g, ' ').trim()).filter((l) => l)
  const out: TooltipLine[] = []
  const errors: string[] = []

  const name = findItemName(lines)
  if (!name) {
    errors.push('No item name recognized — crop the screenshot to the tooltip')
    return {
      baseId: null,
      equipped: null,
      lines: lines.map((text) => ({ text, status: 'ignored' as const })),
      errors,
    }
  }
  const base = getItem(name.baseId)
  if (!base) {
    errors.push(`Unknown base item id: ${name.baseId}`)
    return { baseId: null, equipped: null, lines: [], errors }
  }

  const acc: ParseAccumulator = {
    implicitOverrides: {},
    skillBonusOverrides: {},
    forgedMods: [],
    affixList: [],
    socketCount: null,
    augment: undefined,
    allSkillsClassId: undefined,
  }
  const pendingGemLines: PendingGemLine[] = []

  lines.forEach((line, idx) => {
    if (idx <= name.nameEndIndex) {
      out.push({ text: line, status: 'matched', detail: `item: ${base.name}` })
      return
    }
    if (IGNORED_PREFIXES.some((re) => re.test(line))) {
      out.push({ text: line, status: 'ignored' })
      return
    }
    const sockets = line.match(/^sockets?\s*\(?(\d+)\)?/i)
    if (sockets) {
      acc.socketCount = Number(sockets[1])
      out.push({ text: line, status: 'matched', detail: `sockets: ${acc.socketCount}` })
      return
    }
    const special = matchAugmentLine(line) ?? matchProcLine(base, line)
    if (special) {
      special.apply?.(acc)
      out.push({ text: line, status: special.status, detail: special.detail })
      return
    }
    const stat = matchStatLine(base, line)
    if (stat) {
      stat.apply?.(acc)
      if (stat.gemCandidate) {
        pendingGemLines.push({ lineIndex: out.length, ...stat.gemCandidate })
      }
      out.push({ text: line, status: stat.status, detail: stat.detail })
      return
    }
    out.push({ text: line, status: 'ignored' })
  })

  const socketCount = acc.socketCount ?? base.sockets ?? 0
  const gemFill = resolveSocketedGems(pendingGemLines, socketCount)
  if (gemFill) {
    for (const [lineIndex, detail] of gemFill.lineDetails) {
      const prev = out[lineIndex]!
      out[lineIndex] = { text: prev.text, status: 'matched', detail }
    }
  }
  const socketed = new Array<string | null>(socketCount).fill(null)
  gemFill?.gemIds.forEach((id, i) => {
    if (i < socketCount) socketed[i] = id
  })
  const equipped: EquippedItem = {
    baseId: base.id,
    affixes: acc.affixList,
    socketCount,
    socketed,
    socketTypes: new Array(socketCount).fill('normal'),
    stars: 0,
    forgedMods: acc.forgedMods.length > 0 ? acc.forgedMods : undefined,
    augment: acc.augment,
    implicitOverrides:
      Object.keys(acc.implicitOverrides).length > 0
        ? acc.implicitOverrides
        : undefined,
    skillBonusOverrides:
      Object.keys(acc.skillBonusOverrides).length > 0
        ? acc.skillBonusOverrides
        : undefined,
    allSkillsClassId: acc.allSkillsClassId,
  }

  return { baseId: base.id, equipped, lines: out, errors }
}
