import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { FORGE_KIND_LABEL, getItem } from '../data'
import type { ForgeKind } from '../data'
import { useBuild } from '../store/build'
import { formatValue, statName } from '../utils/item/stats'
import type { SourceContribution, SourceType } from '../utils/item/stats'
import type { StatBreakdown, StatTypeSubtotal } from '../lib/calc/bridge'
import type { EquippedItem, RangedValue } from '../types'
import ItemTooltip from './ItemTooltip'
import Tooltip from './Tooltip'
import TreeNodeMiniMap from './TreeNodeMiniMap'
import { CornerMarks } from './CornerMarks'
import { findTreeNodeById, findTreeNodeByName } from '../utils/treeNodes'

const SOURCE_COLOR: Record<SourceType, string> = {
  class: 'text-text/70',
  allocated: 'text-accent',
  level: 'text-muted',
  attribute: 'text-emerald-400',
  item: 'text-cyan-400',
  socket: 'text-pink-400',
  skill: 'text-yellow-300',
  subskill: 'text-yellow-300',
  custom: 'text-orange-400',
  tree: 'text-amber-400',
}

const SOURCE_LABEL: Record<SourceType, string> = {
  class: 'Class',
  allocated: 'Allocated',
  level: 'Level',
  attribute: 'Attribute',
  item: 'Item',
  socket: 'Socket',
  skill: 'Skill',
  subskill: 'Subtree',
  custom: 'Config',
  tree: 'Tree',
}

const FORGE_COLOR: Record<ForgeKind, string> = {
  satanic_crystal: 'text-stat-red',
}

function extractItemName(label: string): string | null {
  const trimmed = label.trim()
  if (!trimmed) return null
  const socketInItem = trimmed.match(
    /\bin (.+?) #\d+(?:\s*\((?:Rainbow|Transform)\))?\s*$/,
  )
  if (socketInItem?.[1]) return socketInItem[1].trim()
  const parens = trimmed.match(/\(([^()]+)\)\s*$/)
  const inner = parens?.[1]
  if (inner) return inner.trim()
  return trimmed
}

