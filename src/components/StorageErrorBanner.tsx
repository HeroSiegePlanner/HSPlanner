import { useBuild } from '../store/build'

export default function StorageErrorBanner() {
  // Fixed banner shown whenever the build store records a `storageError` — i.e.
  // a write to localStorage was rejected, most often because the browser's
  // origin storage quota is exhausted. Saved builds live only in localStorage,
  // so a failed save must be impossible to miss instead of silently lost.
  const storageError = useBuild((s) => s.storageError)
  const dismissStorageError = useBuild((s) => s.dismissStorageError)

  if (!storageError) return null

  return (
    <div
      role="alert"
      className="fixed inset-x-0 bottom-0 z-300 flex items-start gap-3 border-t border-stat-red/60 px-4 py-3"
      style={{
        background:
          'linear-gradient(180deg, color-mix(in srgb, var(--color-stat-red) 14%, var(--color-panel)), var(--color-panel-2))',
        boxShadow: '0 -12px 32px rgba(0,0,0,0.55)',
      }}
    >
      <svg
        aria-hidden
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth={2}
        strokeLinecap="round"
        strokeLinejoin="round"
        className="mt-0.5 h-4 w-4 shrink-0 text-stat-red"
      >
        <path d="M12 3 2 20h20L12 3Z" />
        <path d="M12 10v4" />
        <path d="M12 17h.01" />
      </svg>
      <div className="min-w-0 flex-1">
        <div className="font-mono text-[11px] uppercase tracking-[0.16em] text-stat-red">
          Save failed
        </div>
        <p className="mt-0.5 text-[13px] leading-relaxed text-muted">
          {storageError}
        </p>
      </div>
      <button
        type="button"
        onClick={dismissStorageError}
        aria-label="Dismiss"
        className="shrink-0 rounded-[3px] border border-border-2 bg-panel-2 px-3 py-1 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
      >
        Dismiss
      </button>
    </div>
  )
}
