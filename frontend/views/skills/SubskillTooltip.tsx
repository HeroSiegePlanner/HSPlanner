import { useLayoutEffect, useRef, useState } from 'react'
import type { ReactNode } from 'react'
import { subskillTags } from '@data'
import type { Skill, SubskillEffect, SubskillNode } from '../../types'
import { formatValue, statName } from '../../utils/item/stats'
import {
  diffPerformanceDps,
  diffPerformanceStats,
  type BuildPerformance,
} from '../../utils/build/buildPerformance'
import NetChangeRow from '../../components/NetChangeRow'
import { clampTooltipToViewport } from '../../utils/tooltipPosition'
import {
  TooltipHeader,
  TooltipSection,
  TooltipSectionHeader,
  TooltipStat,
  TooltipText,
} from '../../components/ui/Tooltip'
import { TONE_BORDER, TONE_GLOW } from '../../components/tooltipTones'
import { resolveSubskillIconUrl } from './subskillSprites'

function RankValue({
  current,
  next,
  trailing,
}: {
  current: string
  next?: string | null
  trailing?: ReactNode
}) {
  return (
    <span className="inline-flex items-baseline gap-1">
      <span className="text-muted">{current}</span>
      {next != null && (
        <>
          <span className="text-faint">›</span>
          <span className="text-accent-hot">{next}</span>
        </>
      )}
      {trailing}
    </span>
  )
}

function atRank(
  base: number | undefined,
  per: number | undefined,
  rank: number,
): number {
  return (base ?? 0) + (per ?? 0) * rank
}

function effectRows(
  effects: SubskillEffect | undefined,
  rank: number,
  nextRank: number,
): { key: string; current: number; next: number }[] {
  const keys = new Set<string>([
    ...Object.keys(effects?.base ?? {}),
    ...Object.keys(effects?.perRank ?? {}),
  ])
  return Array.from(keys).map((k) => ({
    key: k,
    current: atRank(effects?.base?.[k], effects?.perRank?.[k], rank),
    next: atRank(effects?.base?.[k], effects?.perRank?.[k], nextRank),
  }))
}

