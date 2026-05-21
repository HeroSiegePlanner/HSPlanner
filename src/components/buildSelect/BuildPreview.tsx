import { getClass, getClassIcon, skills } from '../../data'
import { rangedMax, rangedMin } from '../../utils/stats'
import type { RangedValue } from '../../types'
import type { SavedBuild } from '../../utils/savedBuilds'
import type { BuildMeta } from './useBuildLibrary'
import { classColor, classInitial, stripHtml, tagTone } from './helpers'
import { usePreviewStats } from './usePreviewStats'

interface BuildPreviewProps {
  build: SavedBuild | null
  meta: BuildMeta | undefined
  onOpen: (id: string) => void
  onCopy: (id: string) => void
}

function compact(n: number): string {
  // Formats a large number compactly (2.4M, 7.8k, 920).
  const abs = Math.abs(n)
  const strip = (s: string) => s.replace(/\.0$/, '')
  if (abs >= 1e9) return `${strip((n / 1e9).toFixed(1))}B`
  if (abs >= 1e6) return `${strip((n / 1e6).toFixed(1))}M`
  if (abs >= 1e4) return `${strip((n / 1e3).toFixed(1))}k`
  return Math.round(n).toLocaleString()
}

function rangeText(v: RangedValue | undefined, fmt: (n: number) => string): string {
  if (v === undefined) return '—'
  const lo = rangedMin(v)
  const hi = rangedMax(v)
  return Math.abs(lo - hi) < 0.5 ? fmt(lo) : `${fmt(lo)}–${fmt(hi)}`
}

function StatCell({
  label,
  value,
  tone,
}: {
  label: string
  value: string
  tone?: string
}) {
  return (
    <div className="flex items-baseline justify-between gap-2 text-[11.5px]">
      <span className="text-muted">{label}</span>
      <span className={`font-mono ${tone ?? 'text-text'}`}>{value}</span>
    </div>
  )
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="border-t border-border/60 px-[18px] py-3">
      <h5 className="m-0 mb-2 font-mono text-[9.5px] uppercase tracking-[0.2em] text-faint">
        {title}
      </h5>
      {children}
    </div>
  )
}

