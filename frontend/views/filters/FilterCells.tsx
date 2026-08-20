import type { CSSProperties } from 'react'
import type { CellState } from './filterModel'

export const FILTER_BTN_CLASS =
  'inline-flex h-[28px] shrink-0 items-center gap-1.5 rounded-[3px] border border-border-2 px-3 text-[12px] font-medium text-muted transition-colors hover:border-accent-deep hover:text-accent-hot disabled:cursor-not-allowed disabled:opacity-40'

export const FILTER_BTN_PRIMARY_CLASS =
  'inline-flex h-[28px] shrink-0 items-center gap-1.5 rounded-[3px] border border-accent-deep px-3 text-[12px] font-medium text-accent-hot transition-colors hover:border-accent-hot disabled:cursor-not-allowed disabled:opacity-40'

export const FILTER_CHIP_CLASS =
  'inline-flex h-[22px] shrink-0 items-center gap-1.5 rounded-[3px] border px-2 font-mono text-[9.5px] uppercase tracking-[0.14em] transition-colors'

const CELL_CLASS: Record<CellState, string> = {
  hidden: 'border-border-2 hover:border-muted',
  visible: 'border-accent-deep/55 hover:border-accent-deep',
  highlighted: 'border-accent-hot hover:brightness-110',
}

const CELL_STYLE: Record<CellState, CSSProperties> = {
  hidden: {
    background: 'color-mix(in srgb, var(--color-bg) 65%, transparent)',
    boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
  },
  visible: {
    background:
      'color-mix(in srgb, var(--color-accent-deep) 34%, var(--color-panel))',
  },
  highlighted: {
    background: 'var(--color-accent-hot)',
    boxShadow: '0 0 7px rgba(224,184,100,0.55)',
  },
}

interface FilterCellProps {
  state: CellState
  title: string
  onToggle: () => void
  onHighlight?: () => void
}

export function FilterCell({ state, title, onToggle, onHighlight }: FilterCellProps) {
  return (
    <button
      type="button"
      title={title}
      aria-label={title}
      onClick={onToggle}
      onContextMenu={(e) => {
        e.preventDefault()
        onHighlight?.()
      }}
      className={`h-[16px] w-[16px] shrink-0 rounded-[2px] border transition-[filter,border-color,background] ${CELL_CLASS[state]}`}
      style={CELL_STYLE[state]}
    />
  )
}

function LegendSwatch({ state, label }: { state: CellState; label: string }) {
  return (
    <span className="inline-flex items-center gap-1.5">
      <span
        aria-hidden
        className={`h-[11px] w-[11px] rounded-[2px] border ${CELL_CLASS[state]}`}
        style={CELL_STYLE[state]}
      />
      {label}
    </span>
  )
}

export function CellLegend() {
  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5 font-mono text-[9.5px] uppercase tracking-[0.14em] text-faint">
      <LegendSwatch state="visible" label="Shown" />
      <LegendSwatch state="hidden" label="Hidden" />
      <LegendSwatch state="highlighted" label="Highlighted" />
      <span aria-hidden className="h-[12px] w-px bg-border" />
      <span className="normal-case tracking-[0.08em]">
        Left-click toggles · right-click highlights
      </span>
    </div>
  )
}
