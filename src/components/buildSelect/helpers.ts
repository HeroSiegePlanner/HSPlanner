// Shared presentation helpers for the Build Select library screen.

/** Deterministic per-class accent colour derived from the class id hash. */
export function classColor(classId: string | null): string {
  // Returns a stable HSL accent for a class id (used for the fallback letter
  // glyph and selection glow). Mirrors the hashing used by the old build
  // picker so colours stay consistent across the app.
  if (!classId) return '#5a5448'
  let hash = 0
  for (let i = 0; i < classId.length; i++) {
    hash = (hash * 31 + classId.charCodeAt(i)) | 0
  }
  const hue = Math.abs(hash) % 360
  return `hsl(${hue} 58% 58%)`
}

/** Uppercase first letter of a class name, used as the fallback glyph. */
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

/** Tailwind text-colour class for a tag chip, by known tag name. */
export function tagTone(tag: string): string {
  return KNOWN_TAG_TONES[tag.trim().toLowerCase()] ?? 'text-muted'
}

/** Human-friendly relative timestamp ("Today, 14:32", "May 08, 09:10"). */
export function formatTimestamp(iso: string): string {
  try {
    const d = new Date(iso)
    if (Number.isNaN(d.getTime())) return iso
    const now = new Date()
    const time = d.toLocaleTimeString(undefined, {
      hour: '2-digit',
      minute: '2-digit',
    })
    const sameDate = (a: Date, b: Date) =>
      a.getFullYear() === b.getFullYear() &&
      a.getMonth() === b.getMonth() &&
      a.getDate() === b.getDate()
    if (sameDate(d, now)) return `Today, ${time}`
    const yesterday = new Date(now)
    yesterday.setDate(now.getDate() - 1)
    if (sameDate(d, yesterday)) return `Yesterday, ${time}`
    const day = d.toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
    })
    return `${day}, ${time}`
  } catch {
    return iso
  }
}

/** Strips HTML tags from a notes string, returning collapsed plain text. */
export function stripHtml(html: string): string {
  if (!html) return ''
  if (typeof document === 'undefined') return html.replace(/<[^>]*>/g, ' ')
  const el = document.createElement('div')
  el.innerHTML = html
  return (el.textContent ?? '').replace(/\s+/g, ' ').trim()
}

/** Rough byte size of the persisted library, for the "~K KB used" footer. */
export function approxKB(value: unknown): string {
  try {
    const bytes = JSON.stringify(value).length
    return `${Math.max(1, Math.round(bytes / 1024))} KB`
  } catch {
    return '— KB'
  }
}
