import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  extractGistId,
  fetchBuildCodeFromGist,
  isGistReference,
  isGistSharingConfigured,
  uploadBuildToGist,
} from './gistShare'

afterEach(() => {
  vi.unstubAllEnvs()
  vi.unstubAllGlobals()
})

const HEX = 'a'.repeat(32)

describe('isGistSharingConfigured', () => {
  it('is false when the token env var is empty', () => {
    vi.stubEnv('VITE_GIST_TOKEN', '')
    expect(isGistSharingConfigured()).toBe(false)
  })

  it('is true when the token env var is set', () => {
    vi.stubEnv('VITE_GIST_TOKEN', 'ghp_test')
    expect(isGistSharingConfigured()).toBe(true)
  })
})

describe('extractGistId / isGistReference', () => {
  it('accepts a bare hex id', () => {
    expect(extractGistId(HEX)).toBe(HEX)
    expect(isGistReference(HEX)).toBe(true)
  })

  it('extracts the id from a gist.github.com URL', () => {
    expect(extractGistId(`https://gist.github.com/someuser/${HEX}`)).toBe(HEX)
  })

  it('extracts the id from a raw gist URL', () => {
    expect(
      extractGistId(`https://gist.githubusercontent.com/u/${HEX}/raw/abc/file`),
    ).toBe(HEX)
  })

  it('rejects non-gist hosts', () => {
    expect(extractGistId(`https://example.com/${HEX}`)).toBeNull()
    expect(isGistReference(`https://example.com/${HEX}`)).toBe(false)
  })

  it('rejects an arbitrary build code', () => {
    expect(isGistReference('N4Ig1234-not-a-gist+code$$')).toBe(false)
  })
})

function jsonResponse(status: number, body: unknown): Response {
  return { status, json: async () => body } as unknown as Response
}

describe('uploadBuildToGist', () => {
  it('throws no-token when the token is missing', async () => {
    vi.stubEnv('VITE_GIST_TOKEN', '')
    await expect(uploadBuildToGist('CODE')).rejects.toMatchObject({
      kind: 'no-token',
    })
  })

  it('posts a secret .hsp gist and returns the html_url', async () => {
    vi.stubEnv('VITE_GIST_TOKEN', 'ghp_test')
    const fetchMock = vi
      .fn()
      .mockResolvedValue(
        jsonResponse(201, { id: 'abc123', html_url: 'https://gist.github.com/u/abc123' }),
      )
    vi.stubGlobal('fetch', fetchMock)

    const result = await uploadBuildToGist('CODE', { className: 'amazon', level: 50 })

    expect(result).toEqual({ id: 'abc123', url: 'https://gist.github.com/u/abc123' })
    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toBe('https://api.github.com/gists')
    expect(init.method).toBe('POST')
    expect(init.headers.Authorization).toBe('Bearer ghp_test')
    const body = JSON.parse(init.body)
    expect(body.public).toBe(false)
    expect(body.files['hsplanner-build.hsp'].content).toBe('CODE')
  })

  it('maps 401 to auth and 403 to rate-limit', async () => {
    vi.stubEnv('VITE_GIST_TOKEN', 'ghp_test')
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(jsonResponse(401, {})))
    await expect(uploadBuildToGist('CODE')).rejects.toMatchObject({ kind: 'auth' })

    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(jsonResponse(403, {})))
    await expect(uploadBuildToGist('CODE')).rejects.toMatchObject({ kind: 'rate-limit' })
  })

  it('maps a network failure to network', async () => {
    vi.stubEnv('VITE_GIST_TOKEN', 'ghp_test')
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('offline')))
    await expect(uploadBuildToGist('CODE')).rejects.toMatchObject({ kind: 'network' })
  })
})

describe('fetchBuildCodeFromGist', () => {
  it('returns the .hsp file content', async () => {
    vi.stubEnv('VITE_GIST_TOKEN', '')
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(
        jsonResponse(200, {
          files: { 'hsplanner-build.hsp': { content: 'BUILDCODE', truncated: false } },
        }),
      ),
    )
    await expect(fetchBuildCodeFromGist(`https://gist.github.com/u/${HEX}`)).resolves.toBe(
      'BUILDCODE',
    )
  })

  it('falls back to the first file when the named file is absent', async () => {
    vi.stubEnv('VITE_GIST_TOKEN', '')
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(
        jsonResponse(200, { files: { 'other.txt': { content: 'X', truncated: false } } }),
      ),
    )
    await expect(fetchBuildCodeFromGist(HEX)).resolves.toBe('X')
  })

  it('throws too-large when the file is truncated', async () => {
    vi.stubEnv('VITE_GIST_TOKEN', '')
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(
        jsonResponse(200, {
          files: { 'hsplanner-build.hsp': { content: 'X', truncated: true } },
        }),
      ),
    )
    await expect(fetchBuildCodeFromGist(HEX)).rejects.toMatchObject({ kind: 'too-large' })
  })

  it('throws not-found on 404', async () => {
    vi.stubEnv('VITE_GIST_TOKEN', '')
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(jsonResponse(404, {})))
    await expect(fetchBuildCodeFromGist(HEX)).rejects.toMatchObject({ kind: 'not-found' })
  })

  it('throws not-found for input that is not a gist', async () => {
    await expect(fetchBuildCodeFromGist('https://example.com/x')).rejects.toMatchObject({
      kind: 'not-found',
    })
  })
})
