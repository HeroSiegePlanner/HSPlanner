import type { ReactNode } from 'react'
import Tooltip from './Tooltip'

interface IconActionProps {
  label: string
  danger?: boolean
  disabled?: boolean
  active?: boolean
  onClick: () => void
  children: ReactNode
  /**
   * Show the label through the styled Tooltip at this placement instead of the
   * native `title`. Worth setting for controls near a window edge: the webview
   * clips its own tooltips there and they cannot be repositioned.
   */
  tooltipPlacement?: 'left' | 'right' | 'top' | 'bottom'
}

export function IconAction({
  label,
  danger,
  disabled,
  active,
  onClick,
  children,
  tooltipPlacement,
}: IconActionProps) {
  const tone = danger
    ? 'hover:bg-stat-red/10 hover:text-stat-red'
    : 'hover:bg-accent-hot/10 hover:text-accent-hot'
  const activeTone = danger ? 'text-stat-red' : 'text-accent-hot'
  // A disabled button emits no pointer events, so the Tooltip would never open
  // on the control that most needs to say why it is off. Let the wrapper have them.
  const passThrough = tooltipPlacement ? 'disabled:pointer-events-none' : ''
  const button = (
    <button
      type="button"
      disabled={disabled}
      {...(tooltipPlacement ? {} : { title: label })}
      aria-label={label}
      onClick={onClick}
      className={`flex h-[24px] w-[24px] items-center justify-center rounded-[2px] transition-colors disabled:cursor-not-allowed disabled:opacity-25 ${passThrough} ${
        active ? activeTone : 'text-faint'
      } ${tone}`}
    >
      {children}
    </button>
  )

  if (!tooltipPlacement) return button

  return (
    <Tooltip
      className="inline-flex"
      placement={tooltipPlacement}
      content={<span className="font-mono text-[11px]">{label}</span>}
    >
      {button}
    </Tooltip>
  )
}
