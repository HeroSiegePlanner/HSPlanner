import { useState } from 'react'
import UpdateModal from './UpdateModal'
import type { UpdateInfo } from '../../utils/version'
import { useUpdate, type CheckState } from './useUpdateCheck'

/**
 * The update badge plus the modal behind it. Self-contained so any chrome that
 * wants it — the planner's bottom bar, the library footer — just renders it.
 */
export default function UpdateStatus() {
  const { check, hasRepo, onCheck } = useUpdate()
  const [modalOpen, setModalOpen] = useState(false)

  return (
    <>
      <UpdateBadge
        state={check}
        hasRepo={hasRepo}
        onCheck={onCheck}
        onOpenModal={() => setModalOpen(true)}
      />
      {modalOpen && check.kind === 'available' && (
        <UpdateModal info={check.info} onClose={() => setModalOpen(false)} />
      )}
    </>
  )
}

function UpdateBadge({
  state,
  hasRepo,
  onCheck,
  onOpenModal,
}: {
  state: CheckState
  hasRepo: boolean
  onCheck: () => void
  onOpenModal: () => void
}) {
  if (!hasRepo) {
    return (
      <button
        type="button"
        disabled
        title="Set GITHUB_REPO in src/utils/version.ts to enable update checks"
        className="cursor-not-allowed rounded-[3px] border border-border bg-panel-2/40 px-2 py-0.5 font-mono text-[10px] uppercase tracking-[0.14em] text-faint"
      >
        Check for updates
      </button>
    )
  }

  if (state.kind === 'checking') {
    return (
      <span className="inline-flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
        <span
          aria-hidden
          className="inline-block h-1 w-1 animate-pulse rotate-45 bg-faint"
        />
        Checking…
      </span>
    )
  }

  if (state.kind === 'ok') {
    return (
      <span className="inline-flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-stat-green">
        <span
          aria-hidden
          className="h-1.5 w-1.5 rounded-full bg-stat-green"
          style={{ boxShadow: '0 0 6px rgba(116,201,138,0.6)' }}
        />
        Up to date
      </span>
    )
  }

  if (state.kind === 'available') {
    return <UpdateAvailableButton info={state.info} onClick={onOpenModal} />
  }

  if (state.kind === 'error') {
    return (
      <span
        className="inline-flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-stat-red"
        title={state.message}
      >
        <span
          aria-hidden
          className="h-1.5 w-1.5 rounded-full bg-stat-red"
          style={{ boxShadow: '0 0 6px rgba(217,107,90,0.6)' }}
        />
        Check failed
      </span>
    )
  }

  return (
    <button
      type="button"
      onClick={onCheck}
      className="rounded-[3px] border border-border-2 bg-panel-2 px-2 py-0.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
    >
      Check
    </button>
  )
}

/** The one "vN available" affordance — the badge and Settings both render this. */
export function UpdateAvailableButton({
  info,
  onClick,
  padding = 'px-2 py-0.5',
}: {
  info: UpdateInfo
  onClick: () => void
  padding?: string
}) {
  const label = `v${info.latest} available`
  return (
    <button
      type="button"
      onClick={onClick}
      title={info.releaseName ?? label}
      className={`inline-flex items-center gap-1.5 rounded-[3px] border border-accent-deep ${padding} font-mono text-[10px] uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]`}
      style={{ background: 'linear-gradient(180deg, #3a2f1a, #2a2418)' }}
    >
      <span
        aria-hidden
        className="inline-block h-1 w-1 rotate-45 bg-accent-hot"
        style={{ boxShadow: '0 0 6px rgba(224,184,100,0.65)' }}
      />
      {label}
    </button>
  )
}
