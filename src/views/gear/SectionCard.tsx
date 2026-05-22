type SectionTone = 'default' | 'satanic' | 'angelic' | 'set' | 'warning'

const SECTION_TONE: Record<
  SectionTone,
  {
    border: string
    accentDot: string
    label: string
    bg: string
  }
> = {
  default: {
    border: 'border-accent-deep/25',
    accentDot: 'bg-accent-deep',
    label: 'text-accent-hot/75',
    bg: 'linear-gradient(180deg, rgba(28,29,36,0.55), rgba(13,14,18,0.35))',
  },
  satanic: {
    border: 'border-red-500/30',
    accentDot: 'bg-red-500',
    label: 'text-red-300/85',
    bg: 'linear-gradient(180deg, rgba(120,30,30,0.18), rgba(60,15,15,0.12))',
  },
  angelic: {
    border: 'border-yellow-200/30',
    accentDot: 'bg-yellow-200',
    label: 'text-yellow-200/85',
    bg: 'linear-gradient(180deg, rgba(120,100,40,0.18), rgba(50,40,15,0.12))',
  },
  set: {
    border: 'border-green-500/30',
    accentDot: 'bg-green-400',
    label: 'text-green-300/85',
    bg: 'linear-gradient(180deg, rgba(40,90,55,0.18), rgba(15,40,25,0.12))',
  },
  warning: {
    border: 'border-amber-500/30',
    accentDot: 'bg-amber-400',
    label: 'text-amber-200/85',
    bg: 'linear-gradient(180deg, rgba(120,90,30,0.16), rgba(50,40,15,0.1))',
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
  // Shared "section card" wrapper used by every editor block in the GearSlotModal right column. Renders a panel-2 gradient frame with a mono-uppercase header (rotated diamond accent dot + label + optional right-slot for counts/buttons) and a configurable body, so SocketsSection/StarsSection/AffixesSection/etc. all share the same dossier-card visual language as the modal shell.
  const t = SECTION_TONE[tone]
  return (
    <div
      className={`relative overflow-hidden rounded-sm border ${t.border}`}
      style={{ background: t.bg }}
    >
      <header
        className={`flex items-center justify-between gap-2 border-b ${t.border} px-3.5 py-2`}
      >
        <div className="flex items-center gap-2">
          <span
            className={`inline-block h-1 w-1 rotate-45 ${t.accentDot}`}
            aria-hidden="true"
          />
          <span
            className={`font-mono text-[10px] uppercase tracking-[0.18em] ${t.label}`}
          >
            {label}
          </span>
        </div>
        {rightSlot && (
          <div className="flex items-center gap-2">{rightSlot}</div>
        )}
      </header>
      {children !== undefined && (
        <div className={bodyClassName}>{children}</div>
      )}
    </div>
  )
}

export function ConfigSectionHeader({
  label,
  accent,
}: {
  label: string
  accent?: string
}) {
  // Sticky header that sits at the top of the GearSlotModal right column, mirroring the small-caps "GEAR SLOT · X" treatment used by the modal's main header but scoped to whatever the right pane is currently showing (item configuration / compare view).
  return (
    <div
      className="sticky top-0 z-1 flex items-center gap-2 border-b border-accent-deep/30 px-4 py-2"
      style={{ background: 'var(--color-panel-2)' }}
    >
      <span
        className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
        style={{ boxShadow: '0 0 8px rgba(224,184,100,0.6)' }}
        aria-hidden="true"
      />
      <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
        {label}
      </span>
      {accent && (
        <>
          <span className="text-faint">·</span>
          <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-accent-hot truncate">
            {accent}
          </span>
        </>
      )}
    </div>
  )
}

export function ConfigEmptyState({
  title,
  hint,
  tone = 'default',
}: {
  title: string
  hint: string
  tone?: 'default' | 'warning'
}) {
  // Centered empty/locked state for the GearSlotModal right column: rotated-diamond glyph in a subtle accent ring, a strong title, and a mono-uppercase hint. Visually echoes the modal's corner-bracket framing so the empty pane never looks blank or unfinished.
  const isWarning = tone === 'warning'
  const ring = isWarning ? 'border-amber-400/40' : 'border-accent-deep/40'
  const glow = isWarning ? 'rgba(245,180,80,0.16)' : 'rgba(201,165,90,0.14)'
  const iconColor = isWarning ? 'bg-amber-400' : 'bg-accent-deep'
  const titleColor = isWarning ? 'text-amber-200' : 'text-muted'

  return (
    <div className="flex flex-1 items-center justify-center p-8 text-center text-faint">
      <div className="flex flex-col items-center">
        <div
          className={`mb-4 flex h-14.5 w-14.5 items-center justify-center rounded-full border border-dashed ${ring}`}
          style={{
            background: `radial-gradient(circle, ${glow}, transparent 70%)`,
          }}
        >
          <span
            className={`block h-3 w-3 rotate-45 ${iconColor}`}
            aria-hidden="true"
          />
        </div>
        <div
          className={`mb-1.5 text-[14px] font-semibold tracking-[0.08em] ${titleColor}`}
        >
          {title}
        </div>
        <div className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
          {hint}
        </div>
      </div>
    </div>
  )
}
