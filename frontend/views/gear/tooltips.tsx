/* eslint-disable react-refresh/only-export-components */
import type { ReactNode } from 'react'
import { TooltipFooter, TooltipHeader, TooltipSection, TooltipSectionHeader, TooltipStat, TooltipText } from '../../components/ui/Tooltip'
import NetChangeRow from '../../components/NetChangeRow'
import { formatValue, statName } from '../../utils/item/stats'
import { gameConfig } from '@data'
import type { BuildStatDiff } from '../../utils/build/buildPerformance'
import type { Affix } from '../../types'
import { affixTierLabel } from './lib/affixGroups'
import { augmentIconForId, socketableIconForName } from './lib/gearIcons'

const PERCENT_STAT_KEYS = new Set(
  gameConfig.stats.filter((s) => s.format === 'percent').map((s) => s.key),
)

export function NetChangeBlock({
  previous,
  next,
  dpsDiffs,
  activeSkillName,
}: {
  previous: Record<string, number>
  next: Record<string, number>
  dpsDiffs?: BuildStatDiff[]
  activeSkillName?: string | null
}) {
  const allKeys = new Set([
    ...Object.keys(previous),
    ...Object.keys(next),
  ])
  const diffs: BuildStatDiff[] = []
  for (const key of allKeys) {
    const before = previous[key] ?? 0
    const after = next[key] ?? 0
    const delta = after - before
    if (Math.abs(delta) < 0.01) continue
    diffs.push({
      key,
      label: statName(key),
      beforeMin: before,
      beforeMax: before,
      afterMin: after,
      afterMax: after,
      delta,
      isPercent: PERCENT_STAT_KEYS.has(key),
      kind: delta > 0 ? 'up' : 'down',
    })
  }
  diffs.sort((a, b) => b.delta - a.delta)
  const hasDps = !!dpsDiffs && dpsDiffs.length > 0
  return (
    <TooltipSection>
      <div className="mb-2 flex items-center gap-2.5 font-mono text-[10px] font-semibold uppercase tracking-[0.18em] text-muted">
        <span>Net Change</span>
        <span className="h-px flex-1 bg-border" />
      </div>
      {hasDps && (
        <div className={diffs.length > 0 ? 'mb-1.5' : undefined}>
          <div className="mb-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
            Active Skill
            {activeSkillName ? (
              <span className="ml-1 text-faint">· {activeSkillName}</span>
            ) : null}
          </div>
          <div className="space-y-0.5">
            {dpsDiffs.map((d) => (
              <NetChangeRow key={d.key} diff={d} />
            ))}
          </div>
        </div>
      )}
      {diffs.length === 0 ? (
        !hasDps && <p className="text-[11px] text-faint italic">No stat changes</p>
      ) : (
        <div>
          {hasDps && (
            <div className="mb-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
              Stats
            </div>
          )}
          <div className="space-y-0.5">
            {diffs.map((d) => (
              <NetChangeRow key={d.key} diff={d} />
            ))}
          </div>
        </div>
      )}
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
        subtitle={`Affix · ${a.name} · Tier ${affixTierLabel(a.tier)}`}
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

export function buildAffixGroupTooltip(tiers: Affix[], label: string): ReactNode {
  const first = tiers[0]
  if (!first) return null
  const suffix = first.format === 'percent' ? '%' : ''
  return (
    <>
      <TooltipHeader
        title={label}
        subtitle={`Affix · ${tiers.length} tier${tiers.length === 1 ? '' : 's'}`}
      />
      <TooltipSection>
        {tiers.map((t) => (
          <TooltipStat
            key={t.id}
            label={`${affixTierLabel(t.tier)} · ${t.name}`}
            value={
              t.valueMin === null || t.valueMax === null
                ? '—'
                : t.valueMin === t.valueMax
                  ? `${t.sign}${t.valueMin}${suffix}`
                  : `${t.sign}${t.valueMin}${suffix} – ${t.sign}${t.valueMax}${suffix}`
            }
          />
        ))}
      </TooltipSection>
      {first.statKey && (
        <TooltipSection>
          <TooltipStat label="Stat" value={statName(first.statKey)} />
        </TooltipSection>
      )}
      {first.kind && (
        <TooltipFooter>
          {first.kind === 'prefix' ? 'Prefix' : 'Suffix'} · group {first.groupId}
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
          <span className="font-mono text-[11px] tabular-nums tracking-[0.06em] text-accent-hot">
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
        image={augmentIconForId(aug.id)}
      />
      <TooltipSection>
        <TooltipText>{aug.description}</TooltipText>
      </TooltipSection>
      {lvl && lines.length > 0 && (
        <TooltipSection>
          <TooltipSectionHeader
            tone="gold"
            trailing={`lv ${lvl.level} · ${lvl.cost} keys`}
          >
            Max Level Stats
          </TooltipSectionHeader>
          {lines}
        </TooltipSection>
      )}
      {aug.rangedOnly && (
        <TooltipFooter>Ranged weapon required</TooltipFooter>
      )}
    </>
  )
}
