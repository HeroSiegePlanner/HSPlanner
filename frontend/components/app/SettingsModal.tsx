import { useEffect, useState } from 'react'
import { Modal } from '../ui/Modal'
import Logo from '../ui/Logo'
import { useSettings } from '../../store/settings'
import {
  NUMBER_SCALES,
  compact,
  type NumberScale,
} from '../../utils/compactNumber'
import { openExternalLink } from '../../utils/externalUrl'
import { inTauriRuntime } from '../../utils/installUpdate'
import {
  isAutoCheckEnabled,
  readLastCheck,
  setAutoCheckEnabled,
} from '../../utils/updatePrefs'
import UpdateModal from './UpdateModal'
import { UpdateAvailableButton } from './UpdateStatus'
import { useUpdate, type CheckState } from './useUpdateCheck'
import { UI_ZOOM_STEPS } from '../../utils/uiZoom'
import { APP_VERSION, GITHUB_REPO } from '../../utils/version'

const IS_MAC =
  typeof navigator !== 'undefined' && /Mac/i.test(navigator.platform)
const SAVE_SHORTCUT = IS_MAC ? '⌘S' : 'Ctrl+S'

const SCALE_LABEL: Record<NumberScale, string> = {
  none: 'None',
  thousands: 'Thousands',
  millions: 'Millions',
  billions: 'Billions',
}

const SCALE_SAMPLE: Record<NumberScale, string> = {
  none: '12,345',
  thousands: '12.3k',
  millions: '12.3M',
  billions: '12.3B',
}

const PREVIEW_SAMPLES = [45_678, 12_345_678, 2_500_000_000]

interface SettingsModalProps {
  onClose: () => void
}

