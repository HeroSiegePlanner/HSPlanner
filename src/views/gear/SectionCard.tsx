type SectionTone = 'default' | 'satanic' | 'angelic' | 'set'

const SECTION_TONE: Record<
  SectionTone,
  {
    border: string
    dot: string
    label: string
    bg: string
  }
> = {
  default: {
    border: 'border-border',
    dot: 'bg-accent',
    label: 'text-text',
    bg: 'var(--color-panel-2)',
  },
  satanic: {
    border: 'border-stat-red/25',
    dot: 'bg-stat-red',
    label: 'text-stat-red/90',
    bg: 'color-mix(in srgb, var(--color-stat-red) 6%, var(--color-panel-2))',
  },
  angelic: {
    border: 'border-yellow-200/25',
    dot: 'bg-yellow-200',
    label: 'text-yellow-200/90',
    bg: 'color-mix(in srgb, #ece59a 6%, var(--color-panel-2))',
  },
  set: {
    border: 'border-stat-green/25',
    dot: 'bg-stat-green',
    label: 'text-stat-green/90',
    bg: 'color-mix(in srgb, var(--color-stat-green) 6%, var(--color-panel-2))',
  },
}

export function SectionCard({
  label,
  tone = 'default',
  rightSlot,
  bodyClassName = 'p-3',
  children,
}: {
  label: string
  tone?: SectionTone
  rightSlot?: React.ReactNode
  bodyClassName?: string
  children?: React.ReactNode
}) {
  const t = SECTION_TONE[tone]
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

