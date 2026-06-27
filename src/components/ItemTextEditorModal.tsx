import { useEffect, useRef, useState } from 'react'
import type { EquippedItem, ItemBase } from '../types'
import {
  parseItemText,
  serializeEquippedItem,
  type ParseError,
  type ParseResult,
} from '../utils/item/itemTextFormat'
import { Modal } from './Modal'

interface Props {
  slotName: string
  equipped: EquippedItem
  base: ItemBase
  onSave: (next: EquippedItem) => void
  onClose: () => void
}

export default function ItemTextEditorModal({
  slotName,
  equipped,
  base,
  onSave,
  onClose,
}: Props) {
  const [text, setText] = useState<string | null>(null)
  const [result, setResult] = useState<ParseResult>({
    equipped: null,
    errors: [],
  })
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  useEffect(() => {
    let cancelled = false
    serializeEquippedItem(equipped, base).then((t) => {
      if (!cancelled) setText(t)
    })
    return () => {
      cancelled = true
    }
  }, [equipped, base])

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])

  useEffect(() => {
    if (text === null) return
    let cancelled = false
    const handle = setTimeout(() => {
      parseItemText(text, base.id).then((r) => {
        if (!cancelled) setResult(r)
      })
    }, 200)
    return () => {
      cancelled = true
      clearTimeout(handle)
    }
  }, [text, base.id])

  useEffect(() => {
    textareaRef.current?.focus()
  }, [])

  const errorCount = result.errors.filter((e) => e.severity === 'error').length
  const warnCount = result.errors.filter((e) => e.severity === 'warning').length
  const canSave = !!result.equipped && errorCount === 0

  function handleSave() {
    if (!canSave || !result.equipped) return
    onSave(result.equipped)
    onClose()
  }

  return (
    <Modal
      onClose={onClose}
      panelClassName="h-[88vh] w-[1100px] max-w-[96vw]"
      eyebrow={<>Edit Item · {slotName}</>}
      title={base.name}
    >
        <div className="flex min-h-0 flex-1 flex-row">
          <div className="flex min-w-0 flex-[3] flex-col border-r border-border">
            <div className="flex items-center gap-2 border-b border-border px-4 py-2 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
              <span
                className="inline-block h-1 w-1 rotate-45 bg-accent-deep"
                aria-hidden="true"
              />
              Text · edit affixes, stars, sockets, augment
            </div>
            <textarea
              ref={textareaRef}
              value={text ?? ''}
              onChange={(e) => setText(e.target.value)}
              spellCheck={false}
              wrap="off"
              className="min-h-0 flex-1 resize-none border-0 px-4 py-3 font-mono text-[12px] leading-[1.55] text-text outline-none focus:outline-none"
              style={{
                background:
                  'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
                boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
                tabSize: 2,
              }}
            />
          </div>

          <div className="flex w-[360px] shrink-0 flex-col">
            <div className="flex items-center gap-2 border-b border-border px-4 py-2 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
              <span
                className="inline-block h-1 w-1 rotate-45 bg-accent-deep"
                aria-hidden="true"
              />
              Validation
              {errorCount > 0 && (
                <span className="ml-auto text-stat-red">
                  {errorCount} error{errorCount !== 1 ? 's' : ''}
                </span>
              )}
              {errorCount === 0 && warnCount > 0 && (
                <span className="ml-auto text-stat-orange">
                  {warnCount} warning{warnCount !== 1 ? 's' : ''}
                </span>
              )}
              {errorCount === 0 && warnCount === 0 && (
                <span className="ml-auto text-accent-hot">all clear</span>
              )}
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto px-3 py-3 space-y-2">
              {result.errors.length === 0 ? (
                <div className="rounded-[3px] border border-border-2 bg-panel-2/60 px-3 py-3 font-mono text-[11px] text-muted">
                  All clear · save to apply changes to this slot.
                </div>
              ) : (
                result.errors.map((err, i) => (
                  <ErrorRow key={i} err={err} />
                ))
              )}
            </div>
          </div>
        </div>

        <footer className="flex items-center justify-between gap-3 border-t border-border bg-black/30 px-4 py-3">
          <div
            className={`flex min-w-0 flex-1 items-center gap-2 font-mono text-[11px] tracking-[0.06em] ${
              canSave ? 'text-accent-hot' : 'text-faint'
            }`}
          >
            <span
              className="h-1.5 w-1.5 shrink-0 rounded-full"
              style={{
                background: canSave
                  ? 'var(--color-accent-hot)'
                  : 'var(--color-faint)',
                boxShadow: canSave
                  ? '0 0 8px rgba(224,184,100,0.6)'
                  : '0 0 6px var(--color-faint)',
              }}
            />
            <span className="truncate">
              {canSave
                ? 'Ready to save'
                : errorCount > 0
                  ? 'Fix errors before saving'
                  : 'Parsing…'}
            </span>
          </div>
          <div className="flex shrink-0 gap-2">
            <button
              onClick={onClose}
              className="rounded-[3px] border border-border-2 bg-transparent px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
            >
              Cancel
            </button>
            <button
              onClick={handleSave}
              disabled={!canSave}
              className={`rounded-[3px] border px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] transition-colors ${
                canSave
                  ? 'border-accent-deep bg-accent-hot/10 text-accent-hot hover:bg-accent-hot/20'
                  : 'cursor-not-allowed border-border-2 bg-transparent text-faint opacity-60'
              }`}
            >
              Save
            </button>
          </div>
        </footer>
    </Modal>
  )
}

function ErrorRow({ err }: { err: ParseError }) {
  const isError = err.severity === 'error'
  const cls = isError
    ? 'border-stat-red/40 bg-stat-red/10 text-stat-red'
    : 'border-stat-orange/40 bg-stat-orange/10 text-stat-orange'
  const tag = isError ? 'ERR' : 'WARN'
  return (
    <div
      className={`rounded-[3px] border px-3 py-2 font-mono text-[11px] leading-[1.5] ${cls}`}
    >
      <div className="mb-0.5 flex items-center gap-2 text-[10px] uppercase tracking-[0.14em] opacity-80">
        <span>{tag}</span>
        <span>·</span>
        <span>line {err.line}</span>
      </div>
      <div className="text-[11px]">{err.message}</div>
    </div>
  )
}
