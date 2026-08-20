export function classColor(classId: string | null): string {
  if (!classId) return 'var(--color-faint)'
  let hash = 0
  for (let i = 0; i < classId.length; i++) {
    hash = (hash * 31 + classId.charCodeAt(i)) | 0
  }
  const hue = Math.abs(hash) % 360
  return `hsl(${hue} 58% 58%)`
}

export function classInitial(className: string | undefined): string {
  return (className?.[0] ?? '?').toUpperCase()
}

const KNOWN_TAG_TONES: Record<string, string> = {
  hardcore: 'text-stat-red',
  hc: 'text-stat-red',
  ssf: 'text-stat-orange',
  softcore: 'text-stat-green',
  endgame: 'text-accent-hot',
  starter: 'text-stat-blue',
  draft: 'text-faint',
}

export function tagTone(tag: string): string {
  return KNOWN_TAG_TONES[tag.trim().toLowerCase()] ?? 'text-muted'
}

export function stripHtml(html: string): string {
  if (!html) return ''
  if (typeof document === 'undefined') return html.replace(/<[^>]*>/g, ' ')
  const el = document.createElement('div')
  el.innerHTML = html
  return (el.textContent ?? '').replace(/\s+/g, ' ').trim()
}

export function approxKB(value: unknown): string {
  try {
    const bytes = JSON.stringify(value).length
    return `${Math.max(1, Math.round(bytes / 1024))} KB`
  } catch {
    return '— KB'
  }
}
