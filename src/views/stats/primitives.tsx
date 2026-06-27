import type * as React from 'react'

export function HeroStat({ k, v }: { k: string; v: string }) {
  return (
    <div className="flex flex-col gap-0.5">
      <span className="font-mono text-[9px] uppercase tracking-[0.14em] text-faint">
        {k}
      </span>
      <span className="font-mono text-[13px] font-semibold tabular-nums text-text">
        {v}
      </span>
    </div>
  )
}

export function BDLine({
  label,
  value,
  indent,
}: {
  label: string
  value: React.ReactNode
  indent?: boolean
}) {
  return (
    <div className="flex items-baseline justify-between gap-3 py-0.5 tabular-nums">
      <span className={`text-faint ${indent ? 'pl-3.5 text-muted' : ''}`}>
        {label}
      </span>
      <span className="text-right font-mono font-medium">{value}</span>
    </div>
  )
}

export function BDSection({
  title,
  children,
}: {
  title: string
  children: React.ReactNode
}) {
  return (
    <div className="mt-2 first:mt-0">
      <div className="mb-1 flex items-center gap-2">
        <span className="font-mono text-[9px] font-semibold uppercase tracking-[0.18em] text-accent-deep">
          {title}
        </span>
        <span className="h-px flex-1 bg-border" />
      </div>
      {children}
    </div>
  )
}

export function SubSectionLabel({
  children,
  first,
}: {
  children: React.ReactNode
  first?: boolean
}) {
  return (
    <div className={`mb-1 flex items-center gap-2 ${first ? 'mt-0' : 'mt-3'}`}>
      <span className="font-mono text-[9px] font-semibold uppercase tracking-[0.18em] text-accent-deep">
        {children}
      </span>
      <span className="h-px flex-1 bg-border" />
    </div>
  )
}

export function SectionHeading({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-2.5 font-mono text-[10px] font-semibold uppercase tracking-[0.2em] text-muted">
      <span className="h-px w-3.5 bg-accent-deep" />
      {children}
    </div>
  )
}

export function Panel({
  title,
  meta,
  children,
  padded,
}: {
  title?: string
  meta?: string
  children: React.ReactNode
  padded?: boolean
}) {
  return (
    <div
      className={`relative overflow-hidden rounded-md border border-border ${padded ? 'px-4 pb-3.5 pt-3.5' : 'px-4 pb-3.5 pt-4'}`}
      style={{
        background:
          'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
        boxShadow:
          'inset 0 1px 0 rgba(255,255,255,0.02), 0 8px 24px rgba(0,0,0,0.35)',
      }}
    >
      <CornerMark pos="tl" />
      <CornerMark pos="br" />
      {title && (
        <div className="mb-3 flex items-center gap-2 border-b border-accent-deep/20 pb-2">
          <span
            aria-hidden
            className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
            style={{ boxShadow: '0 0 6px rgba(224,184,100,0.5)' }}
          />
          <h3 className="m-0 font-mono text-[10px] font-semibold uppercase tracking-[0.18em] text-accent-hot/70">
            {title}
          </h3>
          {meta && (
            <span className="ml-auto font-mono text-[9px] uppercase tracking-[0.14em] text-faint">
              {meta}
            </span>
          )}
        </div>
      )}
      {children}
    </div>
  )
}

function CornerMark({ pos }: { pos: 'tl' | 'br' }) {
  const base: React.CSSProperties = {
    position: 'absolute',
    width: 8,
    height: 8,
    border: '1px solid var(--color-accent-deep)',
    opacity: 0.45,
    pointerEvents: 'none',
  }
  if (pos === 'tl') {
    return (
      <span
        style={{
          ...base,
          top: -1,
          left: -1,
          borderRight: 'none',
          borderBottom: 'none',
        }}
      />
    )
  }
  return (
    <span
      style={{
        ...base,
        bottom: -1,
        right: -1,
        borderLeft: 'none',
        borderTop: 'none',
      }}
    />
  )
}
