import { useEffect, useRef, useState } from 'react'
import { useBuild } from '../store/build'
import { isSafeUrl, sanitizeHtml } from '../utils/sanitizeHtml'

const PRESET_COLORS = [
  '#d4cfbf',
  '#9b9ba1',
  '#e0b864',
  '#ef476f',
  '#06d6a0',
  '#118ab2',
  '#7c5cff',
  '#f78c6b',
]

export default function NotesView() {
  const notes = useBuild((s) => s.notes)
  const setNotes = useBuild((s) => s.setNotes)
  const commitBuildNotes = useBuild((s) => s.commitBuildNotes)
  const editorRef = useRef<HTMLDivElement | null>(null)
  const [linkOpen, setLinkOpen] = useState(false)
  const [linkUrl, setLinkUrl] = useState('')
  const [savedRange, setSavedRange] = useState<Range | null>(null)

  useEffect(() => {
    const el = editorRef.current
    if (!el) return
    // While the user is typing in the focused editor it is the source of truth;
    // re-assigning innerHTML (e.g. because the store's sanitized value differs
    // string-wise from the live DOM) would reset the caret mid-keystroke. Only
    // sync external changes (switching builds) when the editor is not focused.
    if (document.activeElement === el) return
    if (el.innerHTML === notes) return
    el.innerHTML = notes
  }, [notes])

  const persist = () => {
    const el = editorRef.current
    if (!el) return
    setNotes(el.innerHTML)
  }

  const exec = (command: string, value?: string) => {
    const el = editorRef.current
    if (!el) return
    el.focus()
    document.execCommand(command, false, value)
    persist()
  }

  const handleBlur = () => {
    persist()
    commitBuildNotes()
  }

  // Sanitise pasted HTML so the browser can't inject arbitrary markup.
  const handlePaste = (e: React.ClipboardEvent<HTMLDivElement>) => {
    const html = e.clipboardData.getData('text/html')
    const text = e.clipboardData.getData('text/plain')
    if (html) {
      e.preventDefault()
      document.execCommand('insertHTML', false, sanitizeHtml(html))
      persist()
    } else if (text) {
      e.preventDefault()
      document.execCommand('insertText', false, text)
      persist()
    }
  }

  // Save selection so it can be restored after losing focus to a popover (link URL input).
  const saveSelection = () => {
    const sel = window.getSelection()
    if (!sel || sel.rangeCount === 0) return
    const range = sel.getRangeAt(0).cloneRange()
    const editor = editorRef.current
    if (!editor) return
    if (!editor.contains(range.commonAncestorContainer)) return
    setSavedRange(range)
  }

  const restoreSelection = () => {
    if (!savedRange) return
    const sel = window.getSelection()
    if (!sel) return
    sel.removeAllRanges()
    sel.addRange(savedRange)
  }

  const openLinkPrompt = () => {
    saveSelection()
    setLinkUrl('')
    setLinkOpen(true)
  }

  // Prepend `https://` to bare URLs; re-sanitise so the new anchor picks up safe target/rel attributes.
  const insertLink = () => {
    let url = linkUrl.trim()
    if (!url) {
      setLinkOpen(false)
      return
    }
    if (!isSafeUrl(url)) url = `https://${url}`
    restoreSelection()
    exec('createLink', url)
    setLinkOpen(false)
    const el = editorRef.current
    if (el) {
      const cleaned = sanitizeHtml(el.innerHTML)
      if (cleaned !== el.innerHTML) {
        el.innerHTML = cleaned
        persist()
      }
    }
  }

  const applyColor = (color: string) => {
    exec('foreColor', color)
  }

  const clearFormat = () => {
    exec('removeFormat')
    exec('unlink')
  }

  return (
    <div className="mx-auto flex max-w-4xl flex-col gap-3">
      <div className="flex flex-wrap items-center gap-1 rounded-md border border-border bg-panel p-2">
        <ToolbarBtn
          onClick={() => exec('bold')}
          title="Bold (Ctrl+B)"
          label={<b>B</b>}
        />
        <ToolbarBtn
          onClick={() => exec('italic')}
          title="Italic (Ctrl+I)"
          label={<i>I</i>}
        />
        <ToolbarBtn
          onClick={() => exec('underline')}
          title="Underline (Ctrl+U)"
          label={<u>U</u>}
        />
        <ToolbarBtn
          onClick={() => exec('strikeThrough')}
          title="Strikethrough"
          label={<s>S</s>}
        />
        <Divider />
        <ToolbarBtn
          onClick={() => exec('formatBlock', 'H2')}
          title="Heading"
          label="H2"
        />
        <ToolbarBtn
          onClick={() => exec('formatBlock', 'H3')}
          title="Subheading"
          label="H3"
        />
        <ToolbarBtn
          onClick={() => exec('formatBlock', 'P')}
          title="Paragraph"
          label="¶"
        />
        <Divider />
        <ToolbarBtn
          onClick={() => exec('insertUnorderedList')}
          title="Bullet list"
          label="•⁝"
        />
        <ToolbarBtn
          onClick={() => exec('insertOrderedList')}
          title="Numbered list"
          label="1."
        />
        <Divider />
        <ColorPalette onPick={applyColor} />
        <Divider />
        <ToolbarBtn
          onClick={openLinkPrompt}
          title="Insert link"
          label={
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth={2}
              strokeLinecap="round"
              strokeLinejoin="round"
              className="h-3.5 w-3.5"
              aria-hidden
            >
              <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
              <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
            </svg>
          }
        />
        <ToolbarBtn onClick={clearFormat} title="Clear formatting" label="⌫" />
      </div>

      {linkOpen && (
        <div className="flex items-center gap-2 rounded-md border border-border bg-panel-2 p-2">
          <label className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
            URL
          </label>
          <input
            autoFocus
            value={linkUrl}
            onChange={(e) => setLinkUrl(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') insertLink()
              if (e.key === 'Escape') setLinkOpen(false)
            }}
            placeholder="https://..."
            className="flex-1 rounded-[3px] border border-border-2 px-2 py-1 text-xs text-text placeholder:text-faint focus:border-accent-deep focus:outline-none focus:ring-2 focus:ring-accent-hot/15"
            style={{
              background:
                'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
              boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
            }}
          />
          <button
            onClick={insertLink}
            className="rounded-[3px] border border-accent-deep px-3 py-1 font-mono text-[10px] uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]"
            style={{ background: 'linear-gradient(180deg, #3a2f1a, #2a2418)' }}
          >
            Insert
          </button>
          <button
            onClick={() => setLinkOpen(false)}
            className="rounded-[3px] border border-border-2 bg-panel-2 px-3 py-1 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
          >
            Cancel
          </button>
        </div>
      )}

      <div
        ref={editorRef}
        contentEditable
        suppressContentEditableWarning
        onInput={persist}
        onBlur={handleBlur}
        onPaste={handlePaste}
        onMouseUp={saveSelection}
        onKeyUp={saveSelection}
        spellCheck
        className="min-h-[24rem] rounded-md border border-border bg-panel p-4 text-sm leading-relaxed text-text outline-none focus:border-accent-deep notes-editor"
      />

      <div className="text-[10px] text-muted">
        Auto-saves on change · committed to disk on focus loss. Notes are
        shared across all profiles in this build.
      </div>
    </div>
  )
}

