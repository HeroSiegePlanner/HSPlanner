import { useEffect, useMemo, useState } from 'react'
import { motion } from 'motion/react'
import { backdropVariants, panelVariants } from '../lib/motion'
import { resolveSkillIcon } from '../data'
import { SUBTREE_TEMPLATE, getTemplateEdges } from '../utils/tree/subtreeTemplate'
import {
  subskillKey,
  subskillPointsFor,
  useBuild,
} from '../store/build'
import type { Skill, SubskillNode } from '../types'
import { statDef, statName } from '../utils/item/stats'
import {
  diffPerformanceDps,
  diffPerformanceStats,
  type BuildPerformance,
} from '../utils/build/buildPerformance'
import { computeBuildPerformanceAsync } from '../lib/calc/bridge'
import { useBuildPerformanceDeps } from '../hooks/useBuildPerformanceDeps'
import NetChangeRow from './NetChangeRow'
import { CornerMarks } from './CornerMarks'

const VIEWBOX = 600
const NODE_R: Record<string, number> = {
  minor: 18,
  notable: 24,
  keystone: 32,
}

const SUBSKILL_SPRITE_FILES = import.meta.glob<string>(
  '../assets/subskills/**/*.png',
  { eager: true, query: '?url', import: 'default' },
)
const SUBSKILL_SPRITE_BY_KEY: Record<string, string> = {}
for (const [p, url] of Object.entries(SUBSKILL_SPRITE_FILES)) {
  const file = p.split('/').pop() ?? ''
  const key = file.replace(/\.png$/i, '')
  SUBSKILL_SPRITE_BY_KEY[key] = url
}

