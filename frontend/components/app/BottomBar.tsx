import { useEffect, useMemo, useRef, useState } from 'react'
import changelogMd from '../../../CHANGELOG.md?raw'
import { useBuild } from '../../store/build'
import { useSettings } from '../../store/settings'
import { openExternalLink } from '../../utils/externalUrl'
import BugReportButton from './BugReportButton'
import UpdateStatus from './UpdateStatus'
import { APP_VERSION, BUILD_CHANNEL, type UpdateInfo } from '../../utils/version'
import UpdateModal from './UpdateModal'

export default function BottomBar() {
  const [changelogOpen, setChangelogOpen] = useState(false)

  const changelogInfo = useMemo<UpdateInfo>(
    () => ({
      current: APP_VERSION,
      latest: APP_VERSION,
      hasUpdate: false,
      body: changelogMd,
      releaseName: `HSPlanner v${APP_VERSION}`,
    }),
    [],
  )

  return (
    <footer
      data-tour="bottombar"
      className="flex h-9 shrink-0 items-center gap-2.5 border-t border-border px-3 text-[11px] text-muted"
      style={{
        background:
          'linear-gradient(180deg, var(--color-panel), var(--color-panel-2))',
        boxShadow:
          'inset 0 1px 0 rgba(201,165,90,0.08), 0 -1px 0 rgba(0,0,0,0.4)',
      }}
    >
      <span className="flex select-none items-center gap-1.5">
        <span
          aria-hidden
          className="inline-block h-1 w-1 rotate-45 bg-accent-deep"
          style={{ boxShadow: '0 0 6px rgba(138,111,58,0.5)' }}
        />
        <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-accent-deep">
          HSPlanner
        </span>
      </span>
      <span aria-hidden className="text-faint">
        ·
      </span>
      <button
        type="button"
        onClick={() => setChangelogOpen(true)}
        title="View changelog"
        className="cursor-pointer font-mono text-[10px] uppercase tracking-[0.14em] text-faint transition-colors hover:text-accent-hot"
      >
        v{APP_VERSION}
      </button>
      <BuildChannelBadge channel={BUILD_CHANNEL} />

      <span aria-hidden className="h-4 w-px bg-border" />

      <UpdateStatus />

      {changelogOpen && (
        <UpdateModal
          info={changelogInfo}
          mode="changelog"
          onClose={() => setChangelogOpen(false)}
        />
      )}

      <div className="ml-auto flex items-center gap-2.5">
        <BugReportButton />
      </div>

      <a
        href="https://ko-fi.com/zium1337"
        target="_blank"
        rel="noopener noreferrer"
        title="Support HSPlanner on Ko-fi"
        onClick={(e) => openExternalLink(e, 'https://ko-fi.com/zium1337')}
        className="inline-flex items-center gap-1.5 rounded-[3px] border border-accent-deep px-2.5 py-0.5 font-mono text-[10px] uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]"
        style={{
          background:
            'linear-gradient(180deg, rgba(58,46,24,0.5), rgba(42,36,24,0.35))',
        }}
      >
        <KofiIcon className="h-3 w-3" />
        Support on Ko-fi
      </a>
      <span aria-hidden className="h-4 w-px bg-border" />
      <SaveBadge />
    </footer>
  )
}

const IS_MAC =
  typeof navigator !== 'undefined' && /Mac/i.test(navigator.platform)
const SAVE_SHORTCUT = IS_MAC ? '⌘S' : 'Ctrl+S'
const SAVED_FLASH_MS = 1600

function SaveBadge() {
  const autoSave = useSettings((s) => s.autoSave)
  const savedBuildsVersion = useBuild((s) => s.savedBuildsVersion)
  const [flash, setFlash] = useState(false)
  const seenVersion = useRef(savedBuildsVersion)

  useEffect(() => {
    if (seenVersion.current === savedBuildsVersion) return
    seenVersion.current = savedBuildsVersion
    setFlash(true)
    const t = window.setTimeout(() => setFlash(false), SAVED_FLASH_MS)
    return () => window.clearTimeout(t)
  }, [savedBuildsVersion])

  if (autoSave) {
    return (
      <span className="flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-stat-green">
        <span
          aria-hidden
          className="h-1.5 w-1.5 rounded-full bg-stat-green"
          style={{ boxShadow: '0 0 8px rgba(116,201,138,0.65)' }}
        />
        Auto-saved
      </span>
    )
  }

  if (flash) {
    return (
      <span className="flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-stat-green">
        <span
          aria-hidden
          className="h-1.5 w-1.5 rounded-full bg-stat-green"
          style={{ boxShadow: '0 0 8px rgba(116,201,138,0.65)' }}
        />
        Saved
      </span>
    )
  }

  return (
    <span
      title={`Auto-save is off — press ${SAVE_SHORTCUT} to save`}
      className="flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-accent-hot"
    >
      <span
        aria-hidden
        className="h-1.5 w-1.5 rounded-full bg-accent-hot"
        style={{ boxShadow: '0 0 8px rgba(224,184,100,0.65)' }}
      />
      Manual · {SAVE_SHORTCUT}
    </span>
  )
}

function KofiIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
      className={className}
    >
      <path d="M17 8h1a4 4 0 0 1 0 8h-1" />
      <path d="M3 8h14v9a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4V8z" />
      <line x1="6" y1="2" x2="6" y2="4" />
      <line x1="10" y1="2" x2="10" y2="4" />
      <line x1="14" y1="2" x2="14" y2="4" />
    </svg>
  )
}

function BuildChannelBadge({ channel }: { channel: 'dev' | 'stable' }) {
  const isDev = channel === 'dev'
  const label = isDev ? 'DEV' : 'STABLE'
  const className = isDev
    ? 'border-accent-deep/50 text-accent-hot'
    : 'border-stat-green/50 text-stat-green'
  const bg = isDev
    ? 'linear-gradient(180deg, rgba(58,46,24,0.6), rgba(42,36,24,0.4))'
    : 'linear-gradient(180deg, rgba(28,52,34,0.6), rgba(20,38,24,0.4))'
  return (
    <span
      title={isDev ? 'Development build' : 'Stable build'}
      className={`rounded-[3px] border px-1.5 py-px font-mono text-[9px] uppercase tracking-[0.18em] ${className}`}
      style={{ background: bg }}
    >
      {label}
    </span>
  )
}
