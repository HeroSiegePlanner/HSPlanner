import { ItemCard } from '../../components/ItemTooltip'
import { useCalcResult } from '../../hooks/useCalcResult'
import { getItem } from '../../data'
import type { EquippedItem, Inventory, SlotKey } from '../../types'
import {
  attrDiffs, avgHitDiff, BUILD_STAT_KEYS, combinedDpsDiff, computeBuildSummary,
  computeVerdict, formatDeltaNum, formatStatNum, hitDpsDiff, isSameEquipped,
  ITEM_AFFIX_KEYS, pickStatDiffsByKeys, socketDiff,
  type BuildSummary, type BuildSummaryDeps, type StatDiff, type Verdict,
} from './lib/diff'
import { RARITY_TEXT } from './lib/rarity'

function VerdictBadge({ verdict }: { verdict: Verdict }) {
  const config: Record<
    Verdict,
    { label: string; arrow: string; cls: string }
  > = {
    upgrade: {
      label: 'Upgrade',
      arrow: '▲',
      cls: 'border-stat-green text-stat-green bg-stat-green/8 shadow-[0_0_18px_rgba(116,201,138,0.12)]',
    },
    downgrade: {
      label: 'Downgrade',
      arrow: '▼',
      cls: 'border-stat-red text-stat-red bg-stat-red/8 shadow-[0_0_18px_rgba(232,144,122,0.12)]',
    },
    sidegrade: {
      label: 'Sidegrade',
      arrow: '≈',
      cls: 'border-border-2 text-muted bg-panel-2/60',
    },
  }
  const c = config[verdict]
  return (
    <div
      className={`inline-flex items-center gap-2 rounded-xs border px-3 py-1.5 font-mono text-[11px] font-semibold uppercase tracking-[0.22em] ${c.cls}`}
    >
      <span className="text-[14px] leading-none">{c.arrow}</span>
      <span>{c.label}</span>
    </div>
  )
}

function DiffRow({ diff }: { diff: StatDiff }) {
  const beforeText =
    diff.kind === 'new'
      ? 'none'
      : formatStatNum(diff.beforeMin, diff.beforeMax, diff.unit)
  const afterText =
    diff.kind === 'lost'
      ? '—'
      : formatStatNum(diff.afterMin, diff.afterMax, diff.unit)

  const tone =
    diff.kind === 'up' || diff.kind === 'new'
      ? 'border-stat-green/35 bg-stat-green/8 text-stat-green'
      : diff.kind === 'down' || diff.kind === 'lost'
        ? 'border-stat-red/35 bg-stat-red/8 text-stat-red'
        : 'border-border-2 bg-transparent text-muted'

  const afterColor =
    diff.kind === 'up' || diff.kind === 'new'
      ? 'text-stat-green'
      : diff.kind === 'down' || diff.kind === 'lost'
        ? 'text-stat-red'
        : 'text-text'

  const beforeStyle = diff.kind === 'new' ? 'italic text-faint' : 'text-faint'
  const afterStyle = diff.kind === 'lost' ? 'italic text-faint' : afterColor
  const opacity = diff.kind === 'same' ? 'opacity-50' : ''

  return (
    <div
      className={`grid items-center gap-2.5 border-b border-dashed border-border px-1 py-1.5 last:border-b-0 font-mono text-[11px] ${opacity}`}
      style={{ gridTemplateColumns: '1fr auto auto auto auto' }}
    >
      <span className="font-sans text-[12px] text-text/85">{diff.label}</span>
      <span className={`min-w-13.5 text-right tabular-nums ${beforeStyle}`}>
        {beforeText}
      </span>
      <span className="text-faint">→</span>
      <span
        className={`min-w-13.5 text-right font-semibold tabular-nums ${afterStyle}`}
      >
        {afterText}
      </span>
      <span
        className={`min-w-15.5 rounded-xs border px-1.5 py-0.5 text-right text-[10px] font-semibold tabular-nums ${tone}`}
      >
        {formatDeltaNum(diff.delta, diff.unit, diff.kind)}
      </span>
    </div>
  )
}

