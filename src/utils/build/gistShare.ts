const GIST_API = 'https://api.github.com/gists'
const GIST_FILENAME = 'hsplanner-build.hsp'
const GIST_ID_RE = /^[0-9a-f]{20,40}$/i
const GIST_HOSTS = new Set(['gist.github.com', 'gist.githubusercontent.com'])

export type GistErrorKind =
  | 'no-token'
  | 'auth'
  | 'rate-limit'
  | 'not-found'
  | 'too-large'
  | 'network'
  | 'unknown'

export class GistShareError extends Error {
  readonly kind: GistErrorKind
  constructor(kind: GistErrorKind, message: string) {
    super(message)
    this.name = 'GistShareError'
    this.kind = kind
  }
}

export interface GistShareResult {
  id: string
  url: string
}

function getGistToken(): string | null {
  const raw = import.meta.env.VITE_GIST_TOKEN
  return typeof raw === 'string' && raw.trim() ? raw.trim() : null
}

export function isGistSharingConfigured(): boolean {
  return getGistToken() !== null
}

export function extractGistId(input: string): string | null {
  const trimmed = input.trim()
  if (GIST_ID_RE.test(trimmed)) return trimmed.toLowerCase()
  let url: URL
  try {
    url = new URL(trimmed)
  } catch {
    return null
  }
  if (!GIST_HOSTS.has(url.hostname)) return null
  const seg = url.pathname.split('/').find((s) => GIST_ID_RE.test(s))
  return seg ? seg.toLowerCase() : null
}

export function isGistReference(input: string): boolean {
  return extractGistId(input) !== null
}

function gistErrorFromStatus(status: number, op: 'upload' | 'fetch'): GistShareError {
  if (status === 401)
    return new GistShareError('auth', 'Gist token is invalid. Sharing is misconfigured.')
  if (status === 403)
    return new GistShareError('rate-limit', 'GitHub rate limit reached. Try again shortly.')
  if (status === 404)
    return new GistShareError('not-found', 'That Gist does not exist or was removed.')
  if (op === 'upload' && status === 422)
    return new GistShareError('unknown', 'GitHub rejected the gist.')
  return new GistShareError('unknown', `GitHub request failed (${status}).`)
}

export async function uploadBuildToGist(
  code: string,
  meta?: { className?: string; level?: number },
): Promise<GistShareResult> {
  const token = getGistToken()
  if (!token) {
    throw new GistShareError('no-token', 'Gist sharing is not configured in this build.')
  }
  const descParts = ['HSPlanner build']
  if (meta?.className) descParts.push(meta.className)
  if (typeof meta?.level === 'number') descParts.push(`L${meta.level}`)
  const payload = {
    description: descParts.join(' - '),
    public: false,
    files: { [GIST_FILENAME]: { content: code } },
  }
  let res: Response
  try {
    res = await fetch(GIST_API, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${token}`,
        Accept: 'application/vnd.github+json',
        'X-GitHub-Api-Version': '2022-11-28',
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(payload),
    })
  } catch {
    throw new GistShareError('network', 'Network error. Check your connection.')
  }
  if (res.status === 201) {
    const data = (await res.json()) as { id?: string; html_url?: string }
    if (!data.id || !data.html_url) {
      throw new GistShareError('unknown', 'Unexpected response from GitHub.')
    }
    return { id: data.id, url: data.html_url }
  }
  throw gistErrorFromStatus(res.status, 'upload')
}

export async function fetchBuildCodeFromGist(input: string): Promise<string> {
  const id = extractGistId(input)
  if (!id) {
    throw new GistShareError('not-found', 'That does not look like a Gist link.')
  }
  const token = getGistToken()
  const headers: Record<string, string> = {
    Accept: 'application/vnd.github+json',
    'X-GitHub-Api-Version': '2022-11-28',
  }
  if (token) headers.Authorization = `Bearer ${token}`
  let res: Response
  try {
    res = await fetch(`${GIST_API}/${id}`, { headers })
  } catch {
    throw new GistShareError('network', 'Network error. Check your connection.')
  }
  if (res.status !== 200) {
    throw gistErrorFromStatus(res.status, 'fetch')
  }
  const data = (await res.json()) as {
    files?: Record<string, { content?: string; truncated?: boolean } | null>
  }
  const files = data.files ?? {}
  const file = files[GIST_FILENAME] ?? Object.values(files).find((f) => f != null)
  if (!file || typeof file.content !== 'string') {
    throw new GistShareError('not-found', 'No build found in that Gist.')
  }
  if (file.truncated) {
    throw new GistShareError('too-large', 'Build is too large to import from this Gist.')
  }
  return file.content
}