export default function SettingsModal({ onClose }: SettingsModalProps) {
  const autoSave = useSettings((s) => s.autoSave)
  const numberScale = useSettings((s) => s.numberScale)
  const setAutoSave = useSettings((s) => s.setAutoSave)
  const setNumberScale = useSettings((s) => s.setNumberScale)
  const uiZoom = useSettings((s) => s.uiZoom)
  const setUiZoom = useSettings((s) => s.setUiZoom)

  // update prefs live in their own localStorage keys, outside the settings
  // blob, so they are mirrored here instead of read through the store
  const [autoCheck, setAutoCheck] = useState(isAutoCheckEnabled)

  const onAutoCheckChange = (value: boolean) => {
    setAutoCheck(value)
    setAutoCheckEnabled(value)
  }

  // subscribing to the shared check keeps the "last checked" line honest after
  // the manual button runs
  const { check, hasRepo, onCheck, lastCheckedAt } = useUpdate()
  const [updateOpen, setUpdateOpen] = useState(false)
  const isChecking = check.kind === 'checking'
  const lastChecked = lastCheckedAt ?? readLastCheck()

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [onClose])

  return (
    <Modal
      onClose={onClose}
      panelClassName="w-[560px] max-w-[92vw] max-h-[86vh]"
      eyebrow="Preferences"
      title="Settings"
      titleId="settings-modal-title"
      subtitle="Stored on this device"
    >
      <div className="flex flex-col gap-6 overflow-y-auto px-6 py-5">
        <Section title="Saving">
          <ToggleRow
            checked={autoSave}
            onChange={setAutoSave}
            label="Auto-save"
            hint="Saves changes to the active build as you make them."
          />
          <p
            className={`mt-2 font-mono text-[10px] uppercase tracking-[0.14em] ${
              autoSave ? 'text-faint' : 'text-accent-hot/80'
            }`}
          >
            {autoSave
              ? `${SAVE_SHORTCUT} still saves instantly`
              : `Manual mode — press ${SAVE_SHORTCUT} to save the active build`}
          </p>
        </Section>

        <Section title="Numbers">
          <div className="mb-2 text-[13px] font-semibold text-text">
            Largest unit
          </div>
          <div
            role="radiogroup"
            aria-label="Largest number unit"
            className="grid grid-cols-4 gap-1.5"
          >
            {NUMBER_SCALES.map((scale) => {
              const active = numberScale === scale
              return (
                <button
                  key={scale}
                  type="button"
                  role="radio"
                  aria-checked={active}
                  onClick={() => setNumberScale(scale)}
                  className={`flex flex-col items-center gap-1 rounded-[3px] border px-2 py-2 transition-colors ${
                    active
                      ? 'border-accent-deep text-accent-hot'
                      : 'border-border-2 text-muted hover:border-accent-deep/60 hover:text-text'
                  }`}
                  style={
                    active
                      ? {
                          background:
                            'linear-gradient(180deg, rgba(58,46,24,0.55), rgba(42,36,24,0.35))',
                        }
                      : { background: 'var(--color-panel-2)' }
                  }
                >
                  <span className="flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.14em]">
                    <span
                      aria-hidden
                      className={`inline-block h-1 w-1 rotate-45 ${
                        active ? 'bg-accent-hot' : 'bg-faint'
                      }`}
                      style={
                        active
                          ? { boxShadow: '0 0 6px rgba(224,184,100,0.6)' }
                          : undefined
                      }
                    />
                    {SCALE_LABEL[scale]}
                  </span>
                  <span className="font-mono text-[11px] tabular-nums">
                    {SCALE_SAMPLE[scale]}
                  </span>
                </button>
              )
            })}
          </div>
          <p className="mt-2 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
            Preview ·{' '}
            <span className="normal-case text-accent-hot/80">
              {PREVIEW_SAMPLES.map((n) => compact(n, numberScale)).join('  ·  ')}
            </span>
          </p>
        </Section>

        {inTauriRuntime() && (
          <Section title="Display">
            <div className="mb-2 text-[13px] font-semibold text-text">
              UI scale
            </div>
            <div
              role="radiogroup"
              aria-label="UI scale"
              className="grid grid-cols-6 gap-1.5"
            >
              {UI_ZOOM_STEPS.map((step) => {
                const active = uiZoom === step
                return (
                  <button
                    key={step}
                    type="button"
                    role="radio"
                    aria-checked={active}
                    onClick={() => setUiZoom(step)}
                    className={`rounded-[3px] border px-2 py-2 font-mono text-[11px] tabular-nums transition-colors ${
                      active
                        ? 'border-accent-deep text-accent-hot'
                        : 'border-border-2 text-muted hover:border-accent-deep/60 hover:text-text'
                    }`}
                    style={
                      active
                        ? {
                            background:
                              'linear-gradient(180deg, rgba(58,46,24,0.55), rgba(42,36,24,0.35))',
                          }
                        : { background: 'var(--color-panel-2)' }
                    }
                  >
                    {Math.round(step * 100)}%
                  </button>
                )
              })}
            </div>
            <p className="mt-2 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
              Ctrl + / Ctrl - zooms too, this is the one that sticks
            </p>
          </Section>
        )}

        <Section title="Updates">
          <ToggleRow
            checked={autoCheck}
            onChange={onAutoCheckChange}
            label="Check for updates automatically"
            hint="Looks for a new release shortly after launch, at most once every few hours. Nothing is downloaded on its own."
          />
          <div className="mt-3 flex flex-wrap items-center gap-2.5">
            <button
              type="button"
              onClick={onCheck}
              disabled={!hasRepo || isChecking}
              className="rounded-[3px] border border-border-2 bg-panel-2 px-2.5 py-1 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors enabled:hover:border-accent-deep enabled:hover:text-accent-hot disabled:cursor-not-allowed disabled:opacity-50"
            >
              {isChecking ? 'Checking…' : 'Check now'}
            </button>
            <CheckOutcome check={check} onOpen={() => setUpdateOpen(true)} />
            <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
              {isChecking ? '' : lastCheckedLabel(lastChecked)}
            </span>
          </div>
          {updateOpen && check.kind === 'available' && (
            <UpdateModal
              info={check.info}
              onClose={() => setUpdateOpen(false)}
            />
          )}
        </Section>

        <Section title="Credits">
          <div className="flex items-center gap-2.5">
            <Logo size={20} glow title="HSPlanner" />
            <span
              className="font-mono text-[12px] uppercase tracking-[0.18em] text-accent-hot"
              style={{ textShadow: '0 0 10px rgba(224,184,100,0.25)' }}
            >
              HSPlanner
            </span>
            <span className="rounded-[3px] border border-border-2 px-1.5 py-px font-mono text-[10px] tracking-[0.14em] text-muted">
              v{APP_VERSION}
            </span>
          </div>
          <p className="mt-2 text-[12px] text-muted">
            Built and maintained by{' '}
            <span className="text-text">zium</span>.
          </p>
          <div className="mt-2.5 flex items-center gap-2">
            <ExternalChip
              href="https://ko-fi.com/zium1337"
              label="Support on Ko-fi"
            />
            <ExternalChip
              href={`https://github.com/${GITHUB_REPO}`}
              label="GitHub"
            />
          </div>
          <p className="mt-3 border-t border-border pt-2.5 font-mono text-[10px] uppercase tracking-[0.14em] leading-relaxed text-faint">
            Fan-made planner. Hero Siege © Panic Art Studios — not affiliated.
          </p>
        </Section>
      </div>
    </Modal>
  )
}

