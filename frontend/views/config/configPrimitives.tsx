export { Panel } from '../../components/ui/Panel'

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

