import { useEffect, useMemo, useState } from 'react'
import { createPortal } from 'react-dom'
import { resolveSkillIcon } from '@data'
import { SUBTREE_TEMPLATE, getTemplateEdges } from '../../utils/tree/subtreeTemplate'
import {
  subskillKey,
  subskillPointsFor,
  subskillSpentFor,
  useBuild,
} from '../../store/build'
import type { Skill, SubskillNode, SubskillRole } from '../../types'
import { ACCENT_HOT_RGB } from './treeConstants'
import { DAMAGE_COLORS } from '../../utils/damageColors'
import type { BuildPerformance } from '../../utils/build/buildPerformance'
import { computeBuildPerformanceAsync } from '../../utils/calc/bridge'
import { useBuildPerformanceDeps } from '../../hooks/useBuildPerformanceDeps'
import { useCalcResult } from '../../hooks/useCalcResult'
import { useRankProgressionPreview } from '../../hooks/useRankProgressionPreview'
import { allocationStep } from '../../utils/allocationStep'
import { CornerMarks } from '../../components/ui/CornerMarks'
import { Modal, MODAL_BTN_CLASS } from '../../components/ui/Modal'
import ProgressionSlider from '../../components/ProgressionSlider'
import SubskillTooltip from './SubskillTooltip'
import { resolveSubskillIconUrl } from './subskillSprites'

