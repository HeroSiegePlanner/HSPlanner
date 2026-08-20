import { useEffect, useMemo, useRef, useState } from 'react'
import { gameConfig } from '@data'
import { useBuild } from '../../store/build'
import { useCalcResult } from '../../hooks/useCalcResult'
import { parseCustomStatsNative } from '../../utils/calc/bridge'
import {
  dedupeStatDefsByKey,
  formatValue,
  statDef,
  statName,
} from '../../utils/item/stats'
import type { CustomStat } from '../../types'
import { CountBadge, Panel } from './configPrimitives'

const PARSE_DEBOUNCE_MS = 200
const MIN_ROWS = 4
const MAX_ROWS = 12

const LINE_RE = /^([+-]?\d*\.?\d+(?:\s*-\s*[+-]?\d*\.?\d+)?\s*%?)\s+(.+)$/

interface LineIssue {
  line: number
  message: string
}

interface ParsedText {
  stats: CustomStat[]
  issues: LineIssue[]
}

function buildNameToKey(): Map<string, string> {
  const defs = dedupeStatDefsByKey(
    gameConfig.stats.filter((s) => !s.itemOnly && !s.skillScoped),
  )
  const m = new Map<string, string>()
  for (const s of defs) {
    m.set(s.name.toLowerCase(), s.key)
    m.set(s.key.toLowerCase(), s.key)
  }
  return m
}

function serializeStats(stats: CustomStat[]): string {
  return stats
    .map((cs) => `${cs.value} ${statName(cs.statKey)}`.trim())
    .join('\n')
}

function parseText(text: string, nameToKey: Map<string, string>): ParsedText {
  const stats: CustomStat[] = []
  const issues: LineIssue[] = []
  text.split('\n').forEach((raw, idx) => {
    const line = raw.trim()
    if (!line) return
    const m = line.match(LINE_RE)
    if (!m) {
      issues.push({
        line: idx + 1,
        message:
          'Expected "<value> <stat name>", e.g. "100% Faster Cast Rate"',
      })
      return
    }
    const statKey = nameToKey.get(m[2]!.trim().toLowerCase())
    if (statKey === undefined) {
      issues.push({ line: idx + 1, message: `Unknown stat "${m[2]!.trim()}"` })
      return
    }
    stats.push({ statKey, value: m[1]!.replace(/\s+/g, '') })
  })
  return { stats, issues }
}

export default function CustomStatsPanel() {
  const customStats = useBuild((s) => s.customStats)
  const setCustomStats = useBuild((s) => s.setCustomStats)
  const nameToKey = useMemo(() => buildNameToKey(), [])

  const [text, setText] = useState(() => serializeStats(customStats))
  const [issues, setIssues] = useState<LineIssue[]>([])
  const lastSavedRef = useRef(JSON.stringify(customStats))

  useEffect(() => {
    const json = JSON.stringify(customStats)
    if (json === lastSavedRef.current) return
    lastSavedRef.current = json
    setText(serializeStats(customStats))
    setIssues([])
  }, [customStats])

  useEffect(() => {
    const handle = window.setTimeout(() => {
      const parsed = parseText(text, nameToKey)
      setIssues(parsed.issues)
      const json = JSON.stringify(parsed.stats)
      if (json === lastSavedRef.current) return
      lastSavedRef.current = json
      setCustomStats(parsed.stats)
    }, PARSE_DEBOUNCE_MS)
    return () => window.clearTimeout(handle)
  }, [text, nameToKey, setCustomStats])

  const parsedValues = useCalcResult<([number, number] | null)[]>(
    () =>
      customStats.length === 0
        ? []
        : parseCustomStatsNative(customStats.map((cs) => cs.value)),
    [customStats],
    [],
  )

  const rows = Math.min(
    MAX_ROWS,
    Math.max(MIN_ROWS, text.split('\n').length + 1),
  )

  return (
    <Panel
      title="Custom Config"
      subtitle="Add stats the engine doesn't compute yet — one per line: value, then stat name. They stack with regular sources and show up in tooltips. Per-profile."
      trailing={
        <CountBadge
          value={customStats.length}
          highlight={customStats.length > 0}
        />
      }
    >
      <textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        spellCheck={false}
        rows={rows}
        placeholder={'100% Faster Cast Rate\n+50 Life\n12-18 Cold Resistance'}
        className="w-full resize-y rounded-[3px] border border-border-2 px-3 py-2 font-mono text-[12px] leading-[1.55] text-text outline-none transition-colors focus:border-accent-hot"
        style={{
          background: 'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
          boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
          tabSize: 2,
        }}
      />

      {issues.length > 0 && (
        <ul className="mt-2 space-y-1">
          {issues.map((issue) => (
            <li
              key={issue.line}
              className="rounded-[3px] border border-stat-orange/40 bg-stat-orange/10 px-2.5 py-1.5 font-mono text-[10px] tracking-[0.04em] text-stat-orange"
            >
              line {issue.line} · {issue.message}
            </li>
          ))}
        </ul>
      )}

      {customStats.length > 0 && (
        <ul className="mt-2 space-y-0.5">
          {customStats.map((cs, i) => {
            const pair = parsedValues[i] ?? null
            const parsed =
              pair === null ? null : pair[0] === pair[1] ? pair[0] : pair
            const applies = parsed !== null
            return (
              <li
                key={`${cs.statKey}-${i}`}
                className={`font-mono text-[10px] uppercase tracking-[0.14em] ${
                  applies ? 'text-stat-green' : 'text-stat-orange'
                }`}
              >
                {applies
                  ? `→ ${formatValue(parsed, cs.statKey)} ${statDef(cs.statKey)?.name ?? cs.statKey}`
                  : `→ ${statDef(cs.statKey)?.name ?? cs.statKey}: unparseable value "${cs.value}"`}
              </li>
            )
          })}
        </ul>
      )}
    </Panel>
  )
}
