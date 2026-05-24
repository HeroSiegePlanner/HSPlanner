import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { FORGE_KIND_LABEL, getItem } from '../data'
import type { ForgeKind } from '../data'
import { useBuild } from '../store/build'
import { formatValue, statName } from '../utils/stats'
import type { SourceContribution, SourceType } from '../utils/stats'
import type { StatBreakdown, StatTypeSubtotal } from '../lib/calc/bridge'
import type { EquippedItem, RangedValue } from '../types'
import ItemTooltip from './ItemTooltip'
import Tooltip from './Tooltip'
import TreeNodeMiniMap from './TreeNodeMiniMap'
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
  satanic_crystal: 'text-red-300',
}

// Extracts a worn-item display name from a source-contribution label. Rust
// produces labels in four shapes:
//   "Crown of Flames"                         (implicit / set bonus)
//   "Inferno (Crown of Flames)"               (affix / runeword / socketed gem)
//   "Augment: X Lv 5 (Crown of Flames)"       (augment)
// We try the trailing parenthesised group first; if nothing is wrapped, we
// fall back to the whole trimmed label. Returns null only for empty input.
function extractItemName(label: string): string | null {
  const trimmed = label.trim()
  if (!trimmed) return null
  const match = trimmed.match(/\(([^()]+)\)\s*$/)
  if (match) return match[1].trim()
  return trimmed
}

