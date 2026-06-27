interface BuildSelectFooterProps {
  buildCount: number
  folderCount: number
  notice: string | null
  autoOpen: boolean
  onToggleAutoOpen: (checked: boolean) => void
}

export function BuildSelectFooter({
  buildCount,
  folderCount,
  notice,
  autoOpen,
  onToggleAutoOpen,
}: BuildSelectFooterProps) {
  return (
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
            <b className="font-medium text-text">{buildCount}</b> Builds ·{' '}
            <b className="font-medium text-text">{folderCount}</b> Folders
          </>
        )}
      </span>
      <span aria-hidden className="h-3.5 w-px bg-border" />
      <label className="flex cursor-pointer items-center gap-1.5 select-none text-muted">
        <input
          type="checkbox"
          checked={autoOpen}
          onChange={(e) => onToggleAutoOpen(e.target.checked)}
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
  )
}
