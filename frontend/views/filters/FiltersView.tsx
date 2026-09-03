import { useEffect, useMemo, useState } from 'react'
import type { SavedLootFilter } from '../../types'
import { CornerMarks } from '../../components/ui/CornerMarks'
import { IconAction } from '../../components/ui/IconAction'
import { Modal, MODAL_BTN_PRIMARY_CLASS } from '../../components/ui/Modal'
import {
  CopyIcon,
  DeleteIcon,
  RenameIcon,
  StarIcon,
} from '../../components/buildSelect/buildSelectIcons'
import { ConfirmOverlay } from '../../components/buildSelect/overlays'
import { useCopyFeedback } from '../../hooks/useCopyFeedback'
import { useBuild } from '../../store/build'
import { getSavedBuild } from '../../utils/build/savedBuilds'
import { formatTimestamp } from '../../utils/formatTimestamp'
import { decodeLootFilter } from '../../utils/lootfilter/codec'
import {
  createFilter,
  deleteFilter,
  duplicateFilter,
  getSavedFilter,
  importFilter,
  listSavedFilters,
  renameFilter,
  setFilterFavorite,
} from '../../utils/lootfilter/savedFilters'
import { FilterEditor } from './FilterEditor'
import { FILTER_BTN_CLASS, FILTER_BTN_PRIMARY_CLASS } from './FilterCells'
import { filterSummary, type FilterSummary } from './filterModel'
import { GenerateFromBuildModal } from './GenerateFromBuildModal'

const GRID = 'grid-cols-[28px_minmax(0,1fr)_150px_150px]'

export default function FiltersView() {
  const activeBuildId = useBuild((s) => s.activeBuildId)
  return <BuildFilters key={activeBuildId ?? 'unsaved'} activeBuildId={activeBuildId} />
}

