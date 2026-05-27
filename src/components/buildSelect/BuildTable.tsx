import { motion } from 'motion/react'
import { getClassIcon } from '../../data'
import { listContainerVariants, listItemVariants } from '../../lib/motion'
import type { SavedBuild } from '../../utils/build/savedBuilds'
import type { BuildMeta } from './useBuildLibrary'
import { classColor, classInitial, formatTimestamp, tagTone } from './helpers'

export type SortCol = 'favorite' | 'name' | 'class' | 'level' | 'date'
export type SortDir = 'asc' | 'desc'

const GRID = 'grid-cols-[32px_minmax(0,1fr)_160px_80px_130px]'

interface BuildTableProps {
  /** Builds after filtering + sorting. */
  builds: SavedBuild[]
  meta: Record<string, BuildMeta>
  selectedId: string | null
  activeBuildId: string | null
  sortCol: SortCol
  sortDir: SortDir
  onSort: (col: SortCol) => void
  onSelect: (id: string) => void
  onOpen: (id: string) => void
  onContextMenu: (e: React.MouseEvent, buildId: string) => void
  onToggleFavorite: (id: string) => void
  allTags: string[]
  activeTags: string[]
  onToggleTag: (tag: string) => void
  levelFilter: boolean
  onToggleLevelFilter: () => void
  onClearFilters: () => void
  totalCount: number
  /** Animation key — re-staggers the row list when the scope/folder changes. */
  listKey: string
}

function Chip({
  label,
  on,
  onClick,
}: {
  label: string
  on: boolean
  onClick: () => void
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`inline-flex h-[22px] shrink-0 items-center gap-1.5 rounded-[11px] border px-2.5 text-[11px] tracking-[0.04em] transition-colors ${
        on
          ? 'border-accent-deep bg-accent-hot/10 text-accent-hot'
          : 'border-border bg-panel-2 text-muted hover:border-border-2 hover:text-text'
      }`}
    >
      {label}
      {on && <span className="leading-none text-accent-hot">✕</span>}
    </button>
  )
}

function HeaderCell({
  label,
  col,
  sortCol,
  sortDir,
  onSort,
}: {
  label: string
  col: SortCol
  sortCol: SortCol
  sortDir: SortDir
  onSort: (col: SortCol) => void
}) {
  const sorted = sortCol === col
  return (
    <button
      type="button"
      onClick={() => onSort(col)}
      className={`flex items-center gap-1 text-left transition-colors hover:text-muted ${
        sorted ? 'text-accent-hot' : 'text-faint'
      }`}
    >
      {label}
      {sorted && (
        <span className="text-[9px]">{sortDir === 'asc' ? '▴' : '▾'}</span>
      )}
    </button>
  )
}

function ClassGlyph({
  classId,
  className,
  selected,
}: {
  classId: string | null
  className: string
  selected: boolean
}) {
  const icon = classId ? getClassIcon(classId) : undefined
  const color = classColor(classId)
  if (icon) {
    return (
      <span
        aria-hidden
        className="flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded-[3px] border"
        style={{
          borderColor: `${color}55`,
          background: `linear-gradient(135deg, ${color}1f, ${color}08)`,
          boxShadow: selected ? `0 0 8px ${color}55` : undefined,
        }}
      >
        <img
          src={icon}
          alt=""
          aria-hidden
          className="h-[18px] w-[18px] object-contain"
          style={{
            filter: selected ? `drop-shadow(0 0 6px ${color}aa)` : undefined,
          }}
        />
      </span>
    )
  }
  return (
    <span
      aria-hidden
      className="flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded-[3px] border font-mono text-[11px] font-bold"
      style={{
        color,
        borderColor: `${color}55`,
        background: `linear-gradient(135deg, ${color}1f, ${color}08)`,
      }}
    >
      {classInitial(className)}
    </span>
  )
}

