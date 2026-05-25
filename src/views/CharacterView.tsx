import type { ReactNode } from 'react'
import { classes, gameConfig, getClass } from '../data'
import { attrPointsFor, finalAttributes, useBuild } from '../store/build'

// 1 normally, 5 with Shift, all of `cap` with Ctrl/Cmd+Shift.
function attrStep(e: React.MouseEvent, cap: number): number {
  if ((e.ctrlKey || e.metaKey) && e.shiftKey) return cap
  if (e.shiftKey) return Math.min(5, cap)
  return 1
}

export default function CharacterView() {
  const classId = useBuild((s) => s.classId)
  const level = useBuild((s) => s.level)
  const allocated = useBuild((s) => s.allocated)
  const setClass = useBuild((s) => s.setClass)
  const setLevel = useBuild((s) => s.setLevel)
  const incAttr = useBuild((s) => s.incAttr)
  const decAttr = useBuild((s) => s.decAttr)
  const resetAttrs = useBuild((s) => s.resetAttrs)

  const cls = classId ? getClass(classId) : undefined
  const finals = finalAttributes(classId, allocated)
  const spent = Object.values(allocated).reduce((s, v) => s + v, 0)
  const total = attrPointsFor(level)
  const remaining = total - spent
  const primaryName = cls?.primaryAttribute
    ? (gameConfig.attributes.find((a) => a.key === cls.primaryAttribute)?.name ??
      cls.primaryAttribute)
    : null

  return (
    <div className="max-w-5xl space-y-6">
      <header>
        <div className="mb-1 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
          <span
            aria-hidden
            className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
            style={{ boxShadow: '0 0 8px rgba(224,184,100,0.6)' }}
          />
            Config
        </div>
        <h2
          className="m-0 text-[22px] font-semibold tracking-[0.02em] text-accent-hot"
          style={{ textShadow: '0 0 16px rgba(224,184,100,0.18)' }}
        >
          Character
        </h2>
      </header>

      <section className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <Panel title="Class">
          {classes.length === 0 ? (
            <p className="font-mono text-[12px] tracking-[0.04em] text-muted italic">
              No classes found. Add a file in{' '}
              <code className="rounded bg-panel-2 px-1 font-mono text-accent-hot">
                src/data/classes/*.json
              </code>
              .
            </p>
          ) : (
            <PanelSelect
              value={classId ?? ''}
              onChange={(e) => setClass(e.target.value)}
            >
              {classes.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </PanelSelect>
          )}
          {cls?.description && (
            <p className="mt-3 text-[13px] leading-relaxed text-muted">
              {cls.description}
            </p>
          )}
          {primaryName && (
            <div
              className="mt-3 inline-flex items-center gap-1.5 rounded-[3px] border border-accent-deep/40 px-2 py-1 font-mono text-[10px] uppercase tracking-[0.18em] text-accent-hot"
              style={{
                background:
                  'linear-gradient(180deg, rgba(58,46,24,0.6), rgba(42,36,24,0.4))',
              }}
            >
              <span
                aria-hidden
                className="inline-block h-1 w-1 rotate-45 bg-accent-hot"
                style={{ boxShadow: '0 0 6px rgba(224,184,100,0.6)' }}
              />
              Primary · {primaryName}
            </div>
          )}
        </Panel>

        <Panel title="Level">
          <div className="flex items-center gap-3">
            <input
              type="range"
              min={1}
              max={gameConfig.maxCharacterLevel}
              value={level}
              onChange={(e) => setLevel(Number(e.target.value))}
              className="flex-1"
              style={{
                ['--sl-pct' as never]:
                  ((level - 1) /
                    Math.max(1, gameConfig.maxCharacterLevel - 1)) *
                    100 +
                  '%',
              }}
            />
            <PanelInputWrap>
              <input
                type="number"
                min={1}
                max={gameConfig.maxCharacterLevel}
                value={level}
                onChange={(e) => setLevel(Number(e.target.value))}
                className="w-14 bg-transparent text-center font-mono text-[14px] text-accent-hot tabular-nums outline-none"
              />
            </PanelInputWrap>
          </div>
          <div className="mt-3 flex items-center justify-between font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
            <span>1</span>
            <span className="text-muted">
              Max <span className="text-text">{gameConfig.maxCharacterLevel}</span>{' '}
            </span>
          </div>
        </Panel>
      </section>

      <section>
        <Panel
          title="Attributes"
          right={
            <div className="flex items-center gap-3 font-mono text-[10px] uppercase tracking-[0.14em]">
              <span className="text-faint">Points</span>
              <span
                className={`tabular-nums ${remaining > 0 ? 'text-accent-hot' : 'text-muted'}`}
                style={
                  remaining > 0
                    ? { textShadow: '0 0 8px rgba(224,184,100,0.25)' }
                    : undefined
                }
              >
                {remaining}
              </span>
              <span className="text-faint">/ {total}</span>
              <button
                onClick={resetAttrs}
                disabled={spent === 0}
                className="rounded-[3px] border border-border-2 bg-transparent px-3 py-1 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-muted transition-colors hover:border-stat-red hover:text-stat-red disabled:cursor-not-allowed disabled:opacity-40"
              >
                Reset
              </button>
            </div>
          }
        >
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            {gameConfig.attributes.map((attr) => {
              const base =
                (gameConfig.defaultBaseAttributes?.[attr.key] ?? 0) +
                (cls?.baseAttributes[attr.key] ?? 0)
              const added = allocated[attr.key] ?? 0
              const final = finals[attr.key] ?? 0
              const isPrimary = attr.key === cls?.primaryAttribute
              return (
                <div
                  key={attr.key}
                  className={`relative flex items-center justify-between rounded-[3px] border px-3 py-2.5 transition-colors ${
                    isPrimary ? 'border-accent-deep/60' : 'border-border-2'
                  }`}
                  style={{
                    background:
                      'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
                    boxShadow: isPrimary
                      ? 'inset 0 0 0 1px rgba(224,184,100,0.12)'
                      : 'inset 0 1px 2px rgba(0,0,0,0.4)',
                  }}
                >
                  {isPrimary && (
                    <span
                      aria-hidden
                      className="pointer-events-none absolute left-0 top-0 bottom-0 w-0.5 bg-accent-hot"
                      style={{ boxShadow: '0 0 12px rgba(224,184,100,0.4)' }}
                    />
                  )}
                  <div className="min-w-0">
                    <div className="flex items-center gap-1.5">
                      <span
                        className={`text-[13px] font-medium ${isPrimary ? 'text-accent-hot' : 'text-text'}`}
                      >
                        {attr.name}
                      </span>
                      {isPrimary && (
                        <span className="font-mono text-[9px] uppercase tracking-[0.18em] text-accent-deep">
                          Primary
                        </span>
                      )}
                    </div>
                    <div className="mt-0.5 font-mono text-[10px] uppercase tracking-[0.06em] text-faint">
                      <span>Base</span>{' '}
                      <span className="text-muted">{base}</span>
                      {added > 0 && (
                        <>
                          <span className="text-faint">{' · '}</span>
                          <span className="text-accent-hot">+{added}</span>
                        </>
                      )}
                    </div>
                  </div>
                  <div className="flex shrink-0 items-center gap-1.5">
                    <StepperButton
                      onClick={(e) => decAttr(attr.key, attrStep(e, added))}
                      disabled={added === 0}
                      title="−1 · Shift −5 · Ctrl+Shift remove all"
                      label="−"
                    />
                    <div
                      className="w-12 text-center font-mono text-[18px] font-semibold tabular-nums tracking-[0.02em]"
                      style={
                        added > 0
                          ? {
                              color: 'var(--color-accent-hot)',
                              textShadow: '0 0 10px rgba(224,184,100,0.25)',
                            }
                          : { color: 'var(--color-text)' }
                      }
                    >
                      {final}
                    </div>
                    <StepperButton
                      onClick={(e) => incAttr(attr.key, attrStep(e, remaining))}
                      disabled={remaining === 0}
                      title="+1 · Shift +5 · Ctrl+Shift add all"
                      label="+"
                    />
                  </div>
                </div>
              )
            })}
          </div>
        </Panel>
      </section>
    </div>
  )
}

function StepperButton({
  onClick,
  disabled,
  title,
  label,
}: {
  onClick: (e: React.MouseEvent<HTMLButtonElement>) => void
  disabled?: boolean
  title?: string
  label: string
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      title={title}
      className="flex h-7 w-7 items-center justify-center rounded-[3px] border border-border-2 font-mono text-[14px] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot disabled:cursor-not-allowed disabled:opacity-40"
      style={{
        background:
          'linear-gradient(180deg, var(--color-panel), var(--color-panel-2))',
      }}
    >
      {label}
    </button>
  )
}

function PanelSelect({
  className,
  children,
  ...rest
}: React.SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <div
      className={`inline-flex w-full items-center rounded-[3px] border border-border-2 px-3 py-2 transition-colors hover:border-accent-deep focus-within:border-accent-hot ${className ?? ''}`}
      style={{
        background:
          'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
        boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
      }}
    >
      <select
        {...rest}
        className="w-full cursor-pointer bg-transparent text-[13px] text-text outline-none"
      >
        {children}
      </select>
    </div>
  )
}

function PanelInputWrap({ children }: { children: ReactNode }) {
  return (
    <div
      className="inline-flex items-center rounded-[3px] border border-border-2 px-2 py-1.5 transition-colors hover:border-accent-deep focus-within:border-accent-hot"
      style={{
        background: 'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
        boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
      }}
    >
      {children}
    </div>
  )
}

function Panel({
  title,
  right,
  children,
}: {
  title: string
  right?: ReactNode
  children: ReactNode
}) {
  return (
    <div
      className="relative overflow-hidden rounded-md border border-border p-4"
      style={{
        background:
          'linear-gradient(180deg, var(--color-panel), color-mix(in srgb, var(--color-bg) 70%, transparent))',
        boxShadow:
          'inset 0 1px 0 rgba(255,255,255,0.02), 0 8px 24px rgba(0,0,0,0.35)',
      }}
    >
      <PanelCornerMarks />
      <div className="mb-3 flex items-center justify-between gap-3 border-b border-accent-deep/20 pb-2">
        <div className="flex items-center gap-2">
          <span
            aria-hidden
            className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
            style={{ boxShadow: '0 0 6px rgba(224,184,100,0.5)' }}
          />
          <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-accent-hot/70">
            {title}
          </span>
        </div>
        {right}
      </div>
      {children}
    </div>
  )
}

function PanelCornerMarks() {
  const base: React.CSSProperties = {
    position: 'absolute',
    width: 8,
    height: 8,
    border: '1px solid var(--color-accent-deep)',
    opacity: 0.45,
    pointerEvents: 'none',
  }
  return (
    <>
      <span
        style={{
          ...base,
          top: -1,
          left: -1,
          borderRight: 'none',
          borderBottom: 'none',
        }}
      />
      <span
        style={{
          ...base,
          top: -1,
          right: -1,
          borderLeft: 'none',
          borderBottom: 'none',
        }}
      />
      <span
        style={{
          ...base,
          bottom: -1,
          left: -1,
          borderRight: 'none',
          borderTop: 'none',
        }}
      />
      <span
        style={{
          ...base,
          bottom: -1,
          right: -1,
          borderLeft: 'none',
          borderTop: 'none',
        }}
      />
    </>
  )
}
