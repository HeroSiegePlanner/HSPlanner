import {
  useCallback,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react'
import { createPortal } from 'react-dom'
import { useOutsideClick } from '../hooks/useOutsideClick'

export interface DropdownOption {
  id: string
  label: string
  meta?: ReactNode
}

interface DropdownProps {
  value: string | null
  options: DropdownOption[]
  onChange: (id: string | null) => void
  placeholder?: string
  searchPlaceholder?: string
  emptyLabel?: string
  clearLabel?: string
  onHoverChange?: (id: string | null) => void
  onOpenChange?: (open: boolean) => void
  searchable?: boolean
  compact?: boolean
}

interface MenuPos {
  left: number
  top: number
  bottom: number
  width: number
  placeAbove: boolean
  maxHeight: number
}

const MENU_GAP = 4
const MENU_MARGIN = 8
const MENU_MAX_HEIGHT = 380
const MIN_SPACE_BELOW = 220

export default function Dropdown({
  value,
  options,
  onChange,
  placeholder = 'Select…',
  searchPlaceholder = 'Search…',
  emptyLabel = 'No results',
  clearLabel,
  onHoverChange,
  onOpenChange,
  searchable = true,
  compact = false,
}: DropdownProps) {
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')
  const [kb, setKb] = useState(0)
  const [pos, setPos] = useState<MenuPos | null>(null)
  const ref = useRef<HTMLDivElement>(null)
  const menuRef = useRef<HTMLDivElement | null>(null)

  const close = () => {
    setOpen(false)
    setQuery('')
    onHoverChange?.(null)
    onOpenChange?.(false)
  }
  useOutsideClick(ref, open, close, menuRef)

  useLayoutEffect(() => {
    if (!open) return
    function recompute() {
      const el = ref.current
      if (!el) return
      const rect = el.getBoundingClientRect()
      const vh = window.innerHeight
      const spaceBelow = vh - rect.bottom - MENU_MARGIN
      const spaceAbove = rect.top - MENU_MARGIN
      const placeAbove = spaceBelow < MIN_SPACE_BELOW && spaceAbove > spaceBelow
      setPos({
        left: rect.left,
        top: rect.bottom + MENU_GAP,
        bottom: vh - rect.top + MENU_GAP,
        width: rect.width,
        placeAbove,
        maxHeight: Math.min(
          MENU_MAX_HEIGHT,
          Math.max(120, placeAbove ? spaceAbove : spaceBelow),
        ),
      })
    }
    recompute()
    const opts = { capture: true, passive: true } as const
    window.addEventListener('resize', recompute)
    window.addEventListener('scroll', recompute, opts)
    return () => {
      window.removeEventListener('resize', recompute)
      window.removeEventListener('scroll', recompute, opts)
    }
  }, [open])

  useLayoutEffect(() => {
    const menu = menuRef.current
    if (!open || !pos || !menu) return
    const vw = window.innerWidth
    const clampedLeft = Math.max(
      MENU_MARGIN,
      Math.min(pos.left, vw - MENU_MARGIN - menu.offsetWidth),
    )
    menu.style.left = `${clampedLeft}px`
  }, [open, pos])

  const selected = options.find((o) => o.id === value) ?? null

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase()
    if (!q) return options
    return options.filter((o) => o.label.toLowerCase().includes(q))
  }, [options, query])

  const showClear = clearLabel != null && value !== null
  const entryCount = (showClear ? 1 : 0) + filtered.length
  const activeKb = entryCount > 0 ? Math.min(kb, entryCount - 1) : 0

  const pick = (id: string | null) => {
    onChange(id)
    close()
  }

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setKb(Math.min(activeKb + 1, entryCount - 1))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setKb(Math.max(activeKb - 1, 0))
    } else if (e.key === 'Enter') {
      e.preventDefault()
      if (showClear && activeKb === 0) {
        pick(null)
        return
      }
      const o = filtered[showClear ? activeKb - 1 : activeKb]
      if (o) pick(o.id)
    } else if (e.key === 'Escape') {
      e.preventDefault()
      close()
    }
  }

  const setMenuNode = useCallback(
    (node: HTMLDivElement | null) => {
      menuRef.current = node
      if (node && !searchable) node.focus()
    },
    [searchable],
  )

  return (
    <div
      className={`hs-dd${compact ? ' hs-dd--compact' : ''}`}
      data-open={open}
      ref={ref}
    >
      <button
        type="button"
        className="hs-dd-trigger"
        onClick={() => {
          const next = !open
          setOpen(next)
          setKb(0)
          onHoverChange?.(null)
          onOpenChange?.(next)
        }}
      >
        <span className={`hs-dd-trigger-label${selected ? '' : ' is-empty'}`}>
          {selected?.label ?? placeholder}
        </span>
        <span className="hs-dd-chev" aria-hidden />
      </button>

      {open &&
        pos &&
        createPortal(
          <div
            className={`hs-dd-menu${compact ? ' hs-dd-menu--compact' : ''}`}
            role="listbox"
            ref={setMenuNode}
            tabIndex={searchable ? undefined : -1}
            onKeyDown={onKeyDown}
            style={{
              position: 'fixed',
              left: pos.left,
              top: pos.placeAbove ? undefined : pos.top,
              bottom: pos.placeAbove ? pos.bottom : undefined,
              right: 'auto',
              width: compact ? 'max-content' : pos.width,
              minWidth: compact ? Math.max(pos.width, 200) : undefined,
              maxWidth: compact ? 300 : undefined,
              maxHeight: pos.maxHeight,
            }}
          >
            {searchable && (
              <div className="hs-dd-search">
                <svg
                  className="hs-dd-search-icon"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth={2}
                  aria-hidden
                >
                  <circle cx="11" cy="11" r="7" />
                  <path d="m20 20-3.5-3.5" />
                </svg>
                <input
                  autoFocus
                  value={query}
                  onChange={(e) => {
                    setQuery(e.target.value)
                    setKb(0)
                  }}
                  placeholder={searchPlaceholder}
                />
              </div>
            )}

            <div
              className="hs-dd-list"
              onMouseLeave={() => onHoverChange?.(null)}
            >
              {showClear && (
                <div
                  role="option"
                  aria-selected={false}
                  className={`hs-dd-item is-none${activeKb === 0 ? ' is-keyboard' : ''}`}
                  onClick={() => pick(null)}
                  onMouseEnter={() => {
                    setKb(0)
                    onHoverChange?.(null)
                  }}
                >
                  <div className="hs-dd-item-name">{clearLabel}</div>
                </div>
              )}
              {filtered.length === 0 ? (
                <div className="hs-dd-empty">{emptyLabel}</div>
              ) : (
                filtered.map((o, i) => {
                  const idx = showClear ? i + 1 : i
                  const active = o.id === value
                  return (
                    <div
                      key={o.id}
                      role="option"
                      aria-selected={active}
                      className={`hs-dd-item${
                        active ? ' is-active' : ''
                      }${activeKb === idx ? ' is-keyboard' : ''}`}
                      onClick={() => pick(o.id)}
                      onMouseEnter={() => {
                        setKb(idx)
                        onHoverChange?.(o.id)
                      }}
                    >
                      <div className="hs-dd-item-name">{o.label}</div>
                      {o.meta != null && (
                        <div className="hs-dd-item-meta">{o.meta}</div>
                      )}
                    </div>
                  )
                })
              )}
            </div>

            {searchable && (
              <div className="hs-dd-foot">
                <span>
                  <kbd>↑</kbd>
                  <kbd>↓</kbd> navigate <kbd>↵</kbd> select
                </span>
                <span>
                  <kbd>esc</kbd> close
                </span>
              </div>
            )}
          </div>,
          document.body,
        )}
    </div>
  )
}
