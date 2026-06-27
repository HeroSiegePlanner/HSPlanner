import { useMemo, useState, type ReactNode } from 'react'
import PickerModal, { type PickerPanelState, type PickerRow } from '../../../components/PickerModal'
import Tooltip, { TooltipPanel } from '../../../components/Tooltip'
import { getGem, getRune } from '../../../data'
import { RAINBOW_MULTIPLIER } from '../../../store/build'
import type { EquippedItem, ItemBase, SocketType } from '../../../types'
import { buildSocketableTooltip, NetChangeBlock } from '../tooltips'
import { SectionCard } from '../SectionCard'

function SocketPickerTrigger({
  socketIndex,
  socketed,
  socketType,
  rows,
  onChange,
}: {
  socketIndex: number
  socketed: string | null
  socketType: SocketType
  rows: PickerRow[]
  onChange: (id: string | null) => void
}) {
  const [open, setOpen] = useState(false)
  const multiplier = socketType === 'rainbow' ? RAINBOW_MULTIPLIER : 1
  const previousStats = useMemo<Record<string, number>>(() => {
    if (!socketed) return {}
    const src = getGem(socketed) ?? getRune(socketed)
    if (!src) return {}
    const out: Record<string, number> = {}
    for (const [k, v] of Object.entries(src.stats)) out[k] = v * multiplier
    return out
  }, [socketed, multiplier])

  const enrichedRows = useMemo<PickerRow[]>(
    () =>
      rows.map((r) => {
        const gem = getGem(r.id)
        const rune = !gem ? getRune(r.id) : undefined
        const src = gem ?? rune
        if (!src) return r
        const kind: 'GEM' | 'JEWEL' | 'RUNE' = rune
          ? 'RUNE'
          : src.name.toLowerCase().includes('jewel')
            ? 'JEWEL'
            : 'GEM'
        return {
          ...r,
          tooltip: buildSocketableTooltip(src, kind, {
            previousStats,
            multiplier,
          }),
        }
      }),
    [rows, previousStats, multiplier],
  )

  const triggerTooltip = useMemo<ReactNode>(() => {
    if (!socketed) return null
    const gem = getGem(socketed)
    const rune = !gem ? getRune(socketed) : undefined
    const src = gem ?? rune
    if (!src) return null
    const kind: 'GEM' | 'JEWEL' | 'RUNE' = rune
      ? 'RUNE'
      : src.name.toLowerCase().includes('jewel')
        ? 'JEWEL'
        : 'GEM'
    return buildSocketableTooltip(src, kind, { multiplier })
  }, [socketed, multiplier])

  const renderSelectedPanel = (state: PickerPanelState): ReactNode => {
    if (!triggerTooltip || !state.selectedId) return null
    let hoveredScaled: Record<string, number> | undefined
    if (state.hoveredId && state.hoveredId !== state.selectedId) {
      const hg = getGem(state.hoveredId)
      const hr = !hg ? getRune(state.hoveredId) : undefined
      const hsrc = hg ?? hr
      if (hsrc) {
        const out: Record<string, number> = {}
        for (const [k, v] of Object.entries(hsrc.stats)) {
          out[k] = v * multiplier
        }
        hoveredScaled = out
      }
    }
    return (
      <TooltipPanel className="w-full">
        {triggerTooltip}
        {hoveredScaled && (
          <NetChangeBlock previous={previousStats} next={hoveredScaled} />
        )}
      </TooltipPanel>
    )
  }

  const current = socketed ? rows.find((r) => r.id === socketed) : undefined

  const trigger = (
    <button
      type="button"
      onClick={() => setOpen(true)}
      className="group flex w-full items-center justify-between gap-2 rounded-xs border border-accent-deep/25 bg-panel-2/40 px-2 py-1 text-left transition-colors hover:border-accent-hot/50"
    >
      <span className="flex min-w-0 items-center gap-1.5">
        {current?.iconUrl ? (
          <img
            src={current.iconUrl}
            alt=""
            width={16}
            height={16}
            style={{ imageRendering: 'pixelated' }}
          />
        ) : current ? (
          <span
            className="block h-3 w-3 rotate-45 rounded-[1px]"
            style={{
              background: `linear-gradient(135deg, ${
                current.iconColor ?? 'var(--color-faint)'
              }, #0d0b07)`,
              border: `1px solid color-mix(in srgb, ${
                current.iconColor ?? 'var(--color-faint)'
              } 60%, #000)`,
            }}
            aria-hidden="true"
          />
        ) : (
          <span
            className="block h-3 w-3 rotate-45 rounded-[1px] border border-dashed border-accent-deep/40"
            aria-hidden="true"
          />
        )}
        <span
          className={`truncate text-[12px] ${
            current
              ? 'text-text group-hover:text-accent-hot'
              : 'italic text-faint'
          }`}
        >
          {current ? current.name : 'Empty socket'}
        </span>
        {current?.tier !== undefined && (
          <span className="ml-1 rounded-xs border border-accent-deep/40 px-1 py-px font-mono text-[9px] tabular-nums text-accent-hot/75">
            T{current.tier}
          </span>
        )}
      </span>
      <span className="font-mono text-[9px] uppercase tracking-[0.14em] text-faint group-hover:text-accent-hot">
        Browse →
      </span>
    </button>
  )

  return (
    <>
      {triggerTooltip ? (
        <Tooltip content={triggerTooltip} placement="right" delay={120}>
          {trigger}
        </Tooltip>
      ) : (
        trigger
      )}
      {open && (
        <PickerModal
          title="Insert Socketable"
          sectionLabel="Socket"
          sectionAccent={`#${socketIndex + 1}`}
          rows={enrichedRows}
          selectedId={socketed}
          searchPlaceholder="Search gems / runes…"
          emptyMessage="No matches"
          width={680}
          allowClear={!!socketed}
          onClear={() => onChange(null)}
          onSelect={(id) => onChange(id)}
          onClose={() => setOpen(false)}
          selectedPanel={renderSelectedPanel}
        />
      )}
    </>
  )
}

