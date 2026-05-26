const ALLOWED_TAGS = new Set([
  'B',
  'STRONG',
  'I',
  'EM',
  'U',
  'S',
  'STRIKE',
  'DEL',
  'A',
  'UL',
  'OL',
  'LI',
  'H1',
  'H2',
  'H3',
  'H4',
  'BR',
  'P',
  'DIV',
  'SPAN',
  'FONT',
  'BLOCKQUOTE',
  'CODE',
  'PRE',
])

const DANGEROUS_TAGS = new Set([
  'SCRIPT',
  'STYLE',
  'IFRAME',
  'OBJECT',
  'EMBED',
  'NOEMBED',
  'NOSCRIPT',
  'TEMPLATE',
  'SVG',
  'MATH',
  'XMP',
  'PLAINTEXT',
])

const ALLOWED_ATTRS_BY_TAG: Record<string, Set<string>> = {
  A: new Set(['href', 'title', 'target', 'rel']),
  FONT: new Set(['color']),
}

const ALLOWED_STYLE_PROPS = new Set([
  'color',
  'background-color',
  'font-weight',
  'font-style',
  'text-decoration',
])

export const SAFE_URL_RE = /^(https?:|mailto:|#|\/)/i

export function isSafeUrl(url: string): boolean {
  return SAFE_URL_RE.test(url.trim())
}

// `style` is always allowed here because sanitizeStyle filters its body separately.
function isAllowedAttribute(tagName: string, attr: string): boolean {
  if (attr === 'style') return true
  const allowed = ALLOWED_ATTRS_BY_TAG[tagName]
  if (!allowed) return false
  return allowed.has(attr)
}

// Rejects url(), expression(), and javascript: payloads.
function sanitizeStyle(value: string): string {
  const out: string[] = []
  for (const decl of value.split(';')) {
    const colon = decl.indexOf(':')
    if (colon === -1) continue
    const prop = decl.slice(0, colon).trim().toLowerCase()
    const val = decl.slice(colon + 1).trim()
    if (!ALLOWED_STYLE_PROPS.has(prop)) continue
    if (/url\s*\(|expression\s*\(|javascript:/i.test(val)) continue
    out.push(`${prop}: ${val}`)
  }
  return out.join('; ')
}

function sanitizeUrl(url: string): string | null {
  const trimmed = url.trim()
  if (!trimmed) return null
  if (!SAFE_URL_RE.test(trimmed)) return null
  return trimmed
}

function sanitizeNode(node: Element): void {
  for (const attr of Array.from(node.attributes)) {
    const name = attr.name.toLowerCase()
    if (name.startsWith('on')) {
      node.removeAttribute(attr.name)
      continue
    }
    if (!isAllowedAttribute(node.tagName, name)) {
      node.removeAttribute(attr.name)
      continue
    }
    if (name === 'style') {
      const cleaned = sanitizeStyle(attr.value)
      if (cleaned) node.setAttribute('style', cleaned)
      else node.removeAttribute('style')
    }
    if (name === 'href') {
      const url = sanitizeUrl(attr.value)
      if (!url) {
        node.removeAttribute('href')
      } else {
        node.setAttribute('href', url)
      }
    }
  }
  if (node.tagName === 'A' && node.getAttribute('href')) {
    const href = node.getAttribute('href')!.trim()
    // Force _blank only on external schemes; in-page (#…) and same-origin
    // relative URLs should stay in the current tab.
    const isExternal = /^https?:/i.test(href) || /^mailto:/i.test(href)
    if (isExternal) {
      node.setAttribute('target', '_blank')
    }
    node.setAttribute('rel', 'noopener noreferrer nofollow')
  }
}

// DANGEROUS_TAGS subtree is removed entirely; unknown-but-safe wrappers get unwrapped (children lifted into parent).
function walk(root: Element): void {
  for (const child of Array.from(root.children)) {
    const tag = child.tagName.toUpperCase()
    if (DANGEROUS_TAGS.has(tag)) {
      child.remove()
      continue
    }
    if (!ALLOWED_TAGS.has(tag)) {
      const parent = child.parentNode
      if (!parent) continue
      while (child.firstChild) parent.insertBefore(child.firstChild, child)
      parent.removeChild(child)
      continue
    }
    sanitizeNode(child)
    walk(child)
  }
}

// Falls back to a tag-stripping regex in non-DOM environments (SSR/Node).
export function sanitizeHtml(html: string): string {
  if (!html) return ''
  if (typeof window === 'undefined' || !('DOMParser' in window)) {
    return html.replace(/<[^>]*>/g, '')
  }
  const parser = new DOMParser()
  // Walk doc.body directly; a wrapper div would let "</div>" smuggle the tail out.
  const doc = parser.parseFromString(html, 'text/html')
  walk(doc.body)
  return doc.body.innerHTML
}
