import { useMemo } from 'react'
import {
  ETHER_MAGIC_FIND_KEY,
  etherMagicFindTotal,
  formatEtherTotal,
  groupEtherSummary,
  summarizeEtherNodes,
} from '../../utils/build/etherSummary'

interface Props {
  allocated: ReadonlySet<number>
}

export function EtherSummaryPanel({ allocated }: Props) {
  const summary = useMemo(() => summarizeEtherNodes(allocated), [allocated])
  const magicFind = useMemo(() => etherMagicFindTotal(allocated), [allocated])
  const groups = useMemo(
    () =>
      groupEtherSummary(summary.filter((e) => e.key !== ETHER_MAGIC_FIND_KEY)),
    [summary],
  )

  return (
    <div
      className="pointer-events-auto flex max-h-[calc(100vh-190px)] w-72 flex-col rounded-[3px] border border-border"
      style={{
        background:
          'linear-gradient(180deg, color-mix(in srgb, var(--color-panel-2) 88%, transparent), color-mix(in srgb, var(--color-bg) 82%, transparent))',
        backdropFilter: 'blur(6px)',
        boxShadow:
          'inset 0 1px 0 rgba(165,116,201,0.08), 0 8px 24px rgba(0,0,0,0.45)',
      }}
    >
      <div className="flex items-center justify-between border-b border-border px-3 py-2">
        <span className="inline-flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.18em] text-stat-purple">
          <span
            aria-hidden
            className="inline-block h-1.5 w-1.5 rotate-45 bg-stat-purple"
            style={{ boxShadow: '0 0 8px rgba(165,116,201,0.6)' }}
          />
          Stat Summary
        </span>
        <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
          {allocated.size} <span className="text-faint/70">nodes</span>
        </span>
      </div>

      <div
        className="flex items-baseline justify-between border-b border-border px-3 py-2"
        title="Unconditional Magic Find from the Ether tree — also shown in the Stats tab"
      >
        <span className="text-[12px] font-medium text-text">Magic Find</span>
        <span className="font-mono text-[12px] text-accent-hot">
          {magicFind > 0 ? `+${magicFind}%` : '—'}
        </span>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-3 py-2">
        {groups.length === 0 ? (
          <p className="m-0 py-2 text-center text-[11px] italic text-muted">
            Allocate nodes to see totals.
          </p>
        ) : (
          groups.map((group, gi) => (
            <div key={group.region} className={gi === 0 ? '' : 'mt-2.5'}>
              <div className="mb-1.5 flex items-center gap-2 font-mono text-[9px] uppercase tracking-[0.18em] text-faint">
                <span
                  aria-hidden
                  className="inline-block h-1 w-1 shrink-0 rotate-45"
                  style={{ backgroundColor: group.color }}
                />
                <span style={{ color: group.color }}>{group.region}</span>
                <span aria-hidden className="h-px flex-1 bg-border" />
              </div>
              <ul className="m-0 flex list-none flex-col gap-2 p-0">
                {group.entries.map((e) => (
                  <li key={e.key} className="leading-snug">
                    <div className="flex items-baseline justify-between gap-2">
                      <span className="min-w-0 truncate text-[11.5px] text-text">
                        <span
                          className="font-mono text-[10px]"
                          style={{ color: group.color }}
                        >
                          {e.count}x
                        </span>{' '}
                        {e.label}
                      </span>
                      <span
                        className="shrink-0 font-mono text-[11px]"
                        style={{ color: group.color }}
                      >
                        {formatEtherTotal(e)}
                      </span>
                    </div>
                    <div className="text-[10px] leading-snug text-faint">
                      {e.desc}
                    </div>
                  </li>
                ))}
              </ul>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
