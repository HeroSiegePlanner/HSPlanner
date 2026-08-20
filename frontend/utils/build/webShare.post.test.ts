import { describe, expect, it, vi, beforeEach } from 'vitest'
import { SHARE_PAYLOAD_FIXTURE } from './sharePayload.fixture'

const fetchMock = vi.fn()
vi.stubGlobal('fetch', fetchMock)

describe('postWebShare', () => {
  beforeEach(() => {
    fetchMock.mockReset()
    vi.stubEnv('VITE_WEB_SHARE_CREATE_URL', 'https://create-build.example.appwrite.run')
  })

  it(
    'POSTs the payload and returns id/url on 201',
    async () => {
      fetchMock.mockResolvedValue({
        status: 201,
        json: async () => ({ id: 'XK3FQ2', url: 'https://hsplanner.app/b/XK3FQ2' }),
      })
      const { postWebShare } = await import('./webShare')
      const result = await postWebShare(SHARE_PAYLOAD_FIXTURE)
      expect(result).toEqual({ id: 'XK3FQ2', url: 'https://hsplanner.app/b/XK3FQ2' })
      expect(fetchMock).toHaveBeenCalledWith(
        'https://create-build.example.appwrite.run',
        expect.objectContaining({ method: 'POST' }),
      )
    },
    20000,
  )

  it('throws a validation WebShareError on 400', async () => {
    fetchMock.mockResolvedValue({ status: 400, json: async () => ({ error: 'invalid_payload' }) })
    const { postWebShare, WebShareError } = await import('./webShare')
    await expect(postWebShare(SHARE_PAYLOAD_FIXTURE)).rejects.toThrow(WebShareError)
  })

  it('throws a too-large WebShareError on 413', async () => {
    fetchMock.mockResolvedValue({ status: 413, json: async () => ({ error: 'snapshot_too_large' }) })
    const { postWebShare, WebShareError } = await import('./webShare')
    try {
      await postWebShare(SHARE_PAYLOAD_FIXTURE)
      expect.unreachable()
    } catch (e) {
      expect(e).toBeInstanceOf(WebShareError)
      expect((e as InstanceType<typeof WebShareError>).kind).toBe('too-large')
    }
  })

  it('throws a server WebShareError on 500', async () => {
    fetchMock.mockResolvedValue({ status: 500, json: async () => ({ error: 'create_failed' }) })
    const { postWebShare, WebShareError } = await import('./webShare')
    try {
      await postWebShare(SHARE_PAYLOAD_FIXTURE)
      expect.unreachable()
    } catch (e) {
      expect(e).toBeInstanceOf(WebShareError)
      expect((e as InstanceType<typeof WebShareError>).kind).toBe('server')
    }
  })

  it('throws a network WebShareError when fetch rejects', async () => {
    fetchMock.mockRejectedValue(new Error('offline'))
    const { postWebShare, WebShareError } = await import('./webShare')
    await expect(postWebShare(SHARE_PAYLOAD_FIXTURE)).rejects.toThrow(WebShareError)
  })
})
