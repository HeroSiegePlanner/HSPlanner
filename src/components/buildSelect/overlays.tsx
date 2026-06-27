import { useEffect, useRef, useState, type ReactNode } from 'react'
import type { Folder } from '../../utils/build/savedBuilds'
import { CaretIcon } from './icons'
import { Modal } from '../Modal'

const GOLD_BTN = 'linear-gradient(180deg, #3a2f1a, #2a2418)'

const BTN_GHOST =
  'rounded-[3px] border border-border-2 bg-transparent px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot'
const BTN_GOLD =
  'rounded-[3px] border border-accent-deep px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4] disabled:cursor-not-allowed disabled:opacity-50'
const BTN_DANGER =
  'rounded-[3px] border border-stat-red/60 px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-stat-red transition-colors hover:border-stat-red hover:bg-stat-red/10'
const INPUT_CLASS =
  'w-full rounded-[3px] border border-border-2 px-3 py-2 text-text placeholder:text-faint focus:border-accent-deep focus:outline-none focus:ring-2 focus:ring-accent-hot/15'
const INPUT_STYLE = {
  background: 'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
  boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
} as const

function OverlayShell({
  section,
  title,
  onClose,
  children,
  footer,
  width = 460,
}: {
  section: string
  title: string
  onClose: () => void
  children: ReactNode
  footer: ReactNode
  width?: number
}) {
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])

  return (
    <Modal
      onClose={onClose}
      eyebrow={section}
      title={title}
      panelClassName="max-w-[92vw]"
      panelStyle={{ width }}
    >
      <div className="flex flex-col gap-3 p-5">{children}</div>
      <footer className="flex items-center justify-end gap-2 border-t border-border bg-black/30 px-5 py-3">
        {footer}
      </footer>
    </Modal>
  )
}

function FieldLabel({ children }: { children: ReactNode }) {
  return (
    <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
      {children}
    </span>
  )
}

export function TextPromptOverlay({
  section,
  title,
  label,
  initial = '',
  placeholder,
  submitLabel,
  hint,
  onSubmit,
  onClose,
}: {
  section: string
  title: string
  label: string
  initial?: string
  placeholder?: string
  submitLabel: string
  hint?: string
  onSubmit: (value: string) => void
  onClose: () => void
}) {
  const [value, setValue] = useState(initial)
  const submit = () => {
    const trimmed = value.trim()
    if (!trimmed) return
    onSubmit(trimmed)
  }
  return (
    <OverlayShell
      section={section}
      title={title}
      onClose={onClose}
      footer={
        <>
          <button type="button" onClick={onClose} className={BTN_GHOST}>
            Cancel
          </button>
          <button
            type="button"
            onClick={submit}
            disabled={!value.trim()}
            className={BTN_GOLD}
            style={{ background: GOLD_BTN }}
          >
            {submitLabel}
          </button>
        </>
      }
    >
      <FieldLabel>{label}</FieldLabel>
      <input
        autoFocus
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder={placeholder}
        onKeyDown={(e) => {
          if (e.key === 'Enter') submit()
        }}
        className={INPUT_CLASS}
        style={INPUT_STYLE}
      />
      {hint && (
        <p className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
          {hint}
        </p>
      )}
    </OverlayShell>
  )
}

export function ConfirmOverlay({
  section,
  title,
  message,
  confirmLabel,
  danger = false,
  onConfirm,
  onClose,
}: {
  section: string
  title: string
  message: ReactNode
  confirmLabel: string
  danger?: boolean
  onConfirm: () => void
  onClose: () => void
}) {
  return (
    <OverlayShell
      section={section}
      title={title}
      onClose={onClose}
      footer={
        <>
          <button type="button" onClick={onClose} className={BTN_GHOST}>
            Cancel
          </button>
          <button
            type="button"
            onClick={onConfirm}
            className={danger ? BTN_DANGER : BTN_GOLD}
            style={danger ? undefined : { background: GOLD_BTN }}
          >
            {confirmLabel}
          </button>
        </>
      }
    >
      <p className="text-[13px] leading-relaxed text-text">{message}</p>
    </OverlayShell>
  )
}

