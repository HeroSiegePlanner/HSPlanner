import type { Folder } from '../../utils/build/savedBuilds'
import { ContextMenu, type ContextMenuItem } from './ContextMenu'
import {
  ConfirmOverlay,
  ImportOverlay,
  MoveToFolderOverlay,
  TagsOverlay,
  TextPromptOverlay,
} from './overlays'
import type { CtxState, Overlay } from './buildSelectTypes'

interface BuildSelectOverlaysProps {
  ctx: CtxState | null
  ctxItems: ContextMenuItem[]
  ctxHeader: string | undefined
  onCloseCtx: () => void

  overlay: Overlay | null
  onCloseOverlay: () => void
  folders: Folder[]

  onImport: (text: string) => Promise<string | null>
  onImportNamed: (code: string, name: string) => void
  onCreateBuild: (name: string) => void
  onRenameBuild: (buildId: string, name: string) => void
  onSaveTags: (buildId: string, tags: string[]) => void
  onMove: (buildId: string, folderId: string | null) => void
  onCreateFolder: (name: string, parentId: string | null) => void
  onRenameFolder: (folderId: string, name: string) => void
  onDeleteBuild: (buildId: string) => void
  onDeleteFolder: (folderId: string) => void
  onAddProfile: (buildId: string, name: string) => void
  onRenameProfile: (buildId: string, profileId: string, name: string) => void
  onDeleteProfile: (buildId: string, profileId: string) => void
}

export function BuildSelectOverlays({
  ctx,
  ctxItems,
  ctxHeader,
  onCloseCtx,
  overlay,
  onCloseOverlay,
  folders,
  onImport,
  onImportNamed,
  onCreateBuild,
  onRenameBuild,
  onSaveTags,
  onMove,
  onCreateFolder,
  onRenameFolder,
  onDeleteBuild,
  onDeleteFolder,
  onAddProfile,
  onRenameProfile,
  onDeleteProfile,
}: BuildSelectOverlaysProps) {
  return (
    <>
      {ctx && (
        <ContextMenu
          x={ctx.x}
          y={ctx.y}
          header={ctxHeader}
          items={ctxItems}
          onClose={onCloseCtx}
        />
      )}

      {overlay?.kind === 'import' && (
        <ImportOverlay onImport={onImport} onClose={onCloseOverlay} />
      )}
      {overlay?.kind === 'nameImport' && (
        <TextPromptOverlay
          section="Import"
          title="Name imported build"
          label="Build name"
          initial={overlay.defaultName}
          submitLabel="Save & open"
          hint="Saved to your library, then opened."
          onSubmit={(name) => onImportNamed(overlay.code, name)}
          onClose={onCloseOverlay}
        />
      )}
      {overlay?.kind === 'newBuild' && (
        <TextPromptOverlay
          section="Create"
          title="New build"
          label="Build name"
          placeholder="e.g. Lightning Marksman"
          submitLabel="Create"
          hint="Saved to your library, then opened."
          onSubmit={onCreateBuild}
          onClose={onCloseOverlay}
        />
      )}
      {overlay?.kind === 'renameBuild' && (
        <TextPromptOverlay
          section="Rename"
          title="Rename build"
          label="New name"
          initial={overlay.current}
          submitLabel="Save"
          onSubmit={(name) => onRenameBuild(overlay.buildId, name)}
          onClose={onCloseOverlay}
        />
      )}
      {overlay?.kind === 'tags' && (
        <TagsOverlay
          initial={overlay.current}
          onSave={(tags) => onSaveTags(overlay.buildId, tags)}
          onClose={onCloseOverlay}
        />
      )}
      {overlay?.kind === 'move' && (
        <MoveToFolderOverlay
          folders={folders}
          currentFolderId={overlay.current}
          onMove={(folderId) => onMove(overlay.buildId, folderId)}
          onClose={onCloseOverlay}
        />
      )}
      {overlay?.kind === 'newFolder' && (
        <TextPromptOverlay
          section="Organise"
          title="New folder"
          label="Folder name"
          placeholder="e.g. Season 7"
          submitLabel="Create"
          onSubmit={(name) => onCreateFolder(name, overlay.parentId)}
          onClose={onCloseOverlay}
        />
      )}
      {overlay?.kind === 'renameFolder' && (
        <TextPromptOverlay
          section="Organise"
          title="Rename folder"
          label="New name"
          initial={overlay.current}
          submitLabel="Save"
          onSubmit={(name) => onRenameFolder(overlay.folderId, name)}
          onClose={onCloseOverlay}
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
              <span className="text-accent-hot">{overlay.name}</span> and all its
              profiles? This cannot be undone.
            </>
          }
          onConfirm={() => onDeleteBuild(overlay.buildId)}
          onClose={onCloseOverlay}
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
          onConfirm={() => onDeleteFolder(overlay.folderId)}
          onClose={onCloseOverlay}
        />
      )}

      {overlay?.kind === 'addProfile' && (
        <TextPromptOverlay
          section="Profiles"
          title="New profile"
          label="Profile name"
          placeholder="e.g. Boss setup"
          submitLabel="Create"
          hint="Seeded from this build's active profile."
          onSubmit={(name) => onAddProfile(overlay.buildId, name)}
          onClose={onCloseOverlay}
        />
      )}
      {overlay?.kind === 'renameProfile' && (
        <TextPromptOverlay
          section="Profiles"
          title="Rename profile"
          label="New name"
          initial={overlay.current}
          submitLabel="Save"
          onSubmit={(name) =>
            onRenameProfile(overlay.buildId, overlay.profileId, name)
          }
          onClose={onCloseOverlay}
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
              <span className="text-accent-hot">{overlay.name}</span>? This cannot
              be undone.
            </>
          }
          onConfirm={() => onDeleteProfile(overlay.buildId, overlay.profileId)}
          onClose={onCloseOverlay}
        />
      )}
    </>
  )
}
