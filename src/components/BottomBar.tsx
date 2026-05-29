import { useEffect, useMemo, useRef, useState } from 'react'
import changelogMd from '../../CHANGELOG.md?raw'
import { useBuild } from '../store/build'
import {
  inTauriRuntime,
  installUpdateOnQuit,
} from '../utils/installUpdate'
import { getSavedBuild } from '../utils/build/savedBuilds'
import { readStorage } from '../utils/storage'
import {
  APP_VERSION,
  BUILD_CHANNEL,
  GITHUB_REPO,
  UpdateCheckError,
  checkForUpdate,
  isMockEnabled,
  type UpdateInfo,
} from '../utils/version'
import UpdateModal from './UpdateModal'

const AUTO_INSTALL_KEY = 'hsplanner.update.auto_install'

type CheckState =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'ok'; info: UpdateInfo }
  | { kind: 'available'; info: UpdateInfo }
  | { kind: 'error'; message: string }

export default function BottomBar() {
  const activeBuildId = useBuild((s) => s.activeBuildId)
  const savedBuildsVersion = useBuild((s) => s.savedBuildsVersion)
  const buildName = useMemo(() => {
    if (!activeBuildId) return 'Unnamed'
    const b = getSavedBuild(activeBuildId)
    return b?.name ?? 'Unnamed'
    // savedBuildsVersion is the store cache-buster: re-read on every store mutation.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeBuildId, savedBuildsVersion])

  const [check, setCheck] = useState<CheckState>({ kind: 'idle' })
  const [modalOpen, setModalOpen] = useState(false)
  const [changelogOpen, setChangelogOpen] = useState(false)

  const changelogInfo = useMemo<UpdateInfo>(
    () => ({
      current: APP_VERSION,
      latest: APP_VERSION,
      hasUpdate: false,
      body: changelogMd,
      releaseName: `HSPlanner v${APP_VERSION}`,
    }),
    [],
  )
  const abortRef = useRef<AbortController | null>(null)
  const transientTimer = useRef<number | null>(null)
  const checkRef = useRef<CheckState>(check)
  useEffect(() => {
    checkRef.current = check
  }, [check])

  useEffect(
    () => () => {
      abortRef.current?.abort()
      if (transientTimer.current !== null)
        window.clearTimeout(transientTimer.current)
    },
    [],
  )

  useEffect(() => {
    if (!inTauriRuntime()) return
    let unlisten: (() => void) | undefined
    let cancelled = false
    let quitting = false
    const QUIT_INSTALL_TIMEOUT_MS = 20000
    ;(async () => {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      if (cancelled) return
      const win = getCurrentWindow()
      unlisten = await win.onCloseRequested(async (event) => {
        if (quitting) return
        quitting = true
        event.preventDefault()

        const auto = readStorage(AUTO_INSTALL_KEY) === '1'
        const cur = checkRef.current
        if (auto && cur.kind === 'available') {
          try {
            await Promise.race([
              installUpdateOnQuit(),
              new Promise((_, reject) =>
                window.setTimeout(
                  () => reject(new Error('install-on-quit timeout')),
                  QUIT_INSTALL_TIMEOUT_MS,
                ),
              ),
            ])
          } catch {
            void 0
          }
        }

        const { exit } = await import('@tauri-apps/plugin-process')
        await exit(0)
      })
    })()
    return () => {
      cancelled = true
      unlisten?.()
    }
  }, [])

  const scheduleRevert = (delayMs: number) => {
    if (transientTimer.current !== null)
      window.clearTimeout(transientTimer.current)
    transientTimer.current = window.setTimeout(
      () => setCheck({ kind: 'idle' }),
      delayMs,
    )
  }

  const onCheck = async () => {
    if (check.kind === 'checking') return
    if (transientTimer.current !== null) {
      window.clearTimeout(transientTimer.current)
      transientTimer.current = null
    }
    abortRef.current?.abort()
    const ctrl = new AbortController()
    abortRef.current = ctrl
    setCheck({ kind: 'checking' })
    try {
      const info = await checkForUpdate(ctrl.signal)
      if (ctrl.signal.aborted) return
      if (info.hasUpdate) {
        setCheck({ kind: 'available', info })
      } else {
        setCheck({ kind: 'ok', info })
        scheduleRevert(4000)
      }
    } catch (err) {
      if (ctrl.signal.aborted) return
      const message =
        err instanceof UpdateCheckError
          ? err.message
          : err instanceof Error
            ? err.message
            : 'Check failed'
      setCheck({ kind: 'error', message })
      scheduleRevert(5000)
    }
  }

  const hasRepo = GITHUB_REPO.length > 0 || isMockEnabled()

  return (
    <footer
      className="flex h-9 shrink-0 items-center gap-2.5 border-t border-border px-3 text-[11px] text-muted"
      style={{
        background:
          'linear-gradient(180deg, var(--color-panel), var(--color-panel-2))',
        boxShadow:
          'inset 0 1px 0 rgba(201,165,90,0.08), 0 -1px 0 rgba(0,0,0,0.4)',
      }}
    >
      <span className="flex select-none items-center gap-1.5">
        <span
          aria-hidden
          className="inline-block h-1 w-1 rotate-45 bg-accent-deep"
          style={{ boxShadow: '0 0 6px rgba(138,111,58,0.5)' }}
        />
        <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-accent-deep">
          HSPlanner
        </span>
      </span>
      <span aria-hidden className="text-faint">
        ·
      </span>
      <button
        type="button"
        onClick={() => setChangelogOpen(true)}
        title="View changelog"
        className="cursor-pointer font-mono text-[10px] uppercase tracking-[0.14em] text-faint transition-colors hover:text-accent-hot"
      >
        v{APP_VERSION}
      </button>
      <BuildChannelBadge channel={BUILD_CHANNEL} />

      <span aria-hidden className="h-4 w-px bg-border" />

      <UpdateBadge
        state={check}
        hasRepo={hasRepo}
        onCheck={onCheck}
        onOpenModal={() => setModalOpen(true)}
      />

      {modalOpen && check.kind === 'available' && (
        <UpdateModal
          info={check.info}
          onClose={() => setModalOpen(false)}
          onSkipVersion={() => setCheck({ kind: 'idle' })}
        />
      )}

      {changelogOpen && (
        <UpdateModal
          info={changelogInfo}
          mode="changelog"
          onClose={() => setChangelogOpen(false)}
        />
      )}

      <a
        href="https://ko-fi.com/zium1337"
        target="_blank"
        rel="noopener noreferrer"
        title="Support HSPlanner on Ko-fi"
        onClick={(e) => {
          if (inTauriRuntime()) {
            e.preventDefault()
            void import('@tauri-apps/plugin-opener').then(({ openUrl }) =>
              openUrl('https://ko-fi.com/zium1337'),
            )
          }
        }}
        className="ml-auto inline-flex items-center gap-1.5 rounded-[3px] border border-accent-deep px-2.5 py-0.5 font-mono text-[10px] uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]"
        style={{
          background:
            'linear-gradient(180deg, rgba(58,46,24,0.5), rgba(42,36,24,0.35))',
        }}
      >
        <KofiIcon className="h-3 w-3" />
        Support on Ko-fi
      </a>
      <span className="flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-stat-green">
        <span
          aria-hidden
          className="h-1.5 w-1.5 rounded-full bg-stat-green"
          style={{ boxShadow: '0 0 8px rgba(116,201,138,0.65)' }}
        />
        Auto-saved
      </span>
      <span aria-hidden className="hidden h-4 w-px bg-border sm:block" />
      <span className="hidden items-center gap-1.5 sm:flex">
        <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
          Build
        </span>
        <span className="max-w-48 truncate font-mono text-[11px] text-accent-hot">
          {buildName}
        </span>
      </span>
    </footer>
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
    const label = `v${state.info.latest} available`
    return (
      <button
        type="button"
        onClick={onOpenModal}
        title={state.info.releaseName ?? label}
        className="inline-flex items-center gap-1.5 rounded-[3px] border border-accent-deep px-2 py-0.5 font-mono text-[10px] uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]"
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

function KofiIcon({ className }: { className?: string }) {
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
      <path d="M17 8h1a4 4 0 0 1 0 8h-1" />
      <path d="M3 8h14v9a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4V8z" />
      <line x1="6" y1="2" x2="6" y2="4" />
      <line x1="10" y1="2" x2="10" y2="4" />
      <line x1="14" y1="2" x2="14" y2="4" />
    </svg>
  )
}

function BuildChannelBadge({ channel }: { channel: 'dev' | 'stable' }) {
  const isDev = channel === 'dev'
  const label = isDev ? 'DEV' : 'STABLE'
  const className = isDev
    ? 'border-accent-deep/50 text-accent-hot'
    : 'border-stat-green/50 text-stat-green'
  const bg = isDev
    ? 'linear-gradient(180deg, rgba(58,46,24,0.6), rgba(42,36,24,0.4))'
    : 'linear-gradient(180deg, rgba(28,52,34,0.6), rgba(20,38,24,0.4))'
  return (
    <span
      title={isDev ? 'Development build' : 'Stable build'}
      className={`rounded-[3px] border px-1.5 py-px font-mono text-[9px] uppercase tracking-[0.18em] ${className}`}
      style={{ background: bg }}
    >
      {label}
    </span>
  )
}
