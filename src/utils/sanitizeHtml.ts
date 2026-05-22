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

const SAFE_URL_RE = /^(https?:|mailto:|#|\/)/i

export function isSafeUrl(url: string): boolean {
  // Returns true when a URL passes the SAFE_URL_RE allowlist (http(s), mailto, anchor, or root-relative). Used by sanitizeUrl and any caller that needs a quick safety check before rendering a user-supplied link.
  return SAFE_URL_RE.test(url.trim())
}

function isAllowedAttribute(tagName: string, attr: string): boolean {
  // Decides whether an attribute is permitted on a given tag according to the per-tag allowlist (style is always allowed because it is filtered separately by sanitizeStyle). Used by sanitizeNode while pruning attributes.
  if (attr === 'style') return true
  const allowed = ALLOWED_ATTRS_BY_TAG[tagName]
  if (!allowed) return false
  return allowed.has(attr)
}

function sanitizeStyle(value: string): string {
  // Filters a CSS inline-style declaration string down to the small ALLOWED_STYLE_PROPS set and rejects values that try to use url(), expression(), or javascript: payloads. Used by sanitizeNode to neutralise potentially dangerous style attributes while keeping basic formatting like color and decoration.
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
  // Trims a URL string and returns it if it passes the SAFE_URL_RE allowlist, otherwise returns null. Used by sanitizeNode to clean up href attributes before they are reinserted into the DOM.
  const trimmed = url.trim()
  if (!trimmed) return null
  if (!SAFE_URL_RE.test(trimmed)) return null
  return trimmed
}

function sanitizeNode(node: Element): void {
  // Strips disallowed attributes from a single element, sanitises any surviving style/href values, and forces external-link safety attributes on anchor tags. Used by walk to clean every node visited during sanitisation.
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
    node.setAttribute('target', '_blank')
    node.setAttribute('rel', 'noopener noreferrer nofollow')
  }
}

function walk(root: Element): void {
  // Recursively walks the children of an element, fully removing any DANGEROUS_TAGS subtree, lifting children out of disallowed-but-safe wrappers, and sanitising allowed elements in place. Used by sanitizeHtml as the recursion engine that traverses the parsed DOM.
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

export function sanitizeHtml(html: string): string {
  // Public entry point that parses an arbitrary HTML string with DOMParser and returns the cleaned innerHTML, falling back to a tag-stripping regex in non-DOM environments. Used to sanitise user-authored notes that are persisted, rendered with dangerouslySetInnerHTML, and shared via URLs.
  if (!html) return ''
  if (typeof window === 'undefined' || !('DOMParser' in window)) {
    return html.replace(/<[^>]*>/g, '')
  }
  const parser = new DOMParser()
  const doc = parser.parseFromString(`<div>${html}</div>`, 'text/html')
  const root = doc.body.firstElementChild
  if (!root) return ''
  walk(root)
  return root.innerHTML
}
