import { useLayoutEffect, useRef, useState } from 'react'
import { etherTree } from '@data'
import type { EtherNode } from '../../types'
import { clampTooltipToViewport } from '../../utils/tooltipPosition'
import {
  TooltipFooter,
  TooltipHeader,
  TooltipSection,
  TooltipStat,
  TooltipText,
} from '../../components/ui/Tooltip'
import { TONE_BORDER, TONE_GLOW } from '../../components/tooltipTones'
import type { TooltipTone } from '../../components/tooltipTones'
import {
  etherRegionLabel,
  formatEtherTotal,
  parseEtherValue,
} from '../../utils/build/etherSummary'
import { etherTypeLabel } from './etherData'

function etherTone(t: EtherNode['t']): TooltipTone {
  if (t === 'root') return 'angelic'
  if (t === 'big') return 'rare'
  return 'neutral'
}

interface Props {
  node: EtherNode
  cursor: { x: number; y: number }
  isAllocated: boolean
  allocatedSameKey: number
  previewAddedCount: number
  previewRemovedCount: number
}

export function EtherNodeTooltip({
  node,
  cursor,
  isAllocated,
  allocatedSameKey,
  previewAddedCount,
  previewRemovedCount,
}: Props) {
  const ref = useRef<HTMLDivElement>(null)
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null)

  useLayoutEffect(() => {
    const el = ref.current
    if (!el) return
    const rect = el.getBoundingClientRect()
    setPos(
      clampTooltipToViewport(
        cursor,
        { width: rect.width, height: rect.height },
        { width: window.innerWidth, height: window.innerHeight },
        16,
      ),
    )
  }, [cursor, node.id])

  const stat = etherTree.stats[node.key]
  if (!stat) return null

  const tone = etherTone(node.t)
  const { num, isPercent } = parseEtherValue(stat.value)
  const clickHint = isAllocated
    ? previewRemovedCount > 1
      ? `Click to remove — ${previewRemovedCount} nodes lost`
      : 'Click to remove'
    : previewAddedCount > 1
      ? `Click to allocate — ${previewAddedCount} nodes`
      : 'Click to allocate'

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
        minWidth: 240,
        maxWidth: 480,
      }}
    >
      <TooltipHeader
        title={stat.label}
        subtitle={`${etherTypeLabel(node.t)} · ${etherRegionLabel(node.key)}`}
        tone={tone}
      />
      <TooltipSection>
        <TooltipText>
          +{stat.value} {stat.desc}
        </TooltipText>
      </TooltipSection>
      {allocatedSameKey > 0 && (
        <TooltipSection>
          <TooltipStat
            label="Allocated of this notable"
            value={`${allocatedSameKey}x`}
          />
          <TooltipStat
            label="Current total"
            value={formatEtherTotal({
              total: Math.round(num * allocatedSameKey * 100) / 100,
              isPercent,
            })}
          />
        </TooltipSection>
      )}
      <TooltipFooter>{clickHint}</TooltipFooter>
    </div>
  )
}