function extractTreeRef(label: string): { id?: number; name?: string } | null {
  const trimmed = label.trim()
  const socketMatch = trimmed.match(/\(Tree Socket #(\d+)\)\s*$/)
  if (socketMatch?.[1]) {
    return { id: Number(socketMatch[1]) }
  }
  if (!trimmed.startsWith('Tree:')) return null
  const idMatch = trimmed.match(/^Tree:\s*(.+?)\s+#(\d+)\b/)
  const idName = idMatch?.[1]
  const idStr = idMatch?.[2]
  if (idName && idStr) {
    return { id: Number(idStr), name: idName.trim() }
  }
  const legacy = trimmed.match(/^Tree:\s*([^:(]+?)(?:\s*\(conditional\)|:|$)/)
  const name = legacy?.[1]?.trim()
  return name ? { name } : null
}

interface Props {
  statKey: string
  sources: SourceContribution[]
  moreSources?: SourceContribution[]
  children: ReactNode
  breakdown?: StatBreakdown | null
  onRequestBreakdown?: () => void
  title?: string
}

function magnitudeOf(value: RangedValue): number {
  if (typeof value === 'number') return Math.abs(value)
  return Math.max(Math.abs(value[0]), Math.abs(value[1]))
}

function sortByMagnitude(sources: SourceContribution[]): SourceContribution[] {
  return [...sources].sort((a, b) => magnitudeOf(b.value) - magnitudeOf(a.value))
}

function orderSources(sources: SourceContribution[]): SourceContribution[] {
  const forgedByParent = new Map<string, SourceContribution[]>()
  for (const s of sources) {
    if (!s.forge) continue
    const list = forgedByParent.get(s.forge.itemName) ?? []
    list.push(s)
    forgedByParent.set(s.forge.itemName, list)
  }
  if (forgedByParent.size === 0) return sources

  const out: SourceContribution[] = []
  const consumed = new Set<SourceContribution>()
  for (const s of sources) {
    if (s.forge) continue
    out.push(s)
    if (s.sourceType !== 'item') continue
    const children = forgedByParent.get(s.label)
    if (!children) continue
    for (const c of children) {
      if (consumed.has(c)) continue
      out.push(c)
      consumed.add(c)
    }
  }
  for (const s of sources) {
    if (s.forge && !consumed.has(s)) out.push(s)
  }
  return out
}

function SourceItem({
  s,
  statKey,
  index,
  itemByName,
}: {
  s: SourceContribution
  statKey: string
  index: number
  itemByName?: Map<string, EquippedItem>
}) {
  if (s.forge) {
    const color = FORGE_COLOR[s.forge.kind]
    const equipped = itemByName?.get(s.forge.itemName)
    const forgedRow = (
      <div className="flex items-baseline justify-between gap-2 leading-[1.5] pl-3 -mx-1 px-1 rounded-[2px] hover:bg-accent-hot/8 transition-colors">
        <span className="flex items-baseline gap-1 min-w-0">
          <span className="text-text/40 shrink-0">⤷</span>
          <span className={`shrink-0 italic ${color}`}>Forged modifier</span>
          <span className="text-text/60 truncate">
            ({FORGE_KIND_LABEL[s.forge.kind]})
          </span>
        </span>
        <span className="font-mono tabular-nums shrink-0 text-accent-hot">
          {formatValue(s.value, statKey)}
        </span>
      </div>
    )
    if (equipped) {
      return (
        <li key={index}>
          <ItemTooltip equipped={equipped} placement="left" zIndex={1200}>
            {forgedRow}
          </ItemTooltip>
        </li>
      )
    }
    return <li key={index}>{forgedRow}</li>
  }
  const displayLabel =
    s.sourceType === 'tree'
      ? s.label.replace(/\s+#\d+/, '')
      : s.label
  const labelRow = (
    <div className="flex items-baseline justify-between gap-2 leading-[1.5] -mx-1 px-1 rounded-[2px] hover:bg-accent-hot/8 transition-colors">
      <span className="flex items-baseline gap-1.5 min-w-0">
        <span
          className={`text-[9px] uppercase tracking-wider shrink-0 ${SOURCE_COLOR[s.sourceType]}`}
        >
          {SOURCE_LABEL[s.sourceType]}
        </span>
        <span className="text-text/80 truncate">{displayLabel}</span>
      </span>
      <span className="font-mono tabular-nums shrink-0 text-accent-hot">
        {formatValue(s.value, statKey)}
      </span>
    </div>
  )
  if (
    itemByName &&
    (s.sourceType === 'item' || s.sourceType === 'socket')
  ) {
    const itemName = extractItemName(s.label)
    const equipped = itemName ? itemByName.get(itemName) : undefined
    if (equipped) {
      return (
        <li key={index}>
          <ItemTooltip equipped={equipped} placement="left" zIndex={1200}>
            {labelRow}
          </ItemTooltip>
        </li>
      )
    }
  }
  if (s.sourceType === 'tree') {
    const ref = extractTreeRef(s.label)
    let treeNode = ref?.id != null ? findTreeNodeById(ref.id) : undefined
    if (!treeNode && ref?.name) {
      treeNode = findTreeNodeByName(ref.name)
    }
    if (treeNode) {
      return (
        <li key={index}>
          <Tooltip
            placement="left"
            zIndex={1200}
            content={<TreeNodeMiniMap node={treeNode} />}
          >
            {labelRow}
          </Tooltip>
        </li>
      )
    }
  }
  return <li key={index}>{labelRow}</li>
}

function sectionLabel(text: string, trailing?: string) {
  return (
    <div className="flex items-center justify-between px-3 py-1 text-[10px] font-semibold uppercase tracking-[0.12em] text-accent-hot/80 bg-accent-deep/10 border-b border-border/40">
      <span>{text}</span>
      {trailing && (
        <span className="font-mono normal-case text-text/70 tracking-normal">
          {trailing}
        </span>
      )}
    </div>
  )
}

function sumRangePct(sources: SourceContribution[]): [number, number] {
  let min = 0
  let max = 0
  for (const s of sources) {
    const [smin, smax] =
      typeof s.value === 'number' ? [s.value, s.value] : s.value
    min += smin
    max += smax
  }
  return [min, max]
}

function fmtSubtotal([min, max]: [number, number]): string {
  const round = (n: number) =>
    Number.isInteger(n) ? n : Math.round(n * 100) / 100
  if (min === max) return `${min >= 0 ? '+' : ''}${round(min)}%`
  return `${min >= 0 ? '+' : ''}${round(min)}–${round(max)}%`
}

function fmtMult([min, max]: [number, number]): string {
  const round = (n: number) => Math.round(n * 1000) / 1000
  const a = round(1 + min / 100)
  const b = round(1 + max / 100)
  if (a === b) return `×${a}`
  return `×${a}–${b}`
}

const TOOLTIP_WIDTH = 320
const TOOLTIP_GAP = 6
const VIEWPORT_PADDING = 8

function rangedToPair(v: RangedValue): [number, number] {
  return typeof v === 'number' ? [v, v] : v
}

function fmtPctRange(v: RangedValue, signed: boolean = true): string {
  const round = (n: number) =>
    Number.isInteger(n) ? n : Math.round(n * 100) / 100
  const [min, max] = rangedToPair(v)
  const sign = (n: number) => (signed && n >= 0 ? '+' : '')
  if (min === max) return `${sign(min)}${round(min)}%`
  return `${sign(min)}${round(min)}–${round(max)}%`
}

function fmtMultRange(v: RangedValue): string {
  const round = (n: number) => Math.round(n * 1000) / 1000
  const [min, max] = rangedToPair(v)
  const a = round(1 + min / 100)
  const b = round(1 + max / 100)
  if (a === b) return `×${a}`
  return `×${a}–${b}`
}

function fmtFlatRange(v: RangedValue): string {
  const round = (n: number) =>
    Number.isInteger(n) ? n : Math.round(n * 100) / 100
  const [min, max] = rangedToPair(v)
  const sign = (n: number) => (n >= 0 ? '+' : '')
  if (min === max) return `${sign(min)}${round(min)}`
  return `${sign(min)}${round(min)}–${round(max)}`
}

function fmtBreakdownValue(v: RangedValue, isPercent: boolean): string {
  return isPercent ? fmtPctRange(v) : fmtFlatRange(v)
}

function isZeroRanged(v: RangedValue): boolean {
  const [min, max] = rangedToPair(v)
  return min === 0 && max === 0
}

function humanizeStatKey(key: string): string {
  return key
    .split('_')
    .filter(Boolean)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ')
}

function CalculationSection({ breakdown }: { breakdown: StatBreakdown }) {
  const {
    hasMore,
    hasIncreased,
    additiveSum,
    increasedSum,
    moreSum,
    combined,
    isPercent,
  } = breakdown
  const hasAnyMultiplier = hasIncreased || hasMore
  if (!hasAnyMultiplier) {
    if (isZeroRanged(combined)) return null
    return (
      <div className="border-b border-border/40">
        {sectionLabel('Calculation', fmtBreakdownValue(combined, isPercent))}
        <div className="px-3 py-2 text-[11px]">
          <div className="flex items-baseline justify-between gap-2 text-text/70">
            <span>Total</span>
            <span className="font-mono tabular-nums text-accent-hot">
              {fmtBreakdownValue(combined, isPercent)}
            </span>
          </div>
        </div>
      </div>
    )
  }
  if (hasIncreased || !isPercent) {
    return (
      <div className="border-b border-border/40">
        {sectionLabel('Calculation', fmtFlatRange(combined))}
        <div className="space-y-1 px-3 py-2 text-[11px]">
          <div className="flex items-baseline justify-between gap-2 text-text/70">
            <span>Additive (flat)</span>
            <span className="font-mono tabular-nums text-text/85">
              {fmtFlatRange(additiveSum)}
            </span>
          </div>
          {hasIncreased && (
            <div className="flex items-baseline justify-between gap-2 text-text/70">
              <span>Increased (+%)</span>
              <span className="font-mono tabular-nums text-text/85">
                {fmtPctRange(increasedSum)}
              </span>
            </div>
          )}
          {hasMore && (
            <div className="flex items-baseline justify-between gap-2 text-text/70">
              <span>More (×)</span>
              <span className="font-mono tabular-nums text-text/85">
                {fmtMultRange(moreSum)}
              </span>
            </div>
          )}
          <div className="border-t border-dashed border-border/60 pt-1 text-text/40 font-mono text-[10px] leading-tight">
            flat × (1 + inc/100) × (1 + more/100)
          </div>
          <div className="flex items-baseline justify-between gap-2 border-t border-border/40 pt-1">
            <span className="font-semibold text-accent-hot/90">Combined</span>
            <span className="font-mono tabular-nums text-accent-hot">
              {fmtFlatRange(combined)}
            </span>
          </div>
        </div>
      </div>
    )
  }
  return (
    <div className="border-b border-border/40">
      {sectionLabel('Calculation', fmtPctRange(combined))}
      <div className="space-y-1 px-3 py-2 text-[11px]">
        <div className="flex items-baseline justify-between gap-2 text-text/70">
          <span>Additive (+)</span>
          <span className="font-mono tabular-nums text-text/85">
            {fmtPctRange(additiveSum)}
          </span>
        </div>
        <div className="flex items-baseline justify-between gap-2 text-text/70">
          <span>Multiplicative (×)</span>
          <span className="font-mono tabular-nums text-text/85">
            {fmtMultRange(moreSum)}
          </span>
        </div>
        <div className="border-t border-dashed border-border/60 pt-1 text-text/40 font-mono text-[10px] leading-tight">
          (1 + add/100) × (1 + more/100) − 1
        </div>
        <div className="flex items-baseline justify-between gap-2 border-t border-border/40 pt-1">
          <span className="font-semibold text-accent-hot/90">Combined</span>
          <span className="font-mono tabular-nums text-accent-hot">
            {fmtPctRange(combined)}
          </span>
        </div>
      </div>
    </div>
  )
}

function BySourceTypeSection({ breakdown }: { breakdown: StatBreakdown }) {
  const {
    additiveByType,
    increasedByType,
    moreByType,
    hasIncreased,
    hasMore,
    isPercent,
  } = breakdown
  if (
    additiveByType.length === 0 &&
    increasedByType.length === 0 &&
    moreByType.length === 0
  ) {
    return null
  }
  const additiveAsPercent = isPercent && !hasIncreased
  const renderRow = (
    sub: StatTypeSubtotal,
    kind: 'additive' | 'increased' | 'more',
  ) => (
    <div
      key={`${sub.sourceType}-${kind}`}
      className="flex items-baseline justify-between gap-2 px-3 py-0.5 text-[11px]"
    >
      <span className="flex items-baseline gap-1.5 min-w-0">
        <span
          className={`text-[9px] uppercase tracking-wider shrink-0 ${SOURCE_COLOR[sub.sourceType]}`}
        >
          {SOURCE_LABEL[sub.sourceType]}
        </span>
        <span className="text-text/60 font-mono text-[10px]">
          ×{sub.count}
        </span>
      </span>
      <span className="font-mono tabular-nums text-accent-hot">
        {kind === 'more'
          ? fmtMultRange(sub.sum)
          : kind === 'increased'
            ? fmtPctRange(sub.sum)
            : fmtBreakdownValue(sub.sum, additiveAsPercent)}
      </span>
    </div>
  )
  return (
    <div className="border-b border-border/40">
      {sectionLabel('By source')}
      <div className="py-1">
        {additiveByType.map((s) => renderRow(s, 'additive'))}
        {hasIncreased && increasedByType.length > 0 && (
          <>
            <div className="my-1 mx-3 border-t border-dashed border-border/40" />
            <div className="px-3 py-0.5 text-[9px] uppercase tracking-[0.14em] text-text/40">
              Increased
            </div>
            {increasedByType.map((s) => renderRow(s, 'increased'))}
          </>
        )}
        {hasMore && moreByType.length > 0 && (
          <>
            <div className="my-1 mx-3 border-t border-dashed border-border/40" />
            <div className="px-3 py-0.5 text-[9px] uppercase tracking-[0.14em] text-text/40">
              Multiplicative
            </div>
            {moreByType.map((s) => renderRow(s, 'more'))}
          </>
        )}
      </div>
    </div>
  )
}

function SourcesBody({
  statKey,
  sources,
  moreSources,
  hasMore,
  extended,
  breakdown,
  itemByName,
}: {
  statKey: string
  sources: SourceContribution[]
  moreSources?: SourceContribution[]
  hasMore: boolean
  extended?: boolean
  breakdown?: StatBreakdown | null
  itemByName?: Map<string, EquippedItem>
}) {
  const orderedAdd = orderSources(sortByMagnitude(sources))
  const orderedMore = hasMore
    ? orderSources(sortByMagnitude(moreSources!))
    : []
  const addSubtotal = sumRangePct(sources)
  const moreSubtotal = hasMore
    ? sumRangePct(moreSources!)
    : ([0, 0] as [number, number])
  const breakdownHead = extended ? (
    breakdown ? (
      <>
        <CalculationSection breakdown={breakdown} />
        <BySourceTypeSection breakdown={breakdown} />
      </>
    ) : (
      <div className="border-b border-border/40 px-3 py-2 text-[11px] italic text-text/50">
        Loading breakdown...
      </div>
    )
  ) : null
  if (hasMore) {
    return (
      <>
        {breakdownHead}
        {sectionLabel('Additive (+)', fmtSubtotal(addSubtotal))}
        <ul className="space-y-1 px-3 py-2">
          {orderedAdd.length === 0 ? (
            <li className="italic text-text/40">No additive sources</li>
          ) : (
            orderedAdd.map((s, i) => (
              <SourceItem key={i} s={s} statKey={statKey} index={i} itemByName={itemByName} />
            ))
          )}
        </ul>
        {sectionLabel('Multiplicative (Total)', fmtMult(moreSubtotal))}
        <ul className="space-y-1 px-3 py-2">
          {orderedMore.map((s, i) => (
            <SourceItem key={i} s={s} statKey={statKey} index={i} itemByName={itemByName} />
          ))}
        </ul>
      </>
    )
  }
  return (
    <>
      {breakdownHead}
      <ul className="space-y-1 px-3 py-2">
        {orderedAdd.map((s, i) => (
          <SourceItem key={i} s={s} statKey={statKey} index={i} itemByName={itemByName} />
        ))}
      </ul>
    </>
  )
}

export default function SourceTooltip({
  statKey,
  sources,
  moreSources,
  children,
  breakdown,
  onRequestBreakdown,
  title,
}: Props) {
  const hasMore = !!moreSources && moreSources.length > 0
  const triggerRef = useRef<HTMLDivElement>(null)
  const tooltipRef = useRef<HTMLDivElement>(null)
  const [open, setOpen] = useState(false)
  const [pinned, setPinned] = useState(false)
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null)
  const onRequestBreakdownRef = useRef(onRequestBreakdown)
  useEffect(() => {
    onRequestBreakdownRef.current = onRequestBreakdown
  })
  useEffect(() => {
    if (!pinned || breakdown != null) return
    onRequestBreakdownRef.current?.()
  }, [pinned, breakdown])
  const inventory = useBuild((s) => s.inventory)
  const itemByName = useMemo(() => {
    const map = new Map<string, EquippedItem>()
    for (const equipped of Object.values(inventory)) {
      if (!equipped) continue
      const base = getItem(equipped.baseId)
      if (!base || !base.name) continue
      if (!map.has(base.name)) map.set(base.name, equipped)
    }
    return map
  }, [inventory])

  useLayoutEffect(() => {
    if (!open || pinned || !triggerRef.current) return
    const reposition = () => {
      const trigger = triggerRef.current
      if (!trigger) return
      const rect = trigger.getBoundingClientRect()
      const tooltipEl = tooltipRef.current
      const tooltipH = tooltipEl?.offsetHeight ?? 0
      const tooltipW = tooltipEl?.offsetWidth ?? TOOLTIP_WIDTH
      const vw = window.innerWidth
      const vh = window.innerHeight
      let left = rect.right - tooltipW
      let top = rect.bottom + TOOLTIP_GAP
      if (top + tooltipH + VIEWPORT_PADDING > vh) {
        const above = rect.top - tooltipH - TOOLTIP_GAP
        if (above >= VIEWPORT_PADDING) top = above
        else top = Math.max(VIEWPORT_PADDING, vh - tooltipH - VIEWPORT_PADDING)
      }
      if (left + tooltipW + VIEWPORT_PADDING > vw)
        left = vw - tooltipW - VIEWPORT_PADDING
      if (left < VIEWPORT_PADDING) left = VIEWPORT_PADDING
      setPos({ left, top })
    }
    reposition()
    window.addEventListener('scroll', reposition, true)
    window.addEventListener('resize', reposition)
    return () => {
      window.removeEventListener('scroll', reposition, true)
      window.removeEventListener('resize', reposition)
      setPos(null)
    }
  }, [open, pinned])

  useEffect(() => {
    if (!pinned) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setPinned(false)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [pinned])

  if (sources.length === 0 && !hasMore) return <>{children}</>

  const openPinned = () => {
    setPinned(true)
    setOpen(false)
  }
  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault()
    openPinned()
  }
  const handleClick = (e: React.MouseEvent) => {
    if (e.defaultPrevented) return
    const selection = window.getSelection()
    if (selection && selection.toString().length > 0) return
    openPinned()
  }

  return (
    <>
      <div
        ref={triggerRef}
        className="cursor-help"
        onMouseEnter={() => setOpen(true)}
        onMouseLeave={() => setOpen(false)}
        onFocus={() => setOpen(true)}
        onBlur={() => setOpen(false)}
        onClick={handleClick}
        onContextMenu={handleContextMenu}
      >
        {children}
      </div>
      {open &&
        !pinned &&
        createPortal(
          <div
            ref={tooltipRef}
            className="fixed z-[1000] overflow-hidden rounded-[4px] border border-accent-deep/60 bg-panel text-xs shadow-[0_8px_32px_rgba(0,0,0,0.8)] pointer-events-none"
            style={{
              left: pos?.left ?? -9999,
              top: pos?.top ?? -9999,
              width: TOOLTIP_WIDTH,
              opacity: pos ? 1 : 0,
              transition: 'opacity 80ms ease-out',
            }}
          >
            <div
              className="flex items-center justify-between gap-2 border-b border-border/70 px-3 py-1.5"
              style={{
                background:
                  'linear-gradient(180deg, rgba(201,165,90,0.14), rgba(201,165,90,0.04))',
              }}
            >
              <span className="text-[10px] font-semibold uppercase tracking-[0.12em] text-accent-hot">
                Sources
              </span>
              <span className="text-[9px] uppercase tracking-[0.12em] text-text/40">
                Right-click to pin
              </span>
            </div>
            <SourcesBody
              statKey={statKey}
              sources={sources}
              moreSources={moreSources}
              hasMore={hasMore}
              itemByName={itemByName}
            />
          </div>,
          document.body,
        )}
      {pinned &&
        createPortal(
          <div
            className="fixed inset-0 z-[1100] flex items-center justify-center backdrop-blur-sm"
            style={{
              background:
                'radial-gradient(ellipse at 50% 0%, rgba(201,165,90,0.06), rgba(0,0,0,0.78) 60%)',
            }}
            onClick={(e) => {
              const tooltips = document.querySelectorAll('[role="tooltip"]')
              for (const tt of tooltips) {
                const rect = tt.getBoundingClientRect()
                if (rect.width === 0 || rect.height === 0) continue
                if (
                  e.clientX >= rect.left &&
                  e.clientX <= rect.right &&
                  e.clientY >= rect.top &&
                  e.clientY <= rect.bottom
                ) {
                  return
                }
              }
              setPinned(false)
            }}
            onContextMenu={(e) => {
              e.preventDefault()
              setPinned(false)
            }}
          >
            <div
              className="relative flex max-h-[80vh] w-[min(90vw,520px)] flex-col overflow-hidden rounded-[6px] border border-border text-xs"
              style={{
                background:
                  'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 80%, transparent))',
                boxShadow:
                  'inset 0 1px 0 rgba(255,255,255,0.02), 0 24px 64px rgba(0,0,0,0.7)',
              }}
              onClick={(e) => e.stopPropagation()}
              onContextMenu={(e) => e.stopPropagation()}
            >
              <CornerMarks size={8} opacity={0.5} />
              <div
                className="flex items-center justify-between gap-2 border-b border-border/70 px-4 py-2"
                style={{
                  background:
                    'linear-gradient(180deg, rgba(201,165,90,0.18), rgba(201,165,90,0.06))',
                }}
              >
                <div className="flex items-baseline gap-2 min-w-0">
                  <span className="text-[11px] font-semibold uppercase tracking-[0.14em] text-accent-hot truncate">
                    {(() => {
                      const resolved = title ?? statName(statKey)
                      return resolved === statKey
                        ? humanizeStatKey(statKey)
                        : resolved
                    })()}
                  </span>
                  <span className="font-mono text-[10px] text-text/60 truncate">
                    {statKey}
                  </span>
                </div>
                <button
                  type="button"
                  onClick={() => setPinned(false)}
                  className="flex h-6 w-6 items-center justify-center rounded-[2px] text-[14px] leading-none text-faint transition-colors hover:bg-accent-hot/10 hover:text-accent-hot"
                  aria-label="Close source breakdown"
                  title="Close (Esc)"
                >
                  ×
                </button>
              </div>
              <div className="overflow-y-auto">
                <SourcesBody
                  statKey={statKey}
                  sources={sources}
                  moreSources={moreSources}
                  hasMore={hasMore}
                  extended
                  breakdown={breakdown}
                  itemByName={itemByName}
                />
              </div>
              <div className="border-t border-border/70 bg-panel-2/50 px-4 py-1.5 font-mono text-[9px] uppercase tracking-[0.14em] text-faint">
                <kbd className="rounded-[2px] border border-border-2 bg-bg/60 px-1.5 py-[1px]">
                  Esc
                </kbd>
                <span className="ml-1.5">close</span>
                <span className="mx-2 text-text/30">·</span>
                <span>click outside to dismiss</span>
              </div>
            </div>
          </div>,
          document.body,
        )}
    </>
  )
}
