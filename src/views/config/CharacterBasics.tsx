import type { ReactNode } from 'react'
import Dropdown from '../../components/Dropdown'
import { Panel } from './primitives'
import { classes, gameConfig, getClass } from '../../data'
import { attrPointsFor, finalAttributes, useBuild } from '../../store/build'

function attrStep(e: React.MouseEvent, cap: number): number {
  if ((e.ctrlKey || e.metaKey) && e.shiftKey) return cap
  if (e.shiftKey) return Math.min(5, cap)
  return 1
}

export default function CharacterBasics() {
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

  return (
    <>
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <Panel title="Class" subtitle="The class this build is based on.">
          {classes.length === 0 ? (
            <p className="font-mono text-[12px] tracking-[0.04em] text-muted italic">
              No classes found. Add a file in{' '}
              <code className="rounded-[3px] bg-panel-2 px-1 font-mono text-accent-hot">
                src/data/classes/*.json
              </code>
              .
            </p>
          ) : (
            <Dropdown
              value={classId ?? null}
              onChange={(id) => {
                if (id) setClass(id)
              }}
              options={classes.map((c) => ({ id: c.id, label: c.name }))}
              placeholder="Select a class…"
              searchPlaceholder="Search class…"
            />
          )}
          {cls?.description && (
            <p className="mt-3 text-[12px] leading-relaxed text-muted">
              {cls.description}
            </p>
          )}
        </Panel>

        <Panel
          title="Level"
          subtitle="Sets how many attribute and skill points you have."
        >
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
                className="w-12 bg-transparent text-center font-mono text-[12px] tabular-nums text-accent-hot outline-none"
              />
            </PanelInputWrap>
          </div>
        </Panel>
      </div>

      <Panel
        title="Attributes"
        subtitle="Allocate attribute points. Shift = ×5 · Ctrl/Cmd+Shift = all."
        trailing={
          <span className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.14em]">
            <span
              className={remaining > 0 ? 'text-accent-hot' : 'text-muted'}
              style={
                remaining > 0
                  ? { textShadow: '0 0 8px rgba(224,184,100,0.25)' }
                  : undefined
              }
            >
              {remaining}
            </span>
            <span className="text-faint">/ {total} free</span>
            <button
              onClick={resetAttrs}
              disabled={spent === 0}
              className="ml-1 rounded-[3px] border border-border-2 bg-transparent px-2 py-1 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-muted transition-colors hover:border-stat-red hover:text-stat-red disabled:cursor-not-allowed disabled:opacity-40"
            >
              Reset
            </button>
          </span>
        }
      >
        <ul className="grid grid-cols-1 gap-2 sm:grid-cols-2">
          {gameConfig.attributes.map((attr) => {
            const base =
              (gameConfig.defaultBaseAttributes?.[attr.key] ?? 0) +
              (cls?.baseAttributes[attr.key] ?? 0)
            const added = allocated[attr.key] ?? 0
            const final = finals[attr.key] ?? 0
            const isPrimary = attr.key === cls?.primaryAttribute
            return (
              <li key={attr.key}>
                <div
                  className={`flex items-center justify-between gap-2 rounded-[3px] border px-3 py-2 transition-colors ${
                    added > 0 ? 'border-accent-deep' : 'border-border-2'
                  }`}
                  style={{
                    background:
                      added > 0
                        ? 'linear-gradient(180deg, rgba(58,46,24,0.5), rgba(28,29,36,0.5))'
                        : 'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
                    boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.4)',
                  }}
                >
                  <span className="min-w-0">
                    <span
                      className={`block truncate text-sm font-medium ${added > 0 ? 'text-accent-hot' : 'text-text'}`}
                    >
                      {attr.name}
                    </span>
                    <span className="block font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
                      Base {base}
                      {isPrimary && (
                        <span className="text-accent-deep"> · Primary</span>
                      )}
                    </span>
                  </span>
                  <div
                    className="inline-flex shrink-0 items-center rounded-[3px] border border-border-2"
                    style={{
                      background:
                        'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
                      boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
                    }}
                  >
                    <SpinButton
                      onClick={(e) => decAttr(attr.key, attrStep(e, added))}
                      disabled={added === 0}
                      label="−"
                      title="−1 · Shift −5 · Ctrl+Shift remove all"
                    />
                    <span className="w-9 text-center font-mono text-[13px] font-semibold tabular-nums text-accent-hot">
                      {final}
                    </span>
                    <SpinButton
                      onClick={(e) => incAttr(attr.key, attrStep(e, remaining))}
                      disabled={remaining === 0}
                      label="+"
                      title="+1 · Shift +5 · Ctrl+Shift add all"
                    />
                  </div>
                </div>
              </li>
            )
          })}
        </ul>
      </Panel>
    </>
  )
}

function SpinButton({
  onClick,
  disabled,
  label,
  title,
}: {
  onClick: (e: React.MouseEvent<HTMLButtonElement>) => void
  disabled?: boolean
  label: string
  title?: string
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      title={title}
      className="px-2.5 py-1.5 font-mono text-[14px] leading-none text-muted transition-colors hover:text-accent-hot disabled:cursor-not-allowed disabled:opacity-30"
    >
      {label}
    </button>
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