function BuildFilters({ activeBuildId }: { activeBuildId: string | null }) {
  const [filters, setFilters] = useState<SavedLootFilter[]>(() =>
    activeBuildId ? listSavedFilters(activeBuildId) : [],
  )
  const [editingId, setEditingId] = useState<string | null>(null)
  const [newFilterCode, setNewFilterCode] = useState<string | null>(null)
  const [pendingDelete, setPendingDelete] = useState<SavedLootFilter | null>(null)
  const [generateOpen, setGenerateOpen] = useState(false)
  const inventory = useBuild((s) => s.inventory)

  const refresh = () => {
    setFilters(activeBuildId ? listSavedFilters(activeBuildId) : [])
  }

  const buildName = useMemo(
    () => (activeBuildId ? getSavedBuild(activeBuildId)?.name ?? null : null),
    [activeBuildId],
  )

  const editing = useMemo(
    () => (editingId ? getSavedFilter(editingId) : null),
    [editingId],
  )
  if (editingId && editing) {
    return (
      <FilterEditor
        saved={editing}
        onBack={() => {
          setEditingId(null)
          refresh()
        }}
      />
    )
  }

  const header = (
    <header className="flex flex-wrap items-end justify-between gap-3">
      <div>
        <div className="mb-1 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
          <span
            aria-hidden
            className="inline-block h-[6px] w-[6px] rotate-45 bg-accent-hot"
            style={{ boxShadow: '0 0 8px rgba(224,184,100,0.6)' }}
          />
          Build · {buildName ?? 'unsaved build'}
        </div>
        <h2
          className="m-0 text-[22px] font-semibold tracking-[0.02em] text-accent-hot"
          style={{ textShadow: '0 0 16px rgba(224,184,100,0.18)' }}
        >
          Loot Filters
        </h2>
      </div>
      {activeBuildId && (
        <div className="flex items-center gap-2">
          <button
            type="button"
            title="Create a filter that highlights the affixes this build already wears"
            onClick={() => setGenerateOpen(true)}
            className={FILTER_BTN_CLASS}
          >
            From build
          </button>
          <button
            type="button"
            onClick={() => setNewFilterCode('')}
            className={FILTER_BTN_PRIMARY_CLASS}
          >
            + New filter
          </button>
        </div>
      )}
    </header>
  )

  if (!activeBuildId) {
    return (
      <div className="mx-auto max-w-[1100px] space-y-6">
        {header}
        <div
          className="rounded-md border border-dashed border-border-2 px-6 py-12 text-center"
          style={{
            background:
              'linear-gradient(180deg, var(--color-panel), color-mix(in srgb, var(--color-bg) 70%, transparent))',
          }}
        >
          <div className="font-mono text-[11px] uppercase tracking-[0.2em] text-muted">
            Loot filters are stored per build
          </div>
          <p className="mx-auto mt-2 max-w-[440px] text-[12px] leading-relaxed text-faint">
            This build isn't saved yet. Save it first (Builds → Save…), then
            create or import loot filters for it.
          </p>
        </div>
      </div>
    )
  }

  return (
    <div className="mx-auto max-w-[1100px] space-y-6">
      {header}

      <div
        className="relative overflow-hidden rounded-md border border-border"
        style={{
          background:
            'linear-gradient(180deg, var(--color-panel), color-mix(in srgb, var(--color-bg) 70%, transparent))',
          boxShadow:
            'inset 0 1px 0 rgba(255,255,255,0.02), 0 8px 24px rgba(0,0,0,0.35)',
        }}
      >
        <CornerMarks size={8} opacity={0.45} />
        <div
          className={`grid ${GRID} items-center border-b border-border px-4 py-2 font-mono text-[10px] uppercase tracking-[0.14em] text-faint`}
          style={{ background: 'rgba(255,255,255,0.008)' }}
        >
          <span>★</span>
          <span>Name</span>
          <span>Modified</span>
          <span aria-hidden />
        </div>

        {filters.length === 0 ? (
          <div className="flex flex-col items-center gap-1.5 px-6 py-12 text-center">
            <div className="font-mono text-[11px] uppercase tracking-[0.2em] text-faint">
              No loot filters yet
            </div>
            <div className="text-[11px] text-muted">
              Create a new filter or import an export string from the game.
            </div>
          </div>
        ) : (
          filters.map((f) => (
            <FilterRow
              key={f.id}
              filter={f}
              onOpen={() => setEditingId(f.id)}
              onChanged={refresh}
              onRequestDelete={() => setPendingDelete(f)}
            />
          ))
        )}

        <button
          type="button"
          onClick={() => setNewFilterCode('')}
          onDragOver={(e) => e.preventDefault()}
          onDrop={(e) => {
            e.preventDefault()
            const text = e.dataTransfer.getData('text')
            if (text) setNewFilterCode(text.trim())
          }}
          className="mx-4 my-5 flex w-[calc(100%-2rem)] items-center gap-4 rounded-[6px] border border-dashed border-border-2 px-6 py-5 text-left transition-colors hover:border-accent-deep"
          style={{
            background: 'color-mix(in srgb, var(--color-panel) 60%, transparent)',
          }}
        >
          <span
            className="grid h-[42px] w-[42px] shrink-0 place-items-center rounded-[3px] border border-border-2 text-accent-deep"
            style={{
              background: 'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
            }}
          >
            <FunnelIcon className="h-[20px] w-[20px]" />
          </span>
          <span>
            <span className="mb-1 block text-[13px] font-semibold text-text">
              Import a filter from the game
            </span>
            <span className="block text-[12px] leading-normal text-muted">
              Click here or drop an export string — you'll find it in the in-game
              loot filter window.
            </span>
          </span>
        </button>
      </div>

      {newFilterCode !== null && (
        <NewFilterModal
          buildId={activeBuildId}
          initialCode={newFilterCode}
          onClose={() => setNewFilterCode(null)}
          onDone={(record) => {
            setNewFilterCode(null)
            setEditingId(record.id)
          }}
        />
      )}

      {generateOpen && (
        <GenerateFromBuildModal
          buildId={activeBuildId}
          buildName={buildName}
          inventory={inventory}
          onClose={() => setGenerateOpen(false)}
          onDone={(record) => {
            setGenerateOpen(false)
            setEditingId(record.id)
          }}
        />
      )}

      {pendingDelete && (
        <ConfirmOverlay
          section="Delete"
          title="Delete loot filter"
          danger
          confirmLabel="Delete filter"
          message={
            <>
              Permanently delete{' '}
              <span className="text-accent-hot">{pendingDelete.name}</span>? This
              cannot be undone.
            </>
          }
          onConfirm={() => {
            deleteFilter(pendingDelete.id)
            setPendingDelete(null)
            refresh()
          }}
          onClose={() => setPendingDelete(null)}
        />
      )}
    </div>
  )
}

function FunnelIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.8}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden
    >
      <path d="M3 5h18l-7 8v5.5L10 21v-8L3 5z" />
    </svg>
  )
}

function CodeIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden
    >
      <path d="M5.8 5.2 L2.8 8 L5.8 10.8" />
      <path d="M10.2 5.2 L13.2 8 L10.2 10.8" />
      <path d="M9.1 3.6 L6.9 12.4" />
    </svg>
  )
}

function CheckIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.8}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden
    >
      <path d="M3.4 8.4 L6.5 11.5 L12.6 5.2" />
    </svg>
  )
}

function MetaChips({ meta }: { meta: FilterSummary | null }) {
  if (!meta) {
    return <span className="text-[10px] text-stat-red/80">unreadable</span>
  }
  if (meta.editedTypes === 0) {
    return (
      <span className="rounded-[2px] border border-border bg-panel-2 px-1.5 py-[1px] font-mono text-[9px] uppercase tracking-[0.12em] text-faint">
        Default
      </span>
    )
  }
  return (
    <>
      <span className="rounded-[2px] border border-border bg-panel-2 px-1.5 py-[1px] font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
        {meta.editedTypes} {meta.editedTypes === 1 ? 'type' : 'types'}
      </span>
      {meta.hiddenStats > 0 && (
        <span className="rounded-[2px] border border-border bg-panel-2 px-1.5 py-[1px] font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
          {meta.hiddenStats} hidden
        </span>
      )}
      {meta.highlightedStats > 0 && (
        <span className="rounded-[2px] border border-accent-deep bg-accent-hot/10 px-1.5 py-[1px] font-mono text-[9px] uppercase tracking-[0.12em] text-accent-hot">
          {meta.highlightedStats}{' '}
          {meta.highlightedStats === 1 ? 'highlight' : 'highlights'}
        </span>
      )}
    </>
  )
}

function FilterRow({
  filter,
  onOpen,
  onChanged,
  onRequestDelete,
}: {
  filter: SavedLootFilter
  onOpen: () => void
  onChanged: () => void
  onRequestDelete: () => void
}) {
  const [copied, copyCode] = useCopyFeedback()
  const [renaming, setRenaming] = useState(false)
  const [draft, setDraft] = useState(filter.name)

  const [meta, setMeta] = useState<FilterSummary | null>(null)
  useEffect(() => {
    let cancelled = false
    void decodeLootFilter(filter.code).then((decoded) => {
      if (!cancelled) setMeta(decoded ? filterSummary(decoded) : null)
    })
    return () => {
      cancelled = true
    }
  }, [filter.code])

  const startRename = () => {
    setDraft(filter.name)
    setRenaming(true)
  }

  const commitRename = () => {
    const trimmed = draft.trim()
    if (trimmed && trimmed !== filter.name) {
      renameFilter(filter.id, trimmed)
      onChanged()
    }
    setRenaming(false)
  }

  const modified = formatTimestamp(filter.updatedAt)
  const isToday = /^Today/i.test(modified)

  return (
    <div
      onClick={() => {
        if (!renaming) onOpen()
      }}
      className={`group grid ${GRID} relative cursor-pointer items-center border-b border-border px-4 py-2.5 transition-colors hover:bg-white/[0.022]`}
    >
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation()
          setFilterFavorite(filter.id, !filter.favorite)
          onChanged()
        }}
        className={`flex h-[18px] w-[18px] items-center justify-center transition-colors ${
          filter.favorite ? 'text-accent-hot' : 'text-faint hover:text-muted'
        }`}
        title={filter.favorite ? 'Unfavorite' : 'Favorite'}
      >
        <StarIcon filled={filter.favorite} />
      </button>

      <div className="flex min-w-0 items-center gap-3 overflow-hidden pr-2">
        <span
          aria-hidden
          className="grid h-[34px] w-[34px] shrink-0 place-items-center rounded-[3px] border border-border-2 text-accent-deep"
          style={{
            background: 'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
            boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.6)',
          }}
        >
          <FunnelIcon className="h-[17px] w-[17px]" />
        </span>
        <div className="min-w-0">
          {renaming ? (
            <input
              value={draft}
              autoFocus
              onClick={(e) => e.stopPropagation()}
              onChange={(e) => setDraft(e.target.value)}
              onFocus={(e) => e.target.select()}
              onBlur={commitRename}
              onKeyDown={(e) => {
                if (e.key === 'Enter') commitRename()
                if (e.key === 'Escape') setRenaming(false)
              }}
              aria-label="Filter name"
              className="w-full max-w-[320px] rounded-[3px] border border-accent-deep bg-panel-2 px-1.5 py-[3px] text-[13.5px] font-semibold text-text outline-none"
            />
          ) : (
            <div className="truncate text-[13.5px] font-semibold text-text transition-colors group-hover:text-accent-hot">
              {filter.name}
            </div>
          )}
          <div className="mt-[5px] flex items-center gap-1.5 overflow-hidden empty:hidden">
            <MetaChips meta={meta} />
          </div>
        </div>
      </div>

      <div
        className={`font-mono text-[11px] tracking-[0.02em] ${
          isToday ? 'text-muted' : 'text-faint'
        }`}
      >
        {modified}
      </div>

      <div
        className="flex items-center justify-end gap-0.5 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100"
        onClick={(e) => e.stopPropagation()}
      >
        <IconAction
          label={copied ? 'Copied!' : 'Copy game code'}
          active={copied}
          onClick={() => void copyCode(filter.code)}
        >
          {copied ? <CheckIcon className="h-[14px] w-[14px]" /> : <CodeIcon className="h-[14px] w-[14px]" />}
        </IconAction>
        <IconAction label="Rename" onClick={startRename}>
          <RenameIcon className="h-[14px] w-[14px]" />
        </IconAction>
        <IconAction
          label="Duplicate"
          onClick={() => {
            duplicateFilter(filter.id)
            onChanged()
          }}
        >
          <CopyIcon className="h-[14px] w-[14px]" />
        </IconAction>
        <IconAction label="Delete" danger onClick={onRequestDelete}>
          <DeleteIcon className="h-[14px] w-[14px]" />
        </IconAction>
      </div>
    </div>
  )
}

