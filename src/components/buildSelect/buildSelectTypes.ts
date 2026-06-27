import type { Scope } from './FolderTree'

export const RECENT_LIMIT = 12

export interface BuildSelectProps {
  onOpenBuild: (buildId: string) => void
  onNewBuild: () => void
  onClose: () => void
  canClose: boolean
}

export type Overlay =
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

export interface CtxState {
  x: number
  y: number
  kind: 'build' | 'folder'
  id: string
}

export const SCOPE_LABEL: Record<Scope['kind'], string> = {
  recent: 'Recent',
  all: 'All Builds',
  favorites: 'Favorites',
  unfiled: 'Unfiled',
  folder: 'Folder',
}
