import type { ReactNode } from 'react'
import { CornerMarks } from '../../components/CornerMarks'

export function GroupHeading({
  title,
  subtitle,
}: {
  title: string
  subtitle?: string
}) {
  return (
    <div className="pt-1">
      <div className="flex items-center gap-3">
        <span
          aria-hidden
          className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
          style={{ boxShadow: '0 0 8px rgba(224,184,100,0.6)' }}
        />
        <span className="font-mono text-[11px] font-semibold uppercase tracking-[0.2em] text-accent-hot/80">
          {title}
        </span>
        <span
          aria-hidden
          className="h-px flex-1"
          style={{
            background:
              'linear-gradient(90deg, rgba(201,165,90,0.35), transparent)',
          }}
        />
      </div>
      {subtitle && (
        <p className="mt-1.5 pl-4 text-[12px] leading-relaxed text-muted">
          {subtitle}
        </p>
      )}
    </div>
  )
}

export function CountBadge({
  value,
  total,
  highlight,
}: {
  value: number
  total?: number
  highlight?: boolean
}) {
  return (
    <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
      <span className={highlight ? 'text-accent-hot' : 'text-muted'}>
        {value}
      </span>
      {total !== undefined && <span className="text-faint"> / {total}</span>}{' '}
      {total !== undefined ? 'active' : 'override' + (value === 1 ? '' : 's')}
    </span>
  )
}

export function Panel({
  title,
  subtitle,
  trailing,
  children,
}: {
  title: string
  subtitle?: string
  trailing?: ReactNode
  children: ReactNode
}) {
  return (
    <section
      className="relative overflow-hidden rounded-md border border-border p-4"
      style={{
        background:
          'linear-gradient(180deg, var(--color-panel), color-mix(in srgb, var(--color-bg) 70%, transparent))',
        boxShadow:
          'inset 0 1px 0 rgba(255,255,255,0.02), 0 8px 24px rgba(0,0,0,0.35)',
      }}
    >
      <CornerMarks size={8} opacity={0.45} />
      <div className="mb-3 flex items-center justify-between gap-3 border-b border-accent-deep/20 pb-2">
        <div className="flex items-center gap-2">
          <span
            aria-hidden
            className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
            style={{ boxShadow: '0 0 6px rgba(224,184,100,0.5)' }}
          />
          <h3 className="m-0 font-mono text-[10px] font-semibold uppercase tracking-[0.18em] text-accent-hot/70">
            {title}
          </h3>
        </div>
        {trailing}
      </div>
      {subtitle && (
        <p className="mb-3 text-[12px] leading-relaxed text-muted">{subtitle}</p>
      )}
      {children}
    </section>
  )
}