// Parses a Rust-produced tree label into an identifier the mini-map can use.
// Modern labels embed the node id as `#N` so multiple nodes sharing a display
// name still resolve uniquely:
//   "Tree: Pyromaniac #123"
//   "Tree: Pyromaniac #123 (conditional)"
//   "Tree: Pyromaniac #123: 20% of Strength"      (conversion — also OK)
// Falls back to the plain name match for backwards compatibility with any
// callers that haven't migrated yet.
function extractTreeRef(label: string): { id?: number; name?: string } | null {
  const trimmed = label.trim()
  if (!trimmed.startsWith('Tree:')) return null
  const idMatch = trimmed.match(/^Tree:\s*(.+?)\s+#(\d+)\b/)
  if (idMatch) {
    return { id: Number(idMatch[2]), name: idMatch[1].trim() }
  }
  const nameMatch = trimmed.match(/^Tree:\s*(.+?)(?:\s*\(conditional\))?$/)
  if (!nameMatch) return null
  const name = nameMatch[1].trim()
  if (name.includes(':')) return null
  return name ? { name } : null
}

interface Props {
  statKey: string
  sources: SourceContribution[]
  moreSources?: SourceContribution[]
  children: ReactNode
  // Optional Rust-computed breakdown rendered in the pinned modal only.
  // When set, the pinned modal shows a Math section (additive + more + combined)
  // and a By Source Type section (Tree / Items / Skills subtotals).
  breakdown?: StatBreakdown | null
  // Fired the first time the user opens the pinned modal — caller uses it to
  // kick off the async Rust fetch for the breakdown. Won't be called for hover.
  onRequestBreakdown?: () => void
  // Title for the pinned modal header (defaults to the resolved statName).
  title?: string
}

// Returns the larger magnitude across the value's min and max endpoints, so
// ranged contributions sort by their "biggest impact". Used as the comparison
// key in `sortByMagnitude`.
function magnitudeOf(value: RangedValue): number {
  if (typeof value === 'number') return Math.abs(value)
  return Math.max(Math.abs(value[0]), Math.abs(value[1]))
}

// Sorts a contribution list by absolute magnitude (biggest at the top) so the
// most impactful sources show up first. Forge children get the same key as
// their value; subsequent `orderSources` re-groups them under their parent
// item, so the parent's position dictates where the cluster lands.
function sortByMagnitude(sources: SourceContribution[]): SourceContribution[] {
  return [...sources].sort((a, b) => magnitudeOf(b.value) - magnitudeOf(a.value))
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
  itemByName,
}: {
  s: SourceContribution
  statKey: string
  index: number
  // Lookup from worn item's display name to its `EquippedItem` record. When
  // we can match a source's parsed item name against an entry here, the row
  // gets wrapped in <ItemTooltip> so hovering it shows the full item card.
  itemByName?: Map<string, EquippedItem>
}) {
  // Renders a single contribution row inside the tooltip: an indented "Forged modifier" entry when the source carries forge metadata, or a normal "tag · label · value" row for everything else. Used by SourceTooltip's list rendering.
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
  // For tree sources, the Rust label carries `#N` (node id) so the mini-map
  // resolves to the correct allocated node — but it'd be noisy in the UI, so
  // we strip it from the displayed string. The parser elsewhere reads the
  // original `s.label` directly.
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
  // Item / socket rows: try to enrich with a hover ItemTooltip pulled from the
  // current build's inventory. Falls back to the plain row when we can't find
  // a matching item (e.g. the worn name changed mid-session).
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
  // Tree rows: pull the node's (x, y) from the static tree data and show a
  // mini-map highlighting its location. Prefer the id-based lookup (modern
  // labels embed `#N`) and only fall back to name lookup for legacy strings.
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

// --- breakdown-mode renderers (only used by the pinned modal) ---

function rangedToPair(v: RangedValue): [number, number] {
  return typeof v === 'number' ? [v, v] : v
}

function fmtPctRange(v: RangedValue, signed: boolean = true): string {
  // Renders an already-percent-typed value as either "+X%" or "X%" with two-decimal precision when fractional. Used by the breakdown sections where the Rust side already established the value is in percent units.
  const round = (n: number) =>
    Number.isInteger(n) ? n : Math.round(n * 100) / 100
  const [min, max] = rangedToPair(v)
  const sign = (n: number) => (signed && n >= 0 ? '+' : '')
  if (min === max) return `${sign(min)}${round(min)}%`
  return `${sign(min)}${round(min)}–${round(max)}%`
}

function fmtMultRange(v: RangedValue): string {
  // Renders a more-percent range as a `×N` multiplier ("×1.30" for 30%). Used by the breakdown's Multiplicative line.
  const round = (n: number) => Math.round(n * 1000) / 1000
  const [min, max] = rangedToPair(v)
  const a = round(1 + min / 100)
  const b = round(1 + max / 100)
  if (a === b) return `×${a}`
  return `×${a}–${b}`
}

function fmtFlatRange(v: RangedValue): string {
  // Renders a flat (non-percent) range as "X" or "X–Y", signed. Used by breakdown sections for stats like life / mana / defense.
  const round = (n: number) =>
    Number.isInteger(n) ? n : Math.round(n * 100) / 100
  const [min, max] = rangedToPair(v)
  const sign = (n: number) => (n >= 0 ? '+' : '')
  if (min === max) return `${sign(min)}${round(min)}`
  return `${sign(min)}${round(min)}–${round(max)}`
}

function fmtBreakdownValue(v: RangedValue, isPercent: boolean): string {
  // Picks the right format for a Rust-computed breakdown value depending on whether the stat is percent-formatted.
  return isPercent ? fmtPctRange(v) : fmtFlatRange(v)
}

function isZeroRanged(v: RangedValue): boolean {
  const [min, max] = rangedToPair(v)
  return min === 0 && max === 0
}

function CalculationSection({ breakdown }: { breakdown: StatBreakdown }) {
  // PoB-style "how this number was derived" panel. Three shapes:
  //   1. Multiplied-flat stats (life, mana, replenishes): the engine does
  //      `flat × (1 + sum(increased)/100) × (1 + sum(more)/100)`. We show
  //      Additive (flat), Increased (%), More (×), and Combined.
  //   2. Percent stats with _more (e.g. increased_fire_damage): the engine
  //      does `(1 + add/100) × (1 + more/100) − 1`. We show Additive (+),
  //      Multiplicative (×), and Combined.
  //   3. Plain percent or flat stats with no multipliers: single Total line.
  // Numbers come from Rust; this component only renders.
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
  // Multiplied-flat stat path (life / mana / replenishes). isPercent is false
  // — the additive sum is a flat unit and the combined value is too.
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
  // Percent stat with _more companion (e.g. increased_fire_damage).
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
  // Per-source-type subtotals, sorted by magnitude (Rust does the sort).
  // Renders three optional buckets, divider-separated:
  //   - Additive (flat for multiplied stats, or +% for percent stats)
  //   - Increased (only present on multiplied-flat stats — life, mana, etc.)
  //   - Multiplicative (more%, when present)
  // Bucket order matches CalculationSection's row order so the eye can map
  // each subtotal back to its formula component.
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
  // For multiplied-flat stats, the Additive bucket carries flat units; for
  // percent stats it carries percentages. fmtBreakdownValue handles both.
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
  // When true (pinned modal), prepend the Rust-computed Calculation and
  // BySourceType sections before the standard contributor lists. Hover-mode
  // callers leave this off so the compact tooltip stays compact.
  extended?: boolean
  breakdown?: StatBreakdown | null
  itemByName?: Map<string, EquippedItem>
}) {
  // Shared body of the source breakdown — used by both the hover tooltip and the pinned (right-click) modal so the rendering stays in one place. Sort by magnitude first, then group forge children under their parent items.
  const orderedAdd = orderSources(sortByMagnitude(sources))
  const orderedMore = hasMore
    ? orderSources(sortByMagnitude(moreSources!))
    : []
  const addSubtotal = sumRangePct(sources)
  const moreSubtotal = hasMore
    ? sumRangePct(moreSources!)
    : ([0, 0] as [number, number])
  const breakdownHead = extended && breakdown ? (
    <>
      <CalculationSection breakdown={breakdown} />
      <BySourceTypeSection breakdown={breakdown} />
    </>
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
  // Stat-source breakdown popover. Hover shows a small (~320px) anchored tooltip, left- or right-click pins a larger modal (centered, scrollable up to 80vh) with the Rust-computed Calculation + BySourceType sections on top. Both surfaces render into document.body via portal so ancestor `overflow-hidden` doesn't clip them.
  const hasMore = !!moreSources && moreSources.length > 0
  const triggerRef = useRef<HTMLDivElement>(null)
  const tooltipRef = useRef<HTMLDivElement>(null)
  const [open, setOpen] = useState(false)
  const [pinned, setPinned] = useState(false)
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null)
  const breakdownRequestedRef = useRef(false)
  // Reset the "already requested" latch when the cached breakdown drops back
  // to null — that happens when StatsView invalidates its cache after the
  // user edits the build. Without this reset the modal would never re-fetch.
  useEffect(() => {
    if (breakdown == null) {
      breakdownRequestedRef.current = false
    }
  }, [breakdown])
  const inventory = useBuild((s) => s.inventory)
  const itemByName = useMemo(() => {
    // Reverse-lookup map populated from the build's current inventory. Used by
    // SourceItem to enrich item / socket / forge rows with a hover ItemTooltip.
    // When two equipped items share a base name we keep the first match — both
    // copies carry identical stat ranges, so the tooltip is representative.
    const map = new Map<string, EquippedItem>()
    for (const equipped of Object.values(inventory)) {
      if (!equipped) continue
      const base = getItem(equipped.baseId)
      if (!base || !base.name) continue
      if (!map.has(base.name)) map.set(base.name, equipped)
    }
    return map
  }, [inventory])
  const requestBreakdown = () => {
    if (breakdownRequestedRef.current) return
    if (!onRequestBreakdown) return
    breakdownRequestedRef.current = true
    onRequestBreakdown()
  }

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

  const openPinned = () => {
    requestBreakdown()
    setPinned(true)
    setOpen(false)
  }
  const handleContextMenu = (e: React.MouseEvent) => {
    // Right-click pins the breakdown into a centered, scrollable modal so dense stats (lots of contributors) can be browsed without going off-screen.
    e.preventDefault()
    openPinned()
  }
  const handleClick = (e: React.MouseEvent) => {
    // Left-click also pins the breakdown — gives keyboard/click users the same affordance as right-click. Doesn't fire when the click started a drag/selection.
    if (e.defaultPrevented) return
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
                <div className="flex items-baseline gap-2 min-w-0">
                  <span className="text-[11px] font-semibold uppercase tracking-[0.14em] text-accent-hot truncate">
                    {title ?? statName(statKey)}
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