function resolveSubskillIconUrl(icon?: string): string | undefined {
  if (!icon) return undefined
  if (/^https?:\/\//i.test(icon)) return icon
  if (SUBSKILL_SPRITE_BY_KEY[icon]) return SUBSKILL_SPRITE_BY_KEY[icon]
  return undefined
}

interface Props {
  skill: Skill
  onClose: () => void
}

export default function SubtreeOverlay({ skill, onClose }: Props) {
  const level = useBuild((s) => s.level)
  const subskillRanks = useBuild((s) => s.subskillRanks)
  const incSubskillRank = useBuild((s) => s.incSubskillRank)
  const decSubskillRank = useBuild((s) => s.decSubskillRank)
  const resetSubskillsFor = useBuild((s) => s.resetSubskillsFor)

  const buildDeps = useBuildPerformanceDeps()

  const [currentPerformance, setCurrentPerformance] =
    useState<BuildPerformance | null>(null)
  useEffect(() => {
    let cancelled = false
    computeBuildPerformanceAsync(buildDeps).then((p) => {
      if (!cancelled) setCurrentPerformance(p)
    })
    return () => {
      cancelled = true
    }
  }, [buildDeps])

  const totalPoints = subskillPointsFor(level)
  const spent = useMemo(
    () =>
      Object.entries(subskillRanks).reduce(
        (s, [, r]) => s + r,
        0,
      ),
    [subskillRanks],
  )
  const remaining = totalPoints - spent

  const subskillsByPos: Record<number, SubskillNode> = useMemo(() => {
    const out: Record<number, SubskillNode> = {}
    for (const ss of skill.subskills ?? []) {
      out[ss.positionIndex] = ss
    }
    return out
  }, [skill])

  const edges = getTemplateEdges()
  const px = (n: number) => n * VIEWBOX

  const [hover, setHover] = useState<{
    sub: SubskillNode
    x: number
    y: number
    isKeystone: boolean
  } | null>(null)

  const [previewPerformance, setPreviewPerformance] =
    useState<BuildPerformance | null>(null)
  useEffect(() => {
    if (!hover) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setPreviewPerformance(null)
      return
    }
    const key = subskillKey(skill.id, hover.sub.id)
    const currentRank = subskillRanks[key] ?? 0
    if (currentRank >= hover.sub.maxRank) {
      setPreviewPerformance(null)
      return
    }
    const previewRanks = { ...subskillRanks, [key]: currentRank + 1 }
    let cancelled = false
    computeBuildPerformanceAsync({
      ...buildDeps,
      subskillRanks: previewRanks,
    }).then((p) => {
      if (!cancelled) setPreviewPerformance(p)
    })
    return () => {
      cancelled = true
    }
  }, [hover, skill.id, subskillRanks, buildDeps])

  const skillIcon = !skill.icon || skill.icon.startsWith('http') ? '✦' : skill.icon

  return (
    <motion.div
      role="presentation"
      className="fixed inset-0 z-50 flex items-center justify-center p-6 backdrop-blur-sm"
      onClick={onClose}
      variants={backdropVariants}
      initial="initial"
      animate="animate"
      style={{
        background:
          'radial-gradient(ellipse at 50% 0%, rgba(201,165,90,0.06), rgba(0,0,0,0.78) 60%)',
      }}
    >
      <motion.div
        onClick={(e) => e.stopPropagation()}
        variants={panelVariants}
        initial="initial"
        animate="animate"
        className="relative flex max-h-full w-full max-w-3xl flex-col overflow-hidden rounded-sm border border-border"
        style={{
          background:
            'linear-gradient(180deg, var(--color-panel-2), var(--color-bg))',
          boxShadow:
            'inset 0 1px 0 rgba(255,255,255,0.02), 0 24px 64px rgba(0,0,0,0.7)',
        }}
      >
        <CornerMarks />

        <header
          className="flex items-start justify-between gap-4 border-b border-border px-5 py-4"
          style={{
            background:
              'linear-gradient(180deg, rgba(201,165,90,0.05), transparent)',
          }}
        >
          <div className="flex items-start gap-3">
            <span
              className="mt-2 inline-block h-2 w-2 shrink-0 rotate-45 bg-accent-hot"
              style={{ boxShadow: '0 0 8px rgba(224,184,100,0.5)' }}
            />
            <div>
              <h2
                className="m-0 text-[18px] font-semibold tracking-[0.04em] text-accent-hot"
                style={{
                  fontFamily: 'var(--font-sans)',
                  textShadow: '0 0 16px rgba(224,184,100,0.18)',
                }}
              >
                {skill.name}
                <span className="ml-2 align-[1px] font-mono text-[10px] font-normal uppercase tracking-[0.2em] text-faint">
                  · Subtree
                </span>
              </h2>
              <div className="mt-1 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
                Specialize · Boost · Change how this skill works
              </div>
            </div>
          </div>

          <div className="flex items-center gap-3.5">
            <div className="flex items-baseline gap-1.5 font-mono text-[11px] tracking-[0.08em] text-muted tabular-nums">
              <span>Points</span>
              <span className="text-[13px] font-semibold text-accent-hot">
                {spent}
                <span className="text-faint">/</span>
                {totalPoints}
              </span>
              {remaining > 0 ? (
                <span className="text-[10px] uppercase tracking-[0.14em] text-accent-deep">
                  {remaining} LEFT
                </span>
              ) : (
                <span className="text-[10px] uppercase tracking-[0.14em] text-faint">
                  ALL SPENT
                </span>
              )}
            </div>
            <button
              onClick={() => resetSubskillsFor(skill.id)}
              className="rounded-xs border border-border-2 bg-transparent px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
            >
              Reset
            </button>
            <button
              onClick={onClose}
              className="rounded-xs border border-accent-deep px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-accent-hot transition-all hover:border-accent-hot"
              style={{
                background: 'linear-gradient(180deg, #3a2f1a, #2a2418)',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.boxShadow =
                  '0 0 10px rgba(224,184,100,0.25)'
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.boxShadow = 'none'
              }}
            >
              Close
            </button>
          </div>
        </header>

        <div className="flex-1 overflow-auto p-6">
          <svg
            viewBox={`0 0 ${VIEWBOX} ${VIEWBOX}`}
            className="mx-auto block"
            style={{ width: '100%', maxWidth: 580 }}
          >
            <defs>
              <radialGradient id="hsplanner-core-fill" cx="35%" cy="30%" r="70%">
                <stop offset="0%" stopColor="#5a1a14" />
                <stop offset="100%" stopColor="#2a0d0a" />
              </radialGradient>
              <radialGradient id="hsplanner-node-fill" cx="35%" cy="30%" r="70%">
                <stop offset="0%" stopColor="#1a1a22" />
                <stop offset="100%" stopColor="#0a0a0e" />
              </radialGradient>
            </defs>

            <g fill="none">
              {edges.map(([a, b], i) => {
                const na = SUBTREE_TEMPLATE[a]!
                const nb = SUBTREE_TEMPLATE[b]!
                const subA = subskillsByPos[a]
                const subB = subskillsByPos[b]
                const rankA =
                  subA != null
                    ? subskillRanks[subskillKey(skill.id, subA.id)] ?? 0
                    : 0
                const rankB =
                  subB != null
                    ? subskillRanks[subskillKey(skill.id, subB.id)] ?? 0
                    : 0
                const both = rankA > 0 && rankB > 0
                const touchesKeystone =
                  na.role === 'keystone' || nb.role === 'keystone'
                let stroke = '#8a6f3a'
                let strokeOpacity = 0.55
                let strokeWidth = 1.4
                if (touchesKeystone) {
                  stroke = '#d96b5a'
                  strokeOpacity = both ? 0.85 : 0.55
                  strokeWidth = 1.8
                } else if (both) {
                  stroke = '#e0b864'
                  strokeOpacity = 0.9
                  strokeWidth = 2
                }
                return (
                  <line
                    key={i}
                    x1={px(na.x)}
                    y1={px(na.y)}
                    x2={px(nb.x)}
                    y2={px(nb.y)}
                    stroke={stroke}
                    strokeOpacity={strokeOpacity}
                    strokeWidth={strokeWidth}
                    style={
                      touchesKeystone
                        ? { filter: 'drop-shadow(0 0 4px rgba(217,107,90,0.35))' }
                        : both
                          ? { filter: 'drop-shadow(0 0 3px rgba(224,184,100,0.4))' }
                          : undefined
                    }
                  />
                )
              })}
            </g>

            <g>
              {SUBTREE_TEMPLATE.map((tn) => {
                const sub = subskillsByPos[tn.index]
                const r = NODE_R[tn.role]!
                const cx = px(tn.x)
                const cy = px(tn.y)
                const has = !!sub
                const isRoot = tn.role === 'keystone'
                const isOuter = tn.role === 'notable'
                const interactive = has && !isRoot
                const rank = sub
                  ? subskillRanks[subskillKey(skill.id, sub.id)] ?? 0
                  : 0
                const allocated = rank > 0

                let fill = 'url(#hsplanner-node-fill)'
                let stroke: string
                let strokeWidth: number
                let glow: string | undefined

                if (isRoot) {
                  fill = 'url(#hsplanner-core-fill)'
                  stroke = '#d96b5a'
                  strokeWidth = 2
                  glow = '0 0 18px rgba(217,107,90,0.5)'
                } else if (isOuter) {
                  stroke = allocated ? '#e0b864' : '#8a6f3a'
                  strokeWidth = allocated ? 2 : 1.5
                  glow = allocated
                    ? '0 0 14px rgba(224,184,100,0.45)'
                    : '0 0 10px rgba(138,111,58,0.25)'
                } else if (has) {
                  stroke = allocated ? '#e0b864' : '#4a4854'
                  strokeWidth = allocated ? 1.8 : 1.5
                  if (allocated) glow = '0 0 10px rgba(224,184,100,0.35)'
                } else {
                  stroke = '#2a2b35'
                  strokeWidth = 1
                }

                const iconColor = isRoot
                  ? '#ffd86b'
                  : isOuter
                    ? '#fff0c4'
                    : '#e0b864'

                const labelColor = allocated ? '#e0b864' : '#8a8276'

                return (
                  <g key={tn.index}>
                    {(isRoot || (isOuter && allocated)) && (
                      <circle
                        cx={cx}
                        cy={cy}
                        r={r + 6}
                        fill="none"
                        stroke={isRoot ? '#d96b5a' : '#e0b864'}
                        strokeOpacity={isRoot ? 0.18 : 0.15}
                        strokeWidth={1}
                        pointerEvents="none"
                      />
                    )}
                    <circle
                      cx={cx}
                      cy={cy}
                      r={r}
                      fill={fill}
                      stroke={stroke}
                      strokeWidth={strokeWidth}
                      style={{
                        cursor: interactive ? 'pointer' : 'default',
                        filter: glow ? `drop-shadow(${glow})` : undefined,
                        transition: 'all 0.15s',
                      }}
                      onClick={() => {
                        if (interactive && sub)
                          incSubskillRank(skill.id, sub.id, sub.maxRank)
                      }}
                      onContextMenu={(e) => {
                        e.preventDefault()
                        if (interactive && sub) decSubskillRank(skill.id, sub.id)
                      }}
                      onMouseEnter={(e) => {
                        if (sub) {
                          const rect = (
                            e.currentTarget.ownerSVGElement as SVGSVGElement
                          ).getBoundingClientRect()
                          setHover({
                            sub,
                            x: rect.left + (cx / VIEWBOX) * rect.width,
                            y: rect.top + (cy / VIEWBOX) * rect.height,
                            isKeystone: isRoot,
                          })
                        }
                      }}
                      onMouseLeave={() => setHover(null)}
                    />
                    {has && (() => {
                      const iconUrl = isRoot
                        ? resolveSkillIcon(skill)
                        : resolveSubskillIconUrl(sub!.icon)
                      if (iconUrl) {
                        const imgR = r * 0.9
                        return (
                          <image
                            href={iconUrl}
                            x={cx - imgR}
                            y={cy - imgR}
                            width={imgR * 2}
                            height={imgR * 2}
                            pointerEvents="none"
                            style={{
                              imageRendering: 'pixelated',
                              filter: isRoot
                                ? 'drop-shadow(0 0 8px rgba(255,200,80,0.55))'
                                : 'drop-shadow(0 0 4px rgba(0,0,0,0.6))',
                            }}
                          />
                        )
                      }
                      const glyph = isRoot
                        ? skillIcon
                        : sub!.icon && !/^https?:\/\//i.test(sub!.icon)
                          ? sub!.icon
                          : '◆'
                      return (
                        <text
                          x={cx}
                          y={cy + r * 0.32}
                          textAnchor="middle"
                          fontSize={r * 0.95}
                          fill={iconColor}
                          pointerEvents="none"
                          style={{
                            filter: isRoot
                              ? 'drop-shadow(0 0 6px rgba(255,200,80,0.6))'
                              : 'drop-shadow(0 0 4px rgba(224,184,100,0.25))',
                          }}
                        >
                          {glyph}
                        </text>
                      )
                    })()}
                    {has && !isRoot && (
                      <text
                        x={cx}
                        y={cy + r + 16}
                        textAnchor="middle"
                        fontSize={10}
                        fill={labelColor}
                        fontFamily="var(--font-mono)"
                        letterSpacing="0.08em"
                        pointerEvents="none"
                      >
                        <tspan fill={allocated ? '#e0b864' : '#a39e8d'}>
                          {rank}
                        </tspan>
                        <tspan fill="#5a5448">/</tspan>
                        <tspan>{sub!.maxRank}</tspan>
                      </text>
                    )}
                  </g>
                )
              })}
            </g>
          </svg>
          {(skill.subskills?.length ?? 0) === 0 && (
            <p className="mt-4 text-center font-mono text-[11px] uppercase tracking-[0.14em] text-faint">
              No subskills defined for {skill.name} yet.
            </p>
          )}
        </div>
      </motion.div>
      {hover && currentPerformance && (
        <SubskillTooltip
          sub={hover.sub}
          rank={subskillRanks[subskillKey(skill.id, hover.sub.id)] ?? 0}
          x={hover.x}
          y={hover.y}
          isKeystone={hover.isKeystone}
          currentPerformance={currentPerformance}
          previewPerformance={previewPerformance}
        />
      )}
    </motion.div>
  )
}

