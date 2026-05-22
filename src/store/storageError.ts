import { StorageWriteError } from '../utils/build/savedBuilds'

export function guardStorage<T>(
  reportError: (message: string) => void,
  fallback: T,
  body: () => T,
): T {
  // Runs a persisting build-store action. A StorageWriteError thrown by the
  // savedBuilds layer (e.g. the localStorage quota is exhausted) is converted
  // into a reported error message plus the supplied fallback return value, so
  // a full disk surfaces to the user instead of silently dropping their build.
  // Any other error is a genuine bug and propagates unchanged.
  try {
    return body()
  } catch (err) {
    if (err instanceof StorageWriteError) {
      reportError(err.message)
      return fallback
    }
    throw err
  }
}