export function SaveOverlay({
  canOverwrite,
  onOverwrite,
  onSaveAsNew,
  onClose,
}: {
  canOverwrite: boolean
  onOverwrite: () => void
  onSaveAsNew: (name: string) => void
  onClose: () => void
}) {
  const [name, setName] = useState('')
  const submit = () => onSaveAsNew(name.trim() || 'Untitled build')
  return (
    <OverlayShell
      section="Save"
      title="Save build"
      onClose={onClose}
      footer={
        <>
          <button type="button" onClick={onClose} className={BTN_GHOST}>
            Cancel
          </button>
          <button
            type="button"
            onClick={submit}
            className={BTN_GOLD}
            style={{ background: GOLD_BTN }}
          >
            Save as new
          </button>
        </>
      }
    >
      {canOverwrite && (
        <button
          type="button"
          onClick={onOverwrite}
          className="w-full justify-center rounded-[3px] border border-accent-deep px-3.5 py-2 font-mono text-[11px] font-semibold uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]"
          style={{ background: GOLD_BTN }}
        >
          Update active profile
        </button>
      )}
      <FieldLabel>New build name</FieldLabel>
      <input
        autoFocus
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="e.g. Lightning Marksman"
        onKeyDown={(e) => {
          if (e.key === 'Enter') submit()
        }}
        className={INPUT_CLASS}
        style={INPUT_STYLE}
      />
      <p className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
        Creates a new build with one profile seeded from current state.
      </p>
    </OverlayShell>
  )
}

export function ImportOverlay({
  onImport,
  onClose,
}: {
  onImport: (text: string) => string | null
  onClose: () => void
}) {
  const [text, setText] = useState('')
  const [error, setError] = useState<string | null>(null)
  const fileRef = useRef<HTMLInputElement>(null)

  const submit = (value: string) => {
    const err = onImport(value)
    if (err) setError(err)
  }
  const handleFile = (file: File) => {
    const reader = new FileReader()
    reader.onload = () => submit(String(reader.result ?? ''))
    reader.onerror = () => setError('Could not read file')
    reader.readAsText(file)
  }

  return (
    <OverlayShell
      section="Import"
      title="Import build"
      onClose={onClose}
      width={520}
      footer={
        <>
          <button type="button" onClick={onClose} className={BTN_GHOST}>
            Cancel
          </button>
          <button
            type="button"
            onClick={() => submit(text)}
            className={BTN_GOLD}
            style={{ background: GOLD_BTN }}
          >
            Import &amp; open
          </button>
        </>
      }
    >
      <FieldLabel>Paste a build code or share URL — or load from file</FieldLabel>
      <textarea
        autoFocus
        value={text}
        onChange={(e) => {
          setText(e.target.value)
          setError(null)
        }}
        placeholder="Paste shared build code…"
        rows={6}
        className={`${INPUT_CLASS} resize-none font-mono text-[11px]`}
        style={INPUT_STYLE}
      />
      <button
        type="button"
        onClick={() => fileRef.current?.click()}
        className="self-start rounded-[3px] border border-border-2 bg-panel-2 px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
      >
        Choose file…
      </button>
      {error && (
        <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-stat-red">
          {error}
        </span>
      )}
      <input
        ref={fileRef}
        type="file"
        accept=".txt,.hsbuild,.hspb,application/json,text/plain"
        className="hidden"
        onChange={(e) => {
          const f = e.target.files?.[0]
          if (f) handleFile(f)
          e.target.value = ''
        }}
      />
    </OverlayShell>
  )
}

