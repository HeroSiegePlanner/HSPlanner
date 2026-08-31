import { APP_VERSION, BUILD_CHANNEL } from './version'

export type BugReportKind = 'bug' | 'data' | 'idea'

export type BugReportErrorKind =
  | 'not-configured'
  | 'validation'
  | 'rate-limited'
  | 'server'
  | 'network'
  | 'unknown'

export class BugReportError extends Error {
  readonly kind: BugReportErrorKind
  constructor(kind: BugReportErrorKind, message: string) {
    super(message)
    this.name = 'BugReportError'
    this.kind = kind
  }
}

export const MIN_TITLE_LENGTH = 3
export const MAX_TITLE_LENGTH = 100
export const MIN_DESCRIPTION_LENGTH = 10
export const MAX_DESCRIPTION_LENGTH = 1000
export const MAX_STEPS_LENGTH = 600
export const MAX_EXPECTED_LENGTH = 400
export const MAX_CONTACT_LENGTH = 80
export const MAX_BUILD_LABEL_LENGTH = 120
export const MAX_SCREENSHOTS = 3
export const MAX_SCREENSHOT_BYTES = 8 * 1024 * 1024

export const KIND_LABEL: Record<BugReportKind, string> = {
  bug: 'Bug report',
  data: 'Wrong data',
  idea: 'Idea or request',
}

// App accent palette, so the embed stripe reads at a glance in the channel.
const KIND_COLOR: Record<BugReportKind, number> = {
  bug: 0xd96b5a,
  data: 0xe0b864,
  idea: 0x74c98a,
}

// The build code is only ever pasted back into the planner, so an extension
// Discord does not preview keeps it a compact card instead of a wall of text.
const BUILD_FILE_NAME = 'build.hsp'

export interface BugReport {
  kind: BugReportKind
  title: string
  description: string
  steps?: string
  expected?: string
  contact?: string
  buildLabel?: string | null
  buildCode?: string | null
  screenshots?: File[]
}

interface EmbedField {
  name: string
  value: string
  inline?: boolean
}

interface ReportFile {
  name: string
  body: Blob
}

interface ReportPayload {
  embeds: {
    author: { name: string }
    title: string
    description: string
    color: number
    fields: EmbedField[]
    image?: { url: string }
    footer: { text: string }
    timestamp: string
  }[]
  attachments: { id: number; filename: string }[]
  allowed_mentions: { parse: [] }
}

function endpoint(): string | null {
  const raw = import.meta.env.VITE_BUG_REPORT_URL
  return typeof raw === 'string' && raw.trim() ? raw.trim() : null
}

export function isBugReportConfigured(): boolean {
  return endpoint() !== null
}

function platformLabel(): string {
  const ua = typeof navigator === 'undefined' ? '' : navigator.userAgent
  if (/Windows NT 1[01]/i.test(ua)) return 'Windows 10/11'
  if (/Windows/i.test(ua)) return 'Windows'
  if (/Mac OS X|Macintosh/i.test(ua)) return 'macOS'
  if (/Linux|X11/i.test(ua)) return 'Linux'
  return 'Unknown OS'
}

function clamp(value: string, max: number): string {
  const trimmed = value.trim()
  return trimmed.length > max ? `${trimmed.slice(0, max - 1)}…` : trimmed
}

export function screenshotName(file: File, index: number): string {
  const subtype = file.type.split('/')[1]?.replace(/[^a-z0-9]/gi, '') ?? ''
  const ext = subtype === 'jpeg' ? 'jpg' : subtype || 'png'
  return `shot-${index + 1}.${ext}`
}

// Screenshots first: the embed points its image at the first upload by name.
export function reportFiles(report: BugReport): ReportFile[] {
  const files: ReportFile[] = (report.screenshots ?? []).map((shot, i) => ({
    name: screenshotName(shot, i),
    body: shot,
  }))
  if (report.buildCode) {
    files.push({
      name: BUILD_FILE_NAME,
      body: new Blob([report.buildCode], { type: 'application/octet-stream' }),
    })
  }
  return files
}

