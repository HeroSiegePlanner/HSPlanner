import { useLayoutEffect, useMemo, useRef, useState } from 'react'
import {
  diffPerformanceDps,
  diffPerformanceStats,
  type BuildPerformance,
} from '../../utils/build/buildPerformance'
import type { NodeLineClassification } from '../../lib/calc/bridge'
import NetChangeRow from '../../components/NetChangeRow'
import type { TreeSocketContent } from '../../types'
import {
  TooltipHeader,
  TooltipSection,
  TooltipText,
  UnsupportedModsList,
} from '../../components/Tooltip'
import { TONE_BORDER, TONE_GLOW } from '../../components/tooltip-tones'
import type { TreeNodeInfo } from '../../utils/tree/treeStats'
import { tierLabel, tierTone, type TreeNode } from './treeData'
import { JewelrySocketSection } from './JewelrySocketSection'

export function NodeTooltip({
  node,
  info,
  classification,
  cursor,
  socketContent,
  isJewelry,
  isAllocated,
  currentPerformance,
  previewPerformance,
  singleNodePerformance,
  previewAddedCount,
  previewRemovedCount,
}: {
  node: TreeNode
  info: TreeNodeInfo | null
  classification: NodeLineClassification | null
  cursor: { x: number; y: number }
  socketContent: TreeSocketContent | null
  isJewelry: boolean
  isAllocated: boolean
  currentPerformance: BuildPerformance
  previewPerformance: BuildPerformance | null
  singleNodePerformance: BuildPerformance | null
  previewAddedCount: number
  previewRemovedCount: number
}) {
  const ref = useRef<HTMLDivElement>(null)
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null)
  const tone = tierTone(info?.n, node.tier)
  const tierName = tierLabel(info?.n, node.tier)
  const lineGroups = info ? classification : null

  const singleDpsDiffs = useMemo(
    () =>
      singleNodePerformance
        ? diffPerformanceDps(currentPerformance, singleNodePerformance)
        : [],
    [currentPerformance, singleNodePerformance],
  )
  const singleStatDiffs = useMemo(
    () =>
      singleNodePerformance
        ? diffPerformanceStats(currentPerformance, singleNodePerformance)
        : [],
    [currentPerformance, singleNodePerformance],
  )
  const pathDpsDiffs = useMemo(
    () =>
      previewPerformance
        ? diffPerformanceDps(currentPerformance, previewPerformance)
        : [],
    [currentPerformance, previewPerformance],
  )
  const pathStatDiffs = useMemo(
    () =>
      previewPerformance
        ? diffPerformanceStats(currentPerformance, previewPerformance)
        : [],
    [currentPerformance, previewPerformance],
  )
  const singleHasContent = singleDpsDiffs.length > 0 || singleStatDiffs.length > 0
  const pathHasContent = pathDpsDiffs.length > 0 || pathStatDiffs.length > 0
  const pathDiffersFromSingle =
    pathHasContent &&
    (previewAddedCount > 1 ||
      previewRemovedCount > 1 ||
      pathDpsDiffs.length !== singleDpsDiffs.length ||
      pathStatDiffs.length !== singleStatDiffs.length)
  const netChangeVisible = !isJewelry && (singleHasContent || pathHasContent)

  const [colCount, setColCount] = useState(1)
  const measuredColsRef = useRef(false)

  useLayoutEffect(() => {
    const el = ref.current
    if (!el) return
    const margin = 12
    const offset = 16
    const vw = window.innerWidth
    const vh = window.innerHeight
    const heightLimit = vh - 2 * margin

    if (!measuredColsRef.current) {
      const prevColumnCount = el.style.columnCount
      const prevMaxHeight = el.style.maxHeight
      el.style.columnCount = '1'
      el.style.maxHeight = 'none'
      const naturalHeight = el.scrollHeight
      el.style.columnCount = prevColumnCount
      el.style.maxHeight = prevMaxHeight

      const needed = Math.max(
        1,
        Math.min(Math.ceil(naturalHeight / heightLimit), 5),
      )
      measuredColsRef.current = true
      if (needed !== colCount) {
        setColCount(needed)
        return
      }
    }

    const rect = el.getBoundingClientRect()
    let left = cursor.x + offset
    if (left + rect.width + margin > vw) left = cursor.x - rect.width - offset
    left = Math.max(margin, Math.min(left, vw - rect.width - margin))
    let top = cursor.y + offset
    if (top + rect.height + margin > vh) top = cursor.y - rect.height - offset
    top = Math.max(margin, Math.min(top, vh - rect.height - margin))
    setPos({ left, top })
  }, [cursor.x, cursor.y, info, node.id, colCount])

  return (
    <div
      ref={ref}
      role="tooltip"
      className={`fixed z-[1000] bg-panel border ${TONE_BORDER[tone]} ${TONE_GLOW[tone]} rounded-[4px] overflow-hidden pointer-events-none select-none shadow-[0_8px_32px_rgba(0,0,0,0.8)]`}
      style={{
        left: pos?.left ?? -9999,
        top: pos?.top ?? -9999,
        opacity: pos ? 1 : 0,
        transition: 'opacity 80ms ease-out',
        columnCount: colCount,
        columnGap: colCount > 1 ? 0 : undefined,
        columnFill: 'auto',
        columnRule:
          colCount > 1 ? '1px solid var(--color-border)' : undefined,
        width: colCount > 1 ? `${colCount * 320}px` : undefined,
        minWidth: colCount === 1 ? 240 : undefined,
        maxWidth: colCount === 1 ? 360 : 'calc(100vw - 24px)',
        maxHeight: 'calc(100vh - 24px)',
      }}
    >
      <TooltipHeader
        title={info?.t ?? `Node #${node.id}`}
        subtitle={tierName}
        tone={tone}
      />
      {isJewelry ? (
        <JewelrySocketSection
          content={socketContent}
          isAllocated={isAllocated}
        />
      ) : (
        <>
          {lineGroups && lineGroups.parsed.length > 0 && (
            <TooltipSection>
              <div className="space-y-0.5">
                {lineGroups.parsed.map((line, i) => (
                  <TooltipText key={i}>{line}</TooltipText>
                ))}
              </div>
            </TooltipSection>
          )}
          {lineGroups && lineGroups.unsupported.length > 0 && (
            <TooltipSection>
              <UnsupportedModsList lines={lineGroups.unsupported} />
            </TooltipSection>
          )}
          {info?.note && (
            <TooltipSection>
              <div className="text-[12px] leading-[1.55] text-accent-hot italic">
                {info.note}
              </div>
            </TooltipSection>
          )}
          {netChangeVisible && (
            <TooltipSection>
              <div className="mb-2 flex items-center gap-2.5 font-mono text-[10px] font-semibold uppercase tracking-[0.18em] text-muted">
                <span>Net Change</span>
                <span className="h-px flex-1 bg-border" />
                {isAllocated && previewRemovedCount > 1 && (
                  <span className="font-normal tracking-[0.14em] text-faint">
                    −{previewRemovedCount} on click
                  </span>
                )}
                {!isAllocated && previewAddedCount > 1 && (
                  <span className="font-normal tracking-[0.14em] text-faint">
                    +{previewAddedCount} on click
                  </span>
                )}
              </div>
              {singleHasContent && (
                <div className={pathDiffersFromSingle ? 'mb-3' : undefined}>
                  <div className="mb-1 flex items-baseline gap-2 font-mono text-[9px] uppercase tracking-[0.16em] text-faint">
                    <span className="text-accent-deep">This Node</span>
                    {pathDiffersFromSingle && (
                      <span className="text-faint/70 normal-case">— hovered alone</span>
                    )}
                  </div>
                  {singleDpsDiffs.length > 0 && (
                    <div className="mb-1.5">
                      <div className="mb-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
                        Active Skill
                        {currentPerformance.activeSkillName ? (
                          <span className="ml-1 text-faint">
                            · {currentPerformance.activeSkillName}
                          </span>
                        ) : null}
                      </div>
                      <div className="space-y-0.5">
                        {singleDpsDiffs.map((d) => (
                          <NetChangeRow key={d.key} diff={d} />
                        ))}
                      </div>
                    </div>
                  )}
                  {singleStatDiffs.length > 0 && (
                    <div>
                      {singleDpsDiffs.length > 0 && (
                        <div className="mb-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
                          Stats
                        </div>
                      )}
                      <div className="space-y-0.5">
                        {singleStatDiffs.map((d) => (
                          <NetChangeRow key={d.key} diff={d} />
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              )}
              {pathDiffersFromSingle && (
                <div>
                  <div className="mb-1 flex items-baseline gap-2 font-mono text-[9px] uppercase tracking-[0.16em] text-faint">
                    <span className="text-accent-deep">
                      {isAllocated ? 'With Cleanup' : 'Full Path'}
                    </span>
                    <span className="text-faint/70 normal-case">
                      —{' '}
                      {isAllocated
                        ? `${previewRemovedCount} nodes lost`
                        : `${previewAddedCount} nodes allocated`}
                    </span>
                  </div>
                  {pathDpsDiffs.length > 0 && (
                    <div className="mb-1.5">
                      <div className="mb-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
                        Active Skill
                        {currentPerformance.activeSkillName ? (
                          <span className="ml-1 text-faint">
                            · {currentPerformance.activeSkillName}
                          </span>
                        ) : null}
                      </div>
                      <div className="space-y-0.5">
                        {pathDpsDiffs.map((d) => (
                          <NetChangeRow key={d.key} diff={d} />
                        ))}
                      </div>
                    </div>
                  )}
                  {pathStatDiffs.length > 0 && (
                    <div>
                      {pathDpsDiffs.length > 0 && (
                        <div className="mb-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
                          Stats
                        </div>
                      )}
                      <div className="space-y-0.5">
                        {pathStatDiffs.map((d) => (
                          <NetChangeRow key={d.key} diff={d} />
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              )}
            </TooltipSection>
          )}
        </>
      )}
      {info && info.g && info.g.length > 0 && (
        <TooltipSection className="bg-panel-2/40">
          <div className="flex flex-wrap gap-1">
            {info.g.map((tag, i) => (
              <span
                key={i}
                className="inline-block rounded-[2px] border border-border-2 px-1.5 py-0.5 text-[10px] uppercase tracking-[0.08em] text-muted"
              >
                {tag}
              </span>
            ))}
          </div>
        </TooltipSection>
      )}
      {!info && (
        <TooltipSection>
          <TooltipText>
            <span className="text-faint">No data available</span>
          </TooltipText>
        </TooltipSection>
      )}
    </div>
  )
}
