import { useMemo, useRef, useState, type ReactNode } from 'react'
import { useOutsideClick } from '../hooks/useOutsideClick'

export interface DropdownOption {
  id: string
  label: string
  meta?: ReactNode
  rarity?: string
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
}

// Searchable, keyboard-navigable select built on the .hs-dd* styles in index.css.
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
}: DropdownProps) {
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')
  const [kb, setKb] = useState(0)
  const ref = useRef<HTMLDivElement>(null)

  const close = () => {
    setOpen(false)
    setQuery('')
    onHoverChange?.(null)
    onOpenChange?.(false)
  }
  useOutsideClick(ref, open, close)

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

  return (
    <div className="hs-dd" data-open={open} ref={ref}>
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

      {open && (
        <div className="hs-dd-menu" role="listbox">
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
              onKeyDown={onKeyDown}
              placeholder={searchPlaceholder}
            />
          </div>

          <div className="hs-dd-list" onMouseLeave={() => onHoverChange?.(null)}>
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
                    className={`hs-dd-item${o.rarity ? ` rarity-${o.rarity}` : ''}${
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

          <div className="hs-dd-foot">
            <span>
              <kbd>↑</kbd>
              <kbd>↓</kbd> navigate <kbd>↵</kbd> select
            </span>
            <span>
              <kbd>esc</kbd> close
            </span>
          </div>
        </div>
      )}
    </div>
  )
}
