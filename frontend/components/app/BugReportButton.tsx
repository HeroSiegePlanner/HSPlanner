import { useState } from 'react'
import { activeSeasonId, getClass } from '@data'
import { useBuild } from '../../store/build'
import { encodeBuildToShare } from '../../utils/build/shareBuild'
import BugReportModal, { type BugReportModalProps } from './BugReportModal'

type OpenState = Omit<BugReportModalProps, 'onClose'>

export default function BugReportButton() {
  const exportSnapshot = useBuild((s) => s.exportBuildSnapshot)
  const [open, setOpen] = useState<OpenState | null>(null)

  const onOpen = () => {
    if (open) {
      setOpen(null)
      return
    }
    const { notes } = useBuild.getState()
    const snap = exportSnapshot()
    const className = snap.classId ? (getClass(snap.classId)?.name ?? snap.classId) : null
    setOpen({
      buildCode: encodeBuildToShare(snap, notes),
      buildLabel: className ? `${className} ${snap.level} · ${activeSeasonId}` : null,
    })
  }

  return (
    <>
      <button
        type="button"
        onClick={onOpen}
        title="Report a bug, wrong data or an idea"
        className="inline-flex items-center gap-1.5 rounded-[3px] border border-border-2 bg-panel-2 px-2 py-0.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
      >
        <BugIcon className="h-3 w-3" />
        Report a bug
      </button>

      {open && <BugReportModal {...open} onClose={() => setOpen(null)} />}
    </>
  )
}

function BugIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
      className={className}
    >
      <path d="M8 6a4 4 0 0 1 8 0v1H8V6z" />
      <rect x="8" y="7" width="8" height="12" rx="4" />
      <line x1="3" y1="11" x2="6" y2="11" />
      <line x1="18" y1="11" x2="21" y2="11" />
      <line x1="4" y1="17" x2="7" y2="15" />
      <line x1="20" y1="17" x2="17" y2="15" />
    </svg>
  )
}