function DiffSection({
  title,
  diffs,
  emptyHint,
}: {
  title: string
  diffs: StatDiff[]
  emptyHint?: string
}) {
  if (diffs.length === 0 && !emptyHint) return null
  return (
    <div className="mt-4 first:mt-0">
      <div className="mb-2 flex items-center gap-2.5 font-mono text-[10px] font-semibold uppercase tracking-[0.18em] text-muted">
        <span>{title}</span>
        <span className="h-px flex-1 bg-border" />
        {diffs.length > 0 && (
          <span className="font-normal tracking-[0.14em] text-faint">
            {diffs.length} change{diffs.length === 1 ? '' : 's'}
          </span>
        )}
      </div>
      {diffs.length === 0 ? (
        <p className="text-[11px] text-faint italic">{emptyHint}</p>
      ) : (
        diffs.map((d) => <DiffRow key={d.key} diff={d} />)
      )}
    </div>
  )
}

function CompareSummary({
  before,
  after,
}: {
  before: BuildSummary
  after: BuildSummary
}) {
  const beforeColor = before.itemRarity
    ? RARITY_TEXT[before.itemRarity]
    : 'text-faint'
  const afterColor = after.itemRarity
    ? RARITY_TEXT[after.itemRarity]
    : 'text-faint'
  return (
    <div className="mt-3 grid grid-cols-2 gap-px overflow-hidden rounded-[3px] border border-border bg-border">
      <div className="bg-panel-2/80 px-3 py-2">
        <div className="font-mono text-[9px] uppercase tracking-[0.16em] text-muted">
          Currently Equipped
        </div>
        <div className={`mt-1 truncate text-[13px] font-medium ${beforeColor}`}>
          {before.itemName ?? <span className="italic text-faint">Empty slot</span>}
        </div>
      </div>
      <div className="bg-panel-2/80 px-3 py-2">
        <div className="font-mono text-[9px] uppercase tracking-[0.16em] text-muted">
          Selected
        </div>
        <div className={`mt-1 truncate text-[13px] font-medium ${afterColor}`}>
          {after.itemName ?? <span className="italic text-faint">Empty slot</span>}
        </div>
      </div>
    </div>
  )
}

function CompareItemCards({
  baselineEquipped,
  currentEquipped,
}: {
  baselineEquipped: EquippedItem | null
  currentEquipped: EquippedItem | null
}) {
  const baselineBase = baselineEquipped ? getItem(baselineEquipped.baseId) : null
  const currentBase = currentEquipped ? getItem(currentEquipped.baseId) : null
  if (!baselineBase && !currentBase) return null

  const sameItem =
    baselineEquipped &&
    currentEquipped &&
    isSameEquipped(baselineEquipped, currentEquipped)

  if (sameItem && currentBase && currentEquipped) {
    return (
      <div className="mb-4">
        <div className="mb-2 flex items-center gap-2.5 font-mono text-[10px] font-semibold uppercase tracking-[0.18em] text-muted">
          <span>Item</span>
          <span className="h-px flex-1 bg-border" />
          <span className="font-normal tracking-[0.14em] text-faint">unchanged</span>
        </div>
        <ItemCard
          equipped={currentEquipped}
          base={currentBase}
          className="w-full text-[12px]"
        />
      </div>
    )
  }

  return (
    <div className="mb-4">
      <div className="mb-3 flex items-center gap-2.5 font-mono text-[10px] font-semibold uppercase tracking-[0.18em] text-muted">
        <span>Items</span>
        <span className="h-px flex-1 bg-border" />
      </div>
      <div className="flex items-start gap-3">
        <ItemCard
          equipped={baselineEquipped ?? undefined}
          base={baselineBase ?? undefined}
          className="min-w-0 flex-1 text-[12px]"
        />
        <ItemCard
          equipped={currentEquipped ?? undefined}
          base={currentBase ?? undefined}
          className="min-w-0 flex-1 text-[12px]"
        />
      </div>
    </div>
  )
}