export default function SubskillTooltip({
  skill,
  sub,
  rank,
  x,
  y,
  isKeystone,
  currentPerformance,
  previewPerformance,
}: {
  skill: Skill
  sub: SubskillNode
  rank: number
  x: number
  y: number
  isKeystone: boolean
  currentPerformance: BuildPerformance
  previewPerformance: BuildPerformance | null
}) {
  const nextRank = Math.min(rank + 1, sub.maxRank)
  const hasNext = nextRank > rank

  const dpsDiffs = previewPerformance
    ? diffPerformanceDps(currentPerformance, previewPerformance)
    : []
  const statDiffs = previewPerformance
    ? diffPerformanceStats(currentPerformance, previewPerformance)
    : []
  const netChangeVisible = !isKeystone && (dpsDiffs.length > 0 || statDiffs.length > 0)

  const statRows = effectRows(sub.effects, rank, nextRank)
  const tagChange = subskillTags[skill.id]?.[sub.id]

  const proc = sub.proc
  const procChanceCurrent = atRank(
    proc?.chance.base,
    proc?.chance.perRank,
    rank,
  )
  const procChanceNext = atRank(
    proc?.chance.base,
    proc?.chance.perRank,
    nextRank,
  )
  const procStatRows = proc ? effectRows(proc.effects, rank, nextRank) : []

  const ref = useRef<HTMLDivElement>(null)
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null)
  useLayoutEffect(() => {
    const el = ref.current
    if (!el) return
    const rect = el.getBoundingClientRect()
    setPos(
      clampTooltipToViewport(
        { x, y },
        { width: rect.width, height: rect.height },
        { width: window.innerWidth, height: window.innerHeight },
      ),
    )
  }, [x, y, netChangeVisible, sub.id, rank, previewPerformance])

  const iconUrl = resolveSubskillIconUrl(skill, sub)
  const glyph =
    !iconUrl && sub.icon && !/^https?:\/\//i.test(sub.icon) ? sub.icon : null

  return (
    <div
      ref={ref}
      role="tooltip"
      className={`fixed z-[1000] bg-panel border ${TONE_BORDER.rare} ${TONE_GLOW.rare} rounded-[4px] overflow-y-auto overflow-x-hidden pointer-events-none select-none shadow-[0_8px_32px_rgba(0,0,0,0.8)]`}
      style={{
        left: pos?.left ?? -9999,
        top: pos?.top ?? -9999,
        opacity: pos ? 1 : 0,
        transition: 'opacity 80ms ease-out',
        width: 'max-content',
        minWidth: netChangeVisible ? 300 : 240,
        maxWidth: 'min(480px, calc(100vw - 24px))',
        maxHeight: 'calc(100vh - 24px)',
      }}
    >
      <TooltipHeader
        tone="rare"
        image={iconUrl}
        title={
          glyph ? (
            <>
              <span className="mr-1.5">{glyph}</span>
              {sub.name}
            </>
          ) : (
            sub.name
          )
        }
        subtitle={
          isKeystone ? undefined : (
            <>
              Rank <span className="tabular-nums text-accent-hot">{rank}</span>
              <span className="text-faint"> / {sub.maxRank}</span>
            </>
          )
        }
      />
      {sub.description && (
        <TooltipSection>
          <TooltipText>{sub.description}</TooltipText>
        </TooltipSection>
      )}
      {tagChange && (
        <TooltipSection>
          <div className="flex flex-wrap items-center gap-1 font-mono text-[9px] uppercase tracking-[0.14em]">
            <span className="mr-0.5 text-faint">Tags</span>
            {tagChange.add?.map((t) => (
              <span
                key={t}
                className="rounded-xs border border-accent-hot/70 px-1.5 py-0.5 font-semibold text-accent-hot"
              >
                +{t}
              </span>
            ))}
            {tagChange.remove?.map((t) => (
              <span
                key={t}
                className="rounded-xs border border-border px-1.5 py-0.5 font-semibold text-faint line-through"
              >
                {t}
              </span>
            ))}
          </div>
        </TooltipSection>
      )}
      {statRows.length > 0 && (
        <TooltipSection>
          <div className="space-y-0.5">
            {statRows.map(({ key, current, next }) => (
              <TooltipStat
                key={key}
                label={statName(key)}
                value={
                  <RankValue
                    current={formatValue(current, key, false)}
                    next={
                      hasNext && current !== next
                        ? formatValue(next, key, false)
                        : null
                    }
                  />
                }
              />
            ))}
          </div>
        </TooltipSection>
      )}
      {proc && (
        <TooltipSection>
          <TooltipSectionHeader
            trailing={
              proc.appliesStates?.length || proc.tags?.length ? (
                <span className="flex gap-1 uppercase tracking-[0.12em] text-accent-hot/80">
                  {proc.appliesStates?.map((s, i) => {
                    const name = typeof s === 'string' ? s : s.state
                    return (
                      <span key={`s-${name}-${i}`}>
                        applies {name.replace(/_/g, ' ')}
                      </span>
                    )
                  })}
                  {proc.tags?.map((t) => (
                    <span key={`t-${t}`}>{t}</span>
                  ))}
                </span>
              ) : undefined
            }
          >
            {proc.trigger.replace('_', ' ')} proc
          </TooltipSectionHeader>
          <div className="space-y-0.5">
            <TooltipStat
              label="Proc Chance"
              value={
                <RankValue
                  current={`${procChanceCurrent}%`}
                  next={
                    hasNext && procChanceCurrent !== procChanceNext
                      ? `${procChanceNext}%`
                      : null
                  }
                />
              }
            />
            {procStatRows.map(({ key, current, next }) => {
              const avg = (procChanceCurrent / 100) * current
              return (
                <TooltipStat
                  key={key}
                  label={statName(key)}
                  value={
                    <RankValue
                      current={formatValue(current, key, false)}
                      next={
                        hasNext && current !== next
                          ? formatValue(next, key, false)
                          : null
                      }
                      trailing={
                        avg > 0 ? (
                          <span className="ml-1 text-[10px] text-faint">
                            (avg {formatValue(avg, key, false)})
                          </span>
                        ) : undefined
                      }
                    />
                  }
                />
              )
            })}
            {proc.appliesStates?.map((s, i) => {
              if (typeof s === 'string') return null
              const cur = atRank(s.amount?.base, s.amount?.perRank, rank)
              const nxt = atRank(s.amount?.base, s.amount?.perRank, nextRank)
              return (
                <TooltipStat
                  key={`state-${s.state}-${i}`}
                  label={
                    <span className="capitalize">
                      {s.state.replace(/_/g, ' ')}
                    </span>
                  }
                  value={
                    <RankValue
                      current={`${cur}%`}
                      next={hasNext && cur !== nxt ? `${nxt}%` : null}
                    />
                  }
                />
              )
            })}
          </div>
        </TooltipSection>
      )}
      {netChangeVisible && (
        <TooltipSection>
          <div className="mb-2 flex items-center gap-2.5 font-mono text-[10px] font-semibold uppercase tracking-[0.18em] text-muted">
            <span>Net Change</span>
            <span className="h-px flex-1 bg-border" />
            <span className="font-normal tracking-[0.14em] text-faint">
              +1 rank
            </span>
          </div>
          {dpsDiffs.length > 0 && (
            <div className={statDiffs.length > 0 ? 'mb-1.5' : ''}>
              <div className="mb-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
                Active Skill
                {currentPerformance.activeSkillName ? (
                  <span className="ml-1 text-faint">
                    · {currentPerformance.activeSkillName}
                  </span>
                ) : null}
              </div>
              <div className="space-y-0.5">
                {dpsDiffs.map((d) => (
                  <NetChangeRow key={d.key} diff={d} />
                ))}
              </div>
            </div>
          )}
          {statDiffs.length > 0 && (
            <div>
              {dpsDiffs.length > 0 && (
                <div className="mb-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
                  Stats
                </div>
              )}
              <div className="space-y-0.5">
                {statDiffs.map((d) => (
                  <NetChangeRow key={d.key} diff={d} />
                ))}
              </div>
            </div>
          )}
        </TooltipSection>
      )}
    </div>
  )
}
