import {
  Fragment,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactNode,
} from 'react'
import { useOutsideClick } from '../hooks/useOutsideClick'
import HoverPortal from './HoverPortal'

export type DropdownRarity = string

export interface DropdownItem {
  id: string
  name: string
  rarity?: DropdownRarity
  type?: string
  grade?: string
  sockets?: number | string | null
  def?: string
  group?: string
  meta?: ReactNode
}

export interface DropdownProps {
  items: DropdownItem[]
  value?: string | null
  onChange?: (id: string | null, item: DropdownItem | null) => void
  placeholder?: string
  allowNone?: boolean
  className?: string
  sidePanel?: (hoveredId: string | null) => ReactNode
}

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function Highlight({ text, query }: { text: string; query: string }) {
  if (!query) return <>{text}</>
  // Capture-group split: every odd-indexed part is the matched substring.
  const re = new RegExp(`(${escapeRegExp(query)})`, 'i')
  const parts = String(text).split(re)
  return (
    <>
      {parts.map((p, i) =>
        i % 2 === 1 ? <mark key={i}>{p}</mark> : <Fragment key={i}>{p}</Fragment>,
      )}
    </>
  )
}

export function Dropdown({
  items,
  value = null,
  onChange,
  placeholder = 'Choose item…',
  allowNone = true,
  className = '',
  sidePanel,
}: DropdownProps) {
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')
  const [kb, setKb] = useState(-1)
  const [hoveredId, setHoveredId] = useState<string | null>(null)
  const rootRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)
  const listRef = useRef<HTMLDivElement>(null)

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase()
    if (!q) return items
    return items.filter(
      (it) =>
        it.name.toLowerCase().includes(q) ||
        (it.type ?? '').toLowerCase().includes(q) ||
        (it.grade ?? '').toLowerCase().includes(q),
    )
  }, [items, query])

  const selected = items.find((it) => it.id === value) ?? null

  useEffect(() => {
    if (!open) return
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setQuery('')
    setKb(-1)
    setHoveredId(null)
    const t = setTimeout(() => inputRef.current?.focus(), 20)
    return () => clearTimeout(t)
  }, [open])

  useOutsideClick(rootRef, open, () => {
    setOpen(false)
    setHoveredId(null)
  })

  useEffect(() => {
    const els = listRef.current?.querySelectorAll<HTMLElement>('.hs-dd-item')
    const el = els?.[kb]
    el?.scrollIntoView({ block: 'nearest' })
    if (el) {
      const id = el.dataset.id
      setHoveredId(id && id !== '__none__' ? id : null)
    }
  }, [kb])

  function pick(id: string | null) {
    const it = id === null ? null : items.find((x) => x.id === id) ?? null
    onChange?.(id, it)
    setOpen(false)
  }

  function onKey(e: KeyboardEvent<HTMLInputElement>) {
    const els =
      listRef.current?.querySelectorAll<HTMLElement>('.hs-dd-item') ?? []
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setKb((k) => Math.min(els.length - 1, k + 1))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setKb((k) => Math.max(0, k - 1))
    } else if (e.key === 'Enter') {
      e.preventDefault()
      const el = els[kb]
      if (el) {
        const id = el.dataset.id
        pick(id === '__none__' ? null : id ?? null)
      }
    } else if (e.key === 'Escape') {
      e.preventDefault()
      setOpen(false)
    }
  }

  const groups = useMemo(() => {
    const m = new Map<string, DropdownItem[]>()
    for (const it of filtered) {
      const g = it.group ?? ''
      const arr = m.get(g)
      if (arr) arr.push(it)
      else m.set(g, [it])
    }
    return m
  }, [filtered])
  const hasGroups = [...groups.keys()].some(Boolean)
  const indexById = useMemo(() => {
    const m = new Map<string, number>()
    filtered.forEach((it, i) => m.set(it.id, i))
    return m
  }, [filtered])

  return (
    <div
      ref={rootRef}
      className={['hs-dd', className].filter(Boolean).join(' ')}
      data-open={open}
    >
      <button
        type="button"
        className="hs-dd-trigger"
        onClick={() => setOpen((o) => !o)}
      >
        <span
          className={['hs-dd-trigger-label', selected ? '' : 'is-empty']
            .filter(Boolean)
            .join(' ')}
        >
          {selected ? selected.name : placeholder}
        </span>
        <span className="hs-dd-chev" />
      </button>

      {open && sidePanel && (
        <HoverPortal anchorRef={rootRef}>{sidePanel(hoveredId)}</HoverPortal>
      )}
      {open && (
        <div className="hs-dd-menu">
          <div className="hs-dd-search">
            <svg
              className="hs-dd-search-icon"
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              strokeWidth={1.4}
            >
              <circle cx="7" cy="7" r="4.5" />
              <path d="M10.5 10.5 L14 14" />
            </svg>
            <input
              ref={inputRef}
              type="text"
              placeholder="Search…"
              autoComplete="off"
              value={query}
              onChange={(e) => {
                setQuery(e.target.value)
                setKb(0)
              }}
              onKeyDown={onKey}
            />
          </div>

          <div
            ref={listRef}
            className="hs-dd-list"
            onMouseLeave={() => setHoveredId(null)}
          >
            {allowNone && !query && (
              <div
                className={['hs-dd-item', 'is-none', kb === 0 ? 'is-keyboard' : '']
                  .filter(Boolean)
                  .join(' ')}
                data-id="__none__"
                onClick={() => pick(null)}
              >
                <div className="hs-dd-item-name">— none —</div>
              </div>
            )}
            {filtered.length === 0 ? (
              <div className="hs-dd-empty">No items match "{query}"</div>
            ) : (
              [...groups].map(([g, arr]) => (
                <Fragment key={g || '__'}>
                  {g && hasGroups && <div className="hs-dd-group">{g}</div>}
                  {arr.map((it) => {
                    const idx =
                      (indexById.get(it.id) ?? 0) + (allowNone && !query ? 1 : 0)
                    const cls = [
                      'hs-dd-item',
                      `rarity-${it.rarity ?? 'normal'}`,
                      it.id === value ? 'is-active' : '',
                      idx === kb ? 'is-keyboard' : '',
                    ]
                      .filter(Boolean)
                      .join(' ')
                    return (
                      <div
                        key={it.id}
                        className={cls}
                        data-id={it.id}
                        onClick={() => pick(it.id)}
                        onMouseEnter={() => setHoveredId(it.id)}
                      >
                        <div className="hs-dd-item-name">
                          <Highlight text={it.name} query={query} />
                        </div>
                        {it.meta != null ? (
                          <div className="hs-dd-item-meta">{it.meta}</div>
                        ) : (
                          (it.type ||
                            it.grade ||
                            (it.sockets !== undefined && it.sockets !== null) ||
                            it.def) && (
                            <div className="hs-dd-item-meta">
                              {it.type && (
                                <span>
                                  <Highlight text={it.type} query={query} />
                                </span>
                              )}
                              {it.grade && (
                                <>
                                  <span className="sep">·</span>
                                  <span className="grade">
                                    Grade{' '}
                                    <Highlight text={it.grade} query={query} />
                                  </span>
                                </>
                              )}
                              {it.sockets !== undefined &&
                                it.sockets !== null && (
                                  <>
                                    <span className="sep">·</span>
                                    <span>{it.sockets} sockets</span>
                                  </>
                                )}
                              {it.def && (
                                <>
                                  <span className="sep">·</span>
                                  <span>Def {it.def}</span>
                                </>
                              )}
                            </div>
                          )
                        )}
                      </div>
                    )
                  })}
                </Fragment>
              ))
            )}
          </div>

          <div className="hs-dd-foot">
            <span>
              <kbd>↑</kbd>
              <kbd>↓</kbd> navigate
            </span>
            <span>
              <kbd>↵</kbd> select <kbd>Esc</kbd> close
            </span>
          </div>
        </div>
      )}
    </div>
  )
}

export default Dropdown