export function BuildTable({
  builds,
  meta,
  selectedId,
  activeBuildId,
  sortCol,
  sortDir,
  onSort,
  onSelect,
  onOpen,
  onContextMenu,
  onToggleFavorite,
  allTags,
  activeTags,
  onToggleTag,
  levelFilter,
  onToggleLevelFilter,
  onClearFilters,
  totalCount,
  listKey,
}: BuildTableProps) {
  // Centre column: the filter-chip row, the sortable table header, and the
  // scrolling list of build rows.
  const filtersActive = activeTags.length > 0 || levelFilter

  return (
    <section className="flex min-w-0 flex-col" style={{ background: 'var(--color-bg)' }}>
      {/* Filter chips */}
      <div
        className="flex shrink-0 items-center gap-2 overflow-x-auto border-b border-border px-4 py-2.5"
        style={{ background: 'rgba(255,255,255,0.005)' }}
      >
        <span className="mr-0.5 shrink-0 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
          Filter
        </span>
        <Chip label="All" on={!filtersActive} onClick={onClearFilters} />
        {allTags.map((tag) => (
          <Chip
            key={tag}
            label={tag}
            on={activeTags.includes(tag)}
            onClick={() => onToggleTag(tag)}
          />
        ))}
        <Chip label="Lv 90+" on={levelFilter} onClick={onToggleLevelFilter} />
        <div className="flex-1" />
        <span className="shrink-0 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
          <b className="font-medium text-accent-hot">{builds.length}</b> of{' '}
          <b className="font-medium text-accent-hot">{totalCount}</b>
        </span>
      </div>

      {/* Header */}
      <div
        className={`grid ${GRID} shrink-0 items-center border-b border-border px-4 py-2 font-mono text-[10px] uppercase tracking-[0.14em]`}
        style={{ background: 'rgba(255,255,255,0.008)' }}
      >
        <HeaderCell label="★" col="favorite" sortCol={sortCol} sortDir={sortDir} onSort={onSort} />
        <HeaderCell label="Name" col="name" sortCol={sortCol} sortDir={sortDir} onSort={onSort} />
        <HeaderCell label="Class" col="class" sortCol={sortCol} sortDir={sortDir} onSort={onSort} />
        <HeaderCell label="Lv" col="level" sortCol={sortCol} sortDir={sortDir} onSort={onSort} />
        <HeaderCell label="Modified" col="date" sortCol={sortCol} sortDir={sortDir} onSort={onSort} />
      </div>

      {/* Body */}
      <motion.div
        key={listKey}
        variants={listContainerVariants}
        initial="initial"
        animate="animate"
        className="min-h-0 flex-1 overflow-y-auto"
      >
        {builds.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center gap-1.5 px-6 text-center">
            <div className="font-mono text-[11px] uppercase tracking-[0.2em] text-faint">
              No builds here
            </div>
            <div className="text-[11px] text-muted">
              Create a new build, import one, or pick another folder.
            </div>
          </div>
        ) : (
          builds.map((b) => {
            const m = meta[b.id]
            const selected = b.id === selectedId
            const isActive = b.id === activeBuildId
            const modified = formatTimestamp(b.updatedAt)
            const isToday = /^Today/i.test(modified)
            return (
              <motion.div
                key={b.id}
                variants={listItemVariants}
                onClick={() => onSelect(b.id)}
                onDoubleClick={() => onOpen(b.id)}
                onContextMenu={(e) => onContextMenu(e, b.id)}
                className={`grid ${GRID} relative cursor-pointer items-center border-b border-border px-4 py-2.5 transition-colors ${
                  selected ? '' : 'hover:bg-white/[0.022]'
                }`}
                style={
                  selected
                    ? {
                        background:
                          'linear-gradient(90deg, rgba(201,165,90,0.08) 0%, rgba(201,165,90,0.02) 60%, transparent 100%)',
                      }
                    : undefined
                }
              >
                {selected && (
                  <span
                    aria-hidden
                    className="absolute left-0 top-0 bottom-0 w-0.5 bg-accent-hot"
                  />
                )}
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation()
                    onToggleFavorite(b.id)
                  }}
                  className={`flex h-[18px] w-[18px] items-center justify-center text-[14px] leading-none transition-colors ${
                    b.favorite
                      ? 'text-accent-hot'
                      : 'text-faint hover:text-muted'
                  }`}
                  title={b.favorite ? 'Unfavorite' : 'Favorite'}
                >
                  ★
                </button>
                <div className="flex min-w-0 items-center gap-2.5 overflow-hidden pr-2">
                  <ClassGlyph
                    classId={b.classId}
                    className={m?.className ?? 'Unknown'}
                    selected={selected}
                  />
                  <span
                    className={`truncate font-mono text-[12.5px] ${
                      selected ? 'text-accent-hot' : 'text-text'
                    }`}
                  >
                    {b.name}
                  </span>
                  <div className="ml-1 flex shrink-0 items-center gap-1 overflow-hidden">
                    {isActive && (
                      <span className="rounded-[2px] border border-accent-deep bg-accent-hot/10 px-1.5 py-[1px] font-mono text-[9px] uppercase tracking-[0.12em] text-accent-hot">
                        Active
                      </span>
                    )}
                    {b.profiles.length > 1 && (
                      <span
                        className="rounded-[2px] border border-accent-deep bg-accent-hot/10 px-1.5 py-[1px] font-mono text-[9px] uppercase tracking-[0.12em] text-accent-hot"
                        title={`${b.profiles.length} profiles`}
                      >
                        {b.profiles.length}P
                      </span>
                    )}
                    {b.tags.slice(0, 3).map((t) => (
                      <span
                        key={t}
                        className={`rounded-[2px] border border-border bg-panel-2 px-1.5 py-[1px] font-mono text-[9px] uppercase tracking-[0.12em] ${tagTone(t)}`}
                      >
                        {t}
                      </span>
                    ))}
                    {!m?.decoded && (
                      <span className="text-[10px] text-stat-red/80">
                        unreadable
                      </span>
                    )}
                  </div>
                </div>
                <div className="truncate font-mono text-[10.5px] uppercase tracking-[0.14em] text-muted">
                  {m?.className ?? '—'}
                </div>
                <div className="font-mono text-[12px] text-text">
                  {m?.level ?? '—'}
                </div>
                <div
                  className={`font-mono text-[11px] tracking-[0.02em] ${
                    isToday ? 'text-muted' : 'text-faint'
                  }`}
                >
                  {modified}
                </div>
              </motion.div>
            )
          })
        )}
      </motion.div>
    </section>
  )
}
