import { useEffect, useMemo, useState, type ReactNode } from 'react'
import { motion } from 'motion/react'
import Logo from '../Logo'
import { useBuild } from '../../store/build'
import { getActiveProfile, type Folder } from '../../utils/build/savedBuilds'
import { decodeShareToBuild, parseBuildCodeFromInput } from '../../utils/build/shareBuild'
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
import {
  CopyIcon,
  DeleteIcon,
  ImportIcon,
  NewFolderIcon,
  PlusIcon,
  RenameIcon,
  SaveIcon,
  SearchIcon,
} from './icons'
import { T_VIEW } from '../../lib/motion'

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
  | { kind: 'addProfile'; buildId: string }
  | {
      kind: 'renameProfile'
      buildId: string
      profileId: string
      current: string
    }
  | { kind: 'deleteProfile'; buildId: string; profileId: string; name: string }

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
  const switchSavedBuildProfile = useBuild((s) => s.switchSavedBuildProfile)
  const addSavedBuildProfile = useBuild((s) => s.addSavedBuildProfile)
  const renameSavedBuildProfile = useBuild((s) => s.renameSavedBuildProfile)
  const duplicateSavedBuildProfile = useBuild(
    (s) => s.duplicateSavedBuildProfile,
  )
  const removeSavedBuildProfile = useBuild((s) => s.removeSavedBuildProfile)
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
    // "Recent" = the RECENT_LIMIT most recently updated builds. Pick them by
    // recency BEFORE applying the display sort, otherwise sorting by another
    // column would change which builds the slice keeps.
    if (scope.kind === 'recent') {
      list = [...list]
        .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))
        .slice(0, RECENT_LIMIT)
    }
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
    <motion.div
      className="grid h-screen w-screen grid-rows-[44px_38px_1fr_28px] overflow-hidden text-text"
      style={{ background: 'var(--color-panel)' }}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={T_VIEW}
    >
      {/* Titlebar */}
      <header
        className="relative flex h-11 shrink-0 items-center gap-0 pl-3 pr-3"
        style={{
          background:
            'linear-gradient(180deg, var(--color-panel-2), var(--color-panel))',
          boxShadow: 'inset 0 -1px 0 rgba(201,165,90,0.08)',
        }}
      >
        <div className="flex items-center gap-2 border-r border-border pr-3">
          <Logo size={22} glow title="HSPlanner" />
          <span
            className="select-none font-mono text-[11px] uppercase tracking-[0.18em] text-accent-hot"
            style={{ textShadow: '0 0 10px rgba(224,184,100,0.25)' }}
          >
            HSPlanner
          </span>
        </div>
        <div className="ml-2.5 flex items-center gap-2 font-mono text-[11px] uppercase tracking-[0.16em] text-muted">
          <span className="text-faint">Builds</span>
          <span className="text-faint">/</span>
          <span className="text-accent-hot">{breadcrumb}</span>
        </div>
        <div className="ml-auto flex items-center gap-2.5">
          {canClose && (
            <button
              type="button"
              onClick={onClose}
              className="inline-flex items-center gap-1.5 rounded-[3px] border border-border-2 px-2.5 py-1 font-mono text-[10px] uppercase tracking-[0.18em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
              style={{
                background:
                  'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
                boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
              }}
            >
              <span aria-hidden className="font-mono">
                ←
              </span>
              <span>Planner</span>
            </button>
          )}
        </div>
      </header>

      {/* Toolbar */}
      <nav
        className="flex h-full items-center gap-[2px] px-3"
        style={{ background: 'var(--color-panel)' }}
      >
        <ToolButton
          label="New"
          icon={<PlusIcon className="h-3.5 w-3.5" />}
          onClick={onNewBuild}
        />
        <ToolButton
          label="Import…"
          icon={<ImportIcon className="h-3.5 w-3.5" />}
          onClick={() => setOverlay({ kind: 'import' })}
        />
        <ToolButton
          label="Save…"
          icon={<SaveIcon className="h-3.5 w-3.5" />}
          onClick={() => setOverlay({ kind: 'save' })}
        />
        <ToolSep />
        <ToolButton
          label="Copy"
          icon={<CopyIcon className="h-3.5 w-3.5" />}
          disabled={!selectedBuild}
          onClick={() => selectedBuild && handleCopy(selectedBuild.id)}
        />
        <ToolButton
          label="Rename"
          icon={<RenameIcon className="h-3.5 w-3.5" />}
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
          icon={<DeleteIcon className="h-3.5 w-3.5" />}
          danger
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
          icon={<NewFolderIcon className="h-3.5 w-3.5" />}
          onClick={() =>
            setOverlay({
              kind: 'newFolder',
              parentId: scope.kind === 'folder' ? scope.id : null,
            })
          }
        />
        <div className="flex-1" />
        <div className="flex h-[26px] w-[280px] items-center gap-2 rounded-[3px] border border-border bg-panel-2 px-2.5 transition-colors focus-within:border-accent-deep">
          <SearchIcon className="h-3.5 w-3.5 shrink-0 text-faint" />
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search builds, classes, tags…"
            className="min-w-0 flex-1 bg-transparent text-[12px] text-text outline-none placeholder:text-faint"
          />
        </div>
      </nav>

      {/* Body */}
      <main className="grid min-h-0 grid-cols-[240px_1fr_360px] border-t border-border">
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
              <span className="mb-1 block text-[9px] font-semibold uppercase tracking-[0.18em] text-faint">
                Local Library
              </span>
              <span className="text-muted">{lib.builds.length}</span> builds ·{' '}
              <span className="text-muted">
                {approxKB({ builds: lib.builds, folders: lib.folders })}
              </span>
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
          listKey={
            scope.kind === 'folder' ? `folder:${scope.id}` : scope.kind
          }
        />

        <BuildPreview
          build={selectedBuild}
          meta={selectedBuild ? lib.meta[selectedBuild.id] : undefined}
          onOpen={onOpenBuild}
          onCopy={handleExport}
          onSwitchProfile={(buildId, profileId) => {
            if (switchSavedBuildProfile(buildId, profileId))
              flash('Profile switched')
          }}
          onAddProfile={(buildId) => setOverlay({ kind: 'addProfile', buildId })}
          onRenameProfile={(buildId, profileId, current) =>
            setOverlay({ kind: 'renameProfile', buildId, profileId, current })
          }
          onDuplicateProfile={(buildId, profileId) => {
            if (duplicateSavedBuildProfile(buildId, profileId))
              flash('Profile duplicated')
          }}
          onRemoveProfile={(buildId, profileId, name) =>
            setOverlay({ kind: 'deleteProfile', buildId, profileId, name })
          }
        />
      </main>

      {/* Status bar */}
      <footer
        className="flex items-center gap-3.5 border-t border-border px-3 font-mono text-[11px] tracking-[0.06em] text-muted"
        style={{ background: 'var(--color-panel)' }}
      >
        <span
          aria-hidden
          className="h-1.5 w-1.5 shrink-0 rounded-full bg-stat-green"
          style={{ boxShadow: '0 0 6px rgba(116,201,138,0.6)' }}
        />
        <span className="flex items-center gap-1.5">
          {notice ? (
            <span className="text-accent-hot">{notice}</span>
          ) : (
            <>
              <b className="font-medium text-text">{lib.builds.length}</b> Builds
              · <b className="font-medium text-text">{lib.folders.length}</b>{' '}
              Folders
            </>
          )}
        </span>
        <span aria-hidden className="h-3.5 w-px bg-border" />
        <label className="flex cursor-pointer items-center gap-1.5 select-none text-muted">
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
        <span className="inline-flex items-center gap-1.5 text-faint">
          <span className="rounded-[2px] border border-border bg-panel-2 px-[5px] py-[1px] text-[10px] text-muted">
            ↵
          </span>
          Open
        </span>
        <span className="inline-flex items-center gap-1.5 text-faint">
          <span className="rounded-[2px] border border-border bg-panel-2 px-[5px] py-[1px] text-[10px] text-muted">
            Del
          </span>
          Delete
        </span>
        <span className="inline-flex items-center gap-1.5 text-faint">
          <span className="rounded-[2px] border border-border bg-panel-2 px-[5px] py-[1px] text-[10px] text-muted">
            F2
          </span>
          Rename
        </span>
        <span className="inline-flex items-center gap-1.5 text-faint">
          <span className="rounded-[2px] border border-border bg-panel-2 px-[5px] py-[1px] text-[10px] text-muted">
            Ctrl
          </span>
          <span className="rounded-[2px] border border-border bg-panel-2 px-[5px] py-[1px] text-[10px] text-muted">
            N
          </span>
          New
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

      {/* Profile overlays */}
      {overlay?.kind === 'addProfile' && (
        <TextPromptOverlay
          section="Profiles"
          title="New profile"
          label="Profile name"
          placeholder="e.g. Boss setup"
          submitLabel="Create"
          hint="Seeded from this build's active profile."
          onSubmit={(name) => {
            const id = addSavedBuildProfile(overlay.buildId, name)
            setOverlay(null)
            flash(
              id
                ? 'Profile added'
                : 'Could not add profile — build code unreadable',
            )
          }}
          onClose={() => setOverlay(null)}
        />
      )}
      {overlay?.kind === 'renameProfile' && (
        <TextPromptOverlay
          section="Profiles"
          title="Rename profile"
          label="New name"
          initial={overlay.current}
          submitLabel="Save"
          onSubmit={(name) => {
            renameSavedBuildProfile(overlay.buildId, overlay.profileId, name)
            setOverlay(null)
            flash('Profile renamed')
          }}
          onClose={() => setOverlay(null)}
        />
      )}
      {overlay?.kind === 'deleteProfile' && (
        <ConfirmOverlay
          section="Delete"
          title="Delete profile"
          danger
          confirmLabel="Delete profile"
          message={
            <>
              Permanently delete profile{' '}
              <span className="text-accent-hot">{overlay.name}</span>? This
              cannot be undone.
            </>
          }
          onConfirm={() => {
            removeSavedBuildProfile(overlay.buildId, overlay.profileId)
            setOverlay(null)
            flash('Profile deleted')
          }}
          onClose={() => setOverlay(null)}
        />
      )}
    </motion.div>
  )
}

function ToolSep() {
  return <span aria-hidden className="mx-1 h-4 w-px bg-border" />
}

function ToolButton({
  label,
  icon,
  disabled,
  danger,
  onClick,
}: {
  label: string
  icon: ReactNode
  disabled?: boolean
  danger?: boolean
  onClick: () => void
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={`group inline-flex h-7 items-center gap-1.5 rounded-[3px] border border-transparent px-2.5 text-[12px] text-muted transition-colors hover:border-border hover:bg-panel-2 disabled:cursor-not-allowed disabled:opacity-50 ${
        danger ? 'hover:text-stat-red' : 'hover:text-accent-hot'
      }`}
    >
      <span
        aria-hidden
        className={`flex w-3.5 items-center justify-center transition-colors ${
          danger
            ? 'text-accent group-hover:text-stat-red'
            : 'text-accent'
        }`}
      >
        {icon}
      </span>
      {label}
    </button>
  )
}
