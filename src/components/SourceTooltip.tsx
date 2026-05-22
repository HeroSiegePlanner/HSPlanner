import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { FORGE_KIND_LABEL } from '../data'
import type { ForgeKind } from '../data'
import { formatValue } from '../utils/item/stats'
import type { SourceContribution, SourceType } from '../utils/item/stats'

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
  satanic_crystal: 'text-red-300',
}

interface Props {
  statKey: string
  sources: SourceContribution[]
  moreSources?: SourceContribution[]
  children: ReactNode
}

function orderSources(sources: SourceContribution[]): SourceContribution[] {
  // Reorders a flat list of source contributions so each item-source is immediately followed by its forged crystal mod children, with any orphaned forged mods appended at the end. Used by SourceTooltip to render the parent/child indented layout.
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
}: {
  s: SourceContribution
  statKey: string
  index: number
}) {
  // Renders a single contribution row inside the tooltip: an indented "Forged modifier" entry when the source carries forge metadata, or a normal "tag · label · value" row for everything else. Used by SourceTooltip's list rendering.
  if (s.forge) {
    const color = FORGE_COLOR[s.forge.kind]
    return (
      <li
        key={index}
        className="flex items-baseline justify-between gap-2 leading-[1.5] pl-3"
      >
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
      </li>
    )
  }
  return (
    <li
      key={index}
      className="flex items-baseline justify-between gap-2 leading-[1.5]"
    >
      <span className="flex items-baseline gap-1.5 min-w-0">
        <span
          className={`text-[9px] uppercase tracking-wider shrink-0 ${SOURCE_COLOR[s.sourceType]}`}
        >
          {SOURCE_LABEL[s.sourceType]}
        </span>
        <span className="text-text/80 truncate">{s.label}</span>
      </span>
      <span className="font-mono tabular-nums shrink-0 text-accent-hot">
        {formatValue(s.value, statKey)}
      </span>
    </li>
  )
}

function sectionLabel(text: string, trailing?: string) {
  // Renders the gold sub-section bar inside the tooltip with an optional right-aligned trailing string (e.g. a subtotal). Used by SourceTooltip to separate the "Additive" and "Multiplicative (Total)" groups.
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
  // Sums a list of source contributions into a single `[min, max]` percentage range. Used to compute the additive and multiplicative subtotals shown in the tooltip's section bars.
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
  // Formats an additive subtotal range as a signed percentage (e.g. "+12%" or "+12–18%"). Used by SourceTooltip's "Additive" section header.
  const round = (n: number) =>
    Number.isInteger(n) ? n : Math.round(n * 100) / 100
  if (min === max) return `${min >= 0 ? '+' : ''}${round(min)}%`
  return `${min >= 0 ? '+' : ''}${round(min)}–${round(max)}%`
}

function fmtMult([min, max]: [number, number]): string {
  // Formats a Total / multiplicative range as a `×N` (or `×A–B`) multiplier string. Used by SourceTooltip's "Multiplicative" section header so the user can see the effective multiplier at a glance.
  const round = (n: number) => Math.round(n * 1000) / 1000
  const a = round(1 + min / 100)
  const b = round(1 + max / 100)
  if (a === b) return `×${a}`
  return `×${a}–${b}`
}

const TOOLTIP_WIDTH = 320
const TOOLTIP_GAP = 6
const VIEWPORT_PADDING = 8

function SourcesBody({
  statKey,
  sources,
  moreSources,
  hasMore,
}: {
  statKey: string
  sources: SourceContribution[]
  moreSources?: SourceContribution[]
  hasMore: boolean
}) {
  // Shared body of the source breakdown — used by both the hover tooltip and the pinned (right-click) modal so the rendering stays in one place.
  const orderedAdd = orderSources(sources)
  const orderedMore = hasMore ? orderSources(moreSources!) : []
  const addSubtotal = sumRangePct(sources)
  const moreSubtotal = hasMore
    ? sumRangePct(moreSources!)
    : ([0, 0] as [number, number])
  if (hasMore) {
    return (
      <>
        {sectionLabel('Additive (+)', fmtSubtotal(addSubtotal))}
        <ul className="space-y-1 px-3 py-2">
          {orderedAdd.length === 0 ? (
            <li className="italic text-text/40">No additive sources</li>
          ) : (
            orderedAdd.map((s, i) => (
              <SourceItem key={i} s={s} statKey={statKey} index={i} />
            ))
          )}
        </ul>
        {sectionLabel('Multiplicative (Total)', fmtMult(moreSubtotal))}
        <ul className="space-y-1 px-3 py-2">
          {orderedMore.map((s, i) => (
            <SourceItem key={i} s={s} statKey={statKey} index={i} />
          ))}
        </ul>
      </>
    )
  }
  return (
    <ul className="space-y-1 px-3 py-2">
      {orderedAdd.map((s, i) => (
        <SourceItem key={i} s={s} statKey={statKey} index={i} />
      ))}
    </ul>
  )
}

export default function SourceTooltip({
  statKey,
  sources,
  moreSources,
  children,
}: Props) {
  // Stat-source breakdown popover. Hover shows a small (~320px) anchored tooltip, right-click pins a larger modal (centered, scrollable up to 80vh) for stats with too many contributors to fit in the hover tooltip. Both render into document.body via portal so ancestor `overflow-hidden` doesn't clip them.
  const hasMore = !!moreSources && moreSources.length > 0
  const triggerRef = useRef<HTMLDivElement>(null)
  const tooltipRef = useRef<HTMLDivElement>(null)
  const [open, setOpen] = useState(false)
  const [pinned, setPinned] = useState(false)
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null)

  // Position the hover tooltip relative to its trigger and clamp to the viewport. Re-runs after mount (so we can read tooltip's real height) and on scroll/resize while open.
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

  // ESC closes the pinned modal.
  useEffect(() => {
    if (!pinned) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setPinned(false)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [pinned])

  if (sources.length === 0 && !hasMore) return <>{children}</>

  const handleContextMenu = (e: React.MouseEvent) => {
    // Right-click pins the breakdown into a centered, scrollable modal so dense stats (lots of contributors) can be browsed without going off-screen.
    e.preventDefault()
    setPinned(true)
    setOpen(false)
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
            />
          </div>,
          document.body,
        )}
      {pinned &&
        createPortal(
          <div
            className="fixed inset-0 z-[1100] flex items-center justify-center bg-black/50 backdrop-blur-[2px]"
            onClick={() => setPinned(false)}
            onContextMenu={(e) => {
              // Right-click outside the modal (i.e. on the backdrop) also closes it; right-click on the modal itself stays open so links/text are still selectable.
              e.preventDefault()
              setPinned(false)
            }}
          >
            <div
              className="relative flex max-h-[80vh] w-[min(90vw,520px)] flex-col overflow-hidden rounded-[6px] border border-accent-deep bg-panel text-xs shadow-[0_16px_48px_rgba(0,0,0,0.85)]"
              onClick={(e) => e.stopPropagation()}
              onContextMenu={(e) => e.stopPropagation()}
            >
              <div
                className="flex items-center justify-between gap-2 border-b border-border/70 px-4 py-2"
                style={{
                  background:
                    'linear-gradient(180deg, rgba(201,165,90,0.18), rgba(201,165,90,0.06))',
                }}
              >
                <div className="flex items-baseline gap-2">
                  <span className="text-[11px] font-semibold uppercase tracking-[0.14em] text-accent-hot">
                    Sources
                  </span>
                  <span className="font-mono text-[10px] text-text/60">
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
