import { useEffect, useMemo, useState } from 'react'
import { gems, getAffix, runes } from '../data'
import {
  JEWEL_AFFIX_POOL,
  JEWEL_AFFIX_POOL_BY_GROUP,
} from '../utils/item/jewelAffixes'
import type { Affix, EquippedAffix, Gem, Rune, TreeSocketContent } from '../types'
import { UNCUT_JEWEL_MAX_AFFIXES } from '../types'
import { Modal } from './Modal'

const SOCKETABLE_ICONS = import.meta.glob<string>(
  '../assets/socketable/*.png',
  { eager: true, query: '?url', import: 'default' },
)
const SOCKETABLE_ICON_BY_NAME: Record<string, string> = {}
for (const [p, url] of Object.entries(SOCKETABLE_ICONS)) {
  const file = p.split('/').pop() ?? ''
  const key = file.replace(/_spr\.png$/i, '').replace(/_/g, ' ')
  SOCKETABLE_ICON_BY_NAME[key.toLowerCase()] = url
}
function iconForName(name: string): string | undefined {
  return SOCKETABLE_ICON_BY_NAME[name.toLowerCase()]
}

const GEM_COLOR: Record<string, string> = {
  amethyst: '#c97acc',
  diamond: 'var(--color-text)',
  emerald: 'var(--color-stat-green)',
  ruby: 'var(--color-stat-red)',
  sapphire: 'var(--color-stat-blue)',
  topaz: 'var(--color-accent-hot)',
  skull: '#7a6a5a',
}
function gemColorForName(name: string): string {
  const last = name.split(' ').slice(-1)[0]?.toLowerCase() ?? ''
  return GEM_COLOR[last] ?? 'var(--color-faint)'
}

type Tab = 'items' | 'uncut'

interface Props {
  nodeId: number
  current: TreeSocketContent | null
  onClose: () => void
  onApply: (content: TreeSocketContent | null) => void
}

export default function JewelSocketModal({
  nodeId,
  current,
  onClose,
  onApply,
}: Props) {
  const [tab, setTab] = useState<Tab>(
    current?.kind === 'uncut' ? 'uncut' : 'items',
  )
  const [pending, setPending] = useState<TreeSocketContent | null>(current)

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])

  const dirty = !sameContent(pending, current)

  const statusText = pending
    ? pending.kind === 'uncut'
      ? `Uncut Jewel · ${pending.affixes.length} affix${
          pending.affixes.length === 1 ? '' : 'es'
        }`
      : describeItem(pending.id)
    : 'Empty socket'

  return (
    <Modal
      onClose={onClose}
      panelClassName="h-[88vh] w-[640px] max-w-[94vw]"
      eyebrow={
        <>
          Jewelry Socket <span className="text-accent-hot">#{nodeId}</span>
        </>
      }
      title="Insert Socketable"
    >
        {/* Tabs */}
        <div className="grid grid-cols-2 border-b border-border bg-bg">
          {(
            [
              { id: 'items', label: 'Gems / Runes / Jewels' },
              { id: 'uncut', label: 'Craft Uncut Jewel' },
            ] as const
          ).map((t) => (
            <button
              key={t.id}
              onClick={() => setTab(t.id)}
              className={`select-none border-b-2 py-3.5 font-mono text-[11px] font-semibold uppercase tracking-[0.18em] transition-colors ${
                tab === t.id
                  ? 'border-accent-hot text-accent-hot'
                  : 'border-transparent text-faint hover:text-muted'
              }`}
              style={
                tab === t.id
                  ? {
                      background:
                        'linear-gradient(180deg, rgba(224,184,100,0.04), transparent)',
                      textShadow: '0 0 12px rgba(224,184,100,0.3)',
                    }
                  : undefined
              }
            >
              {t.label}
            </button>
          ))}
        </div>

        {/* Tab body */}
        <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
          {tab === 'items' ? (
            <ItemsTab
              pending={pending}
              onSelect={(id) => setPending({ kind: 'item', id })}
            />
          ) : (
            <UncutTab
              pending={pending}
              onChange={(content) => setPending(content)}
            />
          )}
        </div>

        {/* Footer */}
        <footer className="flex items-center justify-between border-t border-border bg-black/30 px-4 py-3">
          <div
            className={`flex items-center gap-2 font-mono text-[11px] tracking-[0.06em] ${
              pending ? 'text-accent-hot' : 'text-faint'
            }`}
          >
            <span
              className="h-1.5 w-1.5 rounded-full"
              style={{
                background: pending
                  ? 'var(--color-accent-hot)'
                  : 'var(--color-faint)',
                boxShadow: pending
                  ? '0 0 8px rgba(224,184,100,0.6)'
                  : '0 0 6px var(--color-faint)',
              }}
            />
            <span>{statusText}</span>
          </div>
          <div className="flex gap-2">
            {pending != null && (
              <button
                onClick={() => setPending(null)}
                className="rounded-[3px] border border-border-2 bg-transparent px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-muted transition-colors hover:border-stat-red hover:text-stat-red"
              >
                Clear
              </button>
            )}
            <button
              onClick={onClose}
              className="rounded-[3px] border border-border-2 bg-transparent px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
            >
              Cancel
            </button>
            <button
              disabled={!dirty}
              onClick={() => {
                onApply(pending)
                onClose()
              }}
              className="rounded-[3px] border border-accent-deep px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-accent-hot transition-all enabled:hover:border-accent-hot enabled:hover:shadow-[0_0_14px_rgba(224,184,100,0.3)] disabled:cursor-not-allowed disabled:border-border-2 disabled:text-faint"
              style={
                !dirty
                  ? undefined
                  : {
                      background:
                        'linear-gradient(180deg, #3a2f1a, #2a2418)',
                    }
              }
            >
              {pending == null && current != null ? 'Remove' : 'Insert'}
            </button>
          </div>
        </footer>
    </Modal>
  )
}

