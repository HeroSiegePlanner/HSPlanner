import { getClassIcon } from '../../data'
import type { SavedBuild } from '../../utils/savedBuilds'
import type { BuildMeta } from './useBuildLibrary'
import { classColor, classInitial, formatTimestamp, tagTone } from './helpers'

export type SortCol = 'favorite' | 'name' | 'class' | 'level' | 'date'
export type SortDir = 'asc' | 'desc'

const GRID = 'grid-cols-[30px_36px_minmax(200px,1fr)_120px_70px_110px]'

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
      className={`inline-flex items-center gap-1.5 rounded-full border px-2.5 py-[3px] text-[11px] transition-colors ${
        on
          ? 'border-accent-deep bg-accent-hot/10 text-accent-hot'
          : 'border-border bg-panel-3 text-muted hover:border-accent-deep hover:text-text'
      }`}
    >
      {label}
      {on && <span className="text-accent-deep">×</span>}
    </button>
  )
}

function HeaderCell({
  label,
  col,
  sortCol,
  sortDir,
  onSort,
  align = 'left',
}: {
  label: string
  col: SortCol
  sortCol: SortCol
  sortDir: SortDir
  onSort: (col: SortCol) => void
  align?: 'left' | 'right'
}) {
  const sorted = sortCol === col
  return (
    <button
      type="button"
      onClick={() => onSort(col)}
      className={`flex h-full items-center gap-1 px-1.5 transition-colors hover:text-text ${
        sorted ? 'text-accent-hot' : 'text-faint'
      } ${align === 'right' ? 'justify-end' : ''}`}
    >
      {label}
      <span className={`text-[8px] ${sorted ? 'opacity-100' : 'opacity-0'}`}>
        {sortDir === 'asc' ? '▲' : '▼'}
      </span>
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
      <img
        src={icon}
        alt=""
        aria-hidden
        className="h-6 w-6 object-contain"
        style={{
          filter: selected ? `drop-shadow(0 0 8px ${color}aa)` : undefined,
        }}
      />
    )
  }
  return (
    <span
      aria-hidden
      className="flex h-6 w-6 items-center justify-center rounded-[3px] border font-mono text-[12px] font-bold"
      style={{
        color,
        borderColor: `${color}55`,
        background: `linear-gradient(180deg, ${color}1a, ${color}05)`,
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
}: BuildTableProps) {
  // Centre column: the filter-chip row, the sortable table header, and the
  // scrolling list of build rows.
  const filtersActive = activeTags.length > 0 || levelFilter

  return (
    <section className="flex min-w-0 flex-col" style={{ background: 'var(--color-bg)' }}>
      {/* Filter chips */}
      <div className="flex h-[30px] shrink-0 items-center gap-2 overflow-x-auto border-b border-border bg-panel-2 px-3.5">
        <span className="shrink-0 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
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
        <span className="shrink-0 font-mono text-[10px] uppercase tracking-[0.12em] text-faint">
          {builds.length} of {totalCount}
        </span>
      </div>

      {/* Header */}
      <div
        className={`grid ${GRID} h-[30px] shrink-0 items-center border-b border-border bg-panel-2 pl-3 pr-2 font-mono text-[10px] uppercase tracking-[0.14em]`}
      >
        <HeaderCell label="★" col="favorite" sortCol={sortCol} sortDir={sortDir} onSort={onSort} />
        <span />
        <HeaderCell label="Name" col="name" sortCol={sortCol} sortDir={sortDir} onSort={onSort} />
        <HeaderCell label="Class" col="class" sortCol={sortCol} sortDir={sortDir} onSort={onSort} />
        <HeaderCell label="Lv" col="level" sortCol={sortCol} sortDir={sortDir} onSort={onSort} align="right" />
        <HeaderCell label="Modified" col="date" sortCol={sortCol} sortDir={sortDir} onSort={onSort} align="right" />
      </div>

      {/* Body */}
      <div className="min-h-0 flex-1 overflow-y-auto">
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
            return (
              <div
                key={b.id}
                onClick={() => onSelect(b.id)}
                onDoubleClick={() => onOpen(b.id)}
                onContextMenu={(e) => onContextMenu(e, b.id)}
                className={`grid ${GRID} relative h-[42px] cursor-pointer items-center border-b border-border/50 pl-3 pr-2 transition-colors ${
                  selected ? '' : 'hover:bg-panel-2/60'
                }`}
                style={
                  selected ? { background: 'var(--color-panel-3)' } : undefined
                }
              >
                {selected && (
                  <span
                    aria-hidden
                    className="absolute left-0 top-0 bottom-0 w-0.5"
                    style={{
                      background:
                        'linear-gradient(180deg, var(--color-accent-hot), var(--color-accent-deep))',
                    }}
                  />
                )}
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation()
                    onToggleFavorite(b.id)
                  }}
                  className={`flex h-[18px] w-[18px] items-center justify-center rounded-[2px] text-[13px] transition-colors ${
                    b.favorite
                      ? 'text-accent-hot'
                      : 'text-faint hover:text-accent-hot'
                  }`}
                  title={b.favorite ? 'Unfavorite' : 'Favorite'}
                >
                  ★
                </button>
                <div className="flex items-center justify-center">
                  <ClassGlyph
                    classId={b.classId}
                    className={m?.className ?? 'Unknown'}
                    selected={selected}
                  />
                </div>
                <div className="flex min-w-0 items-center gap-2 pr-2">
                  <span
                    className={`truncate font-mono text-[12.5px] ${
                      selected ? 'text-accent-hot' : 'text-text'
                    }`}
                  >
                    {b.name}
                  </span>
                  {isActive && (
                    <span className="shrink-0 font-mono text-[9px] uppercase tracking-[0.14em] text-accent-deep">
                      Active
                    </span>
                  )}
                  {b.profiles.length > 1 && (
                    <span
                      className="shrink-0 rounded-[2px] border border-border px-1 font-mono text-[9px] text-muted"
                      title={`${b.profiles.length} profiles`}
                    >
                      {b.profiles.length}P
                    </span>
                  )}
                  {b.tags.slice(0, 3).map((t) => (
                    <span
                      key={t}
                      className={`shrink-0 rounded-[2px] border border-border bg-panel-2 px-1 text-[9px] uppercase tracking-[0.08em] ${tagTone(t)}`}
                    >
                      {t}
                    </span>
                  ))}
                  {!m?.decoded && (
                    <span className="shrink-0 text-[10px] text-stat-red/80">
                      unreadable
                    </span>
                  )}
                </div>
                <div className="truncate px-1.5 text-[10.5px] uppercase tracking-[0.08em] text-muted">
                  {m?.className ?? '—'}
                </div>
                <div className="px-1.5 text-right font-mono text-[12px] text-text">
                  {m?.level ?? '—'}
                </div>
                <div className="px-1.5 text-right font-mono text-[10.5px] tracking-[0.04em] text-faint">
                  {formatTimestamp(b.updatedAt)}
                </div>
              </div>
            )
          })
        )}
      </div>
    </section>
  )
}
