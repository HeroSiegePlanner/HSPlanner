import { StorageWriteError } from '../utils/build/savedBuilds'

export function guardStorage<T>(
  reportError: (message: string) => void,
  fallback: T,
  body: () => T,
): T {
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