function sameContent(
  a: TreeSocketContent | null,
  b: TreeSocketContent | null,
): boolean {
  if (a === b) return true
  if (a == null || b == null) return false
  if (a.kind !== b.kind) return false
  if (a.kind === 'item' && b.kind === 'item') return a.id === b.id
  if (a.kind === 'uncut' && b.kind === 'uncut') {
    if (a.affixes.length !== b.affixes.length) return false
    return a.affixes.every(
      (x, i) =>
        x.affixId === b.affixes[i]!.affixId &&
        x.tier === b.affixes[i]!.tier &&
        x.roll === b.affixes[i]!.roll,
    )
  }
  return false
}

function describeItem(id: string): string {
  const g = gems.find((x) => x.id === id)
  if (g) return `${g.name} · T${g.tier}`
  const r = runes.find((x) => x.id === id)
  if (r) return `${r.name} · T${r.tier}`
  return id
}

function fmtStats(stats: Record<string, number>): string {
  const entries = Object.entries(stats).filter(([, v]) => v !== 0)
  if (entries.length === 0) return '—'
  return entries.map(([k, v]) => `${k}: ${v}`).join(', ')
}

interface Row {
  id: string
  kind: 'gem' | 'rune'
  kindLabel: string
  name: string
  tier: number
  stats: string
}

function buildRows(): Row[] {
  const out: Row[] = []
  for (const g of gems as Gem[]) {
    out.push({
      id: g.id,
      kind: 'gem',
      kindLabel: g.name.toLowerCase().includes('jewel') ? 'Jewel' : 'Gem',
      name: g.name,
      tier: g.tier,
      stats: fmtStats(g.stats),
    })
  }
  for (const r of runes as Rune[]) {
    out.push({
      id: r.id,
      kind: 'rune',
      kindLabel: 'Rune',
      name: r.name,
      tier: r.tier,
      stats: fmtStats(r.stats),
    })
  }
  return out
}

const ALL_ROWS: Row[] = buildRows().sort(
  (a, b) =>
    a.kindLabel.localeCompare(b.kindLabel) ||
    a.tier - b.tier ||
    a.name.localeCompare(b.name),
)

