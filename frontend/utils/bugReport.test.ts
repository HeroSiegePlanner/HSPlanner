import { beforeEach, describe, expect, it, vi } from 'vitest'

const fetchMock = vi.fn()
vi.stubGlobal('fetch', fetchMock)

const WEBHOOK = 'https://discord.com/api/webhooks/1/token'

async function load() {
  return import('./bugReport')
}

const png = (name: string, bytes = 10) =>
  new File([new Uint8Array(bytes)], name, { type: 'image/png' })

const base = {
  kind: 'bug' as const,
  title: 'Frost Nova shows 0 DPS',
  description: 'Frost Nova DPS reads zero with a full cold build',
}

function sentForm(): FormData {
  return (fetchMock.mock.calls[0] as [string, RequestInit])[1].body as FormData
}

function sentPayload(): {
  embeds: {
    author: { name: string }
    title: string
    description: string
    color: number
    fields: { name: string; value: string; inline?: boolean }[]
    image?: { url: string }
    footer: { text: string }
    timestamp: string
  }[]
  attachments: { id: number; filename: string }[]
  allowed_mentions: { parse: string[] }
} {
  return JSON.parse(sentForm().get('payload_json') as string) as ReturnType<typeof sentPayload>
}

describe('isBugReportConfigured', () => {
  it('returns false when the endpoint env var is empty', async () => {
    vi.stubEnv('VITE_BUG_REPORT_URL', '')
    const { isBugReportConfigured } = await load()
    expect(isBugReportConfigured()).toBe(false)
  })

  it('returns true when the endpoint env var is set', async () => {
    vi.stubEnv('VITE_BUG_REPORT_URL', WEBHOOK)
    const { isBugReportConfigured } = await load()
    expect(isBugReportConfigured()).toBe(true)
  })
})

describe('buildReportPayload', () => {
  it('puts the kind in the author line, the title and description in the embed', async () => {
    const { buildReportPayload } = await load()
    const [embed] = buildReportPayload(base).embeds
    expect(embed!.author.name).toBe('Bug report')
    expect(embed!.title).toBe('Frost Nova shows 0 DPS')
    expect(embed!.description).toBe('Frost Nova DPS reads zero with a full cold build')
    expect(embed!.footer.text).toMatch(/HSPlanner v\d+\.\d+\.\d+/)
    expect(Date.parse(embed!.timestamp)).not.toBeNaN()
  })

  it('colours the stripe differently per kind', async () => {
    const { buildReportPayload } = await load()
    const colorOf = (kind: 'bug' | 'data' | 'idea') =>
      buildReportPayload({ ...base, kind }).embeds[0]!.color
    expect(new Set([colorOf('bug'), colorOf('data'), colorOf('idea')]).size).toBe(3)
  })

  it('adds the steps and expected fields only when filled in', async () => {
    const { buildReportPayload } = await load()
    expect(buildReportPayload(base).embeds[0]!.fields).toEqual([])

    const fields = buildReportPayload({
      ...base,
      steps: '1. Go to Skills\n2. Rank Frost Nova to 20',
      expected: 'Hit DPS above zero',
      buildLabel: 'Necromancer 100 · s10',
      contact: 'zium',
    }).embeds[0]!.fields
    expect(fields.map((f) => f.name)).toEqual([
      'Steps to reproduce',
      'Expected instead',
      'Build',
      'Contact',
    ])
    expect(fields[0]!.value).toContain('2. Rank Frost Nova to 20')
  })

  it('references the first screenshot as the embed image', async () => {
    const { buildReportPayload } = await load()
    expect(buildReportPayload(base).embeds[0]!.image).toBeUndefined()
    const withShot = buildReportPayload({ ...base, screenshots: [png('Screenshot 2026.png')] })
    expect(withShot.embeds[0]!.image).toEqual({ url: 'attachment://shot-1.png' })
  })

  it('clamps every field to its cap', async () => {
    const { buildReportPayload, MAX_TITLE_LENGTH, MAX_DESCRIPTION_LENGTH, MAX_STEPS_LENGTH } =
      await load()
    const [embed] = buildReportPayload({
      kind: 'bug',
      title: 'x'.repeat(500),
      description: 'y'.repeat(4000),
      steps: 'z'.repeat(4000),
    }).embeds
    expect(embed!.title.length).toBeLessThanOrEqual(MAX_TITLE_LENGTH)
    expect(embed!.description.length).toBeLessThanOrEqual(MAX_DESCRIPTION_LENGTH)
    expect(embed!.fields[0]!.value.length).toBeLessThanOrEqual(MAX_STEPS_LENGTH)
  })
})

describe('screenshotName', () => {
  it('renames pasted images to a stable indexed name', async () => {
    const { screenshotName } = await load()
    expect(screenshotName(png('image.png'), 0)).toBe('shot-1.png')
    expect(screenshotName(png('image.png'), 1)).toBe('shot-2.png')
  })

  it('maps jpeg to a jpg extension', async () => {
    const { screenshotName } = await load()
    const jpeg = new File([new Uint8Array(4)], 'x', { type: 'image/jpeg' })
    expect(screenshotName(jpeg, 0)).toBe('shot-1.jpg')
  })
})

