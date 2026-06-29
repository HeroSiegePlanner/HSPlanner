import { useEffect, useMemo, useState } from 'react'
import { motion } from 'motion/react'
import Logo from '../Logo'
import { useBuild } from '../../store/build'
import { activeSeasonId } from '../../data'
import { PENDING_IMPORT_KEY, reloadIntoSeason } from '../../data/seasons/registry'
import { getActiveProfile, type Folder } from '../../utils/build/savedBuilds'
import { decodeShareToBuild, parseBuildCodeFromInput } from '../../utils/build/shareBuild'
import {
  GistShareError,
  fetchBuildCodeFromGist,
  isGistReference,
} from '../../utils/build/gistShare'
import { readStorage, writeStorage } from '../../utils/storage'
import { approxKB } from './helpers'
import { useBuildLibrary } from './useBuildLibrary'
import { FolderTree, type Scope, type SmartCounts } from './FolderTree'
import { BuildTable, type SortCol, type SortDir } from './BuildTable'
import { BuildPreview } from './BuildPreview'
import { type ContextMenuItem } from './ContextMenu'
import { BuildSelectToolbar } from './BuildSelectToolbar'
import { BuildSelectFooter } from './BuildSelectFooter'
import { BuildSelectOverlays } from './BuildSelectOverlays'
import {
  RECENT_LIMIT,
  SCOPE_LABEL,
  type BuildSelectProps,
  type CtxState,
  type Overlay,
} from './buildSelectTypes'
import { T_VIEW } from '../../lib/motion'

export const AUTO_OPEN_KEY = 'hsplanner.autoOpenLastBuild.v1'

export default function BuildSelect({
  onOpenBuild,
  onNewBuild,
  onClose,
  canClose,
}: BuildSelectProps) {
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

  useEffect(() => {
    if (!notice) return
    const t = window.setTimeout(() => setNotice(null), 2200)
    return () => window.clearTimeout(t)
  }, [notice])

  const flash = (msg: string) => setNotice(msg)

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

  const handleSort = (col: SortCol) => {
    if (col === sortCol) {
      setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'))
    } else {
      setSortCol(col)
      setSortDir(col === 'name' || col === 'class' ? 'asc' : 'desc')
    }
  }

  const handleImport = async (text: string): Promise<string | null> => {
    let raw = text
    if (isGistReference(text)) {
      try {
        raw = await fetchBuildCodeFromGist(text)
      } catch (e) {
        return e instanceof GistShareError ? e.message : 'Could not fetch the Gist'
      }
    }
    const code = parseBuildCodeFromInput(raw)
    if (!code) return "Couldn't read a build code from input"
    const decoded = decodeShareToBuild(code)
    if (!decoded) return 'Invalid or corrupted build code'
    if (reloadIntoSeason(decoded.season, PENDING_IMPORT_KEY, code, activeSeasonId)) {
      return null
    }
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

      <BuildSelectToolbar
        scope={scope}
        search={search}
        selectedBuild={selectedBuild}
        onSearchChange={setSearch}
        onNewBuild={onNewBuild}
        onOverlay={setOverlay}
        onCopy={handleCopy}
      />

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

      <BuildSelectFooter
        buildCount={lib.builds.length}
        folderCount={lib.folders.length}
        notice={notice}
        autoOpen={autoOpen}
        onToggleAutoOpen={(checked) => {
          setAutoOpen(checked)
          writeStorage(AUTO_OPEN_KEY, checked ? '1' : '0')
        }}
      />

      <BuildSelectOverlays
        ctx={ctx}
        ctxItems={ctxItems}
        ctxHeader={
          ctx?.kind === 'build'
            ? lib.builds.find((b) => b.id === ctx.id)?.name
            : ctx
              ? lib.folders.find((f) => f.id === ctx.id)?.name
              : undefined
        }
        onCloseCtx={() => setCtx(null)}
        overlay={overlay}
        onCloseOverlay={() => setOverlay(null)}
        folders={lib.folders}
        canOverwrite={!!activeBuildId}
        onImport={handleImport}
        onOverwrite={() => {
          if (commitActiveProfile()) flash('Updated active profile')
          setOverlay(null)
        }}
        onSaveAsNew={handleSaveCurrent}
        onRenameBuild={(buildId, name) => {
          renameSavedBuild(buildId, name)
          setOverlay(null)
          flash('Build renamed')
        }}
        onSaveTags={(buildId, tags) => {
          setSavedBuildTags(buildId, tags)
          setOverlay(null)
          flash('Tags updated')
        }}
        onMove={(buildId, folderId) => {
          moveSavedBuildToFolder(buildId, folderId)
          setOverlay(null)
          flash('Build moved')
        }}
        onCreateFolder={(name, parentId) => {
          const folder = createSavedFolder(name, parentId)
          if (folder && parentId) {
            setExpanded((cur) => new Set(cur).add(parentId))
          }
          setOverlay(null)
          flash('Folder created')
        }}
        onRenameFolder={(folderId, name) => {
          renameSavedFolder(folderId, name)
          setOverlay(null)
          flash('Folder renamed')
        }}
        onDeleteBuild={(buildId) => {
          deleteSavedBuild(buildId)
          setOverlay(null)
          flash('Build deleted')
        }}
        onDeleteFolder={(folderId) => {
          deleteSavedFolder(folderId, false)
          if (scope.kind === 'folder' && scope.id === folderId) {
            setScope({ kind: 'all' })
          }
          setOverlay(null)
          flash('Folder deleted')
        }}
        onAddProfile={(buildId, name) => {
          const id = addSavedBuildProfile(buildId, name)
          setOverlay(null)
          flash(
            id
              ? 'Profile added'
              : 'Could not add profile — build code unreadable',
          )
        }}
        onRenameProfile={(buildId, profileId, name) => {
          renameSavedBuildProfile(buildId, profileId, name)
          setOverlay(null)
          flash('Profile renamed')
        }}
        onDeleteProfile={(buildId, profileId) => {
          removeSavedBuildProfile(buildId, profileId)
          setOverlay(null)
          flash('Profile deleted')
        }}
      />
    </motion.div>
  )
}