export function MoveToFolderOverlay({
  folders,
  currentFolderId,
  onMove,
  onClose,
}: {
  folders: Folder[]
  currentFolderId: string | null
  onMove: (folderId: string | null) => void
  onClose: () => void
}) {
  const depth = (f: Folder): number => {
    let d = 0
    let cur: Folder | undefined = f
    const seen = new Set<string>()
    while (cur && cur.parentId !== null && !seen.has(cur.id)) {
      seen.add(cur.id)
      cur = folders.find((x) => x.id === cur!.parentId)
      d++
    }
    return d
  }
  const sorted = [...folders].sort((a, b) => a.name.localeCompare(b.name))
  return (
    <OverlayShell
      section="Organise"
      title="Move to folder"
      onClose={onClose}
      footer={
        <button type="button" onClick={onClose} className={BTN_GHOST}>
          Cancel
        </button>
      }
    >
      <FieldLabel>Destination</FieldLabel>
      <div className="flex max-h-72 flex-col gap-1 overflow-y-auto">
        <FolderChoice
          label="Unfiled"
          active={currentFolderId === null}
          onClick={() => onMove(null)}
        />
        {sorted.map((f) => (
          <FolderChoice
            key={f.id}
            label={f.name}
            indent={depth(f)}
            active={currentFolderId === f.id}
            onClick={() => onMove(f.id)}
          />
        ))}
        {sorted.length === 0 && (
          <p className="px-1 py-2 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
            No folders yet
          </p>
        )}
      </div>
    </OverlayShell>
  )
}

function FolderChoice({
  label,
  indent = 0,
  active,
  onClick,
}: {
  label: string
  indent?: number
  active: boolean
  onClick: () => void
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`flex items-center gap-2 rounded-[3px] px-2.5 py-2 text-left text-[12px] transition-colors ${
        active
          ? 'bg-accent-hot/10 text-accent-hot'
          : 'text-muted hover:bg-white/[0.025] hover:text-text'
      }`}
      style={{ paddingLeft: 10 + indent * 16 }}
    >
      <CaretIcon className="h-2.5 w-2.5 shrink-0 text-accent-deep" />
      <span className="truncate">{label}</span>
    </button>
  )
}

export function TagsOverlay({
  initial,
  onSave,
  onClose,
}: {
  initial: string[]
  onSave: (tags: string[]) => void
  onClose: () => void
}) {
  const [tags, setTags] = useState<string[]>(initial)
  const [draft, setDraft] = useState('')

  const add = () => {
    const t = draft.trim()
    if (!t) return
    if (!tags.some((x) => x.toLowerCase() === t.toLowerCase())) {
      setTags([...tags, t])
    }
    setDraft('')
  }

  const save = () => {
    const t = draft.trim()
    const finalTags =
      t && !tags.some((x) => x.toLowerCase() === t.toLowerCase())
        ? [...tags, t]
        : tags
    onSave(finalTags)
  }
  return (
    <OverlayShell
      section="Organise"
      title="Edit tags"
      onClose={onClose}
      footer={
        <>
          <button type="button" onClick={onClose} className={BTN_GHOST}>
            Cancel
          </button>
          <button
            type="button"
            onClick={save}
            className={BTN_GOLD}
            style={{ background: GOLD_BTN }}
          >
            Save tags
          </button>
        </>
      }
    >
      <FieldLabel>Tags</FieldLabel>
      <div className="flex flex-wrap gap-1.5">
        {tags.map((t) => (
          <span
            key={t}
            className="inline-flex items-center gap-1.5 rounded-[3px] border border-border-2 bg-panel-2 px-2 py-1 text-[11px] text-text"
          >
            {t}
            <button
              type="button"
              onClick={() => setTags(tags.filter((x) => x !== t))}
              className="text-faint hover:text-stat-red"
              aria-label={`Remove ${t}`}
            >
              ×
            </button>
          </span>
        ))}
        {tags.length === 0 && (
          <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
            No tags
          </span>
        )}
      </div>
      <input
        autoFocus
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        placeholder="Type a tag, press Enter…"
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ',') {
            e.preventDefault()
            add()
          }
        }}
        className={INPUT_CLASS}
        style={INPUT_STYLE}
      />
    </OverlayShell>
  )
}
