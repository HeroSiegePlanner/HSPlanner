import { activeSeasonId, getClass, getClassIcon, getMercClass, skills } from '@data'
import { getSeason } from '@data/seasons/registry'
import { rangedMax, rangedMin } from '../../utils/item/stats'
import { compact } from '../../utils/compactNumber'
import { useSettings } from '../../store/settings'
import type { RangedValue } from '../../types'
import type { SavedBuild } from '../../utils/build/savedBuilds'
import { heroLevelFor } from '../../utils/build/heroLevel'
import type { BuildMeta } from './useBuildLibrary'
import { motion } from 'motion/react'
import { T_FAST } from '../../utils/motion'
import { sanitizeHtml } from '../../utils/sanitizeHtml'
import { classColor, classInitial, stripHtml, tagTone } from './buildDisplay'
import { usePreviewStats } from './usePreviewStats'
import { CopyIcon, DeleteIcon, PlayIcon, PlusIcon, RenameIcon } from './buildSelectIcons'
import { IconAction } from '../ui/IconAction'
import { formatEhp, groupEhpRows } from '../../utils/build/ehp'

interface BuildPreviewProps {
  build: SavedBuild | null
  meta: BuildMeta | undefined
  onOpen: (id: string) => void
  onShare: (buildId: string) => void
  onSwitchProfile: (buildId: string, profileId: string) => void
  onAddProfile: (buildId: string) => void
  onRenameProfile: (buildId: string, profileId: string, current: string) => void
  onDuplicateProfile: (buildId: string, profileId: string) => void
  onRemoveProfile: (buildId: string, profileId: string, name: string) => void
}

function range(lo: number, hi: number, fmt: (n: number) => string): string {
  return Math.abs(lo - hi) < 0.5 ? fmt(lo) : `${fmt(lo)}–${fmt(hi)}`
}

function rangeText(v: RangedValue | undefined, fmt: (n: number) => string): string {
  if (v === undefined) return '—'
  return range(rangedMin(v), rangedMax(v), fmt)
}

function StatTile({
  label,
  value,
  tone,
}: {
  label: string
  value: string
  tone?: string
}) {
  return (
    <div className="flex min-w-0 flex-col gap-1 bg-panel px-3 py-2">
      <span className="truncate font-mono text-[9.5px] uppercase tracking-[0.1em] text-faint">
        {label}
      </span>
      <span
        className={`truncate font-mono text-[13px] font-medium tabular-nums ${tone ?? 'text-text'}`}
      >
        {value}
      </span>
    </div>
  )
}

function Section({
  title,
  count,
  children,
}: {
  title: string
  count?: number
  children: React.ReactNode
}) {
  return (
    <div className="border-b border-border py-3.5 last:border-b-0">
      <h5 className="m-0 mb-2 flex items-center justify-between font-mono text-[10px] font-medium uppercase tracking-[0.18em] text-faint">
        <span>{title}</span>
        {count !== undefined && (
          <span className="text-accent-deep">· {count}</span>
        )}
      </h5>
      {children}
    </div>
  )
}

