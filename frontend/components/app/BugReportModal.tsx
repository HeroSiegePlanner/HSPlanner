import { useId, useRef, useState, type ClipboardEvent } from 'react'
import { Modal, MODAL_BTN_PRIMARY_CLASS, MODAL_FOOTER_CLASS } from '../ui/Modal'
import Dropdown from '../ui/Dropdown'
import {
  BugReportError,
  MAX_CONTACT_LENGTH,
  MAX_DESCRIPTION_LENGTH,
  MAX_EXPECTED_LENGTH,
  MAX_SCREENSHOTS,
  MAX_SCREENSHOT_BYTES,
  MAX_STEPS_LENGTH,
  MAX_TITLE_LENGTH,
  MIN_DESCRIPTION_LENGTH,
  MIN_TITLE_LENGTH,
  isBugReportConfigured,
  sendBugReport,
  type BugReportKind,
} from '../../utils/bugReport'

type SendState =
  | { kind: 'idle' }
  | { kind: 'busy' }
  | { kind: 'sent' }
  | { kind: 'error'; message: string }

const KIND_OPTIONS: { id: BugReportKind; label: string }[] = [
  { id: 'bug', label: 'Something is broken' },
  { id: 'data', label: 'Wrong item / skill data' },
  { id: 'idea', label: 'Idea or request' },
]

const TITLE_PLACEHOLDER: Record<BugReportKind, string> = {
  bug: 'Frost Nova shows 0 DPS',
  data: 'Crown of Ages has the wrong implicit',
  idea: 'Let me compare two profiles side by side',
}

const DESCRIPTION_PLACEHOLDER: Record<BugReportKind, string> = {
  bug: 'When I click here, this happens',
  data: 'Which item or skill, and what the values should be',
  idea: 'What would you like the planner to do?',
}

const STEPS_PLACEHOLDER = "1. Go to '...'\n2. Click on '...'\n3. Scroll down to '...'\n4. See error"

const FIELD_CLASS =
  'w-full rounded-[3px] border border-border-2 px-3 py-2 text-[12px] text-text placeholder:text-faint focus:border-accent-deep focus:outline-none focus:ring-2 focus:ring-accent-hot/15'
const LABEL_CLASS = 'font-mono text-[10px] uppercase tracking-[0.18em] text-faint'
const FIELD_BG = { background: 'linear-gradient(180deg, #0d0e12, var(--color-panel-2))' }
const TEXTAREA_STYLE = { ...FIELD_BG, boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)' }

export interface BugReportModalProps {
  buildCode: string | null
  buildLabel: string | null
  onClose: () => void
}

