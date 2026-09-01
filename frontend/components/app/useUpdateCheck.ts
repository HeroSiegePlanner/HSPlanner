import { createContext, useCallback, useContext, useEffect, useRef, useState } from 'react'
import {
  BOOT_CHECK_DELAY_MS,
  markChecked,
  shouldAutoCheck,
} from '../../utils/updatePrefs'
import {
  GITHUB_REPO,
  UpdateCheckError,
  checkForUpdate,
  isMockEnabled,
  type UpdateInfo,
} from '../../utils/version'

const UP_TO_DATE_MS = 4000
const CHECK_FAILED_MS = 5000

export type CheckState =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'ok'; info: UpdateInfo }
  | { kind: 'available'; info: UpdateInfo }
  | { kind: 'error'; message: string }

export interface UpdateCheck {
  check: CheckState
  hasRepo: boolean
  onCheck: () => void
  /**
   * When a check last completed, in memory. Not the same as the persisted
   * cooldown: a check that finds a pending update deliberately leaves the
   * cooldown alone, and reporting that as "never checked" would be a lie.
   */
  lastCheckedAt: number | null
}

/**
 * Owns the automatic check at boot and the manual one behind the buttons.
 * Mounted once at the root so the planner's bottom bar, the library footer and
 * the settings modal all read the same state and share a single check.
 */
export function useUpdateCheck(): UpdateCheck {
  const [check, setCheck] = useState<CheckState>({ kind: 'idle' })
  const [lastCheckedAt, setLastCheckedAt] = useState<number | null>(null)
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

  const scheduleRevert = useCallback((delayMs: number) => {
    if (transientTimer.current !== null)
      window.clearTimeout(transientTimer.current)
    transientTimer.current = window.setTimeout(
      () => setCheck({ kind: 'idle' }),
      delayMs,
    )
  }, [])

  const runCheck = useCallback(
    async (silent: boolean) => {
      if (checkRef.current.kind === 'checking') return
      if (transientTimer.current !== null) {
        window.clearTimeout(transientTimer.current)
        transientTimer.current = null
      }
      abortRef.current?.abort()
      const ctrl = new AbortController()
      abortRef.current = ctrl
      if (!silent) setCheck({ kind: 'checking' })
      try {
        const info = await checkForUpdate(ctrl.signal)
        if (ctrl.signal.aborted) return
        setLastCheckedAt(Date.now())
        // while an update is pending, keep re-checking every boot — the badge
        // lives in memory, so a restart would otherwise hide it for the rest
        // of the cooldown
        if (!info.hasUpdate) markChecked()
        if (info.hasUpdate) {
          setCheck({ kind: 'available', info })
          return
        }
        // nothing to act on: an automatic check stays invisible
        if (silent) return
        setCheck({ kind: 'ok', info })
        scheduleRevert(UP_TO_DATE_MS)
      } catch (err) {
        if (ctrl.signal.aborted) return
        // being offline at boot is not a failure the user asked to hear about
        if (silent) return
        const message =
          err instanceof UpdateCheckError
            ? err.message
            : err instanceof Error
              ? err.message
              : 'Check failed'
        setCheck({ kind: 'error', message })
        scheduleRevert(CHECK_FAILED_MS)
      }
    },
    [scheduleRevert],
  )

  const hasRepo = GITHUB_REPO.length > 0 || isMockEnabled()

  const onCheck = useCallback(() => {
    void runCheck(false)
  }, [runCheck])

  // the app notices releases on its own; a manual check in flight or a state
  // already on screen wins over the automatic one
  useEffect(() => {
    if (!hasRepo || !shouldAutoCheck()) return
    const timer = window.setTimeout(() => {
      if (checkRef.current.kind !== 'idle') return
      void runCheck(true)
    }, BOOT_CHECK_DELAY_MS)
    return () => window.clearTimeout(timer)
  }, [hasRepo, runCheck])

  return { check, hasRepo, onCheck, lastCheckedAt }
}

export const UpdateContext = createContext<UpdateCheck | null>(null)

/** Reads the single app-wide check. Falls back to an inert state when no
 *  provider is mounted, so isolated component tests stay renderable. */
export function useUpdate(): UpdateCheck {
  return (
    useContext(UpdateContext) ?? {
      check: { kind: 'idle' },
      hasRepo: false,
      onCheck: () => {},
      lastCheckedAt: null,
    }
  )
}
