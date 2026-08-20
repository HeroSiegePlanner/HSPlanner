import { invoke } from '@tauri-apps/api/core'
import { BUILD_ID_RE } from './sharePayload'
import { decodeShareToBuild, type DecodedShare } from './shareBuild'
import { WebShareError } from './webShare'

export { WebShareError }

const DEEP_LINK_RE = /^hsp:\/\/b\/([^/?#]+)$/

export function parseDeepLinkUrl(url: string): string | null {
  const match = DEEP_LINK_RE.exec(url.trim())
  if (!match) return null
  const id = match[1]!
  return BUILD_ID_RE.test(id) ? id : null
}

function getBuildUrl(): string | null {
  const raw = import.meta.env.VITE_WEB_SHARE_GET_URL
  return typeof raw === 'string' && raw.trim() ? raw.trim() : null
}

export async function fetchSharedBuildCode(id: string): Promise<string> {
  const base = getBuildUrl()
  if (!base) throw new WebShareError('not-configured', 'Web sharing is not configured in this build.')

  let res: Response
  try {
    res = await fetch(`${base}?id=${id}`)
  } catch {
    throw new WebShareError('network', 'Network error. Check your connection.')
  }
  if (res.status === 404) throw new WebShareError('unknown', 'That build link does not exist or was removed.')
  if (res.status !== 200) throw new WebShareError('server', `Fetching the shared build failed (${res.status}).`)
  const data = (await res.json()) as { code?: string }
  if (!data.code) throw new WebShareError('unknown', 'Unexpected response from the share server.')
  return data.code
}

export async function handleDeepLinkUrls(
  urls: string[],
  onReady: (code: string, decoded: DecodedShare) => void,
  onError: (message: string) => void,
): Promise<void> {
  const id = urls.map(parseDeepLinkUrl).find((parsed): parsed is string => parsed !== null)
  if (!id) return
  try {
    const code = await fetchSharedBuildCode(id)
    const decoded = decodeShareToBuild(code)
    if (!decoded) {
      onError('That shared build could not be read.')
      return
    }
    onReady(code, decoded)
  } catch (e) {
    onError(e instanceof WebShareError ? e.message : 'Failed to open the shared build.')
  }
}

export async function getInitialDeepLinkUrls(): Promise<string[]> {
  try {
    const urls = await invoke<string[] | null>('plugin:deep-link|get_current')
    return urls ?? []
  } catch {
    return []
  }
}

export interface DeepLinkDispatcher {
  dispatchInitial: (urls: string[]) => Promise<void>
  dispatchLive: (urls: string[]) => Promise<void>
}

export function createDeepLinkDispatcher(
  onReady: (code: string, decoded: DecodedShare) => void,
  onError: (message: string) => void,
): DeepLinkDispatcher {
  let initialBatchKey: string | null = null

  return {
    dispatchInitial: async (urls) => {
      if (urls.length === 0) return
      initialBatchKey = JSON.stringify(urls)
      await handleDeepLinkUrls(urls, onReady, onError)
    },
    dispatchLive: async (urls) => {
      const key = JSON.stringify(urls)
      if (key === initialBatchKey) {
        initialBatchKey = null
        return
      }
      await handleDeepLinkUrls(urls, onReady, onError)
    },
  }
}