export function BuildPreview({ build, meta, onOpen, onCopy }: BuildPreviewProps) {
  // Right pane: hero + tags + live-computed stats + main skill + notes + CTA.
  const preview = usePreviewStats(build)

  if (!build) {
    return (
      <aside
        className="flex min-h-0 flex-col border-l border-border"
        style={{ background: 'var(--color-panel-2)' }}
      >
        <PaneHeader />
        <div className="flex flex-1 flex-col items-center justify-center gap-2 px-8 text-center">
          <div className="font-mono text-[11px] uppercase tracking-[0.2em] text-faint">
            No build selected
          </div>
          <div className="text-[11px] text-muted">
            Pick a build from the list to preview it.
          </div>
        </div>
      </aside>
    )
  }

  const cls = build.classId ? getClass(build.classId) : undefined
  const icon = build.classId ? getClassIcon(build.classId) : undefined
  const color = classColor(build.classId)
  const className = meta?.className ?? cls?.name ?? 'Unknown'
  const level = preview.snapshot?.level ?? meta?.level ?? 1
  const nodes = preview.snapshot?.allocatedTreeNodes.size ?? meta?.nodes ?? 0

  const perf = preview.performance
  const stats = perf?.stats ?? {}
  const undecodable = meta ? !meta.decoded && preview.snapshot === null : false

  const mainSkillId = preview.snapshot?.mainSkillId ?? null
  const mainSkillName =
    perf?.activeSkillName ??
    (mainSkillId ? skills.find((s) => s.id === mainSkillId)?.name : undefined)

  const dps =
    perf?.combinedDpsMin !== undefined && perf?.combinedDpsMax !== undefined
      ? Math.abs(perf.combinedDpsMin - perf.combinedDpsMax) < 0.5
        ? compact(perf.combinedDpsMin)
        : `${compact(perf.combinedDpsMin)}–${compact(perf.combinedDpsMax)}`
      : '—'

  const resists = ['fire', 'cold', 'lightning', 'poison']
    .map((t) => {
      const v = stats[`${t}_resistance`]
      return v === undefined ? '0' : String(Math.round(rangedMax(v)))
    })
    .join('/')

  const notesText = stripHtml(build.notes)

  return (
    <aside
      className="flex min-h-0 flex-col border-l border-border"
      style={{ background: 'var(--color-panel-2)' }}
    >
      <PaneHeader />

      <div className="min-h-0 flex-1 overflow-y-auto">
        {/* Hero */}
        <div
          className="grid grid-cols-[54px_1fr] items-center gap-3.5 border-b border-border px-[18px] py-4"
          style={{
            background:
              'linear-gradient(180deg, rgba(201,165,90,0.06), transparent 80%)',
          }}
        >
          {icon ? (
            <img
              src={icon}
              alt=""
              aria-hidden
              className="h-[54px] w-[54px] object-contain"
              style={{ filter: `drop-shadow(0 0 10px ${color}aa)` }}
            />
          ) : (
            <span
              aria-hidden
              className="flex h-[54px] w-[54px] items-center justify-center rounded-[4px] border font-mono text-[24px] font-bold"
              style={{
                color,
                borderColor: `${color}55`,
                background: `linear-gradient(180deg, ${color}1a, ${color}05)`,
              }}
            >
              {classInitial(className)}
            </span>
          )}
          <div className="min-w-0">
            <div className="font-mono text-[14px] leading-tight break-words text-accent-hot">
              {build.name}
            </div>
            <div className="mt-1 font-mono text-[10px] uppercase tracking-[0.16em] text-faint">
              <span className="text-muted">{className}</span> · Lv{' '}
              <span className="text-muted">{level}</span> ·{' '}
              <span className="text-muted">{build.profiles.length}P</span>
            </div>
          </div>
        </div>

        {/* Tags */}
        {build.tags.length > 0 && (
          <div className="flex flex-wrap gap-1.5 px-[18px] pt-2.5">
            {build.tags.map((t) => (
              <span
                key={t}
                className={`rounded-[2px] border border-border bg-panel-2 px-1.5 py-0.5 text-[9.5px] uppercase tracking-[0.1em] ${tagTone(t)}`}
              >
                {t}
              </span>
            ))}
          </div>
        )}

        {/* Stats */}
        <div className="grid grid-cols-2 gap-x-3.5 gap-y-1.5 px-[18px] py-3">
          <StatCell label="DPS" value={dps} tone="text-accent-hot" />
          <StatCell
            label="Life"
            value={rangeText(stats.life, (n) => compact(n))}
            tone="text-stat-red"
          />
          <StatCell
            label="Mana"
            value={rangeText(stats.mana, (n) => compact(n))}
            tone="text-stat-purple"
          />
          <StatCell
            label="Crit"
            value={rangeText(stats.crit_chance, (n) => `${Math.round(n)}%`)}
          />
          <StatCell
            label="Crit Dmg"
            value={rangeText(stats.crit_damage, (n) => `+${Math.round(n)}%`)}
          />
          <StatCell label="Resists" value={resists} />
          <StatCell label="Nodes" value={String(nodes)} />
          <StatCell
            label="Skills"
            value={
              preview.snapshot
                ? String(Object.keys(preview.snapshot.skillRanks).length)
                : '—'
            }
          />
        </div>

        {preview.loading && (
          <div className="px-[18px] pb-1 font-mono text-[9.5px] uppercase tracking-[0.16em] text-faint">
            Computing…
          </div>
        )}
        {!preview.loading && !preview.available && !undecodable && (
          <div className="px-[18px] pb-1 font-mono text-[9.5px] uppercase tracking-[0.16em] text-faint">
            Stats unavailable — calc engine offline
          </div>
        )}
        {undecodable && (
          <div className="px-[18px] pb-1 font-mono text-[9.5px] uppercase tracking-[0.16em] text-stat-red">
            Build data could not be read
          </div>
        )}

        {/* Main skill */}
        {mainSkillName && (
          <Section title="Main Skill">
            <div className="flex items-center gap-2.5 rounded-[3px] border border-border bg-panel-3 px-2 py-1.5">
              <span
                className="flex h-6 w-6 items-center justify-center rounded-[2px] border border-accent-deep font-mono text-[12px] text-accent-hot"
                style={{ background: 'linear-gradient(135deg,#2a2418,#15110a)' }}
                aria-hidden
              >
                ◆
              </span>
              <span className="truncate text-[11.5px] text-text">
                {mainSkillName}
              </span>
            </div>
          </Section>
        )}

        {/* Notes */}
        {notesText && (
          <Section title="Notes">
            <p className="m-0 text-[11.5px] italic leading-relaxed text-muted">
              {notesText.slice(0, 400)}
              {notesText.length > 400 ? '…' : ''}
            </p>
          </Section>
        )}
      </div>

      {/* CTA */}
      <div
        className="flex shrink-0 gap-2 border-t border-border px-[18px] py-3"
        style={{
          background: 'linear-gradient(180deg, transparent, rgba(0,0,0,0.4))',
        }}
      >
        <button
          type="button"
          onClick={() => onCopy(build.id)}
          className="flex h-[34px] flex-1 items-center justify-center gap-1.5 rounded-[3px] border border-border-2 bg-panel-2 font-mono text-[11px] uppercase tracking-[0.06em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
        >
          Copy code
        </button>
        <button
          type="button"
          onClick={() => onOpen(build.id)}
          className="flex h-[34px] flex-1 items-center justify-center gap-1.5 rounded-[3px] border border-accent-deep font-mono text-[11px] uppercase tracking-[0.06em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]"
          style={{
            background: 'linear-gradient(180deg, #3a2f1a, #241e10)',
            boxShadow: '0 0 10px rgba(224,184,100,0.18)',
          }}
        >
          ▶ Open Build
        </button>
      </div>
    </aside>
  )
}

function PaneHeader() {
  return (
    <div
      className="flex h-[30px] shrink-0 items-center border-b border-border px-3"
      style={{
        background: 'linear-gradient(180deg, rgba(201,165,90,0.04), transparent)',
      }}
    >
      <span className="font-mono text-[10px] uppercase tracking-[0.2em] text-accent-deep">
        Preview
      </span>
    </div>
  )
}
