import { useEffect, useMemo, useState } from 'react'
import Logo from '../Logo'
import { useBuild } from '../../store/build'
import { getActiveProfile, type Folder } from '../../utils/savedBuilds'
import { decodeShareToBuild, parseBuildCodeFromInput } from '../../utils/shareBuild'
import { readStorage, writeStorage } from '../../utils/storage'
import { approxKB } from './helpers'
import { useBuildLibrary } from './useBuildLibrary'
import { FolderTree, type Scope, type SmartCounts } from './FolderTree'
import { BuildTable, type SortCol, type SortDir } from './BuildTable'
import { BuildPreview } from './BuildPreview'
import { ContextMenu, type ContextMenuItem } from './ContextMenu'
import {
  ConfirmOverlay,
  ImportOverlay,
  MoveToFolderOverlay,
  SaveOverlay,
  TagsOverlay,
  TextPromptOverlay,
} from './overlays'

/** localStorage flag — when set, boot skips the library and opens the most recent build. */
export const AUTO_OPEN_KEY = 'hsplanner.autoOpenLastBuild.v1'

const RECENT_LIMIT = 12

interface BuildSelectProps {
  /** Load a saved build into the planner. */
  onOpenBuild: (buildId: string) => void
  /** Start a fresh blank build in the planner. */
  onNewBuild: () => void
  /** Return to the planner without changing the active build. */
  onClose: () => void
  /** True when a build is already loaded — enables the "back to planner" affordance. */
  canClose: boolean
}

type Overlay =
  | { kind: 'import' }
  | { kind: 'save' }
  | { kind: 'renameBuild'; buildId: string; current: string }
  | { kind: 'tags'; buildId: string; current: string[] }
  | { kind: 'move'; buildId: string; current: string | null }
  | { kind: 'newFolder'; parentId: string | null }
  | { kind: 'renameFolder'; folderId: string; current: string }
  | { kind: 'deleteBuild'; buildId: string; name: string }
  | { kind: 'deleteFolder'; folderId: string; name: string; count: number }

interface CtxState {
  x: number
  y: number
  kind: 'build' | 'folder'
  id: string
}

const SCOPE_LABEL: Record<Scope['kind'], string> = {
  recent: 'Recent',
  all: 'All Builds',
  favorites: 'Favorites',
  unfiled: 'Unfiled',
  folder: 'Folder',
}

