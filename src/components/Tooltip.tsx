import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import type { ReactNode } from 'react'
import { TONE_BORDER, TONE_GLOW, TONE_RGB, TONE_TEXT } from './tooltip-tones'
import type { TooltipTone } from './tooltip-tones'

interface TooltipProps {
  content: ReactNode
  children: ReactNode
  tone?: TooltipTone
  delay?: number
  className?: string
  disabled?: boolean
  placement?: 'right' | 'left' | 'top' | 'bottom'
  zIndex?: number
}

export default function Tooltip({
  content,
  children,
  tone = 'neutral',
  delay = 80,
  className,
  disabled,
  placement = 'right',
  zIndex = 1000,
}: TooltipProps) {
  const [visible, setVisible] = useState(false)
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null)
  const triggerRef = useRef<HTMLDivElement>(null)
  const tooltipRef = useRef<HTMLDivElement>(null)
  const timer = useRef<number | null>(null)

  const show = () => {
    if (disabled) return
    if (timer.current) clearTimeout(timer.current)
    timer.current = window.setTimeout(() => setVisible(true), delay)
  }
  const hide = () => {
    if (timer.current) clearTimeout(timer.current)
    setVisible(false)
    setPos(null)
  }

  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current)
    },
    [],
  )

  useLayoutEffect(() => {
    if (!visible || !triggerRef.current || !tooltipRef.current) return
    const trigger = triggerRef.current.getBoundingClientRect()
    const t = tooltipRef.current.getBoundingClientRect()
    const margin = 8
    const vw = window.innerWidth
    const vh = window.innerHeight

    const compute = (p: 'right' | 'left' | 'top' | 'bottom') => {
      if (p === 'right') return { left: trigger.right + margin, top: trigger.top }
      if (p === 'left') return { left: trigger.left - t.width - margin, top: trigger.top }
      if (p === 'top') return { left: trigger.left, top: trigger.top - t.height - margin }
      return { left: trigger.left, top: trigger.bottom + margin }
    }

    const fits = (l: number, tp: number) =>
      l >= margin &&
      l + t.width <= vw - margin &&
      tp >= margin &&
      tp + t.height <= vh - margin

    let p = compute(placement)
    if (!fits(p.left, p.top)) {
      const fallbacks: ('right' | 'left' | 'top' | 'bottom')[] = [
        'right',
        'left',
        'bottom',
        'top',
      ]
      for (const f of fallbacks) {
        if (f === placement) continue
        const c = compute(f)
        if (fits(c.left, c.top)) {
          p = c
          break
        }
      }
    }
    const left = Math.min(Math.max(p.left, margin), vw - t.width - margin)
    const top = Math.min(Math.max(p.top, margin), vh - t.height - margin)
    setPos({ left, top })
  }, [visible, placement])

  return (
    <>
      <div
        ref={triggerRef}
        onMouseEnter={show}
        onMouseLeave={hide}
        onFocus={show}
        onBlur={hide}
        className={className}
      >
        {children}
      </div>
      {visible &&
        createPortal(
          <div
            ref={tooltipRef}
            role="tooltip"
            className={`fixed min-w-[220px] max-w-[360px] bg-panel border ${TONE_BORDER[tone]} ${TONE_GLOW[tone]} rounded-[4px] overflow-hidden pointer-events-none select-none shadow-[0_8px_32px_rgba(0,0,0,0.8)]`}
            style={{
              left: pos?.left ?? -9999,
              top: pos?.top ?? -9999,
              opacity: pos ? 1 : 0,
              zIndex,
              transition: 'opacity 80ms ease-out',
            }}
          >
            {content}
          </div>,
          document.body,
        )}
    </>
  )
}

export function TooltipPanel({
  children,
  tone = 'neutral',
  className,
  width,
}: {
  children: ReactNode
  tone?: TooltipTone
  className?: string
  width?: number | string
}) {
  return (
    <div
      className={`bg-panel border ${TONE_BORDER[tone]} ${TONE_GLOW[tone]} rounded-[4px] overflow-hidden select-none shadow-[0_8px_32px_rgba(0,0,0,0.8)] ${className ?? ''}`}
      style={width !== undefined ? { width } : undefined}
    >
      {children}
    </div>
  )
}