function NewFilterModal({
  buildId,
  initialCode,
  onClose,
  onDone,
}: {
  buildId: string
  initialCode: string
  onClose: () => void
  onDone: (record: SavedLootFilter) => void
}) {
  const [name, setName] = useState('New filter')
  const [code, setCode] = useState(initialCode)
  const [error, setError] = useState<string | null>(null)

  const hasCode = code.trim().length > 0

  const submit = async () => {
    const filterName = name.trim() || 'New filter'
    const record = hasCode
      ? await importFilter(buildId, filterName, code)
      : createFilter(buildId, filterName)
    if (!record) {
      setError('Invalid filter string — paste the exact export from the game.')
      return
    }
    onDone(record)
  }

  return (
    <Modal
      onClose={onClose}
      panelClassName="w-[min(560px,92vw)]"
      eyebrow="Loot filter"
      title="New loot filter"
    >
      <div className="flex flex-col gap-3 px-6 py-4">
        <label className="flex flex-col gap-1">
          <span className="font-mono text-[9.5px] uppercase tracking-[0.14em] text-faint">
            Name
          </span>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            autoFocus
            onFocus={(e) => e.target.select()}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void submit()
            }}
            className="rounded-[3px] border border-border-2 bg-panel-2 px-2 py-1.5 text-[12px] text-text outline-none transition-colors focus:border-accent-deep"
          />
        </label>
        <label className="flex flex-col gap-1">
          <span className="font-mono text-[9.5px] uppercase tracking-[0.14em] text-faint">
            Export string · optional
          </span>
          <textarea
            value={code}
            onChange={(e) => {
              setCode(e.target.value)
              setError(null)
            }}
            spellCheck={false}
            rows={5}
            placeholder="Leave empty to start from scratch, or paste an export from the game (eyJ2ZXJzaW9uIjoy…)"
            className="resize-y rounded-[3px] border border-border-2 bg-panel-2 p-2 font-mono text-[10px] leading-relaxed text-muted outline-none transition-colors focus:border-accent-deep"
          />
        </label>
        {error && <span className="text-[11px] text-stat-red">{error}</span>}
      </div>
      <div className="flex items-center justify-end gap-2 border-t border-border px-6 py-3">
        <button type="button" onClick={() => void submit()} className={MODAL_BTN_PRIMARY_CLASS}>
          {hasCode ? 'Import from game' : 'Create'}
        </button>
      </div>
    </Modal>
  )
}
