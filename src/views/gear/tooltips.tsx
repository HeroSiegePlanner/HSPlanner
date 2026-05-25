/* eslint-disable react-refresh/only-export-components */
import type { ReactNode } from 'react'
import { TooltipFooter, TooltipHeader, TooltipSection, TooltipStat, TooltipText } from '../../components/Tooltip'
import { formatValue, statName } from '../../utils/stats'
import type { Affix } from '../../types'
import { socketableIconForName } from './lib/icons'

export function NetChangeBlock({
  previous,
  next,
}: {
  previous: Record<string, number>
  next: Record<string, number>
}) {
  const allKeys = new Set([
    ...Object.keys(previous),
    ...Object.keys(next),
  ])
  const diffs: Array<{ key: string; diff: number }> = []
  for (const key of allKeys) {
    const d = (next[key] ?? 0) - (previous[key] ?? 0)
    if (Math.abs(d) < 0.01) continue
    diffs.push({ key, diff: d })
  }
  diffs.sort((a, b) => b.diff - a.diff)
  if (diffs.length === 0) {
    return (
      <TooltipSection>
        <div className="mb-1 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
          Net Change
        </div>
        <p className="text-[11px] text-faint italic">No stat changes</p>
      </TooltipSection>
    )
  }
  return (
    <TooltipSection>
      <div className="mb-1 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
        Net Change
      </div>
      <ul className="space-y-0.5 text-[12px]">
        {diffs.map(({ key, diff }) => {
          const up = diff > 0
          return (
            <li
              key={key}
              className="flex items-baseline justify-between gap-2"
            >
              <span className="min-w-0 wrap-break-words leading-tight text-muted">
                {statName(key)}
              </span>
              <span
                className={`shrink-0 whitespace-nowrap font-mono tabular-nums ${
                  up ? 'text-stat-green' : 'text-stat-red'
                }`}
              >
                {up ? '▲' : '▼'} {formatValue(diff, key)}
              </span>
            </li>
          )
        })}
      </ul>
    </TooltipSection>
  )
}

function formatStatLines(stats: Record<string, number>): ReactNode[] {
  return Object.entries(stats)
    .filter(([, v]) => v !== 0)
    .map(([key, val]) => (
      <TooltipStat
        key={key}
        label={statName(key)}
        value={formatValue(val, key)}
      />
    ))
}

// Filter out legacy "Tier S/A/B/…" grade tags so they don't pollute the tooltip body.
function isLegacyTierTag(s?: string): boolean {
  if (!s) return false
  return /^\s*tier\s+[a-z]+\s*$/i.test(s)
}

export function buildSocketableTooltip(
  s: { id: string; name: string; tier: number; stats: Record<string, number>; description?: string },
  kind: 'GEM' | 'JEWEL' | 'RUNE',
  opts?: { previousStats?: Record<string, number>; multiplier?: number },
): ReactNode {
  const mult = opts?.multiplier ?? 1
  const scaled: Record<string, number> = {}
  for (const [k, v] of Object.entries(s.stats)) scaled[k] = v * mult
  const lines = formatStatLines(scaled)
  const hasDescription = s.description && !isLegacyTierTag(s.description)
  return (
    <>
      <TooltipHeader
        title={s.name}
        subtitle={`${kind} · Tier ${s.tier}`}
        image={socketableIconForName(s.name)}
      />
      {lines.length > 0 && <TooltipSection>{lines}</TooltipSection>}
      {hasDescription && (
        <TooltipSection>
          <TooltipText>{s.description}</TooltipText>
        </TooltipSection>
      )}
      {opts?.previousStats && (
        <NetChangeBlock
          previous={opts.previousStats}
          next={scaled}
        />
      )}
    </>
  )
}

export function buildAffixTooltip(a: Affix): ReactNode {
  const sign = a.sign
  const suffix = a.format === 'percent' ? '%' : ''
  const range =
    a.valueMin !== null && a.valueMax !== null
      ? a.valueMin === a.valueMax
        ? `${sign}${a.valueMin}${suffix}`
        : `${sign}${a.valueMin}${suffix} – ${sign}${a.valueMax}${suffix}`
      : null
  return (
    <>
      <TooltipHeader
        title={a.description}
        subtitle={`Affix · ${a.name} · Tier ${a.tier}`}
      />
      {range && (
        <TooltipSection>
          <TooltipStat label="Range" value={range} />
          {a.statKey && (
            <TooltipStat label="Stat" value={statName(a.statKey)} />
          )}
        </TooltipSection>
      )}
      {a.kind && (
        <TooltipFooter>
          {a.kind === 'prefix' ? 'Prefix' : 'Suffix'} · group {a.groupId}
        </TooltipFooter>
      )}
    </>
  )
}

