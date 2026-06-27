import type { ReactNode } from 'react'
import type { SavedBuild } from '../../utils/build/savedBuilds'
import type { Scope } from './FolderTree'
import type { Overlay } from './buildSelectTypes'
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

interface BuildSelectToolbarProps {
  scope: Scope
  search: string
  selectedBuild: SavedBuild | null
  onSearchChange: (value: string) => void
  onNewBuild: () => void
  onOverlay: (overlay: Overlay) => void
  onCopy: (buildId: string) => void
}

export function BuildSelectToolbar({
  scope,
  search,
  selectedBuild,
  onSearchChange,
  onNewBuild,
  onOverlay,
  onCopy,
}: BuildSelectToolbarProps) {
  return (
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
        onClick={() => onOverlay({ kind: 'import' })}
      />
      <ToolButton
        label="Save…"
        icon={<SaveIcon className="h-3.5 w-3.5" />}
        onClick={() => onOverlay({ kind: 'save' })}
      />
      <ToolSep />
      <ToolButton
        label="Copy"
        icon={<CopyIcon className="h-3.5 w-3.5" />}
        disabled={!selectedBuild}
        onClick={() => selectedBuild && onCopy(selectedBuild.id)}
      />
      <ToolButton
        label="Rename"
        icon={<RenameIcon className="h-3.5 w-3.5" />}
        disabled={!selectedBuild}
        onClick={() =>
          selectedBuild &&
          onOverlay({
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
          onOverlay({
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
          onOverlay({
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
          onChange={(e) => onSearchChange(e.target.value)}
          placeholder="Search builds, classes, tags…"
          className="min-w-0 flex-1 bg-transparent text-[12px] text-text outline-none placeholder:text-faint"
        />
      </div>
    </nav>
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
          danger ? 'text-accent group-hover:text-stat-red' : 'text-accent'
        }`}
      >
        {icon}
      </span>
      {label}
    </button>
  )
}