export function TooltipHeader({
  title,
  subtitle,
  tone = 'neutral',
  image,
}: {
  title: ReactNode
  subtitle?: ReactNode
  tone?: TooltipTone
  image?: string
}) {
  const rgb = TONE_RGB[tone]
  return (
    <div
      className="relative px-3 py-2 border-b border-border/70 overflow-hidden"
      style={{
        background: `linear-gradient(180deg, rgba(${rgb}, 0.14), rgba(${rgb}, 0.04))`,
      }}
    >
      <div className={image ? 'pr-14' : ''}>
        <div
          className={`relative text-[13px] font-semibold leading-tight tracking-[0.02em] ${TONE_TEXT[tone]}`}
          style={{
            textShadow: `0 0 10px rgba(${rgb}, 0.45), 0 0 4px rgba(${rgb}, 0.25)`,
          }}
        >
          {title}
        </div>
        {subtitle && (
          <div className="relative mt-0.5 text-[10px] font-medium uppercase tracking-[0.12em] text-faint">
            {subtitle}
          </div>
        )}
      </div>
      {image && (
        <div className="absolute top-1/2 right-2 -translate-y-1/2 w-12 h-12 flex items-center justify-center">
          <img
            src={image}
            alt=""
            className="max-w-full max-h-full object-contain"
            style={{ imageRendering: 'pixelated' }}
            draggable={false}
          />
        </div>
      )}
    </div>
  )
}

export function TooltipSection({
  children,
  className,
}: {
  children: ReactNode
  className?: string
}) {
  return (
    <div className={`px-3 py-2 border-t border-border/70 first:border-t-0 ${className ?? ''}`}>
      {children}
    </div>
  )
}

export type TooltipSectionTone =
  | 'gold'
  | 'pink'
  | 'red'
  | 'green'
  | 'muted'
  | 'blue'
  | 'orange'

const TOOLTIP_SECTION_HEADER_TONE: Record<TooltipSectionTone, string> = {
  gold: 'text-accent-hot/85 bg-accent-deep/10',
  pink: 'text-pink-300 bg-pink-400/10',
  red: 'text-red-300 bg-red-500/10',
  green: 'text-green-300 bg-green-500/10',
  muted: 'text-muted bg-panel-2/60',
  blue: 'text-stat-blue bg-stat-blue/10',
  orange: 'text-stat-orange bg-stat-orange/10',
}

export function TooltipSectionHeader({
  children,
  trailing,
  tone = 'gold',
}: {
  children: ReactNode
  trailing?: ReactNode
  tone?: TooltipSectionTone
}) {
  return (
    <div
      className={`-mx-3 -mt-2 mb-2 px-3 py-1 flex items-center justify-between text-[10px] font-semibold uppercase tracking-[0.12em] border-b border-border/40 ${TOOLTIP_SECTION_HEADER_TONE[tone]}`}
    >
      <span>{children}</span>
      {trailing != null && (
        <span className="font-mono normal-case text-text/70 tracking-normal">
          {trailing}
        </span>
      )}
    </div>
  )
}

export function TooltipFooter({ children }: { children: ReactNode }) {
  return (
    <div className="px-3 py-1.5 border-t border-border/70 bg-panel-2 text-[10px] font-medium uppercase tracking-[0.12em] text-faint">
      {children}
    </div>
  )
}

export function TooltipStat({
  label,
  value,
  variant = 'default',
}: {
  label: ReactNode
  value: ReactNode
  variant?: 'default' | 'muted' | 'red' | 'blue' | 'green'
}) {
  const valueColor = {
    default: 'text-accent-hot',
    muted: 'text-faint',
    red: 'text-stat-red',
    blue: 'text-stat-blue',
    green: 'text-stat-green',
  }[variant]
  return (
    <div className="flex items-baseline justify-between gap-3 leading-[1.65] text-[12px]">
      <span className="text-text/90">{label}</span>
      <span className={`font-mono tabular-nums ${valueColor}`}>{value}</span>
    </div>
  )
}

export function TooltipText({ children }: { children: ReactNode }) {
  return <div className="text-[12px] leading-[1.55] text-text/90">{children}</div>
}

export function UnsupportedModsList({ lines }: { lines: ReactNode[] }) {
  return (
    <>
      <div className="mb-1 text-[10px] uppercase tracking-[0.12em] text-muted">
        Not Yet Supported
      </div>
      <ul className="space-y-0.5 opacity-60">
        {lines.map((line, i) => (
          <li key={i} className="text-[12px] leading-[1.55] text-text/90">
            {line}
          </li>
        ))}
      </ul>
      <p className="mt-1 text-[10px] italic text-muted/70">
        These mods are not yet calculated by the planner.
      </p>
    </>
  )
}
