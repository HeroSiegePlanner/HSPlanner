import { useCallback, useEffect, useRef, useState } from 'react'
import type { Dispatch, SetStateAction } from 'react'
import type { LootFilter, SavedLootFilter } from '../../types'
import { useCopyFeedback } from '../../hooks/useCopyFeedback'
import {
  createDefaultLootFilter,
  decodeLootFilter,
  encodeLootFilter,
} from '../../utils/lootfilter/codec'
import { ITEM_TYPES } from '../../utils/lootfilter/constants'
import { renameFilter, updateFilterCode } from '../../utils/lootfilter/savedFilters'
import { AffixPanel } from './AffixPanel'
import { FILTER_BTN_CLASS, FILTER_BTN_PRIMARY_CLASS } from './FilterCells'
import { TypePanel } from './TypePanel'
import { TypeRail } from './TypeRail'

const SAVE_DEBOUNCE_MS = 400

export type ApplyFilter = Dispatch<SetStateAction<LootFilter>>

interface FilterEditorProps {
  saved: SavedLootFilter
  onBack: () => void
}

export function FilterEditor({ saved, onBack }: FilterEditorProps) {
  const [filter, setFilter] = useState<LootFilter | null>(null)
  const [name, setName] = useState(saved.name)
  const [typeId, setTypeId] = useState(ITEM_TYPES[0]!.id)
  const [copied, copyToClipboard] = useCopyFeedback()
  const [showCode, setShowCode] = useState(false)
  const [code, setCode] = useState(saved.code)

  useEffect(() => {
    let cancelled = false
    void decodeLootFilter(saved.code).then((decoded) => {
      if (!cancelled) setFilter(decoded ?? createDefaultLootFilter())
    })
    return () => {
      cancelled = true
    }
  }, [saved.code])

  useEffect(() => {
    if (!filter) return
    let cancelled = false
    void encodeLootFilter(filter).then((next) => {
      if (!cancelled) setCode(next)
    })
    return () => {
      cancelled = true
    }
  }, [filter])

  const apply: ApplyFilter = (update) =>
    setFilter((prev) =>
      prev === null ? prev : typeof update === 'function' ? update(prev) : update,
    )

  const codeRef = useRef(code)
  const lastSavedRef = useRef(saved.code)

  const flush = useCallback(() => {
    if (codeRef.current === lastSavedRef.current) return
    updateFilterCode(saved.id, codeRef.current)
    lastSavedRef.current = codeRef.current
  }, [saved.id])

  useEffect(() => {
    codeRef.current = code
    if (code === lastSavedRef.current) return
    const timer = window.setTimeout(flush, SAVE_DEBOUNCE_MS)
    return () => window.clearTimeout(timer)
  }, [code, flush])

  useEffect(() => () => flush(), [flush])

  const commitName = () => {
    const trimmed = name.trim() || 'Loot filter'
    setName(trimmed)
    renameFilter(saved.id, trimmed)
  }

  const back = () => {
    flush()
    onBack()
  }

  const copyCode = async () => {
    if (!(await copyToClipboard(code))) setShowCode(true)
  }

  if (!filter) {
    return (
      <div className="py-8 text-center font-mono text-[11px] uppercase tracking-[0.2em] text-faint">
        Loading filter…
      </div>
    )
  }

  return (
    <div className="mx-auto max-w-[1500px] space-y-4">
      <header className="flex flex-wrap items-end justify-between gap-3">
        <div className="min-w-[260px] flex-1">
          <div className="mb-1 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
            <span
              aria-hidden
              className="inline-block h-[6px] w-[6px] rotate-45 bg-accent-hot"
              style={{ boxShadow: '0 0 8px rgba(224,184,100,0.6)' }}
            />
            Loot filter · changes save automatically
          </div>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            onBlur={commitName}
            onKeyDown={(e) => {
              if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
            }}
            aria-label="Filter name"
            className="w-full min-w-0 border-b border-transparent bg-transparent text-[22px] font-semibold tracking-[0.02em] text-accent-hot outline-none transition-colors focus:border-accent-deep"
            style={{ textShadow: '0 0 16px rgba(224,184,100,0.18)' }}
          />
        </div>
        <div className="flex items-center gap-2">
          <button type="button" onClick={back} className={FILTER_BTN_CLASS}>
            ← Filters
          </button>
          <button
            type="button"
            onClick={() => setShowCode((v) => !v)}
            className={FILTER_BTN_CLASS}
          >
            {showCode ? 'Hide code' : 'Show code'}
          </button>
          <button
            type="button"
            onClick={() => void copyCode()}
            className={FILTER_BTN_PRIMARY_CLASS}
          >
            {copied ? 'Copied!' : 'Copy game code'}
          </button>
        </div>
      </header>

      {showCode && (
        <textarea
          readOnly
          value={code}
          onFocus={(e) => e.target.select()}
          spellCheck={false}
          rows={4}
          className="w-full resize-y rounded-[3px] border border-border-2 bg-panel-2 p-2.5 font-mono text-[10px] leading-relaxed text-muted outline-none transition-colors focus:border-accent-deep"
        />
      )}

      <div className="grid items-start gap-4 lg:grid-cols-[232px_minmax(0,1fr)]">
        <div className="lg:sticky lg:top-0">
          <TypeRail filter={filter} activeId={typeId} onSelect={setTypeId} />
        </div>
        <div className="min-w-0 space-y-4">
          <TypePanel filter={filter} typeId={typeId} apply={apply} />
          <AffixPanel key={typeId} filter={filter} typeId={typeId} apply={apply} />

        </div>
      </div>
    </div>
  )
}