function ItemsTab({
  pending,
  onSelect,
}: {
  pending: TreeSocketContent | null
  onSelect: (id: string) => void
}) {
  const [q, setQ] = useState('')
  const filter = q.trim().toLowerCase()
  const rows = filter
    ? ALL_ROWS.filter(
        (r) =>
          r.name.toLowerCase().includes(filter) ||
          r.stats.toLowerCase().includes(filter),
      )
    : ALL_ROWS

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      {/* Search */}
      <div className="border-b border-border px-4 py-3">
        <div className="relative">
          <svg
            className="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-faint"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth={2}
          >
            <circle cx="11" cy="11" r="7" />
            <path d="m20 20-3.5-3.5" />
          </svg>
          <input
            autoFocus
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="Search gems / runes / jewels…"
            className="w-full rounded-[3px] border border-border-2 px-3 py-2 pl-9 text-text placeholder:text-faint focus:border-accent-deep focus:outline-none focus:ring-2 focus:ring-accent-hot/15"
            style={{
              background:
                'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
              boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
            }}
          />
        </div>
      </div>

      {/* List */}
      <div className="js-list min-h-0 flex-1 overflow-y-auto py-1">
        {rows.length === 0 && (
          <div className="p-8 text-center text-sm text-muted">No matches</div>
        )}
        {rows.map((r) => {
          const selected =
            pending?.kind === 'item' && pending.id === r.id
          const url = iconForName(r.name)
          const tierClass =
            r.tier >= 4
              ? 'text-stat-orange border-stat-orange'
              : r.tier === 3
              ? 'text-[#fff0c4] border-accent-hot'
              : r.tier === 2
              ? 'text-accent-hot border-accent-deep'
              : 'text-accent-deep border-accent-deep'
          return (
            <button
              key={r.id}
              onClick={() => onSelect(r.id)}
              className={`group relative grid w-full cursor-pointer items-center gap-3.5 border-b border-dashed border-border px-4 py-2 text-left transition-colors last:border-b-0 hover:bg-accent-hot/5 ${
                selected ? 'bg-gradient-to-r from-accent-hot/10 to-transparent' : ''
              }`}
              style={{
                gridTemplateColumns: '36px 56px 1fr 56px 1fr',
              }}
            >
              <span
                className={`pointer-events-none absolute left-0 top-0 bottom-0 w-[2px] bg-accent-hot transition-opacity ${
                  selected ? 'opacity-100' : 'opacity-0 group-hover:opacity-60'
                }`}
                style={
                  selected
                    ? { boxShadow: '0 0 12px rgba(224,184,100,0.4)' }
                    : undefined
                }
              />
              {/* Icon: real PNG if exists; otherwise faceted gem dot */}
              <span className="flex items-center justify-center">
                {url ? (
                  <img
                    src={url}
                    alt=""
                    width={24}
                    height={24}
                    style={{ imageRendering: 'pixelated' }}
                  />
                ) : (
                  <GemBadge kind={r.kind} name={r.name} />
                )}
              </span>
              <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
                {r.kindLabel}
              </span>
              <span
                className={`truncate text-[13px] font-medium ${
                  selected ? 'text-accent-hot' : 'text-text group-hover:text-accent-hot'
                }`}
              >
                {r.name}
              </span>
              <span
                className={`w-max rounded-[2px] border px-2 py-0.5 text-center font-mono text-[11px] font-semibold tracking-[0.06em] ${tierClass}`}
                style={{
                  background:
                    'linear-gradient(180deg, #3a2e18, #2a2418)',
                }}
              >
                T{r.tier}
              </span>
              <span
                className={`truncate font-mono text-[10px] tracking-[0.02em] ${
                  r.stats === '—' ? 'italic text-faint tracking-[0.1em]' : 'text-muted'
                }`}
              >
                {r.stats}
              </span>
            </button>
          )
        })}
      </div>
    </div>
  )
}