export function affixAverageStats(a: Affix): Record<string, number> {
  if (!a.statKey || a.valueMin === null || a.valueMax === null) return {}
  const avg = (a.valueMin + a.valueMax) / 2
  const signed = a.sign === '-' ? -avg : avg
  return { [a.statKey]: signed }
}

export function buildCrystalModTooltip(
  m: Affix,
  opts?: { previousStats?: Record<string, number> },
): ReactNode {
  const sign = m.sign
  const suffix = m.format === 'percent' ? '%' : ''
  const range =
    m.valueMin !== null && m.valueMax !== null
      ? m.valueMin === m.valueMax
        ? `${sign}${m.valueMin}${suffix}`
        : `${sign}${m.valueMin}${suffix} – ${sign}${m.valueMax}${suffix}`
      : null
  return (
    <>
      <TooltipHeader
        title={m.name}
        subtitle={`Satanic Crystal · Tier ${m.tier}`}
        tone="satanic"
      />
      <TooltipSection>
        <TooltipText>{m.description}</TooltipText>
      </TooltipSection>
      {range && (
        <TooltipSection>
          <TooltipStat label="Range" value={range} />
          {m.statKey && (
            <TooltipStat label="Stat" value={statName(m.statKey)} />
          )}
        </TooltipSection>
      )}
      {opts?.previousStats && (
        <NetChangeBlock
          previous={opts.previousStats}
          next={affixAverageStats(m)}
        />
      )}
    </>
  )
}

export function buildRunewordTooltip(rw: {
  id: string
  name: string
  runes: string[]
  allowedBaseTypes: string[]
  stats: Record<string, number>
  requiresLevel?: number
  description?: string
}): ReactNode {
  const seq = rw.runes.map((r) => r.replace(/^rune_/, '').toUpperCase()).join(' → ')
  const lines = formatStatLines(rw.stats)
  return (
    <>
      <TooltipHeader
        title={rw.name}
        subtitle={`Runeword · ${rw.runes.length} runes`}
        tone="rare"
      />
      <TooltipSection>
        <TooltipText>
          <span className="font-mono text-[11px] tabular-nums tracking-[0.06em] text-amber-300">
            {seq}
          </span>
        </TooltipText>
      </TooltipSection>
      {lines.length > 0 && <TooltipSection>{lines}</TooltipSection>}
      <TooltipFooter>
        Bases · {rw.allowedBaseTypes.join(', ')}
        {rw.requiresLevel ? ` · Lvl ${rw.requiresLevel}` : ''}
      </TooltipFooter>
      {rw.description && (
        <TooltipSection>
          <TooltipText>{rw.description}</TooltipText>
        </TooltipSection>
      )}
    </>
  )
}

export function buildAugmentTooltip(aug: {
  id: string
  name: string
  description: string
  triggerNote: string
  rangedOnly?: boolean
  levels: { level: number; stats: Record<string, number>; cost: number }[]
}): ReactNode {
  const lvl = aug.levels[aug.levels.length - 1] ?? aug.levels[0]
  const lines = lvl ? formatStatLines(lvl.stats) : []
  return (
    <>
      <TooltipHeader
        title={aug.name}
        subtitle={`Angelic Augment · ${aug.triggerNote}`}
        tone="angelic"
      />
      <TooltipSection>
        <TooltipText>{aug.description}</TooltipText>
      </TooltipSection>
      {lvl && lines.length > 0 && (
        <TooltipSection>
          <div className="mb-1 font-mono text-[9px] uppercase tracking-[0.18em] text-yellow-200/70">
            Max level · {lvl.level} · cost {lvl.cost} keys
          </div>
          {lines}
        </TooltipSection>
      )}
      {aug.rangedOnly && (
        <TooltipFooter>Ranged weapon required</TooltipFooter>
      )}
    </>
  )
}