const VIEWBOX = 600
const BOARD_MAX = 560
const TILE_PCT: Record<SubskillRole, number> = {
  minor: 7.8,
  notable: 9.8,
  keystone: 12.4,
}
const STAT_RED = '#d96b5a'
const goldA = (alpha: number) => `rgba(${ACCENT_HOT_RGB},${alpha})`
const HOVER_PREVIEW_DEBOUNCE_MS = 100

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

  const currentPerformance = useCalcResult<BuildPerformance | null>(
    () => computeBuildPerformanceAsync(buildDeps),
    [buildDeps],
    null,
  )

  const totalPoints = subskillPointsFor(level)
  const spent = useMemo(
    () => subskillSpentFor(subskillRanks, skill.id),
    [subskillRanks, skill.id],
  )
  const remaining = totalPoints - spent

  const skillSubRanks = useMemo(
    () =>
      Object.fromEntries(
        Object.entries(subskillRanks).filter(([k]) =>
          k.startsWith(skill.id + ':'),
        ),
      ),
    [subskillRanks, skill.id],
  )
  const { progressStep, setProgressStep, visibleRanks, markerId, isPreview, total } =
    useRankProgressionPreview(skillSubRanks)

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

  const [debouncedHover, setDebouncedHover] = useState<typeof hover>(null)
  useEffect(() => {
    const t = setTimeout(
      () => setDebouncedHover(hover),
      HOVER_PREVIEW_DEBOUNCE_MS,
    )
    return () => clearTimeout(t)
  }, [hover])

  const previewPerformance = useCalcResult<BuildPerformance | null>(
    () => {
      if (!debouncedHover) return null
      const key = subskillKey(skill.id, debouncedHover.sub.id)
      const currentRank = subskillRanks[key] ?? 0
      if (currentRank >= debouncedHover.sub.maxRank) return null
      const previewRanks = { ...subskillRanks, [key]: currentRank + 1 }
      return computeBuildPerformanceAsync({
        ...buildDeps,
        subskillRanks: previewRanks,
      })
    },
    [debouncedHover, skill.id, subskillRanks, buildDeps],
    null,
  )

  const rgb = skill.damageType
    ? DAMAGE_COLORS[skill.damageType].rgb
    : ACCENT_HOT_RGB
  const skillIcon = !skill.icon || skill.icon.startsWith('http') ? '✦' : skill.icon

  return (
    <>
      <Modal
        onClose={onClose}
        dataTour="subtree-overlay"
        titleId="subtree-overlay-title"
        eyebrow="Skill Subtree"
        title={skill.name}
        subtitle="Specialize · Boost · Change how this skill works"
        panelClassName="w-[min(680px,calc(100vw-3rem))] max-h-[calc(100vh-3rem)]"
        headerActions={
          <>
            <div className="flex items-baseline gap-1.5 font-mono text-[11px] tracking-[0.08em] text-faint tabular-nums">
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
              className={MODAL_BTN_CLASS}
            >
              Reset
            </button>
          </>
        }
      >
        <div className="relative flex min-h-0 flex-1 flex-col">
          <div className="min-h-0 flex-1 overflow-auto p-5">
            <section
              className="relative mx-auto rounded-md border p-4"
              style={{
                width: '100%',
                // board is square, so cap it by viewport height too — otherwise the modal scrolls
                maxWidth: `min(${BOARD_MAX + 32}px, calc(100vh - 230px))`,
                borderColor: `rgba(${rgb},0.22)`,
                background: `linear-gradient(180deg, rgba(${rgb},0.06), var(--color-panel) 22%, color-mix(in srgb, var(--color-bg) 70%, transparent))`,
                boxShadow: `inset 0 1px 0 rgba(${rgb},0.18), 0 8px 24px rgba(0,0,0,0.35)`,
              }}
            >
              <CornerMarks size={8} opacity={0.45} />
              <div className="relative aspect-square w-full">
                <svg
                  viewBox={`0 0 ${VIEWBOX} ${VIEWBOX}`}
                  className="pointer-events-none absolute inset-0 h-full w-full"
                >
                  {edges.map(([a, b], i) => {
                    const na = SUBTREE_TEMPLATE[a]!
                    const nb = SUBTREE_TEMPLATE[b]!
                    const subA = subskillsByPos[a]
                    const subB = subskillsByPos[b]
                    const rankA =
                      subA != null
                        ? visibleRanks[subskillKey(skill.id, subA.id)] ?? 0
                        : 0
                    const rankB =
                      subB != null
                        ? visibleRanks[subskillKey(skill.id, subB.id)] ?? 0
                        : 0
                    const both = rankA > 0 && rankB > 0
                    const touchesKeystone =
                      na.role === 'keystone' || nb.role === 'keystone'
                    return (
                      <line
                        key={i}
                        x1={px(na.x)}
                        y1={px(na.y)}
                        x2={px(nb.x)}
                        y2={px(nb.y)}
                        stroke={
                          both
                            ? goldA(0.55)
                            : touchesKeystone
                              ? 'rgba(217,107,90,0.35)'
                              : 'rgba(120,110,95,0.3)'
                        }
                        strokeWidth={2}
                        strokeDasharray="4 5"
                        strokeLinecap="round"
                      />
                    )
                  })}
                </svg>

                {SUBTREE_TEMPLATE.map((tn) => {
                  const sub = subskillsByPos[tn.index]
                  const isRoot = tn.role === 'keystone'
                  const interactive = !!sub && !isRoot
                  const rank = sub
                    ? visibleRanks[subskillKey(skill.id, sub.id)] ?? 0
                    : 0
                  const allocated = rank > 0
                  const isMarker =
                    !!sub && markerId === subskillKey(skill.id, sub.id)

                  const pos = {
                    left: `${tn.x * 100}%`,
                    top: `${tn.y * 100}%`,
                    width: `${TILE_PCT[tn.role]}%`,
                    aspectRatio: '1 / 1',
                  }

                  if (!sub) {
                    return (
                      <span
                        key={tn.index}
                        aria-hidden
                        className="absolute -translate-x-1/2 -translate-y-1/2 rounded-full border border-dashed border-border opacity-50"
                        style={pos}
                      />
                    )
                  }

                  const iconUrl =
                    resolveSubskillIconUrl(skill, sub) ??
                    (isRoot ? resolveSkillIcon(skill) : undefined)
                  const glyph = isRoot
                    ? skillIcon
                    : sub.icon && !/^https?:\/\//i.test(sub.icon)
                      ? sub.icon
                      : '◆'

                  const shadows: string[] = []
                  if (isMarker) {
                    shadows.push(
                      `0 0 0 2.5px ${goldA(0.9)}`,
                      `0 0 16px ${goldA(0.6)}`,
                    )
                  }
                  if (isRoot) shadows.push('0 0 16px rgba(217,107,90,0.35)')
                  else if (allocated) shadows.push(`0 0 12px ${goldA(0.35)}`)

                  const borderColor = isRoot
                    ? STAT_RED
                    : allocated
                      ? goldA(0.9)
                      : tn.role === 'notable'
                        ? 'var(--color-accent-deep)'
                        : 'var(--color-border-2)'

                  const onEnter = (e: React.MouseEvent<HTMLElement>) => {
                    const r = e.currentTarget.getBoundingClientRect()
                    setHover({
                      sub,
                      x: r.left + r.width / 2,
                      y: r.top + r.height / 2,
                      isKeystone: isRoot,
                    })
                  }

                  const tile = iconUrl ? (
                    <img
                      src={iconUrl}
                      alt=""
                      draggable={false}
                      className="h-full w-full object-contain"
                      style={{ imageRendering: 'pixelated' }}
                    />
                  ) : (
                    <span
                      className="font-mono text-[15px] leading-none"
                      style={{ color: isRoot ? '#ffd86b' : goldA(0.85) }}
                    >
                      {glyph}
                    </span>
                  )

                  const tileStyle = {
                    borderColor,
                    borderWidth: allocated || isRoot ? 2 : 1,
                    background: isRoot
                      ? 'radial-gradient(circle at 35% 30%, #5a1a14, #2a0d0a)'
                      : 'linear-gradient(180deg, var(--color-panel-2), var(--color-bg))',
                    boxShadow: shadows.length ? shadows.join(', ') : undefined,
                  }
                  // every sprite is a round badge, so the frame hugs it
                  const tileClass = `flex h-full w-full items-center justify-center overflow-hidden rounded-full border p-[5%] transition-all ${
                    allocated || isRoot ? '' : 'opacity-65'
                  }`

                  return (
                    <div
                      key={tn.index}
                      className="absolute -translate-x-1/2 -translate-y-1/2"
                      style={pos}
                    >
                      {interactive ? (
                        <button
                          type="button"
                          className={`${tileClass} cursor-pointer hover:scale-105`}
                          style={tileStyle}
                          aria-label={`${sub.name} — rank ${rank} of ${sub.maxRank}`}
                          onMouseEnter={onEnter}
                          onMouseLeave={() => setHover(null)}
                          onClick={(e) => {
                            if (isPreview) {
                              setProgressStep(null)
                              return
                            }
                            incSubskillRank(
                              skill.id,
                              sub.id,
                              sub.maxRank,
                              allocationStep(e, remaining),
                            )
                          }}
                          onContextMenu={(e) => {
                            e.preventDefault()
                            if (isPreview) {
                              setProgressStep(null)
                              return
                            }
                            decSubskillRank(
                              skill.id,
                              sub.id,
                              allocationStep(
                                e,
                                subskillRanks[subskillKey(skill.id, sub.id)] ?? 0,
                              ),
                            )
                          }}
                        >
                          {tile}
                        </button>
                      ) : (
                        <div
                          className={tileClass}
                          style={tileStyle}
                          onMouseEnter={onEnter}
                          onMouseLeave={() => setHover(null)}
                        >
                          {tile}
                        </div>
                      )}
                      {!isRoot && (
                        <span
                          className={`absolute left-1/2 top-[calc(100%+5px)] -translate-x-1/2 whitespace-nowrap font-mono text-[10px] tabular-nums ${
                            allocated ? 'text-accent-hot' : 'text-faint'
                          }`}
                        >
                          {rank}
                          <span className="text-node-base">/</span>
                          {sub.maxRank}
                        </span>
                      )}
                    </div>
                  )
                })}
              </div>
            </section>
            {(skill.subskills?.length ?? 0) === 0 && (
              <p className="mt-4 text-center font-mono text-[11px] uppercase tracking-[0.14em] text-faint">
                No subskills defined for {skill.name} yet.
              </p>
            )}
          </div>
          <ProgressionSlider
            total={total}
            value={progressStep}
            onChange={setProgressStep}
          />
        </div>
        <div className="flex items-center justify-between gap-3 border-t border-border px-6 py-2.5 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
          <span>L-Click add · R-Click remove</span>
          <span>Shift ×5 · Ctrl/Cmd+Shift all</span>
        </div>
      </Modal>
      {hover &&
        currentPerformance &&
        createPortal(
          <SubskillTooltip
            skill={skill}
            sub={hover.sub}
            rank={visibleRanks[subskillKey(skill.id, hover.sub.id)] ?? 0}
            x={hover.x}
            y={hover.y}
            isKeystone={hover.isKeystone}
            currentPerformance={currentPerformance}
            previewPerformance={
              debouncedHover === hover ? previewPerformance : null
            }
          />,
          document.body,
        )}
    </>
  )
}
