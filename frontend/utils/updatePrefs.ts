import { readStorage, writeStorage } from './storage'

const AUTO_CHECK_KEY = 'hsplanner.update.auto_check'
const LAST_CHECK_KEY = 'hsplanner.update.last_check'

/**
 * Floor between two automatic checks. A planner session is long-lived and the
 * GitHub API rate-limits unauthenticated calls per IP, so the boot check is
 * not something to fire on every window open. A manual check ignores this.
 */
export const AUTO_CHECK_COOLDOWN_MS = 6 * 60 * 60 * 1000

/** Boot is already loading data + engine; let it settle before hitting the network. */
export const BOOT_CHECK_DELAY_MS = 3000

/** On by default — an unattended planner should still notice a new release. */
export function isAutoCheckEnabled(): boolean {
  const raw = readStorage(AUTO_CHECK_KEY)
  if (raw === null) return true
  return raw === '1'
}

export function setAutoCheckEnabled(enabled: boolean): void {
  writeStorage(AUTO_CHECK_KEY, enabled ? '1' : '0')
}

export function readLastCheck(): number | null {
  const raw = readStorage(LAST_CHECK_KEY)
  if (!raw) return null
  const parsed = Number.parseInt(raw, 10)
  return Number.isFinite(parsed) && parsed > 0 ? parsed : null
}

export function markChecked(now: number = Date.now()): void {
  writeStorage(LAST_CHECK_KEY, String(now))
}

export function shouldAutoCheck(now: number = Date.now()): boolean {
  if (!isAutoCheckEnabled()) return false
  const last = readLastCheck()
  if (last === null) return true
  // a clock that jumped backwards would otherwise park the check forever
  if (last > now) return true
  return now - last >= AUTO_CHECK_COOLDOWN_MS
}