export function BuildPreview({
  build,
  meta,
  onOpen,
  onShare,
  onSwitchProfile,
  onAddProfile,
  onRenameProfile,
  onDuplicateProfile,
  onRemoveProfile,
}: BuildPreviewProps) {
  const preview = usePreviewStats(build)
  const numberScale = useSettings((s) => s.numberScale)

  if (!build) {
    return (
      <aside
        className="flex min-h-0 flex-col border-l border-border"
        style={{ background: 'var(--color-panel)' }}
      >
        <PaneHeader />
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={T_FAST}
          className="flex flex-1 flex-col items-center justify-center gap-2 px-5 text-center"
        >
          <div className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
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
  const nodes = preview.snapshot
    ? heroLevelFor(preview.snapshot)
    : meta?.nodes ?? 0

  const perf = preview.performance
  const stats = perf?.stats ?? {}
  const undecodable = meta ? !meta.decoded && preview.snapshot === null : false

  const mainSkills = (preview.snapshot?.activeSkillIds ?? []).map((id) => ({
    id,
    name: skills.find((s) => s.id === id)?.name ?? id,
  }))

  const seasonLabel = build.season.toUpperCase()
  const seasonName = getSeason(build.season)?.name ?? build.season
  const isOtherSeason = build.season !== activeSeasonId
  const mercClassId = preview.snapshot?.mercClassId ?? null
  const mercName = mercClassId
    ? (getMercClass(mercClassId)?.name ?? mercClassId)
    : null

  const dps =
    perf?.combinedDpsMin !== undefined && perf?.combinedDpsMax !== undefined
      ? range(perf.combinedDpsMin, perf.combinedDpsMax, (n) =>
          compact(n, numberScale),
        )
      : '—'

  const resists = ['fire', 'cold', 'lightning', 'poison', 'arcane']
    .map((t) => {
      const v = stats[`${t}_resistance`]
      return v === undefined ? '0' : String(Math.round(rangedMax(v)))
    })
    .join('/')

  const skillCount = preview.snapshot
    ? String(Object.keys(preview.snapshot.skillRanks).length)
    : '—'
  const ehpTiles = perf ? groupEhpRows(perf.ehp) : []


  const notesHtml = sanitizeHtml(build.notes)
  const hasNotes = stripHtml(notesHtml).length > 0

  return (
    <aside
      className="flex min-h-0 flex-col border-l border-border"
      style={{ background: 'var(--color-panel)' }}
    >
      <PaneHeader />

      <motion.div
        key={build.id}
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={T_FAST}
        className="min-h-0 flex-1 overflow-y-auto px-4 pb-3.5"
      >
        <div className="flex items-center gap-3.5 border-b border-border py-2 pb-3.5">
          <span
            aria-hidden
            className="grid h-[54px] w-[54px] shrink-0 place-items-center rounded-[3px] border border-border-2"
            style={{
              background:
                'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
              boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.6)',
            }}
          >
            {icon ? (
              <img
                src={icon}
                alt=""
                aria-hidden
                className="h-[40px] w-[40px] object-contain"
                style={{
                  imageRendering: 'pixelated',
                  filter: `drop-shadow(0 0 6px ${color}aa)`,
                }}
              />
            ) : (
              <span
                className="font-mono text-[18px] font-bold"
                style={{ color }}
              >
                {classInitial(className)}
              </span>
            )}
          </span>
          <div className="min-w-0 flex-1">
            <div className="truncate font-mono text-[15px] tracking-[0.02em] text-text">
              {build.name}
            </div>
            <div className="mt-1 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
              <span style={{ color }}>{className}</span> · Lv {level} · Hero Lv{' '}
              {nodes} · {build.profiles.length}P ·{' '}
              <span
                title={seasonName}
                className={isOtherSeason ? 'text-accent-hot' : undefined}
              >
                {seasonLabel}
              </span>
            </div>
            {build.tags.length > 0 && (
              <div className="mt-3 flex flex-wrap gap-1.5">
                {build.tags.map((t) => (
                  <span
                    key={t}
                    className={`rounded-[2px] border border-border bg-panel-2 px-1.5 py-[1px] font-mono text-[9px] uppercase tracking-[0.12em] ${tagTone(t)}`}
                  >
                    {t}
                  </span>
                ))}
              </div>
            )}
          </div>
        </div>

        <div className="border-b border-border py-3.5">
          <div
            className="mb-3.5 rounded-[6px] border border-border px-4 py-3"
            style={{
              background:
                'linear-gradient(180deg, color-mix(in srgb, var(--color-accent-deep) 14%, var(--color-panel-2)), var(--color-panel))',
            }}
          >
            <span
              className="font-mono text-[10px] font-medium uppercase tracking-[0.18em]"
              style={{
                color:
                  'color-mix(in srgb, var(--color-accent-hot) 60%, transparent)',
              }}
            >
              Combined DPS
            </span>
            <div
              className="mt-1 font-mono text-[23px] font-semibold leading-tight tracking-[-0.01em] text-accent-hot tabular-nums"
              style={{ textShadow: '0 0 12px rgba(224,184,100,0.35)' }}
            >
              {dps}
            </div>
          </div>
          <div className="grid grid-cols-2 gap-px overflow-hidden rounded-[6px] border border-border bg-border">
            <StatTile
              label="Life"
              value={rangeText(stats.life, (n) => compact(n, numberScale))}
              tone="text-stat-red"
            />
            <StatTile
              label="Mana"
              value={rangeText(stats.mana, (n) => compact(n, numberScale))}
              tone="text-stat-blue"
            />
            <StatTile
              label="Crit"
              value={rangeText(stats.crit_chance, (n) => `${Math.round(n)}%`)}
            />
            <StatTile
              label="Crit Dmg"
              value={rangeText(stats.crit_damage, (n) => `+${Math.round(n)}%`)}
            />
            <StatTile label="Resists" value={resists} />
            <StatTile label="Nodes · Skills" value={`${nodes} · ${skillCount}`} />
            <StatTile
              label="Ether"
              value={
                preview.snapshot
                  ? String(preview.snapshot.allocatedEtherNodes.size)
                  : '—'
              }
            />
            <StatTile label="Merc" value={mercName ?? '—'} />
            {ehpTiles.map((r) => (
              <StatTile
                key={r.key}
                label={r.label}
                value={formatEhp(r.ehp)}
                tone="text-accent-hot"
              />
            ))}
            {ehpTiles.length % 2 === 1 && (
              <div aria-hidden className="bg-panel" />
            )}
          </div>
        </div>

        {preview.loading && (
          <div className="pt-2 font-mono text-[9.5px] uppercase tracking-[0.16em] text-faint">
            Computing…
          </div>
        )}
        {!preview.loading && !preview.available && !undecodable && (
          <div className="pt-2 font-mono text-[9.5px] uppercase tracking-[0.16em] text-faint">
            Stats unavailable — calc engine offline
          </div>
        )}
        {undecodable && (
          <div className="pt-2 font-mono text-[9.5px] uppercase tracking-[0.16em] text-stat-red">
            Build data could not be read
          </div>
        )}

        <Section title="Profiles" count={build.profiles.length}>
          <div className="flex flex-col gap-[5px]">
            {build.profiles.map((p) => {
              const active = p.id === build.activeProfileId
              return (
                <div
                  key={p.id}
                  className={`flex cursor-pointer items-center gap-2 rounded-[3px] border px-2.5 py-[7px] text-[12px] transition-colors ${
                    active
                      ? 'border-accent-deep bg-accent-hot/5 text-accent-hot'
                      : 'border-border bg-panel-2 text-muted hover:border-border-2 hover:text-text'
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
                    <IconAction
                      label="Rename profile"
                      onClick={() => onRenameProfile(build.id, p.id, p.name)}
                    >
                      <RenameIcon className="h-3 w-3" />
                    </IconAction>
                    <IconAction
                      label="Duplicate profile"
                      onClick={() => onDuplicateProfile(build.id, p.id)}
                    >
                      <CopyIcon className="h-3 w-3" />
                    </IconAction>
                    <IconAction
                      label="Remove profile"
                      danger
                      disabled={build.profiles.length <= 1}
                      onClick={() => onRemoveProfile(build.id, p.id, p.name)}
                    >
                      <DeleteIcon className="h-3 w-3" />
                    </IconAction>
                  </div>
                </div>
              )
            })}
            <button
              type="button"
              onClick={() => onAddProfile(build.id)}
              className="mt-[3px] flex items-center justify-center gap-1.5 rounded-[3px] border border-dashed border-border-2 px-2.5 py-[7px] text-[11px] tracking-[0.04em] text-faint transition-colors hover:border-accent-deep hover:text-accent-hot"
            >
              <PlusIcon className="h-3 w-3" />
              Add profile
            </button>
          </div>
        </Section>

        {mainSkills.length > 0 && (
          <Section
            title={mainSkills.length > 1 ? 'Main Skills' : 'Main Skill'}
            count={mainSkills.length > 1 ? mainSkills.length : undefined}
          >
            <div className="flex flex-col gap-[5px]">
              {mainSkills.map(({ id, name }) => (
                <div
                  key={id}
                  className="flex items-center gap-2 rounded-[3px] border border-border bg-panel-2 px-2.5 py-[7px] font-mono text-[12px] text-text"
                >
                  <span aria-hidden className="text-[10px] text-accent">
                    ◆
                  </span>
                  <span className="truncate">{name}</span>
                </div>
              ))}
            </div>
          </Section>
        )}

        {hasNotes && (
          <Section title="Notes">
            <div
              className="notes-editor rounded-[3px] border border-border bg-panel-2 px-2.5 py-2 font-mono text-[12px] leading-relaxed text-muted"
              dangerouslySetInnerHTML={{ __html: notesHtml }}
            />
          </Section>
        )}
      </motion.div>

      <div className="grid shrink-0 grid-cols-2 gap-2 border-t border-border px-3.5 py-2.5">
        <button
          type="button"
          onClick={() => onShare(build.id)}
          title="Share this build as a code, Gist link or hsplanner.app link"
          className="flex h-[36px] items-center justify-center gap-1.5 rounded-[3px] border border-border bg-panel-2 font-mono text-[11px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
        >
          Share
        </button>
        <button
          type="button"
          onClick={() => onOpen(build.id)}
          className="flex h-[36px] items-center justify-center gap-1.5 rounded-[3px] border border-accent-deep font-mono text-[11px] uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot"
          style={{
            background: 'linear-gradient(180deg, #3a2f1a, #2a2418)',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.boxShadow =
              '0 0 10px rgba(232,217,107,0.18)'
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.boxShadow = ''
          }}
        >
          <PlayIcon className="h-3 w-3" />
          Open Build
        </button>
      </div>
    </aside>
  )
}

function PaneHeader() {
  return (
    <div className="flex shrink-0 items-center px-4 pb-2 pt-3">
      <span className="flex items-center gap-2 font-mono text-[11px] uppercase tracking-[0.16em] text-accent-hot">
        <span className="text-[10px] text-accent">◆</span>
        Preview
      </span>
    </div>
  )
}
