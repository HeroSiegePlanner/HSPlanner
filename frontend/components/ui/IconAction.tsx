import type { ReactNode } from 'react'

interface IconActionProps {
  label: string
  danger?: boolean
  disabled?: boolean
  active?: boolean
  onClick: () => void
  children: ReactNode
}

export function IconAction({
  label,
  danger,
  disabled,
  active,
  onClick,
  children,
}: IconActionProps) {
  const tone = danger
    ? 'hover:bg-stat-red/10 hover:text-stat-red'
    : 'hover:bg-accent-hot/10 hover:text-accent-hot'
  const activeTone = danger ? 'text-stat-red' : 'text-accent-hot'
  return (
    <button
      type="button"
      disabled={disabled}
      title={label}
      aria-label={label}
      onClick={onClick}
      className={`flex h-[24px] w-[24px] items-center justify-center rounded-[2px] transition-colors disabled:cursor-not-allowed disabled:opacity-25 ${
        active ? activeTone : 'text-faint'
      } ${tone}`}
    >
      {children}
    </button>
  )
}
