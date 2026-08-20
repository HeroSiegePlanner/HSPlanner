export function readStorage(key: string): string | null {
  if (typeof window === 'undefined') return null
  try {
    return window.localStorage.getItem(key)
  } catch {
    return null
  }
}

export function readStorageWithLegacy(
  key: string,
  legacyKey: string,
): string | null {
  return readStorage(key) ?? readStorage(legacyKey)
}

export function writeStorage(key: string, value: string): boolean {
  if (typeof window === 'undefined') return false
  try {
    window.localStorage.setItem(key, value)
    return true
  } catch {
    return false
  }
}

export function removeStorage(key: string): void {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.removeItem(key)
  } catch (err) {
    void err
  }
}

export class StorageWriteError extends Error {
  constructor(message = 'Could not save to local storage — it may be full.') {
    super(message)
    this.name = 'StorageWriteError'
  }
}

export class StorageCapacityError extends StorageWriteError {
  constructor(message: string) {
    super(message)
    this.name = 'StorageCapacityError'
  }
}

export function newId(prefix: string): string {
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return crypto.randomUUID()
  }
  return `${prefix}_${Date.now().toString(36)}${Math.random()
    .toString(36)
    .slice(2, 8)}`
}