function CheckOutcome({
  check,
  onOpen,
}: {
  check: CheckState
  onOpen: () => void
}) {
  if (check.kind === 'available') {
    return (
      <UpdateAvailableButton info={check.info} onClick={onOpen} padding="px-2 py-1" />
    )
  }
  if (check.kind === 'ok') {
    return (
      <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-stat-green">
        Up to date
      </span>
    )
  }
  if (check.kind === 'error') {
    return (
      <span
        title={check.message}
        className="font-mono text-[10px] uppercase tracking-[0.14em] text-stat-red"
      >
        Check failed
      </span>
    )
  }
  return null
}

// the cooldown is only recorded when a check had nothing to show, so this
// prefers the in-memory timestamp from the current session
function lastCheckedLabel(at: number | null): string {
  if (at === null) return 'Never checked'
  const minutes = Math.floor((Date.now() - at) / 60_000)
  if (minutes < 1) return 'Checked just now'
  if (minutes < 60) return `Checked ${minutes}m ago`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `Checked ${hours}h ago`
  return `Checked ${Math.floor(hours / 24)}d ago`
}

function ToggleRow({
  checked,
  onChange,
  label,
  hint,
}: {
  checked: boolean
  onChange: (value: boolean) => void
  label: string
  hint: string
}) {
  return (
    <label className="flex cursor-pointer flex-col gap-1">
      <span className="flex items-center gap-2.5">
        <input
          type="checkbox"
          checked={checked}
          onChange={(e) => onChange(e.target.checked)}
          className="shrink-0"
        />
        <span className="text-[13px] font-semibold text-text">{label}</span>
      </span>
      <span className="pl-6 text-[12px] leading-snug text-muted">{hint}</span>
    </label>
  )
}

function Section({
  title,
  children,
}: {
  title: string
  children: React.ReactNode
}) {
  return (
    <section>
      <div className="mb-2.5 flex items-center gap-2 border-b border-accent-deep/20 pb-1.5">
        <span aria-hidden className="inline-block h-1 w-1 rotate-45 bg-accent-deep" />
        <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-accent-hot/70">
          {title}
        </span>
      </div>
      {children}
    </section>
  )
}

function ExternalChip({ href, label }: { href: string; label: string }) {
  return (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      onClick={(e) => openExternalLink(e, href)}
      className="inline-flex items-center gap-1.5 rounded-[3px] border border-accent-deep px-2.5 py-1 font-mono text-[10px] uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]"
      style={{
        background:
          'linear-gradient(180deg, rgba(58,46,24,0.5), rgba(42,36,24,0.35))',
      }}
    >
      <span aria-hidden className="inline-block h-1 w-1 rotate-45 bg-accent-hot" />
      {label}
    </a>
  )
}