function ToolbarBtn({
  onClick,
  title,
  label,
}: {
  onClick: () => void
  title: string
  label: React.ReactNode
}) {
  // mousedown + preventDefault preserves the editor's text selection across the click.
  return (
    <button
      onMouseDown={(e) => {
        e.preventDefault()
        onClick()
      }}
      title={title}
      className="inline-flex h-7 min-w-[1.75rem] items-center justify-center rounded-[3px] border border-border-2 bg-panel-2 px-1.5 text-xs text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
    >
      {label}
    </button>
  )
}

function Divider() {
  return <span className="mx-0.5 h-5 w-px bg-border" />
}

function ColorPalette({ onPick }: { onPick: (color: string) => void }) {
  const [pickerOpen, setPickerOpen] = useState(false)
  const [customColor, setCustomColor] = useState('#e0b864')
  return (
    <div className="relative">
      <button
        onMouseDown={(e) => {
          e.preventDefault()
          setPickerOpen((o) => !o)
        }}
        title="Text color"
        className="inline-flex h-7 items-center gap-1 rounded-[3px] border border-border-2 bg-panel-2 px-1.5 text-xs text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
      >
        <span>A</span>
        <span
          className="h-2 w-4 rounded-[2px]"
          style={{ backgroundColor: customColor }}
        />
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth={3}
          className="h-2.5 w-2.5"
          aria-hidden
        >
          <path d="m6 9 6 6 6-6" />
        </svg>
      </button>
      {pickerOpen && (
        <div className="absolute left-0 top-full z-50 mt-1 flex flex-col gap-2 rounded-[3px] border border-accent-deep bg-panel p-2 shadow-[0_12px_40px_rgba(0,0,0,0.7)]">
          <div className="grid grid-cols-4 gap-1">
            {PRESET_COLORS.map((c) => (
              <button
                key={c}
                onMouseDown={(e) => {
                  e.preventDefault()
                  setCustomColor(c)
                  onPick(c)
                  setPickerOpen(false)
                }}
                className="h-6 w-6 rounded-[3px] border border-border transition-colors hover:border-accent-hot"
                style={{ backgroundColor: c }}
                title={c}
              />
            ))}
          </div>
          <label className="flex items-center gap-1.5 text-[10px] text-muted">
            Custom
            <input
              type="color"
              value={customColor}
              onChange={(e) => setCustomColor(e.target.value)}
              onBlur={() => {
                onPick(customColor)
                setPickerOpen(false)
              }}
              className="h-6 w-10 cursor-pointer rounded-[3px] border border-border-2 bg-panel-2"
            />
          </label>
        </div>
      )}
    </div>
  )
}
