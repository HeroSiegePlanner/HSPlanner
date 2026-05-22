import { getClass, getClassIcon, skills } from '../../data'
import { rangedMax, rangedMin } from '../../utils/stats'
import type { RangedValue } from '../../types'
import type { SavedBuild } from '../../utils/savedBuilds'
import type { BuildMeta } from './useBuildLibrary'
import { motion } from 'motion/react'
import { T_FAST } from '../../lib/motion'
import { sanitizeHtml } from '../../utils/sanitizeHtml'
import { classColor, classInitial, stripHtml, tagTone } from './helpers'
import { usePreviewStats } from './usePreviewStats'
import { CopyIcon, DeleteIcon, PlayIcon, PlusIcon, RenameIcon } from './icons'

interface BuildPreviewProps {
  build: SavedBuild | null
  meta: BuildMeta | undefined
  onOpen: (id: string) => void
  onCopy: (id: string) => void
  /** Make `profileId` the active profile of `buildId`. */
  onSwitchProfile: (buildId: string, profileId: string) => void
  /** Open the "new profile" dialog for `buildId`. */
  onAddProfile: (buildId: string) => void
  /** Open the rename dialog for a profile, pre-filled with `current`. */
  onRenameProfile: (buildId: string, profileId: string, current: string) => void
  /** Duplicate a profile within `buildId`. */
  onDuplicateProfile: (buildId: string, profileId: string) => void
  /** Open the delete-confirmation dialog for a profile. */
  onRemoveProfile: (buildId: string, profileId: string, name: string) => void
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

export function BuildPreview({
  build,
  meta,
  onOpen,
  onCopy,
  onSwitchProfile,
  onAddProfile,
  onRenameProfile,
  onDuplicateProfile,
  onRemoveProfile,
}: BuildPreviewProps) {
  // Right pane: hero + tags + live-computed stats + main skill + notes + CTA.
  const preview = usePreviewStats(build)

  if (!build) {
    return (
      <aside
        className="flex min-h-0 flex-col border-l border-border"
        style={{ background: 'var(--color-panel-2)' }}
      >
        <PaneHeader />
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={T_FAST}
          className="flex flex-1 flex-col items-center justify-center gap-2 px-8 text-center"
        >
          <div className="font-mono text-[11px] uppercase tracking-[0.2em] text-faint">
            No build selected
          </div>
          <div className="text-[11px] text-muted">
            Pick a build from the list to preview it.
          </div>
        </motion.div>
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

  // Notes are HTML authored in the Notes tab. They are sanitised on write, but
  // the value here is read straight from storage (external data) so it is
  // re-sanitised before rendering. `hasNotes` ignores empty markup like <p></p>.
  const notesHtml = sanitizeHtml(build.notes)
  const hasNotes = stripHtml(notesHtml).length > 0

  return (
    <aside
      className="flex min-h-0 flex-col border-l border-border"
      style={{ background: 'var(--color-panel-2)' }}
    >
      <PaneHeader />

      <motion.div
        key={build.id}
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={T_FAST}
        className="min-h-0 flex-1 overflow-y-auto"
      >
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

        {/* Profiles */}
        <Section title={`Profiles · ${build.profiles.length}`}>
          <div className="flex flex-col gap-1">
            {build.profiles.map((p) => {
              const active = p.id === build.activeProfileId
              return (
                <div
                  key={p.id}
                  className={`flex items-center gap-2 rounded-[3px] border px-2 py-1.5 transition-colors ${
                    active
                      ? 'border-accent-deep bg-accent-hot/5'
                      : 'border-border bg-panel-3 hover:border-accent-deep'
                  }`}
                >
                  <button
                    type="button"
                    onClick={() => {
                      if (!active) onSwitchProfile(build.id, p.id)
                    }}
                    title={
                      active ? 'Active profile' : 'Switch to this profile'
                    }
                    className="flex min-w-0 flex-1 items-center gap-2 text-left"
                  >
                    <span
                      aria-hidden
                      className={`h-1.5 w-1.5 shrink-0 rotate-45 ${
                        active ? 'bg-accent-hot' : 'bg-faint'
                      }`}
                      style={
                        active
                          ? { boxShadow: '0 0 6px rgba(224,184,100,0.6)' }
                          : undefined
                      }
                    />
                    <span
                      className={`truncate text-[11.5px] ${
                        active ? 'text-accent-hot' : 'text-text'
                      }`}
                    >
                      {p.name}
                    </span>
                    {active && (
                      <span className="shrink-0 font-mono text-[9px] uppercase tracking-[0.14em] text-accent-deep">
                        Active
                      </span>
                    )}
                  </button>
                  <div className="flex shrink-0 items-center gap-0.5">
                    <ProfileAction
                      label="Rename profile"
                      onClick={() => onRenameProfile(build.id, p.id, p.name)}
                    >
                      <RenameIcon className="h-3 w-3" />
                    </ProfileAction>
                    <ProfileAction
                      label="Duplicate profile"
                      onClick={() => onDuplicateProfile(build.id, p.id)}
                    >
                      <CopyIcon className="h-3 w-3" />
                    </ProfileAction>
                    <ProfileAction
                      label="Remove profile"
                      danger
                      disabled={build.profiles.length <= 1}
                      onClick={() => onRemoveProfile(build.id, p.id, p.name)}
                    >
                      <DeleteIcon className="h-3 w-3" />
                    </ProfileAction>
                  </div>
                </div>
              )
            })}
            <button
              type="button"
              onClick={() => onAddProfile(build.id)}
              className="flex items-center justify-center gap-1.5 rounded-[3px] border border-dashed border-border-2 px-2 py-1.5 font-mono text-[10px] uppercase tracking-[0.12em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
            >
              <PlusIcon className="h-3 w-3" />
              Add profile
            </button>
          </div>
        </Section>

        {/* Main skill */}
        {mainSkillName && (
          <Section title="Main Skill">
            <div className="flex items-center gap-2.5 rounded-[3px] border border-border bg-panel-3 px-2 py-1.5">
              <span
                className="flex h-6 w-6 items-center justify-center rounded-[2px] border border-accent-deep"
                style={{ background: 'linear-gradient(135deg,#2a2418,#15110a)' }}
                aria-hidden
              >
                <span
                  className="h-2 w-2 rotate-45 bg-accent-hot"
                  style={{ boxShadow: '0 0 6px rgba(224,184,100,0.5)' }}
                />
              </span>
              <span className="truncate text-[11.5px] text-text">
                {mainSkillName}
              </span>
            </div>
          </Section>
        )}

        {/* Notes */}
        <Section title="Notes">
          {hasNotes ? (
            <div
              className="notes-editor text-[12px] leading-relaxed text-text"
              dangerouslySetInnerHTML={{ __html: notesHtml }}
            />
          ) : (
            <p className="m-0 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
              No notes
            </p>
          )}
        </Section>
      </motion.div>

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
            background: 'linear-gradient(180deg, #3a2f1a, #2a2418)',
            boxShadow: '0 0 10px rgba(224,184,100,0.18)',
          }}
        >
          <PlayIcon className="h-3 w-3" />
          Open Build
        </button>
      </div>
    </aside>
  )
}

function ProfileAction({
  label,
  danger,
  disabled,
  onClick,
  children,
}: {
  label: string
  danger?: boolean
  disabled?: boolean
  onClick: () => void
  children: React.ReactNode
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      title={label}
      aria-label={label}
      onClick={onClick}
      className={`flex h-6 w-6 items-center justify-center rounded-[2px] text-faint transition-colors disabled:cursor-not-allowed disabled:opacity-25 ${
        danger
          ? 'hover:bg-stat-red/10 hover:text-stat-red'
          : 'hover:bg-accent-hot/10 hover:text-accent-hot'
      }`}
    >
      {children}
    </button>
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
      <span className="flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.2em] text-accent-deep">
        <span
          aria-hidden
          className="inline-block h-1 w-1 rotate-45 bg-accent-deep"
        />
        Preview
      </span>
    </div>
  )
}