function SocketTypeToggle({
  value,
  onChange,
}: {
  value: SocketType
  onChange: (t: SocketType) => void
}) {
  return (
    <div className="flex shrink-0 overflow-hidden rounded-xs border border-accent-deep/30 font-mono text-[10px] font-semibold tracking-[0.06em]">
      <button
        type="button"
        onClick={() => onChange('normal')}
        className={`px-2 py-0.5 transition-colors ${
          value === 'normal'
            ? 'bg-accent-deep/20 text-accent-hot'
            : 'bg-bg/40 text-faint hover:text-muted'
        }`}
      >
        N
      </button>
      <button
        type="button"
        onClick={() => onChange('rainbow')}
        title="Rainbow socket: +50% effect"
        className={`px-2 py-0.5 transition-colors ${
          value === 'rainbow'
            ? 'bg-linear-to-r from-rose-500 via-amber-400 to-sky-400 text-bg'
            : 'bg-bg/40 text-faint hover:text-muted'
        }`}
      >
        R
      </button>
    </div>
  )
}

export function SocketsSection({
  equipped,
  maxSockets,
  base,
  socketPickerRows,
  onSocketCount,
  onSocketed,
  onSocketType,
}: {
  equipped: EquippedItem
  maxSockets: number
  base: ItemBase
  socketPickerRows: PickerRow[]
  onSocketCount: (n: number) => void
  onSocketed: (idx: number, id: string | null) => void
  onSocketType: (idx: number, type: SocketType) => void
}) {
  if (maxSockets === 0) return null
  return (
    <SectionCard
      label="Sockets"
      rightSlot={
        <>
          <button
            onClick={() => onSocketCount(equipped.socketCount - 1)}
            disabled={equipped.socketCount === 0}
            className="flex h-5 w-5 items-center justify-center rounded-xs border border-accent-deep/40 bg-bg/60 font-mono text-[12px] leading-none text-muted transition-colors hover:border-accent-hot hover:text-accent-hot disabled:cursor-not-allowed disabled:opacity-30"
            aria-label="Decrease sockets"
          >
            −
          </button>
          <span className="min-w-10.5 text-center font-mono text-[11px] tabular-nums text-accent-hot">
            {equipped.socketCount}/{maxSockets}
          </span>
          <button
            onClick={() => onSocketCount(equipped.socketCount + 1)}
            disabled={equipped.socketCount >= maxSockets}
            className="flex h-5 w-5 items-center justify-center rounded-xs border border-accent-deep/40 bg-bg/60 font-mono text-[12px] leading-none text-muted transition-colors hover:border-accent-hot hover:text-accent-hot disabled:cursor-not-allowed disabled:opacity-30"
            aria-label="Increase sockets"
          >
            +
          </button>
        </>
      }
      bodyClassName={
        equipped.socketCount > 0
          ? 'grid grid-cols-2 gap-1.5 p-2'
          : 'px-3 py-2'
      }
    >
      {equipped.socketCount > 0 ? (
        <>
          {Array.from({ length: equipped.socketCount }).map((_, i) => {
            const socketed = equipped.socketed[i]
            const type = equipped.socketTypes[i] ?? 'normal'
            return (
              <div
                key={i}
                className="flex items-center gap-1.5 rounded-[3px] border border-accent-deep/15 bg-bg/40 p-1.5"
              >
                <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-xs border border-accent-deep/30 bg-panel-2/60 font-mono text-[10px] tabular-nums text-accent-hot/80">
                  {i + 1}
                </span>
                <SocketTypeToggle
                  value={type}
                  onChange={(t) => onSocketType(i, t)}
                />
                <div className="min-w-0 flex-1">
                  <SocketPickerTrigger
                    socketIndex={i}
                    socketed={socketed ?? null}
                    socketType={type}
                    rows={socketPickerRows}
                    onChange={(id) => onSocketed(i, id)}
                  />
                </div>
              </div>
            )
          })}
          {base.sockets !== undefined &&
            base.sockets !== equipped.socketCount && (
              <div className="col-span-2 pt-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-faint">
                base · {base.sockets}
              </div>
            )}
        </>
      ) : (
        <div className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint italic">
          No sockets allocated
        </div>
      )}
    </SectionCard>
  )
}