export function CompareColumn({
  baselineInventory,
  currentInventory,
  baselineEquipped,
  currentEquipped,
  slot,
  deps,
}: {
  baselineInventory: Inventory
  currentInventory: Inventory
  baselineEquipped: EquippedItem | null
  currentEquipped: EquippedItem | undefined
  slot: SlotKey
  deps: BuildSummaryDeps
}) {
  const summaries = useCalcResult<{
    before: BuildSummary
    after: BuildSummary
  } | null>(
    () =>
      Promise.all([
        computeBuildSummary(baselineInventory, slot, deps),
        computeBuildSummary(currentInventory, slot, deps),
      ]).then(([before, after]) => ({ before, after })),
    [baselineInventory, currentInventory, slot, deps],
    null,
  )

  if (!summaries) {
    return (
      <div className="flex w-150 min-w-0 shrink-0 flex-col items-center justify-center border-l border-border bg-black/15">
        <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
          Calculating compare…
        </span>
      </div>
    )
  }

  const beforeSummary = summaries.before
  const afterSummary = summaries.after

  const verdict = computeVerdict(beforeSummary, afterSummary)

  const hitDps = hitDpsDiff(beforeSummary, afterSummary)
  const combinedDps = combinedDpsDiff(beforeSummary, afterSummary)
  const avgHit = avgHitDiff(beforeSummary, afterSummary)
  const itemAffixDiffs = pickStatDiffsByKeys(
    beforeSummary,
    afterSummary,
    ITEM_AFFIX_KEYS,
  )
  const sock = socketDiff(beforeSummary, afterSummary)
  if (sock) itemAffixDiffs.unshift(sock)

  const buildStatDiffs = [
    ...attrDiffs(beforeSummary, afterSummary),
    ...pickStatDiffsByKeys(beforeSummary, afterSummary, BUILD_STAT_KEYS),
  ]
  const damageRows = [hitDps, combinedDps, avgHit].filter(
    (d): d is StatDiff => d !== null,
  )

  const headerBg =
    verdict === 'upgrade'
      ? 'linear-gradient(180deg, rgba(116,201,138,0.05), transparent)'
      : verdict === 'downgrade'
        ? 'linear-gradient(180deg, rgba(232,144,122,0.05), transparent)'
        : 'linear-gradient(180deg, rgba(138,128,118,0.04), transparent)'

  return (
    <div className="flex w-150 min-w-0 shrink-0 flex-col border-l border-border bg-black/15">
      <div
        className="border-b border-border px-5 py-4"
        style={{ background: headerBg }}
      >
        <div className="flex items-center justify-between gap-3">
          <div>
            <div className="mb-1 font-mono text-[10px] uppercase tracking-[0.18em] text-muted">
              Comparison
            </div>
            <h3 className="m-0 text-[16px] font-semibold tracking-[0.02em] text-text/85">
              Net change
            </h3>
          </div>
          <VerdictBadge verdict={verdict} />
        </div>
        <CompareSummary before={beforeSummary} after={afterSummary} />
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-5 pb-5 pt-3">
        <CompareItemCards
          baselineEquipped={baselineEquipped}
          currentEquipped={currentEquipped ?? null}
        />
        {damageRows.length > 0 && (
          <DiffSection
            title={
              afterSummary.activeSkillName
                ? `Active Skill · ${afterSummary.activeSkillName}`
                : 'Active Skill'
            }
            diffs={damageRows}
          />
        )}
        <DiffSection
          title="Item Affixes"
          diffs={itemAffixDiffs}
          emptyHint={
            beforeSummary.itemBaseId === afterSummary.itemBaseId
              ? 'No item-level changes'
              : undefined
          }
        />
        <DiffSection
          title="Build Stats"
          diffs={buildStatDiffs}
          emptyHint="No build-stat changes"
        />
      </div>
    </div>
  )
}