export function buildReportPayload(report: BugReport, files = reportFiles(report)): ReportPayload {
  const fields: EmbedField[] = []
  if (report.steps?.trim()) {
    fields.push({ name: 'Steps to reproduce', value: clamp(report.steps, MAX_STEPS_LENGTH) })
  }
  if (report.expected?.trim()) {
    fields.push({ name: 'Expected instead', value: clamp(report.expected, MAX_EXPECTED_LENGTH) })
  }
  if (report.buildLabel?.trim()) {
    fields.push({
      name: 'Build',
      value: clamp(report.buildLabel, MAX_BUILD_LABEL_LENGTH),
      inline: true,
    })
  }
  if (report.contact?.trim()) {
    fields.push({ name: 'Contact', value: clamp(report.contact, MAX_CONTACT_LENGTH), inline: true })
  }

  const firstShot = (report.screenshots ?? [])[0]

  return {
    embeds: [
      {
        author: { name: KIND_LABEL[report.kind] },
        title: clamp(report.title, MAX_TITLE_LENGTH),
        description: clamp(report.description, MAX_DESCRIPTION_LENGTH),
        color: KIND_COLOR[report.kind],
        fields,
        ...(firstShot ? { image: { url: `attachment://${screenshotName(firstShot, 0)}` } } : {}),
        footer: { text: `HSPlanner v${APP_VERSION} · ${BUILD_CHANNEL} · ${platformLabel()}` },
        timestamp: new Date().toISOString(),
      },
    ],
    attachments: files.map((file, id) => ({ id, filename: file.name })),
    // parse: [] neutralises @everyone/@role pasted into any of the fields.
    allowed_mentions: { parse: [] },
  }
}

function assertValid(report: BugReport): void {
  if (report.title.trim().length < MIN_TITLE_LENGTH) {
    throw new BugReportError('validation', 'Please give the report a short title.')
  }
  if (report.description.trim().length < MIN_DESCRIPTION_LENGTH) {
    throw new BugReportError(
      'validation',
      `Please describe the problem in at least ${MIN_DESCRIPTION_LENGTH} characters.`,
    )
  }
  const shots = report.screenshots ?? []
  if (shots.length > MAX_SCREENSHOTS) {
    throw new BugReportError('validation', `At most ${MAX_SCREENSHOTS} screenshots.`)
  }
  for (const shot of shots) {
    if (!shot.type.startsWith('image/')) {
      throw new BugReportError('validation', `${shot.name} is not an image.`)
    }
    if (shot.size > MAX_SCREENSHOT_BYTES) {
      throw new BugReportError('validation', `${shot.name} is larger than 8 MB.`)
    }
  }
}

function errorFromStatus(status: number): BugReportError {
  if (status === 429) {
    return new BugReportError('rate-limited', 'Too many reports right now. Try again in a minute.')
  }
  if (status >= 500) {
    return new BugReportError('server', 'The report server had a problem. Try again shortly.')
  }
  return new BugReportError('unknown', `Sending the report failed (${status}).`)
}

export async function sendBugReport(report: BugReport): Promise<void> {
  const url = endpoint()
  if (!url) {
    throw new BugReportError('not-configured', 'Reporting is not configured in this build.')
  }
  assertValid(report)

  const files = reportFiles(report)
  const form = new FormData()
  form.append('payload_json', JSON.stringify(buildReportPayload(report, files)))
  files.forEach((file, i) => form.append(`files[${i}]`, file.body, file.name))

  let res: Response
  try {
    res = await fetch(url, { method: 'POST', body: form })
  } catch {
    throw new BugReportError('network', 'Network error. Check your connection.')
  }
  if (!res.ok) throw errorFromStatus(res.status)
}
