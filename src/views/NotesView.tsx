import { useEffect, useRef, useState } from 'react'
import { useBuild } from '../store/build'
import { isSafeUrl, sanitizeHtml } from '../utils/sanitizeHtml'

const PRESET_COLORS = [
  '#e8e8ea',
  '#9b9ba1',
  '#ffd166',
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
      <div className="flex flex-wrap items-center gap-1 rounded border border-border bg-panel p-2">
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
          label="🔗"
        />
        <ToolbarBtn onClick={clearFormat} title="Clear formatting" label="⌫" />
      </div>

      {linkOpen && (
        <div className="flex items-center gap-2 rounded border border-border bg-panel-2 p-2">
          <label className="text-[10px] uppercase tracking-wider text-muted">
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
            className="flex-1 rounded border border-border bg-panel px-2 py-1 text-xs"
          />
          <button
            onClick={insertLink}
            className="rounded border border-accent/50 bg-accent/10 px-2 py-1 text-xs text-accent hover:bg-accent/20"
          >
            Insert
          </button>
          <button
            onClick={() => setLinkOpen(false)}
            className="rounded border border-border bg-panel px-2 py-1 text-xs text-muted hover:text-text"
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
        className="min-h-[24rem] rounded border border-border bg-panel p-4 text-sm leading-relaxed text-text outline-none focus:border-accent notes-editor"
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
      className="inline-flex h-7 min-w-[1.75rem] items-center justify-center rounded border border-transparent px-1.5 text-xs text-text hover:border-border hover:bg-panel-2"
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
  const [customColor, setCustomColor] = useState('#ffd166')
  return (
    <div className="relative">
      <button
        onMouseDown={(e) => {
          e.preventDefault()
          setPickerOpen((o) => !o)
        }}
        title="Text color"
        className="inline-flex h-7 items-center gap-1 rounded border border-transparent px-1.5 text-xs text-text hover:border-border hover:bg-panel-2"
      >
        <span>A</span>
        <span
          className="h-2 w-4 rounded"
          style={{ backgroundColor: customColor }}
        />
        <span className="text-[10px]">▾</span>
      </button>
      {pickerOpen && (
        <div className="absolute left-0 top-full z-50 mt-1 flex flex-col gap-2 rounded border border-border bg-panel p-2 shadow-lg">
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
                className="h-6 w-6 rounded border border-border hover:scale-110 transition-transform"
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
              className="h-6 w-10 cursor-pointer rounded border border-border bg-panel-2"
            />
          </label>
        </div>
      )}
    </div>
  )
}