function formatStatValue(key: string, value: number): string {
  const def = statDef(key)
  const suffix = def?.format === 'percent' ? '%' : ''
  const rounded = Number.isInteger(value)
    ? String(value)
    : (Math.round(value * 100) / 100).toString()
  return `${rounded}${suffix}`
}

function SubskillTooltip({
  sub,
  rank,
  x,
  y,
  isKeystone,
  currentPerformance,
  previewPerformance,
}: {
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

  const statKeys = new Set<string>([
    ...Object.keys(sub.effects?.base ?? {}),
    ...Object.keys(sub.effects?.perRank ?? {}),
  ])

  const statRows = Array.from(statKeys).map((k) => {
    const base = sub.effects?.base?.[k] ?? 0
    const per = sub.effects?.perRank?.[k] ?? 0
    const current = base + per * rank
    const next = base + per * nextRank
    return { key: k, current, next }
  })

  const proc = sub.proc
  const procChanceCurrent =
    (proc?.chance.base ?? 0) + (proc?.chance.perRank ?? 0) * rank
  const procChanceNext =
    (proc?.chance.base ?? 0) + (proc?.chance.perRank ?? 0) * nextRank

  const procStatKeys = proc
    ? Array.from(
        new Set<string>([
          ...Object.keys(proc.effects?.base ?? {}),
          ...Object.keys(proc.effects?.perRank ?? {}),
        ]),
      )
    : []

  const procStatRows = procStatKeys.map((k) => {
    const base = proc?.effects?.base?.[k] ?? 0
    const per = proc?.effects?.perRank?.[k] ?? 0
    return {
      key: k,
      current: base + per * rank,
      next: base + per * nextRank,
    }
  })

  return (
    <div
      className={`pointer-events-none fixed z-60 ${netChangeVisible ? 'w-96' : 'w-72'} overflow-hidden rounded-sm border border-border`}
      style={{
        left: x + 18,
        top: y + 18,
        background: 'linear-gradient(180deg, var(--color-panel-2), var(--color-bg))',
        boxShadow:
          'inset 0 1px 0 rgba(255,255,255,0.02), 0 12px 32px rgba(0,0,0,0.7)',
      }}
    >
      <div
        className="flex items-center gap-2 border-b border-border px-3 py-2.5"
        style={{
          background:
            'linear-gradient(180deg, rgba(201,165,90,0.06), transparent)',
        }}
      >
        <span
          className="inline-block h-1.5 w-1.5 shrink-0 rotate-45 bg-accent-hot"
          style={{ boxShadow: '0 0 6px rgba(224,184,100,0.5)' }}
        />
        {(() => {
          const iconUrl = resolveSubskillIconUrl(sub.icon)
          if (iconUrl) {
            return (
              <img
                src={iconUrl}
                alt=""
                width={20}
                height={20}
                style={{ imageRendering: 'pixelated' }}
              />
            )
          }
          if (sub.icon && !/^https?:\/\//i.test(sub.icon)) {
            return <span className="text-[15px] text-accent-hot">{sub.icon}</span>
          }
          return null
        })()}
        <div className="min-w-0 flex-1">
          <div className="truncate text-[13px] font-semibold tracking-[0.02em] text-accent-hot">
            {sub.name}
          </div>
          {!isKeystone && (
            <div className="font-mono text-[10px] uppercase tracking-[0.16em] text-faint">
              Rank{' '}
              <span className="tabular-nums text-accent-hot">{rank}</span>
              <span className="text-faint"> / {sub.maxRank}</span>
            </div>
          )}
        </div>
      </div>
      <div className="px-3 py-2.5">
        {sub.description && (
          <p className="mb-2 text-xs leading-relaxed text-text/85">
            {sub.description}
          </p>
        )}
        {statRows.length > 0 && (
          <div className="space-y-0.5 rounded-[3px] border border-border-2 bg-black/30 p-2 text-xs tabular-nums">
            {statRows.map(({ key, current, next }) => (
              <div key={key} className="flex items-center justify-between gap-2">
                <span className="text-text/80">{statName(key)}</span>
                <span className="flex items-center gap-1 font-mono">
                  <span className="text-muted">
                    {formatStatValue(key, current)}
                  </span>
                  {hasNext && current !== next && (
                    <>
                      <span className="text-faint">›</span>
                      <span className="text-accent-hot">
                        {formatStatValue(key, next)}
                      </span>
                    </>
                  )}
                </span>
              </div>
            ))}
          </div>
        )}
        {proc && (
          <div
            className={`${statRows.length > 0 ? 'mt-2' : ''} space-y-0.5 rounded-[3px] border border-border-2 bg-black/30 p-2 text-xs tabular-nums`}
          >
            <div className="mb-1 flex items-center justify-between font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
              <span>{proc.trigger.replace('_', ' ')} proc</span>
              {(proc.appliesStates?.length || proc.tags?.length) && (
                <span className="flex gap-1">
                  {proc.appliesStates?.map((s, i) => {
                    const name = typeof s === 'string' ? s : s.state
                    return (
                      <span key={`s-${name}-${i}`} className="text-accent-hot/80">
                        applies {name.replace(/_/g, ' ')}
                      </span>
                    )
                  })}
                  {proc.tags?.map((t) => (
                    <span key={`t-${t}`} className="text-accent-hot/80">
                      {t}
                    </span>
                  ))}
                </span>
              )}
            </div>
            <div className="flex items-center justify-between gap-2">
              <span className="text-text/80">Proc Chance</span>
              <span className="flex items-center gap-1 font-mono">
                <span className="text-muted">{procChanceCurrent}%</span>
                {hasNext && procChanceCurrent !== procChanceNext && (
                  <>
                    <span className="text-faint">›</span>
                    <span className="text-accent-hot">{procChanceNext}%</span>
                  </>
                )}
              </span>
            </div>
            {procStatRows.map(({ key, current, next }) => {
              const avg = (procChanceCurrent / 100) * current
              return (
                <div
                  key={key}
                  className="flex items-center justify-between gap-2"
                >
                  <span className="text-text/80">{statName(key)}</span>
                  <span className="flex items-center gap-1 font-mono">
                    <span className="text-muted">
                      {formatStatValue(key, current)}
                    </span>
                    {hasNext && current !== next && (
                      <>
                        <span className="text-faint">›</span>
                        <span className="text-accent-hot">
                          {formatStatValue(key, next)}
                        </span>
                      </>
                    )}
                    {avg > 0 && (
                      <span className="ml-1 text-[10px] text-faint">
                        (avg {formatStatValue(key, avg)})
                      </span>
                    )}
                  </span>
                </div>
              )
            })}
            {proc.appliesStates?.map((s, i) => {
              if (typeof s === 'string') return null
              const cur =
                (s.amount?.base ?? 0) + (s.amount?.perRank ?? 0) * rank
              const nxt =
                (s.amount?.base ?? 0) + (s.amount?.perRank ?? 0) * nextRank
              return (
                <div
                  key={`state-${s.state}-${i}`}
                  className="flex items-center justify-between gap-2"
                >
                  <span className="text-text/80 capitalize">
                    {s.state.replace(/_/g, ' ')}
                  </span>
                  <span className="flex items-center gap-1 font-mono">
                    <span className="text-muted">{cur}%</span>
                    {hasNext && cur !== nxt && (
                      <>
                        <span className="text-faint">›</span>
                        <span className="text-accent-hot">{nxt}%</span>
                      </>
                    )}
                  </span>
                </div>
              )
            })}
          </div>
        )}
        {netChangeVisible && (
          <div className="mt-2 rounded-[3px] border border-border-2 bg-black/30 p-2">
            <div className="mb-1.5 flex items-center gap-2 font-mono text-[9px] font-semibold uppercase tracking-[0.18em] text-muted">
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
          </div>
        )}
      </div>
    </div>
  )
}