function GemBadge({ kind, name }: { kind: 'gem' | 'rune'; name: string }) {
  if (kind === 'rune') {
    return (
      <span
        className="flex h-7 w-7 items-center justify-center rounded-[3px] border border-border-2 font-mono text-[10px] text-faint"
        style={{
          background: 'linear-gradient(180deg, #1a1610, #0d0b07)',
        }}
      >
        ᚱ
      </span>
    )
  }
  const c = gemColorForName(name)
  return (
    <span
      className="block h-7 w-7 rotate-45 rounded-[3px]"
      style={{
        background: `linear-gradient(135deg, ${c}, #0d0b07)`,
        border: `1px solid color-mix(in srgb, ${c} 60%, #000)`,
        boxShadow: `inset 0 1px 0 color-mix(in srgb, ${c} 60%, transparent), 0 0 12px color-mix(in srgb, ${c} 50%, transparent)`,
      }}
    />
  )
}

function UncutTab({
  pending,
  onChange,
}: {
  pending: TreeSocketContent | null
  onChange: (content: TreeSocketContent | null) => void
}) {
  const affixes: EquippedAffix[] =
    pending?.kind === 'uncut' ? pending.affixes : []
  const [adding, setAdding] = useState(false)

  const update = (next: EquippedAffix[]) => {
    if (next.length === 0) onChange(null)
    else onChange({ kind: 'uncut', affixes: next })
  }

  const isEmpty = affixes.length === 0

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <div className="font-mono text-[11px] tracking-[0.06em] text-muted">
          <span className="font-semibold text-accent-hot">{affixes.length}</span>{' '}
          / {UNCUT_JEWEL_MAX_AFFIXES} affixes
        </div>
        <button
          disabled={affixes.length >= UNCUT_JEWEL_MAX_AFFIXES}
          onClick={() => setAdding(true)}
          className="flex items-center gap-1.5 rounded-[3px] border border-accent-deep px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-accent-hot transition-all enabled:hover:border-accent-hot enabled:hover:shadow-[0_0_14px_rgba(224,184,100,0.3)] disabled:cursor-not-allowed disabled:opacity-40"
          style={{
            background: 'linear-gradient(180deg, #3a2f1a, #2a2418)',
          }}
        >
          <span className="text-[14px] leading-none text-[#fff0c4]">+</span>
          Add affix
        </button>
      </div>

      {isEmpty ? (
        <div className="flex flex-1 flex-col items-center justify-center px-6 py-12 text-center text-faint">
          <div
            className="mb-4 flex h-[54px] w-[54px] items-center justify-center rounded-full border border-dashed border-border-2 text-faint"
            style={{
              background:
                'radial-gradient(circle, rgba(201,165,90,0.06), transparent 70%)',
            }}
          >
            <svg
              viewBox="0 0 24 24"
              width={20}
              height={20}
              fill="none"
              stroke="currentColor"
              strokeWidth={1.4}
            >
              <path d="M12 3 L20 9 L17 19 H7 L4 9 Z" />
              <circle cx="12" cy="12" r="2" />
            </svg>
          </div>
          <div className="mb-1.5 text-[14px] font-semibold tracking-[0.08em] text-muted">
            No affixes yet
          </div>
          <div className="font-mono text-[10px] tracking-[0.1em] text-faint">
            Click{' '}
            <kbd
              className="rounded-[2px] border px-1.5 py-0.5 font-mono text-[10px] text-accent-hot"
              style={{
                background: 'rgba(224,184,100,0.08)',
                borderColor: 'rgba(224,184,100,0.25)',
              }}
            >
              + Add affix
            </kbd>{' '}
            to roll
          </div>
        </div>
      ) : (
        <div className="min-h-0 flex-1 overflow-y-auto px-4 py-4">
          <div className="mx-auto flex max-w-[540px] flex-col gap-2.5">
            {affixes.map((a, idx) => (
              <UncutAffixCard
                key={`${a.affixId}-${idx}`}
                index={idx + 1}
                affix={a}
                onChange={(patch) =>
                  update(
                    affixes.map((x, i) => (i === idx ? { ...x, ...patch } : x)),
                  )
                }
                onRemove={() => update(affixes.filter((_, i) => i !== idx))}
              />
            ))}
          </div>
        </div>
      )}

      {adding && (
        <AffixGroupPicker
          existingGroupIds={
            new Set(
              affixes
                .map((a) => getAffix(a.affixId)?.groupId)
                .filter((g): g is string => !!g),
            )
          }
          onClose={() => setAdding(false)}
          onPick={(groupId) => {
            setAdding(false)
            const tiers = JEWEL_AFFIX_POOL_BY_GROUP.get(groupId) ?? []
            const top = tiers[tiers.length - 1]
            if (!top) return
            update([
              ...affixes,
              {
                affixId: top.id,
                tier: top.tier,
                roll: 1,
              },
            ])
          }}
        />
      )}
    </div>
  )
}

