import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { motion } from 'motion/react'
import { backdropVariants, panelVariants } from '../lib/motion'
import type { ItemRarity } from '../types'
import Tooltip from './Tooltip'
import type { TooltipTone } from './tooltip-tones'
import { CornerMarks } from './CornerMarks'

const RARITY_TEXT: Record<ItemRarity, string> = {
  common: 'text-white',
  uncommon: 'text-sky-400',
  rare: 'text-accent-hot',
  mythic: 'text-purple-400',
  satanic: 'text-red-500',
  heroic: 'text-lime-400',
  angelic: 'text-yellow-200',
  satanic_set: 'text-green-400',
  unholy: 'text-pink-400',
  relic: 'text-orange-300',
}

export interface PickerRow {
  id: string
  iconUrl?: string
  iconNode?: ReactNode
  iconColor?: string
  kindLabel?: string
  name: string
  tier?: number
  rarity?: ItemRarity
  meta?: ReactNode
  group?: string
  disabled?: boolean
  disabledNote?: string
  searchTerms?: string
  tooltip?: ReactNode
  tooltipTone?: TooltipTone
}

export interface PickerPanelState {
  selectedId: string | null
  hoveredId: string | null
}

export interface PickerModalProps {
  title: string
  sectionLabel?: string
  sectionAccent?: string | number
  rows: PickerRow[]
  selectedId?: string | null
  searchPlaceholder?: string
  emptyMessage?: string
  onSelect: (id: string) => void
  onClose: () => void
  hoverPanel?: (rowId: string | null) => ReactNode
  selectedPanel?: (state: PickerPanelState) => ReactNode
  panelOffsetX?: number
  width?: number
  closeOnSelect?: boolean
  allowClear?: boolean
  onClear?: () => void
  footerStatus?: ReactNode
  footerActions?: ReactNode
}

