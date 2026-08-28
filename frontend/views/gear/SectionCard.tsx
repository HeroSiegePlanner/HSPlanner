/* eslint-disable react-refresh/only-export-components */
import { createContext, useContext, useState } from 'react'

type SectionTone = 'default' | 'satanic' | 'angelic' | 'set'

const SECTION_TONE: Record<
  SectionTone,
  {
    border: string
    dot: string
    icon: string
    label: string
    bg: string
  }
> = {
  default: {
    border: 'border-border',
    dot: 'bg-accent',
    icon: 'text-accent',
    label: 'text-text',
    bg: 'var(--color-panel-2)',
  },
  satanic: {
    border: 'border-stat-red/25',
    dot: 'bg-stat-red',
    icon: 'text-stat-red/90',
    label: 'text-stat-red/90',
    bg: 'color-mix(in srgb, var(--color-stat-red) 6%, var(--color-panel-2))',
  },
  angelic: {
    border: 'border-yellow-200/25',
    dot: 'bg-yellow-200',
    icon: 'text-yellow-200/90',
    label: 'text-yellow-200/90',
    bg: 'color-mix(in srgb, #ece59a 6%, var(--color-panel-2))',
  },
  set: {
    border: 'border-stat-green/25',
    dot: 'bg-stat-green',
    icon: 'text-stat-green/90',
    label: 'text-stat-green/90',
    bg: 'color-mix(in srgb, var(--color-stat-green) 6%, var(--color-panel-2))',
  },
}

// Lets a parent (GearSlotModal) drive expand-all / collapse-all across cards
// keyed by label; without a provider each card falls back to local state.
export interface SectionsOpenState {
  mode: 'expanded' | 'collapsed' | null
  overrides: Record<string, boolean>
  setSection: (label: string, open: boolean) => void
}

export const SectionsOpenContext = createContext<SectionsOpenState | null>(null)

export function SectionCard({
  label,
  tone = 'default',
  icon,
  rightSlot,
  bodyClassName = 'p-3',
  collapsible = false,
  defaultOpen = true,
  children,
}: {
  label: string
  tone?: SectionTone
  icon?: React.ReactNode
  rightSlot?: React.ReactNode
  bodyClassName?: string
  collapsible?: boolean
  defaultOpen?: boolean
  children?: React.ReactNode
}) {
  const t = SECTION_TONE[tone]
  const ctx = useContext(SectionsOpenContext)
  const [localOpen, setLocalOpen] = useState(defaultOpen)
  if (!collapsible) {
    return (
      <div
        className={`relative overflow-hidden rounded-lg border ${t.border}`}
        style={{ background: t.bg }}
      >
        <header
          className={`flex items-center justify-between gap-2 border-b ${t.border} px-4 py-2.5`}
        >
          <div className="flex items-center gap-2">
            <span
              className={`inline-block h-1.5 w-1.5 rounded-full ${t.dot}`}
              aria-hidden="true"
            />
            <span className={`text-[12px] font-medium tracking-[0.01em] ${t.label}`}>
              {label}
            </span>
          </div>
          {rightSlot && <div className="flex items-center gap-2">{rightSlot}</div>}
        </header>
        {children !== undefined && (
          <div className={bodyClassName}>{children}</div>
        )}
      </div>
    )
  }
  const open = ctx
    ? (ctx.overrides[label] ??
      (ctx.mode === 'expanded'
        ? true
        : ctx.mode === 'collapsed'
          ? false
          : defaultOpen))
    : localOpen
  const setOpen = (v: boolean) =>
    ctx ? ctx.setSection(label, v) : setLocalOpen(v)
  return (
    <details
      open={open}
      onToggle={(e) => {
        const v = e.currentTarget.open
        if (v !== open) setOpen(v)
      }}
      className={`group/card relative overflow-hidden rounded-lg border ${t.border}`}
      style={{ background: t.bg }}
    >
      <summary className="flex cursor-pointer list-none items-center justify-between gap-2 bg-black/20 px-4 py-2.5 select-none transition-colors hover:bg-black/10 [&::-webkit-details-marker]:hidden">
        <div className="flex items-center gap-2">
          {icon ? (
            <span
              className={`flex h-3.5 w-3.5 shrink-0 items-center justify-center ${t.icon}`}
              aria-hidden="true"
            >
              {icon}
            </span>
          ) : (
            <span
              className={`inline-block h-1.5 w-1.5 rounded-full ${t.dot}`}
              aria-hidden="true"
            />
          )}
          <span className={`text-[12px] font-medium tracking-[0.01em] ${t.label}`}>
            {label}
          </span>
        </div>
        <div className="flex items-center gap-2.5">
          {rightSlot && (
            <div
              className="flex items-center gap-2"
              // header actions must not toggle the spoiler, but working with one means the body is wanted open
              onClick={(e) => {
                e.preventDefault()
                setOpen(true)
              }}
            >
              {rightSlot}
            </div>
          )}
          <span
            aria-hidden
            className="font-mono text-[13px] leading-none text-muted transition-transform group-open/card:rotate-90"
          >
            ▸
          </span>
        </div>
      </summary>
      {children !== undefined && (
        <div className={`border-t ${t.border} ${bodyClassName}`}>{children}</div>
      )}
    </details>
  )
}
