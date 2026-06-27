import { type ReactNode } from 'react'
import type { Folder } from '../../utils/build/savedBuilds'
import { CaretIcon, PlusIcon } from './icons'

export type Scope =
  | { kind: 'recent' }
  | { kind: 'all' }
  | { kind: 'favorites' }
  | { kind: 'unfiled' }
  | { kind: 'folder'; id: string }

export interface SmartCounts {
  recent: number
  all: number
  favorites: number
  unfiled: number
}

interface FolderTreeProps {
  childFolders: Record<string, Folder[]>
  scope: Scope
  onScopeChange: (scope: Scope) => void
  smartCounts: SmartCounts
  folderCounts: Record<string, number>
  expanded: Set<string>
  onToggleExpand: (folderId: string) => void
  onNewFolder: () => void
  onFolderContextMenu: (e: React.MouseEvent, folder: Folder) => void
  footer: ReactNode
}

function FolderIcon({ filled }: { filled?: boolean }) {
  return (
    <svg className="h-3.5 w-3.5 shrink-0" viewBox="0 0 16 16" aria-hidden>
      {filled && (
        <path fill="currentColor" d="M2 4h5l1 1.5h6v7.5H2z" opacity="0.35" />
      )}
      <path
        fill="none"
        stroke="currentColor"
        strokeWidth="1"
        d="M2 4h5l1 1.5h6v7.5H2z"
      />
    </svg>
  )
}

function StarIcon() {
  return (
    <svg className="h-3.5 w-3.5 shrink-0" viewBox="0 0 16 16" aria-hidden>
      <path
        fill="currentColor"
        d="M8 2 L10 6 L14 6.4 L11 9.4 L12 13.5 L8 11.2 L4 13.5 L5 9.4 L2 6.4 L6 6 Z"
      />
    </svg>
  )
}

function Row({
  icon,
  label,
  count,
  active,
  indent = 0,
  twist,
  onClick,
  onTwist,
  onContextMenu,
}: {
  icon: ReactNode
  label: string
  count: number
  active: boolean
  indent?: number
  twist?: 'open' | 'closed'
  onClick: () => void
  onTwist?: () => void
  onContextMenu?: (e: React.MouseEvent) => void
}) {
  return (
    <div className="px-1.5">
      <button
        type="button"
        onClick={onClick}
        onContextMenu={onContextMenu}
        className={`group relative flex w-full items-center gap-2 rounded-[3px] py-[5px] pr-2 text-left text-[12px] transition-colors ${
          active
            ? 'text-accent-hot'
            : 'text-muted hover:bg-white/[0.025] hover:text-text'
        }`}
        style={{
          paddingLeft: 8 + indent * 16,
          background: active
            ? 'linear-gradient(90deg, rgba(201,165,90,0.10), transparent)'
            : undefined,
          borderLeft: active
            ? '2px solid var(--color-accent-hot)'
            : '2px solid transparent',
        }}
      >
        {twist ? (
          <span
            role="button"
            tabIndex={-1}
            onClick={(e) => {
              e.stopPropagation()
              onTwist?.()
            }}
            className="flex h-3 w-3 shrink-0 items-center justify-center text-faint transition-transform"
            style={{ transform: twist === 'open' ? 'rotate(90deg)' : undefined }}
          >
            <CaretIcon className="h-2.5 w-2.5" />
          </span>
        ) : (
          <span className="w-3 shrink-0" />
        )}
        <span
          className={active ? 'text-accent' : 'text-faint'}
          aria-hidden
        >
          {icon}
        </span>
        <span className="flex-1 truncate">{label}</span>
        <span
          className={`font-mono text-[10px] tracking-[0.04em] tabular-nums ${
            active ? 'text-accent-deep' : 'text-faint'
          }`}
        >
          {count}
        </span>
      </button>
    </div>
  )
}

function GroupLabel({ children }: { children: ReactNode }) {
  return (
    <div className="px-3.5 pb-1 pt-2 font-mono text-[10px] font-semibold uppercase tracking-[0.18em] text-faint">
      {children}
    </div>
  )
}

export function FolderTree({
  childFolders,
  scope,
  onScopeChange,
  smartCounts,
  folderCounts,
  expanded,
  onToggleExpand,
  onNewFolder,
  onFolderContextMenu,
  footer,
}: FolderTreeProps) {
  const renderFolders = (parentKey: string, depth: number): ReactNode => {
    const children = childFolders[parentKey] ?? []
    return children.map((folder) => {
      const isOpen = expanded.has(folder.id)
      const hasChildren = (childFolders[folder.id] ?? []).length > 0
      return (
        <div key={folder.id}>
          <Row
            icon={<FolderIcon filled />}
            label={folder.name}
            count={folderCounts[folder.id] ?? 0}
            active={scope.kind === 'folder' && scope.id === folder.id}
            indent={depth}
            twist={hasChildren ? (isOpen ? 'open' : 'closed') : undefined}
            onClick={() => onScopeChange({ kind: 'folder', id: folder.id })}
            onTwist={() => onToggleExpand(folder.id)}
            onContextMenu={(e) => onFolderContextMenu(e, folder)}
          />
          {isOpen && renderFolders(folder.id, depth + 1)}
        </div>
      )
    })
  }

  const topLevelFolders = childFolders[''] ?? []

  return (
    <aside
      className="flex min-h-0 flex-col border-r border-border"
      style={{ background: 'var(--color-panel)' }}
    >
      <div className="flex shrink-0 items-center justify-between px-3.5 pb-2 pt-3">
        <span className="flex items-center gap-2 font-mono text-[11px] uppercase tracking-[0.16em] text-accent-hot">
          <span className="text-[10px] text-accent">◆</span>
          Library
        </span>
        <button
          type="button"
          onClick={onNewFolder}
          title="New folder"
          className="flex h-[18px] w-[18px] items-center justify-center rounded-[2px] text-faint transition-colors hover:bg-panel-2 hover:text-accent-hot"
        >
          <PlusIcon className="h-3 w-3" />
        </button>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto pb-2">
        <GroupLabel>Smart</GroupLabel>
        <Row
          icon={<FolderIcon />}
          label="Recent"
          count={smartCounts.recent}
          active={scope.kind === 'recent'}
          onClick={() => onScopeChange({ kind: 'recent' })}
        />
        <Row
          icon={<FolderIcon filled />}
          label="All Builds"
          count={smartCounts.all}
          active={scope.kind === 'all'}
          onClick={() => onScopeChange({ kind: 'all' })}
        />
        <Row
          icon={<StarIcon />}
          label="Favorites"
          count={smartCounts.favorites}
          active={scope.kind === 'favorites'}
          onClick={() => onScopeChange({ kind: 'favorites' })}
        />
        <Row
          icon={<FolderIcon />}
          label="Unfiled"
          count={smartCounts.unfiled}
          active={scope.kind === 'unfiled'}
          onClick={() => onScopeChange({ kind: 'unfiled' })}
        />

        <GroupLabel>Folders</GroupLabel>
        {topLevelFolders.length === 0 ? (
          <div className="px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
            No folders
          </div>
        ) : (
          renderFolders('', 0)
        )}
      </div>

      <div className="shrink-0 border-t border-border px-3.5 py-2.5 font-mono text-[10px] leading-relaxed tracking-[0.04em] text-muted">
        {footer}
      </div>
    </aside>
  )
}