export default function PickerModal({
  title,
  sectionLabel,
  sectionAccent,
  rows,
  selectedId = null,
  searchPlaceholder = 'Search…',
  emptyMessage = 'No matches',
  onSelect,
  onClose,
  hoverPanel,
  selectedPanel,
  panelOffsetX = 24,
  width = 640,
  closeOnSelect = true,
  allowClear = false,
  onClear,
  footerStatus,
  footerActions,
}: PickerModalProps) {
  const [q, setQ] = useState('')
  const [hoveredId, setHoveredId] = useState<string | null>(null)
  const dialogRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])

  const filter = q.trim().toLowerCase()
  const filteredRows = useMemo(() => {
    if (!filter) return rows
    return rows.filter(
      (r) =>
        r.name.toLowerCase().includes(filter) ||
        (typeof r.meta === 'string' && r.meta.toLowerCase().includes(filter)) ||
        (r.kindLabel?.toLowerCase().includes(filter) ?? false) ||
        (r.group?.toLowerCase().includes(filter) ?? false) ||
        (r.searchTerms?.toLowerCase().includes(filter) ?? false),
    )
  }, [rows, filter])

  const groupedRows = useMemo(() => {
    const out: { group: string | null; rows: PickerRow[] }[] = []
    const idx = new Map<string, number>()
    for (const r of filteredRows) {
      const g = r.group ?? null
      const key = g ?? '__none__'
      let pos = idx.get(key)
      if (pos === undefined) {
        pos = out.length
        idx.set(key, pos)
        out.push({ group: g, rows: [] })
      }
      out[pos]!.rows.push(r)
    }
    return out
  }, [filteredRows])

  const hasGroupHeaders = groupedRows.some((g) => g.group !== null)

  const handlePick = (r: PickerRow) => {
    if (r.disabled) return
    onSelect(r.id)
    if (closeOnSelect) onClose()
  }

  return createPortal(
    <motion.div
      role="presentation"
      className="fixed inset-0 z-100 flex items-center justify-center bg-black/70 backdrop-blur-sm"
      onMouseDown={onClose}
      variants={backdropVariants}
      initial="initial"
      animate="animate"
      style={{
        background:
          'radial-gradient(ellipse at 50% 0%, rgba(201,165,90,0.06), rgba(0,0,0,0.78) 60%)',
      }}
    >
      <div
        className="flex max-w-[98vw] items-start"
        style={{ gap: panelOffsetX }}
        onMouseDown={(e) => e.stopPropagation()}
      >
        {selectedPanel && (
          <div
            className="shrink-0 self-start max-h-[88vh] overflow-y-auto"
            style={{ minWidth: 260, maxWidth: 320 }}
          >
            {selectedPanel({ selectedId: selectedId ?? null, hoveredId })}
          </div>
        )}

        <motion.div
          ref={dialogRef}
          role="dialog"
          aria-modal="true"
          variants={panelVariants}
          initial="initial"
          animate="animate"
          className="relative flex h-[88vh] flex-col overflow-hidden rounded-md border border-border"
          style={{
            width,
            background:
              'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 80%, transparent))',
            boxShadow:
              'inset 0 1px 0 rgba(255,255,255,0.02), 0 24px 64px rgba(0,0,0,0.7)',
          }}
        >
          <CornerMarks />

          <header
            className="flex items-start justify-between gap-3 border-b border-border px-5 py-4"
            style={{
              background:
                'linear-gradient(180deg, rgba(201,165,90,0.05), transparent)',
            }}
          >
            <div>
              {sectionLabel && (
                <div className="mb-1 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
                  <span
                    className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
                    style={{ boxShadow: '0 0 8px rgba(224,184,100,0.6)' }}
                  />
                  {sectionLabel}
                  {sectionAccent !== undefined && (
                    <span className="text-accent-hot">{sectionAccent}</span>
                  )}
                </div>
              )}
              <h2
                className="m-0 text-[18px] font-semibold tracking-[0.02em] text-accent-hot"
                style={{
                  textShadow: '0 0 16px rgba(224,184,100,0.15)',
                }}
              >
                {title}
              </h2>
            </div>
            <button
              onClick={onClose}
              className="rounded-[3px] border border-border-2 bg-panel-2 px-3.5 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
            >
              Close
            </button>
          </header>

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
                placeholder={searchPlaceholder}
                className="w-full rounded-[3px] border border-border-2 px-3 py-2 pl-9 text-text placeholder:text-faint focus:border-accent-deep focus:outline-none focus:ring-2 focus:ring-accent-hot/15"
                style={{
                  background:
                    'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
                  boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
                }}
              />
            </div>
          </div>

          <div
            className="js-list min-h-0 flex-1 overflow-y-auto"
            onMouseLeave={() => setHoveredId(null)}
          >
            {filteredRows.length === 0 ? (
              <div className="p-8 text-center text-sm text-muted">
                {emptyMessage}
              </div>
            ) : (
              groupedRows.map((g, gi) => (
                <div key={g.group ?? `__${gi}`}>
                  {hasGroupHeaders && g.group && (
                    <div
                      className="sticky top-0 z-1 flex items-center gap-2 border-b border-accent-deep/30 px-4 py-1 font-mono text-[10px] uppercase tracking-[0.18em] text-accent-hot/70"
                      style={{
                        background: 'var(--color-panel-2)',
                      }}
                    >
                      <span
                        className="inline-block h-1 w-1 rotate-45 bg-accent-deep"
                        aria-hidden="true"
                      />
                      {g.group}
                    </div>
                  )}
                  {g.rows.map((r) => {
                    const selected = r.id === selectedId
                    const tierClass =
                      r.tier === undefined
                        ? ''
                        : r.tier >= 4
                          ? 'text-stat-orange border-stat-orange'
                          : r.tier === 3
                            ? 'text-[#fff0c4] border-accent-hot'
                            : r.tier === 2
                              ? 'text-accent-hot border-accent-deep'
                              : 'text-accent-deep border-accent-deep'
                    const nameColor = r.rarity
                      ? RARITY_TEXT[r.rarity]
                      : selected
                        ? 'text-accent-hot'
                        : 'text-text group-hover:text-accent-hot'
                    const showTierBadge = r.tier !== undefined
                    const cols = showTierBadge
                      ? '36px 56px 1fr 56px 1fr'
                      : '36px 56px 1fr 1fr'

                    const rowButton = (
                      <button
                        key={r.id}
                        type="button"
                        disabled={r.disabled}
                        onClick={() => handlePick(r)}
                        onMouseEnter={() =>
                          !r.disabled && setHoveredId(r.id)
                        }
                        title={r.disabledNote}
                        className={`group relative grid w-full items-center gap-3.5 border-b border-dashed border-border px-4 py-2 text-left transition-colors last:border-b-0 ${
                          r.disabled
                            ? 'cursor-not-allowed opacity-30'
                            : 'cursor-pointer hover:bg-accent-hot/5'
                        } ${
                          selected
                            ? 'bg-linear-to-r from-accent-hot/10 to-transparent'
                            : ''
                        }`}
                        style={{ gridTemplateColumns: cols }}
                      >
                        <span
                          className={`pointer-events-none absolute left-0 top-0 bottom-0 w-0.5 bg-accent-hot transition-opacity ${
                            selected
                              ? 'opacity-100'
                              : 'opacity-0 group-hover:opacity-60'
                          }`}
                          style={
                            selected
                              ? {
                                  boxShadow:
                                    '0 0 12px rgba(224,184,100,0.4)',
                                }
                              : undefined
                          }
                        />
                        <span className="flex items-center justify-center">
                          {r.iconUrl ? (
                            <img
                              src={r.iconUrl}
                              alt=""
                              width={24}
                              height={24}
                              style={{ imageRendering: 'pixelated' }}
                            />
                          ) : r.iconNode ? (
                            r.iconNode
                          ) : (
                            <FallbackIcon
                              color={r.iconColor}
                              rarity={r.rarity}
                            />
                          )}
                        </span>
                        <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
                          {r.kindLabel ?? ''}
                        </span>
                        <span
                          className={`truncate text-[13px] font-medium ${nameColor}`}
                        >
                          {r.name}
                        </span>
                        {showTierBadge && (
                          <span
                            className={`w-max rounded-xs border px-2 py-0.5 text-center font-mono text-[11px] font-semibold tracking-[0.06em] ${tierClass}`}
                            style={{
                              background:
                                'linear-gradient(180deg, #3a2e18, #2a2418)',
                            }}
                          >
                            T{r.tier}
                          </span>
                        )}
                        <span
                          className={`truncate font-mono text-[10px] tracking-[0.02em] text-muted ${
                            typeof r.meta === 'string' && r.meta === '—'
                              ? 'italic text-faint tracking-widest'
                              : ''
                          }`}
                        >
                          {r.meta ?? ''}
                        </span>
                      </button>
                    )

                    if (r.tooltip) {
                      return (
                        <Tooltip
                          key={r.id}
                          content={r.tooltip}
                          tone={r.tooltipTone ?? 'neutral'}
                          placement="right"
                          delay={120}
                        >
                          {rowButton}
                        </Tooltip>
                      )
                    }
                    return rowButton
                  })}
                </div>
              ))
            )}
          </div>

          <footer className="flex items-center justify-between gap-3 border-t border-border bg-black/30 px-4 py-3">
            <div
              className={`flex min-w-0 flex-1 items-center gap-2 font-mono text-[11px] tracking-[0.06em] ${
                selectedId ? 'text-accent-hot' : 'text-faint'
              }`}
            >
              <span
                className="h-1.5 w-1.5 shrink-0 rounded-full"
                style={{
                  background: selectedId
                    ? 'var(--color-accent-hot)'
                    : 'var(--color-faint)',
                  boxShadow: selectedId
                    ? '0 0 8px rgba(224,184,100,0.6)'
                    : '0 0 6px var(--color-faint)',
                }}
              />
              <span className="truncate">
                {footerStatus ??
                  (selectedId
                    ? `Selected: ${
                        rows.find((r) => r.id === selectedId)?.name ?? selectedId
                      }`
                    : 'Nothing selected')}
              </span>
            </div>
            <div className="flex shrink-0 gap-2">
              {footerActions}
              {allowClear && selectedId && (
                <button
                  onClick={() => {
                    onClear?.()
                    onClose()
                  }}
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
            </div>
          </footer>
        </motion.div>

        {hoverPanel && (
          <div
            className="shrink-0 self-start max-h-[88vh] overflow-y-auto"
            style={{ minWidth: 260, maxWidth: 320 }}
          >
            {hoverPanel(hoveredId)}
          </div>
        )}
      </div>
    </motion.div>,
    document.body,
  )
}

function FallbackIcon({
  color,
  rarity,
}: {
  color?: string
  rarity?: ItemRarity
}) {
  const c =
    color ??
    (rarity === 'satanic' || rarity === 'satanic_set'
      ? '#d96b5a'
      : rarity === 'angelic'
        ? '#e0d36a'
        : rarity === 'unholy'
          ? '#cf6db0'
          : rarity === 'heroic'
            ? '#96c95a'
            : rarity === 'mythic'
              ? '#a070c8'
              : rarity === 'rare'
                ? '#c9a560'
                : rarity === 'uncommon'
                  ? '#5a8fc9'
                  : rarity === 'relic'
                    ? '#d18a4a'
                    : '#5a5448')
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