describe('sendBugReport', () => {
  beforeEach(() => {
    fetchMock.mockReset()
    vi.stubEnv('VITE_BUG_REPORT_URL', WEBHOOK)
  })

  it('throws a not-configured error when no endpoint is set', async () => {
    vi.stubEnv('VITE_BUG_REPORT_URL', '')
    const { sendBugReport, BugReportError } = await load()
    await expect(sendBugReport(base)).rejects.toThrow(BugReportError)
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it('rejects a missing title without calling fetch', async () => {
    const { sendBugReport } = await load()
    await expect(sendBugReport({ ...base, title: 'x' })).rejects.toMatchObject({
      kind: 'validation',
    })
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it('rejects a description shorter than the minimum without calling fetch', async () => {
    const { sendBugReport } = await load()
    await expect(sendBugReport({ ...base, description: 'nope' })).rejects.toMatchObject({
      kind: 'validation',
    })
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it('rejects a non-image attachment', async () => {
    const { sendBugReport } = await load()
    const notAnImage = new File(['x'], 'save.json', { type: 'application/json' })
    await expect(sendBugReport({ ...base, screenshots: [notAnImage] })).rejects.toMatchObject({
      kind: 'validation',
    })
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it('rejects a screenshot over the size cap', async () => {
    const { sendBugReport, MAX_SCREENSHOT_BYTES } = await load()
    const huge = png('huge.png', MAX_SCREENSHOT_BYTES + 1)
    await expect(sendBugReport({ ...base, screenshots: [huge] })).rejects.toMatchObject({
      kind: 'validation',
    })
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it('rejects more screenshots than the cap', async () => {
    const { sendBugReport, MAX_SCREENSHOTS } = await load()
    const shots = Array.from({ length: MAX_SCREENSHOTS + 1 }, (_, i) => png(`s${i}.png`))
    await expect(sendBugReport({ ...base, screenshots: shots })).rejects.toMatchObject({
      kind: 'validation',
    })
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it('POSTs the embed payload and suppresses mentions', async () => {
    fetchMock.mockResolvedValue({ ok: true, status: 204 })
    const { sendBugReport } = await load()
    await sendBugReport({ ...base, description: 'ping @everyone the DPS is zero' })

    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(url).toBe(WEBHOOK)
    expect(init.method).toBe('POST')
    const payload = sentPayload()
    expect(payload.allowed_mentions.parse).toEqual([])
    expect(payload.embeds[0]!.description).toContain('the DPS is zero')
    expect(sentForm().get('files[0]')).toBeNull()
  })

  it('attaches the build code under a non-previewable name', async () => {
    fetchMock.mockResolvedValue({ ok: true, status: 204 })
    const { sendBugReport } = await load()
    await sendBugReport({ ...base, buildCode: 'NoIgLgngDgpiBcICMD2AjA' })

    const file = sentForm().get('files[0]') as File
    expect(file.name).toBe('build.hsp')
    expect(await file.text()).toBe('NoIgLgngDgpiBcICMD2AjA')
  })

  it('uploads screenshots before the build file so the embed image resolves', async () => {
    fetchMock.mockResolvedValue({ ok: true, status: 204 })
    const { sendBugReport } = await load()
    await sendBugReport({
      ...base,
      buildCode: 'CODE',
      screenshots: [png('image.png'), png('image.png')],
    })

    const form = sentForm()
    expect((form.get('files[0]') as File).name).toBe('shot-1.png')
    expect((form.get('files[1]') as File).name).toBe('shot-2.png')
    expect((form.get('files[2]') as File).name).toBe('build.hsp')

    const payload = sentPayload()
    expect(payload.embeds[0]!.image).toEqual({ url: 'attachment://shot-1.png' })
    expect(payload.attachments).toEqual([
      { id: 0, filename: 'shot-1.png' },
      { id: 1, filename: 'shot-2.png' },
      { id: 2, filename: 'build.hsp' },
    ])
  })

  it('maps 429 to a rate-limited error', async () => {
    fetchMock.mockResolvedValue({ ok: false, status: 429 })
    const { sendBugReport } = await load()
    await expect(sendBugReport(base)).rejects.toMatchObject({ kind: 'rate-limited' })
  })

  it('maps 5xx to a server error', async () => {
    fetchMock.mockResolvedValue({ ok: false, status: 503 })
    const { sendBugReport } = await load()
    await expect(sendBugReport(base)).rejects.toMatchObject({ kind: 'server' })
  })

  it('maps a thrown fetch to a network error', async () => {
    fetchMock.mockRejectedValue(new TypeError('Failed to fetch'))
    const { sendBugReport } = await load()
    await expect(sendBugReport(base)).rejects.toMatchObject({ kind: 'network' })
  })
})