function UncutAffixCard({
  index,
  affix,
  onChange,
  onRemove,
}: {
  index: number
  affix: EquippedAffix
  onChange: (patch: Partial<EquippedAffix>) => void
  onRemove: () => void
}) {
  const def = getAffix(affix.affixId)
  if (!def) {
    return (
      <div
        className="grid items-center gap-3 rounded-[3px] border border-border-2 px-3.5 py-2.5"
        style={{ gridTemplateColumns: 'auto 1fr auto auto' }}
      >
        <span className="font-mono text-[10px] text-stat-red">!</span>
        <span className="text-stat-red text-xs">unknown affix: {affix.affixId}</span>
        <span />
        <button
          onClick={onRemove}
          className="flex h-5 w-5 items-center justify-center rounded-[2px] border border-border-2 text-[12px] leading-none text-faint hover:border-stat-red hover:text-stat-red"
        >
          ×
        </button>
      </div>
    )
  }

  const tiers = JEWEL_AFFIX_POOL_BY_GROUP.get(def.groupId) ?? []
  const min = def.valueMin ?? 0
  const max = def.valueMax ?? min
  const actualValue = rollFractionToValue(min, max, affix.roll)
  const rangeLabel = formatAffixValueLabel(def, actualValue)

  return (
    <div
      className="rounded-[3px] border border-border-2 px-3.5 py-3"
      style={{
        background:
          'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 80%, transparent))',
      }}
    >
      <div
        className="grid items-center gap-3"
        style={{ gridTemplateColumns: 'auto 1fr auto auto' }}
      >
        <span
          className="flex h-[22px] w-[22px] items-center justify-center rounded-full border border-accent-deep font-mono text-[11px] text-accent-hot"
          style={{
            background: 'linear-gradient(180deg, #3a2e18, #2a2418)',
          }}
        >
          {index}
        </span>
        <span className="text-[13px] text-text">{def.description}</span>
        <span className="font-mono text-[11px] text-accent-hot">
          {rangeLabel}
        </span>
        <button
          onClick={onRemove}
          className="flex h-5 w-5 items-center justify-center rounded-[2px] border border-border-2 text-[12px] leading-none text-faint transition-colors hover:border-stat-red hover:text-stat-red"
          aria-label="Remove affix"
        >
          ×
        </button>
      </div>
      <div className="mt-2.5 flex flex-wrap items-center gap-3 pl-[34px] font-mono text-[10px] tracking-[0.06em] text-faint">
        <label className="flex items-center gap-2">
          <span>tier</span>
          <select
            value={affix.affixId}
            onChange={(e) => {
              const next = tiers.find((t) => t.id === e.target.value)
              if (!next) return
              onChange({
                affixId: next.id,
                tier: next.tier,
              })
            }}
            className="rounded-[2px] border border-border-2 bg-panel-2 px-2 py-0.5 text-text"
          >
            {tiers.map((t) => (
              <option key={t.id} value={t.id}>
                T{t.tier} ({t.valueMin}-{t.valueMax})
              </option>
            ))}
          </select>
        </label>
        <label className="flex items-center gap-2">
          <span>roll</span>
          <input
            type="number"
            min={min}
            max={max}
            step={def.format === 'flat' ? 1 : 0.1}
            value={actualValue}
            onChange={(e) => {
              const v = Number(e.target.value)
              if (!Number.isFinite(v)) return
              onChange({ roll: valueToRollFraction(min, max, clamp(v, min, max)) })
            }}
            className="w-20 rounded-[2px] border border-border-2 bg-panel-2 px-2 py-0.5 text-right text-text"
          />
          <span className="text-faint">
            ({min}-{max})
          </span>
        </label>
      </div>
    </div>
  )
}