export default function BugReportModal({ buildCode, buildLabel, onClose }: BugReportModalProps) {
  const ids = useId()
  const fileInput = useRef<HTMLInputElement>(null)
  const [kind, setKind] = useState<BugReportKind>('bug')
  const [title, setTitle] = useState('')
  const [description, setDescription] = useState('')
  const [steps, setSteps] = useState('')
  const [expected, setExpected] = useState('')
  const [contact, setContact] = useState('')
  const [screenshots, setScreenshots] = useState<File[]>([])
  const [attachBuild, setAttachBuild] = useState(true)
  const [state, setState] = useState<SendState>({ kind: 'idle' })

  const configured = isBugReportConfigured()
  const incomplete =
    title.trim().length < MIN_TITLE_LENGTH ||
    description.trim().length < MIN_DESCRIPTION_LENGTH
  const canSend = configured && !incomplete && state.kind !== 'busy' && state.kind !== 'sent'
  const withBuild = attachBuild && buildCode !== null
  const isRepro = kind !== 'idea'

  const addFiles = (incoming: FileList | File[] | null) => {
    const images = Array.from(incoming ?? []).filter(
      (f) => f.type.startsWith('image/') && f.size <= MAX_SCREENSHOT_BYTES,
    )
    if (images.length === 0) return
    setScreenshots((current) => [...current, ...images].slice(0, MAX_SCREENSHOTS))
  }

  const onPasteScreenshot = (e: ClipboardEvent<HTMLTextAreaElement>) => {
    if (e.clipboardData.files.length > 0) addFiles(e.clipboardData.files)
  }

  const onSend = async () => {
    setState({ kind: 'busy' })
    try {
      await sendBugReport({
        kind,
        title: title.trim(),
        description: description.trim(),
        steps: isRepro ? steps.trim() : '',
        expected: isRepro ? expected.trim() : '',
        contact: contact.trim(),
        buildLabel: withBuild ? buildLabel : null,
        buildCode: withBuild ? buildCode : null,
        screenshots,
      })
      setState({ kind: 'sent' })
    } catch (e) {
      const message = e instanceof BugReportError ? e.message : 'Sending the report failed.'
      setState({ kind: 'error', message })
    }
  }

  const status = !configured
    ? 'Reporting is not configured in this build.'
    : state.kind === 'busy'
      ? 'Sending…'
      : state.kind === 'sent'
        ? 'Report sent — thank you!'
        : state.kind === 'error'
          ? state.message
          : incomplete
            ? 'A title and a short description are required.'
            : `Sends app version, OS${withBuild ? ' and your build code' : ''}.`

  const isError = state.kind === 'error' || !configured
  const isSuccess = state.kind === 'sent'

  return (
    <Modal
      onClose={onClose}
      eyebrow="Feedback"
      title="Report a problem"
      subtitle="Goes straight to the dev — no GitHub account needed."
      panelClassName="w-[36rem] max-w-[94vw] max-h-[88vh]"
    >
      <div className="flex flex-col gap-2.5 overflow-y-auto p-5">
        <span className={LABEL_CLASS}>Type</span>
        <Dropdown
          value={kind}
          options={KIND_OPTIONS}
          onChange={(id) => {
            if (id) setKind(id as BugReportKind)
          }}
          searchable={false}
          compact
        />

        <label htmlFor={`${ids}-title`} className={`${LABEL_CLASS} mt-1.5`}>
          Title
        </label>
        <input
          id={`${ids}-title`}
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          maxLength={MAX_TITLE_LENGTH}
          placeholder={TITLE_PLACEHOLDER[kind]}
          className={FIELD_CLASS}
          style={FIELD_BG}
        />

        <label htmlFor={`${ids}-desc`} className={`${LABEL_CLASS} mt-1.5`}>
          Describe your issue
        </label>
        <textarea
          id={`${ids}-desc`}
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          onPaste={onPasteScreenshot}
          maxLength={MAX_DESCRIPTION_LENGTH}
          rows={4}
          placeholder={DESCRIPTION_PLACEHOLDER[kind]}
          className={FIELD_CLASS}
          style={TEXTAREA_STYLE}
        />

        {isRepro && (
          <>
            <label htmlFor={`${ids}-steps`} className={`${LABEL_CLASS} mt-1.5`}>
              Steps to reproduce (optional)
            </label>
            <textarea
              id={`${ids}-steps`}
              value={steps}
              onChange={(e) => setSteps(e.target.value)}
              onPaste={onPasteScreenshot}
              maxLength={MAX_STEPS_LENGTH}
              rows={4}
              placeholder={STEPS_PLACEHOLDER}
              className={FIELD_CLASS}
              style={TEXTAREA_STYLE}
            />

            <label htmlFor={`${ids}-expected`} className={`${LABEL_CLASS} mt-1.5`}>
              What did you expect instead (optional)
            </label>
            <textarea
              id={`${ids}-expected`}
              value={expected}
              onChange={(e) => setExpected(e.target.value)}
              onPaste={onPasteScreenshot}
              maxLength={MAX_EXPECTED_LENGTH}
              rows={2}
              placeholder="A clear and concise description of what you expected to happen."
              className={FIELD_CLASS}
              style={TEXTAREA_STYLE}
            />
          </>
        )}

        <span className={`${LABEL_CLASS} mt-1.5`}>Screenshots (optional)</span>
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            onClick={() => fileInput.current?.click()}
            disabled={screenshots.length >= MAX_SCREENSHOTS}
            className="rounded-[3px] border border-border-2 px-2.5 py-1 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot disabled:cursor-not-allowed disabled:opacity-40"
          >
            Add image
          </button>
          <span className="text-[11px] text-faint">
            or paste with Ctrl+V into any field — up to {MAX_SCREENSHOTS}, 8 MB each
          </span>
          <input
            ref={fileInput}
            type="file"
            accept="image/*"
            multiple
            aria-label="Add screenshots"
            className="hidden"
            onChange={(e) => {
              addFiles(e.target.files)
              e.target.value = ''
            }}
          />
        </div>
        {screenshots.length > 0 && (
          <ul className="flex flex-col gap-1">
            {screenshots.map((shot, i) => (
              <li
                key={`${shot.name}-${i}`}
                className="flex items-center gap-2 text-[11px] text-muted"
              >
                <span className="truncate">{shot.name}</span>
                <button
                  type="button"
                  aria-label={`Remove ${shot.name}`}
                  onClick={() => setScreenshots((c) => c.filter((_, j) => j !== i))}
                  className="text-faint transition-colors hover:text-stat-red"
                >
                  ×
                </button>
              </li>
            ))}
          </ul>
        )}

        <label htmlFor={`${ids}-contact`} className={`${LABEL_CLASS} mt-1.5`}>
          Discord name (optional)
        </label>
        <input
          id={`${ids}-contact`}
          value={contact}
          onChange={(e) => setContact(e.target.value)}
          maxLength={MAX_CONTACT_LENGTH}
          placeholder="So I can ask follow-up questions"
          className={FIELD_CLASS}
          style={FIELD_BG}
        />

        {buildCode !== null && (
          <label
            htmlFor={`${ids}-attach`}
            className="mt-2 flex cursor-pointer items-center gap-2 text-[12px] text-muted"
          >
            <input
              id={`${ids}-attach`}
              type="checkbox"
              checked={attachBuild}
              onChange={(e) => setAttachBuild(e.target.checked)}
              className="accent-[var(--color-accent-deep)]"
            />
            Attach my build ({buildLabel ?? 'current build'}) so it can be reproduced
          </label>
        )}
      </div>

      <footer className={MODAL_FOOTER_CLASS}>
        <span
          className={`mr-auto min-w-0 flex-1 truncate font-mono text-[11px] ${
            isSuccess ? 'text-stat-green' : isError ? 'text-stat-red' : 'text-faint'
          }`}
        >
          {status}
        </span>
        <button
          type="button"
          onClick={isSuccess ? onClose : onSend}
          disabled={!isSuccess && !canSend}
          className={MODAL_BTN_PRIMARY_CLASS}
          style={{ background: 'linear-gradient(180deg, #3a2f1a, #2a2418)' }}
        >
          {isSuccess ? 'Done' : state.kind === 'busy' ? 'Sending…' : 'Send report'}
        </button>
      </footer>
    </Modal>
  )
}