export default function BuildSelect({
  onOpenBuild,
  onNewBuild,
  onClose,
  canClose,
}: BuildSelectProps) {
  // Full-screen build library: folder tree + sortable build table + live
  // stats preview. Replaces the old StartupBuildModal as the app's entry
  // screen and stays reachable from the planner header.
  const lib = useBuildLibrary()
  const activeBuildId = useBuild((s) => s.activeBuildId)

  const importBuildSnapshot = useBuild((s) => s.importBuildSnapshot)
  const saveCurrentAsNewBuild = useBuild((s) => s.saveCurrentAsNewBuild)
  const commitActiveProfile = useBuild((s) => s.commitActiveProfile)
  const duplicateSavedBuild = useBuild((s) => s.duplicateSavedBuild)
  const renameSavedBuild = useBuild((s) => s.renameSavedBuild)
  const deleteSavedBuild = useBuild((s) => s.deleteSavedBuild)
  const setSavedBuildFavorite = useBuild((s) => s.setSavedBuildFavorite)
  const setSavedBuildTags = useBuild((s) => s.setSavedBuildTags)
  const moveSavedBuildToFolder = useBuild((s) => s.moveSavedBuildToFolder)
  const createSavedFolder = useBuild((s) => s.createSavedFolder)
  const renameSavedFolder = useBuild((s) => s.renameSavedFolder)
  const deleteSavedFolder = useBuild((s) => s.deleteSavedFolder)

  const [scope, setScope] = useState<Scope>({ kind: 'recent' })
  const [selectedId, setSelectedId] = useState<string | null>(
    () => useBuild.getState().activeBuildId,
  )
  const [search, setSearch] = useState('')
  const [sortCol, setSortCol] = useState<SortCol>('date')
  const [sortDir, setSortDir] = useState<SortDir>('desc')
  const [activeTags, setActiveTags] = useState<string[]>([])
  const [levelFilter, setLevelFilter] = useState(false)
  const [expanded, setExpanded] = useState<Set<string>>(new Set())
  const [ctx, setCtx] = useState<CtxState | null>(null)
  const [overlay, setOverlay] = useState<Overlay | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [autoOpen, setAutoOpen] = useState(
    () => readStorage(AUTO_OPEN_KEY) === '1',
  )

  // Transient status-bar notice.
  useEffect(() => {
    if (!notice) return
    const t = window.setTimeout(() => setNotice(null), 2200)
    return () => window.clearTimeout(t)
  }, [notice])

  const flash = (msg: string) => setNotice(msg)

  // --- folder helpers ---------------------------------------------------
  const descendantFolderIds = useMemo(() => {
    return (rootId: string): Set<string> => {
      const out = new Set<string>([rootId])
      let added = true
      while (added) {
        added = false
        for (const f of lib.folders) {
          if (f.parentId && out.has(f.parentId) && !out.has(f.id)) {
            out.add(f.id)
            added = true
          }
        }
      }
      return out
    }
  }, [lib.folders])

  const folderCounts = useMemo(() => {
    const direct: Record<string, number> = {}
    for (const b of lib.builds) {
      if (b.folderId) direct[b.folderId] = (direct[b.folderId] ?? 0) + 1
    }
    const memo: Record<string, number> = {}
    const compute = (id: string): number => {
      if (memo[id] !== undefined) return memo[id]!
      let c = direct[id] ?? 0
      for (const child of lib.childFolders[id] ?? []) c += compute(child.id)
      memo[id] = c
      return c
    }
    for (const f of lib.folders) compute(f.id)
    return memo
  }, [lib])

  const smartCounts: SmartCounts = useMemo(
    () => ({
      recent: Math.min(lib.builds.length, RECENT_LIMIT),
      all: lib.builds.length,
      favorites: lib.builds.filter((b) => b.favorite).length,
      unfiled: lib.builds.filter((b) => b.folderId === null).length,
    }),
    [lib.builds],
  )

  const allTags = useMemo(() => {
    const set = new Set<string>()
    for (const b of lib.builds) for (const t of b.tags) set.add(t)
    return [...set].sort((a, b) => a.localeCompare(b))
  }, [lib.builds])

  // --- scoped + filtered + sorted builds --------------------------------
  const scopedBuilds = useMemo(() => {
    if (scope.kind === 'favorites') return lib.builds.filter((b) => b.favorite)
    if (scope.kind === 'unfiled')
      return lib.builds.filter((b) => b.folderId === null)
    if (scope.kind === 'folder') {
      const subtree = descendantFolderIds(scope.id)
      return lib.builds.filter(
        (b) => b.folderId !== null && subtree.has(b.folderId),
      )
    }
    return lib.builds
  }, [lib.builds, scope, descendantFolderIds])

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase()
    let list = scopedBuilds.filter((b) => {
      if (activeTags.length && !activeTags.every((t) => b.tags.includes(t)))
        return false
      if (levelFilter && (lib.meta[b.id]?.level ?? 0) < 90) return false
      if (q) {
        const m = lib.meta[b.id]
        const hay = `${b.name} ${m?.className ?? ''} ${b.tags.join(' ')} ${b.profiles
          .map((p) => p.name)
          .join(' ')}`.toLowerCase()
        if (!hay.includes(q)) return false
      }
      return true
    })
    const dir = sortDir === 'asc' ? 1 : -1
    list = [...list].sort((a, b) => {
      let cmp = 0
      switch (sortCol) {
        case 'favorite':
          cmp = (a.favorite ? 1 : 0) - (b.favorite ? 1 : 0)
          break
        case 'name':
          cmp = a.name.localeCompare(b.name)
          break
        case 'class':
          cmp = (lib.meta[a.id]?.className ?? '').localeCompare(
            lib.meta[b.id]?.className ?? '',
          )
          break
        case 'level':
          cmp = (lib.meta[a.id]?.level ?? 0) - (lib.meta[b.id]?.level ?? 0)
          break
        case 'date':
          cmp = a.updatedAt.localeCompare(b.updatedAt)
          break
      }
      return cmp * dir
    })
    if (scope.kind === 'recent') list = list.slice(0, RECENT_LIMIT)
    return list
  }, [scopedBuilds, search, activeTags, levelFilter, sortCol, sortDir, scope, lib.meta])

  const totalCount =
    scope.kind === 'recent'
      ? Math.min(scopedBuilds.length, RECENT_LIMIT)
      : scopedBuilds.length

  const effectiveSelectedId =
    selectedId && filtered.some((b) => b.id === selectedId)
      ? selectedId
      : (filtered[0]?.id ?? null)
  const selectedBuild =
    lib.builds.find((b) => b.id === effectiveSelectedId) ?? null

  // --- actions ----------------------------------------------------------
  const handleSort = (col: SortCol) => {
    if (col === sortCol) {
      setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'))
    } else {
      setSortCol(col)
      setSortDir(col === 'name' || col === 'class' ? 'asc' : 'desc')
    }
  }

  const handleImport = (text: string): string | null => {
    const code = parseBuildCodeFromInput(text)
    if (!code) return "Couldn't read a build code from input"
    const decoded = decodeShareToBuild(code)
    if (!decoded) return 'Invalid or corrupted build code'
    importBuildSnapshot(decoded.snapshot, decoded.notes)
    setOverlay(null)
    onClose()
    return null
  }

  const handleCopy = (buildId: string) => {
    const rec = duplicateSavedBuild(buildId)
    if (rec) {
      setSelectedId(rec.id)
      flash(`Duplicated "${rec.name}"`)
    }
  }

  const handleExport = (buildId: string) => {
    const build = lib.builds.find((b) => b.id === buildId)
    const profile = build ? getActiveProfile(build) : null
    if (!profile) {
      flash('Nothing to export')
      return
    }
    navigator.clipboard
      ?.writeText(profile.code)
      .then(() => flash('Build code copied to clipboard'))
      .catch(() => flash('Could not access clipboard'))
  }

  const handleSaveCurrent = (name: string) => {
    const notes = useBuild.getState().notes
    const folderId = scope.kind === 'folder' ? scope.id : null
    const rec = saveCurrentAsNewBuild(name, notes, folderId)
    if (rec) {
      setSelectedId(rec.id)
      setOverlay(null)
      flash(`Saved "${rec.name}"`)
    }
  }

  const toggleTag = (tag: string) =>
    setActiveTags((cur) =>
      cur.includes(tag) ? cur.filter((t) => t !== tag) : [...cur, tag],
    )

  const toggleExpand = (folderId: string) =>
    setExpanded((cur) => {
      const next = new Set(cur)
      if (next.has(folderId)) next.delete(folderId)
      else next.add(folderId)
      return next
    })

  const openContextForBuild = (e: React.MouseEvent, buildId: string) => {
    e.preventDefault()
    setSelectedId(buildId)
    setCtx({ x: e.clientX, y: e.clientY, kind: 'build', id: buildId })
  }
  const openContextForFolder = (e: React.MouseEvent, folder: Folder) => {
    e.preventDefault()
    setCtx({ x: e.clientX, y: e.clientY, kind: 'folder', id: folder.id })
  }

  // --- keyboard nav -----------------------------------------------------
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      const tag = (e.target as HTMLElement | null)?.tagName
      if (tag === 'INPUT' || tag === 'TEXTAREA') return
      if (overlay || ctx) return
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n') {
        e.preventDefault()
        onNewBuild()
        return
      }
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault()
        if (filtered.length === 0) return
        const idx = filtered.findIndex((b) => b.id === effectiveSelectedId)
        const next =
          e.key === 'ArrowDown'
            ? Math.min(filtered.length - 1, idx + 1)
            : Math.max(0, idx - 1)
        setSelectedId(filtered[next]!.id)
      } else if (e.key === 'Enter' && effectiveSelectedId) {
        e.preventDefault()
        onOpenBuild(effectiveSelectedId)
      } else if (e.key === 'F2' && selectedBuild) {
        setOverlay({
          kind: 'renameBuild',
          buildId: selectedBuild.id,
          current: selectedBuild.name,
        })
      } else if (
        (e.key === 'Delete' || e.key === 'Backspace') &&
        selectedBuild
      ) {
        setOverlay({
          kind: 'deleteBuild',
          buildId: selectedBuild.id,
          name: selectedBuild.name,
        })
      } else if (e.key === 'Escape' && canClose) {
        onClose()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [filtered, effectiveSelectedId, selectedBuild, overlay, ctx, canClose, onClose, onNewBuild, onOpenBuild])

  // --- context-menu items ----------------------------------------------
  const ctxItems: ContextMenuItem[] = useMemo(() => {
    if (!ctx) return []
    if (ctx.kind === 'build') {
      const build = lib.builds.find((b) => b.id === ctx.id)
      if (!build) return []
      return [
        { label: 'Open Build', kbd: '↵', onClick: () => onOpenBuild(build.id) },
        { label: 'Duplicate', onClick: () => handleCopy(build.id) },
        {
          label: 'Rename…',
          kbd: 'F2',
          onClick: () =>
            setOverlay({
              kind: 'renameBuild',
              buildId: build.id,
              current: build.name,
            }),
        },
        {
          label: build.favorite ? 'Unfavorite' : 'Favorite',
          onClick: () => setSavedBuildFavorite(build.id, !build.favorite),
        },
        {
          label: 'Move to folder…',
          onClick: () =>
            setOverlay({
              kind: 'move',
              buildId: build.id,
              current: build.folderId,
            }),
        },
        {
          label: 'Edit tags…',
          onClick: () =>
            setOverlay({
              kind: 'tags',
              buildId: build.id,
              current: build.tags,
            }),
        },
        { label: 'Export code', onClick: () => handleExport(build.id) },
        {
          label: 'Delete',
          kbd: 'Del',
          danger: true,
          separatorBefore: true,
          onClick: () =>
            setOverlay({
              kind: 'deleteBuild',
              buildId: build.id,
              name: build.name,
            }),
        },
      ]
    }
    const folder = lib.folders.find((f) => f.id === ctx.id)
    if (!folder) return []
    return [
      {
        label: 'New subfolder…',
        onClick: () => setOverlay({ kind: 'newFolder', parentId: folder.id }),
      },
      {
        label: 'Rename folder…',
        onClick: () =>
          setOverlay({
            kind: 'renameFolder',
            folderId: folder.id,
            current: folder.name,
          }),
      },
      {
        label: 'Delete folder',
        danger: true,
        separatorBefore: true,
        onClick: () =>
          setOverlay({
            kind: 'deleteFolder',
            folderId: folder.id,
            name: folder.name,
            count: folderCounts[folder.id] ?? 0,
          }),
      },
    ]
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ctx, lib.builds, lib.folders, folderCounts])

  // --- render -----------------------------------------------------------
  const breadcrumb =
    scope.kind === 'folder'
      ? (lib.folders.find((f) => f.id === scope.id)?.name ?? 'Folder')
      : SCOPE_LABEL[scope.kind]

  return (
    <div
      className="grid h-screen w-screen grid-rows-[40px_38px_1fr_26px] overflow-hidden text-text"
      style={{ background: 'var(--color-bg)' }}
    >
      {/* Titlebar */}
      <header
        className="flex items-center gap-3.5 border-b border-border px-3.5"
        style={{ background: 'var(--color-panel-2)' }}
      >
        <div className="flex items-center gap-2">
          <Logo size={20} glow title="HSPlanner" />
          <span className="font-mono text-[12px] uppercase tracking-[0.22em] text-accent-hot">
            HSPlanner
          </span>
        </div>
        <span aria-hidden className="h-[18px] w-px bg-border" />
        <div className="flex items-center gap-1.5 font-mono text-[11px] text-faint">
          <span>Builds</span>
          <span className="text-faint">/</span>
          <span className="text-muted">{breadcrumb}</span>
        </div>
        <div className="flex-1" />
        {canClose && (
          <button
            type="button"
            onClick={onClose}
            className="rounded-[3px] border border-border-2 px-2.5 py-1 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
          >
            ← Planner
          </button>
        )}
      </header>

      {/* Toolbar */}
      <nav
        className="flex items-center gap-0.5 border-b border-border px-1.5"
        style={{ background: 'var(--color-panel-2)' }}
      >
        <ToolButton
          primary
          label="Open Build"
          icon="▶"
          disabled={!selectedBuild}
          onClick={() => effectiveSelectedId && onOpenBuild(effectiveSelectedId)}
        />
        <ToolSep />
        <ToolButton label="New" icon="＋" onClick={onNewBuild} />
        <ToolButton
          label="Import…"
          icon="⇪"
          onClick={() => setOverlay({ kind: 'import' })}
        />
        <ToolButton
          label="Save…"
          icon="⇫"
          onClick={() => setOverlay({ kind: 'save' })}
        />
        <ToolSep />
        <ToolButton
          label="Copy"
          icon="⎘"
          disabled={!selectedBuild}
          onClick={() => selectedBuild && handleCopy(selectedBuild.id)}
        />
        <ToolButton
          label="Rename"
          icon="✎"
          disabled={!selectedBuild}
          onClick={() =>
            selectedBuild &&
            setOverlay({
              kind: 'renameBuild',
              buildId: selectedBuild.id,
              current: selectedBuild.name,
            })
          }
        />
        <ToolButton
          label="Delete"
          icon="✕"
          disabled={!selectedBuild}
          onClick={() =>
            selectedBuild &&
            setOverlay({
              kind: 'deleteBuild',
              buildId: selectedBuild.id,
              name: selectedBuild.name,
            })
          }
        />
        <ToolSep />
        <ToolButton
          label="New Folder"
          icon="▢"
          onClick={() =>
            setOverlay({
              kind: 'newFolder',
              parentId: scope.kind === 'folder' ? scope.id : null,
            })
          }
        />
        <div className="flex-1" />
        <div
          className="flex h-7 w-60 items-center gap-1.5 rounded-[3px] border border-border px-2.5"
          style={{ background: 'var(--color-panel-3)' }}
        >
          <span className="text-[13px] text-faint">⌕</span>
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search builds, classes, tags…"
            className="min-w-0 flex-1 bg-transparent text-[12px] text-text outline-none placeholder:text-faint"
          />
        </div>
      </nav>

      {/* Body */}
      <main className="grid min-h-0 grid-cols-[240px_1fr_320px]">
        <FolderTree
          childFolders={lib.childFolders}
          scope={scope}
          onScopeChange={setScope}
          smartCounts={smartCounts}
          folderCounts={folderCounts}
          expanded={expanded}
          onToggleExpand={toggleExpand}
          onNewFolder={() =>
            setOverlay({
              kind: 'newFolder',
              parentId: scope.kind === 'folder' ? scope.id : null,
            })
          }
          onFolderContextMenu={openContextForFolder}
          footer={
            <>
              <span className="block font-semibold uppercase tracking-[0.2em]">
                Local Library
              </span>
              {lib.builds.length} builds ·{' '}
              {approxKB({ builds: lib.builds, folders: lib.folders })}
            </>
          }
        />

        <BuildTable
          builds={filtered}
          meta={lib.meta}
          selectedId={effectiveSelectedId}
          activeBuildId={activeBuildId}
          sortCol={sortCol}
          sortDir={sortDir}
          onSort={handleSort}
          onSelect={setSelectedId}
          onOpen={onOpenBuild}
          onContextMenu={openContextForBuild}
          onToggleFavorite={(id) => {
            const b = lib.builds.find((x) => x.id === id)
            if (b) setSavedBuildFavorite(id, !b.favorite)
          }}
          allTags={allTags}
          activeTags={activeTags}
          onToggleTag={toggleTag}
          levelFilter={levelFilter}
          onToggleLevelFilter={() => setLevelFilter((v) => !v)}
          onClearFilters={() => {
            setActiveTags([])
            setLevelFilter(false)
          }}
          totalCount={totalCount}
        />

        <BuildPreview
          build={selectedBuild}
          meta={selectedBuild ? lib.meta[selectedBuild.id] : undefined}
          onOpen={onOpenBuild}
          onCopy={handleExport}
        />
      </main>

      {/* Status bar */}
      <footer
        className="flex items-center gap-3 border-t border-border px-3 font-mono text-[10px] uppercase tracking-[0.1em] text-faint"
        style={{ background: 'var(--color-panel-2)' }}
      >
        <span
          aria-hidden
          className="h-1.5 w-1.5 rounded-full bg-stat-green"
          style={{ boxShadow: '0 0 6px rgba(116,201,138,0.6)' }}
        />
        <span>
          {notice ? (
            <span className="text-accent-hot">{notice}</span>
          ) : (
            <>
              <span className="text-muted">{lib.builds.length}</span> builds ·{' '}
              <span className="text-muted">{lib.folders.length}</span> folders
            </>
          )}
        </span>
        <span aria-hidden className="h-3 w-px bg-border" />
        <label className="flex cursor-pointer items-center gap-1.5">
          <input
            type="checkbox"
            checked={autoOpen}
            onChange={(e) => {
              setAutoOpen(e.target.checked)
              writeStorage(AUTO_OPEN_KEY, e.target.checked ? '1' : '0')
            }}
          />
          <span>Auto-open last build</span>
        </label>
        <div className="flex-1" />
        <span>
          ↵ <b className="text-muted">Open</b> · ⌫{' '}
          <b className="text-muted">Delete</b> · F2{' '}
          <b className="text-muted">Rename</b> · Ctrl+N{' '}
          <b className="text-muted">New</b>
        </span>
      </footer>

      {/* Context menu */}
      {ctx && (
        <ContextMenu
          x={ctx.x}
          y={ctx.y}
          header={
            ctx.kind === 'build'
              ? lib.builds.find((b) => b.id === ctx.id)?.name
              : lib.folders.find((f) => f.id === ctx.id)?.name
          }
          items={ctxItems}
          onClose={() => setCtx(null)}
        />
      )}

      {/* Overlays */}
      {overlay?.kind === 'import' && (
        <ImportOverlay
          onImport={handleImport}
          onClose={() => setOverlay(null)}
        />
      )}
      {overlay?.kind === 'save' && (
        <SaveOverlay
          canOverwrite={!!activeBuildId}
          onOverwrite={() => {
            if (commitActiveProfile()) flash('Updated active profile')
            setOverlay(null)
          }}
          onSaveAsNew={handleSaveCurrent}
          onClose={() => setOverlay(null)}
        />
      )}
      {overlay?.kind === 'renameBuild' && (
        <TextPromptOverlay
          section="Rename"
          title="Rename build"
          label="New name"
          initial={overlay.current}
          submitLabel="Save"
          onSubmit={(name) => {
            renameSavedBuild(overlay.buildId, name)
            setOverlay(null)
            flash('Build renamed')
          }}
          onClose={() => setOverlay(null)}
        />
      )}
      {overlay?.kind === 'tags' && (
        <TagsOverlay
          initial={overlay.current}
          onSave={(tags) => {
            setSavedBuildTags(overlay.buildId, tags)
            setOverlay(null)
            flash('Tags updated')
          }}
          onClose={() => setOverlay(null)}
        />
      )}
      {overlay?.kind === 'move' && (
        <MoveToFolderOverlay
          folders={lib.folders}
          currentFolderId={overlay.current}
          onMove={(folderId) => {
            moveSavedBuildToFolder(overlay.buildId, folderId)
            setOverlay(null)
            flash('Build moved')
          }}
          onClose={() => setOverlay(null)}
        />
      )}
      {overlay?.kind === 'newFolder' && (
        <TextPromptOverlay
          section="Organise"
          title="New folder"
          label="Folder name"
          placeholder="e.g. Season 7"
          submitLabel="Create"
          onSubmit={(name) => {
            const folder = createSavedFolder(name, overlay.parentId)
            if (folder && overlay.parentId) {
              setExpanded((cur) => new Set(cur).add(overlay.parentId!))
            }
            setOverlay(null)
            flash('Folder created')
          }}
          onClose={() => setOverlay(null)}
        />
      )}
      {overlay?.kind === 'renameFolder' && (
        <TextPromptOverlay
          section="Organise"
          title="Rename folder"
          label="New name"
          initial={overlay.current}
          submitLabel="Save"
          onSubmit={(name) => {
            renameSavedFolder(overlay.folderId, name)
            setOverlay(null)
            flash('Folder renamed')
          }}
          onClose={() => setOverlay(null)}
        />
      )}
      {overlay?.kind === 'deleteBuild' && (
        <ConfirmOverlay
          section="Delete"
          title="Delete build"
          danger
          confirmLabel="Delete build"
          message={
            <>
              Permanently delete{' '}
              <span className="text-accent-hot">{overlay.name}</span> and all
              its profiles? This cannot be undone.
            </>
          }
          onConfirm={() => {
            deleteSavedBuild(overlay.buildId)
            setOverlay(null)
            flash('Build deleted')
          }}
          onClose={() => setOverlay(null)}
        />
      )}
      {overlay?.kind === 'deleteFolder' && (
        <ConfirmOverlay
          section="Delete"
          title="Delete folder"
          danger
          confirmLabel="Delete folder"
          message={
            <>
              Delete <span className="text-accent-hot">{overlay.name}</span>?
              {overlay.count > 0 ? (
                <>
                  {' '}
                  Its {overlay.count} build{overlay.count === 1 ? '' : 's'} will
                  be moved to Unfiled.
                </>
              ) : (
                ' It is empty.'
              )}
            </>
          }
          onConfirm={() => {
            deleteSavedFolder(overlay.folderId, false)
            if (scope.kind === 'folder' && scope.id === overlay.folderId) {
              setScope({ kind: 'all' })
            }
            setOverlay(null)
            flash('Folder deleted')
          }}
          onClose={() => setOverlay(null)}
        />
      )}
    </div>
  )
}

function ToolSep() {
  return <span aria-hidden className="mx-1 h-5 w-px bg-border" />
}

function ToolButton({
  label,
  icon,
  primary,
  disabled,
  onClick,
}: {
  label: string
  icon: string
  primary?: boolean
  disabled?: boolean
  onClick: () => void
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={`inline-flex h-7 items-center gap-1.5 rounded-[3px] border px-2.5 text-[12px] transition-colors disabled:cursor-not-allowed disabled:opacity-50 ${
        primary
          ? 'border-accent-deep text-accent-hot hover:border-accent-hot'
          : 'border-transparent text-muted hover:border-border hover:bg-panel-3 hover:text-text'
      }`}
      style={
        primary
          ? { background: 'linear-gradient(180deg,#3a2f1a,#241e10)' }
          : undefined
      }
    >
      <span
        aria-hidden
        className={primary ? 'text-accent-hot' : 'text-accent-deep'}
      >
        {icon}
      </span>
      {label}
    </button>
  )
}