function clamp(n: number, lo: number, hi: number) {
  return Math.max(lo, Math.min(hi, n))
}

function rollFractionToValue(min: number, max: number, roll: number): number {
  if (min === max) return max
  const raw = min + (max - min) * roll
  return Math.round(raw * 100) / 100
}

function valueToRollFraction(min: number, max: number, value: number): number {
  if (min === max) return 1
  return clamp((value - min) / (max - min), 0, 1)
}

function formatAffixValueLabel(def: Affix, value: number): string {
  const sign = def.sign ?? '+'
  const suffix = def.format === 'percent' ? '%' : ''
  const display = def.format === 'flat' ? Math.round(value) : value
  return `${sign}${display}${suffix}`
}

function AffixGroupPicker({
  existingGroupIds,
  onClose,
  onPick,
}: {
  existingGroupIds: Set<string>
  onClose: () => void
  onPick: (groupId: string) => void
}) {
  const [q, setQ] = useState('')
  const groups = useMemo(() => {
    const seen = new Set<string>()
    const out: { groupId: string; description: string; tiers: number }[] = []
    for (const a of JEWEL_AFFIX_POOL) {
      if (seen.has(a.groupId)) continue
      seen.add(a.groupId)
      const tiers = JEWEL_AFFIX_POOL_BY_GROUP.get(a.groupId) ?? []
      const top = tiers[tiers.length - 1]
      out.push({
        groupId: a.groupId,
        description: top?.description ?? a.description,
        tiers: tiers.length,
      })
    }
    return out.sort((a, b) => a.description.localeCompare(b.description))
  }, [])
  const filter = q.trim().toLowerCase()
  const rows = filter
    ? groups.filter((g) => g.description.toLowerCase().includes(filter))
    : groups

  return (
    <Modal
      onClose={onClose}
      eyebrow="Jewel Affix"
      title="Pick an affix"
      panelClassName="h-[80vh] w-[560px] max-w-[92vw]"
    >
      <div className="border-b border-border px-3 py-2">
        <div className="relative">
          <svg
            className="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-faint"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth={2}
          >
            <circle cx="11" cy="11" r="7" />
            <path d="m20 20-3.5-3.5" />
          </svg>
          <input
            autoFocus
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="Search affixes…"
            className="w-full rounded-[3px] border border-border-2 px-3 py-2 pl-9 text-text placeholder:text-faint focus:border-accent-deep focus:outline-none focus:ring-2 focus:ring-accent-hot/15"
            style={{
              background:
                'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
              boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
            }}
          />
        </div>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto">
        {rows.map((g) => {
          const taken = existingGroupIds.has(g.groupId)
          return (
            <button
              key={g.groupId}
              disabled={taken}
              onClick={() => onPick(g.groupId)}
              className={`flex w-full items-center justify-between border-b border-dashed border-border px-4 py-2.5 text-left transition-colors ${
                taken ? 'cursor-not-allowed opacity-30' : 'hover:bg-accent-hot/5'
              }`}
            >
              <span className="text-[13px] text-text">{g.description}</span>
              <span className="font-mono text-[10px] tracking-[0.06em] text-faint">
                {g.tiers} tier{g.tiers === 1 ? '' : 's'}
              </span>
            </button>
          )
        })}
        {rows.length === 0 && (
          <div className="p-6 text-center text-sm text-muted">
            No matches
          </div>
        )}
      </div>
    </Modal>
  )
}
