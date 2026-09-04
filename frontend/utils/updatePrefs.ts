import { readStorage, removeStorage, writeStorage } from './storage'

const AUTO_CHECK_KEY = 'hsplanner.update.auto_check'
const LAST_CHECK_KEY = 'hsplanner.update.last_check'

/** Written by controls that no longer exist; nothing reads them, so boot sweeps them once. */
const RETIRED_KEYS = [
  'hsplanner.update.auto_install',
  'hsplanner.update.skipped_version',
]

export function pruneRetiredUpdateKeys(): void {
  for (const key of RETIRED_KEYS) removeStorage(key)
}

/** Floor between automatic checks — GitHub rate-limits per IP. A manual check ignores it. */
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
