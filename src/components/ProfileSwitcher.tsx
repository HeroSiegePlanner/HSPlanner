import { useCallback, useMemo, useRef, useState } from 'react'
import { useOutsideClick } from '../hooks/useOutsideClick'
import { useBuild } from '../store/build'
import { getSavedBuild } from '../utils/savedBuilds'

export default function ProfileSwitcher() {
  const activeBuildId = useBuild((s) => s.activeBuildId)
  const activeProfileId = useBuild((s) => s.activeProfileId)
  const savedBuildsVersion = useBuild((s) => s.savedBuildsVersion)
  const switchActiveProfile = useBuild((s) => s.switchActiveProfile)
  const addProfileToActiveBuild = useBuild((s) => s.addProfileToActiveBuild)
  const duplicateActiveProfile = useBuild((s) => s.duplicateActiveProfile)
  const renameActiveProfile = useBuild((s) => s.renameActiveProfile)
  const removeActiveProfile = useBuild((s) => s.removeActiveProfile)
  const commitActiveProfile = useBuild((s) => s.commitActiveProfile)

  const build = useMemo(
    () => (activeBuildId ? getSavedBuild(activeBuildId) : null),
    [activeBuildId, savedBuildsVersion],
  )

  const [popover, setPopover] = useState<'menu' | 'add' | 'rename' | null>(null)
  const [renameValue, setRenameValue] = useState('')
  const [renameTargetId, setRenameTargetId] = useState<string | null>(null)
  const [addValue, setAddValue] = useState('')
  const [pendingDeleteId, setPendingDeleteId] = useState<string | null>(null)
  const rootRef = useRef<HTMLDivElement | null>(null)

  const closeAll = useCallback(() => {
    setPopover(null)
    setPendingDeleteId(null)
  }, [])
  useOutsideClick(rootRef, popover !== null, closeAll)

  if (!activeBuildId || !build) return null

  const handleSwitch = (profileId: string) => {
    if (profileId === activeProfileId) return
    switchActiveProfile(profileId)
  }

  const handleAdd = () => {
    const name = addValue.trim() || `Profile ${build.profiles.length + 1}`
    addProfileToActiveBuild(name)
    setAddValue('')
    setPopover(null)
  }

  const handleStartRename = (profileId: string, currentName: string) => {
    setRenameTargetId(profileId)
    setRenameValue(currentName)
    setPopover('rename')
  }

  const handleRename = () => {
    if (!renameTargetId) return
    const name = renameValue.trim()
    if (name) renameActiveProfile(renameTargetId, name)
    setPopover(null)
    setRenameTargetId(null)
  }

  // Two-click confirmed remove; arms on first call, removes on second. Refuses to delete the last profile.
  const handleRemove = (profileId: string) => {
    if (build.profiles.length <= 1) return
    if (pendingDeleteId !== profileId) {
      setPendingDeleteId(profileId)
      return
    }
    removeActiveProfile(profileId)
    setPendingDeleteId(null)
  }

  const activeProfile =
    build.profiles.find((p) => p.id === activeProfileId) ?? build.profiles[0]
  if (!activeProfile) return null

  return (
    <div className="relative" ref={rootRef}>
      <div className="flex items-center gap-1">
        <span className="text-muted uppercase tracking-wider text-[10px]">
          Profile
        </span>
        <button
          onClick={() => {
            commitActiveProfile()
            setPopover((p) => (p === 'menu' ? null : 'menu'))
          }}
          className="inline-flex items-center gap-1.5 rounded border border-border bg-panel-2 px-2 py-1 text-xs text-text hover:border-accent hover:text-accent"
          title="Switch profile"
        >
          <span className="max-w-[10rem] truncate">{activeProfile.name}</span>
          <span className="text-muted text-[10px]">
            ({build.profiles.length})
          </span>
          <svg
            width="10"
            height="10"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <polyline points="6 9 12 15 18 9" />
          </svg>
        </button>
      </div>

      {popover === 'menu' && (
        <div className="absolute right-0 top-full z-50 mt-2 w-72 max-w-[90vw] rounded border border-border bg-panel p-2 text-xs shadow-lg">
          <div className="mb-1.5 px-1 text-[10px] uppercase tracking-wider text-muted">
            {build.name} · profiles
          </div>
          <ul className="max-h-72 overflow-y-auto space-y-0.5">
            {build.profiles.map((p) => (
              <li
                key={p.id}
                className={`flex items-center gap-1 rounded px-1.5 py-1 hover:bg-panel-2 ${
                  p.id === activeProfileId ? 'bg-accent/10' : ''
                }`}
              >
                <button
                  onClick={() => handleSwitch(p.id)}
                  className="flex-1 min-w-0 text-left"
                >
                  <div
                    className={`truncate ${
                      p.id === activeProfileId
                        ? 'text-accent font-medium'
                        : 'text-text'
                    }`}
                  >
                    {p.name}
                    {p.id === activeProfileId && (
                      <span className="ml-1 text-[10px] text-muted">
                        (active)
                      </span>
                    )}
                  </div>
                </button>
                <button
                  onClick={() => duplicateActiveProfile(p.id)}
                  className="text-muted hover:text-text px-1"
                  title="Duplicate"
                  aria-label="Duplicate"
                >
                  ⎘
                </button>
                <button
                  onClick={() => handleStartRename(p.id, p.name)}
                  className="text-muted hover:text-text px-1"
                  title="Rename"
                  aria-label="Rename"
                >
                  ✎
                </button>
                {pendingDeleteId === p.id ? (
                  <button
                    onClick={() => handleRemove(p.id)}
                    className="rounded border border-red-500/60 bg-red-500/10 px-1.5 py-0.5 text-[10px] text-red-400 hover:bg-red-500/20"
                    title="Click again to confirm"
                    aria-label="Confirm remove"
                  >
                    Confirm?
                  </button>
                ) : (
                  <button
                    onClick={() => handleRemove(p.id)}
                    disabled={build.profiles.length <= 1}
                    className="text-muted hover:text-red-400 px-1 disabled:cursor-not-allowed disabled:opacity-30"
                    title={
                      build.profiles.length <= 1
                        ? 'Cannot remove the last profile'
                        : 'Remove'
                    }
                    aria-label="Remove"
                  >
                    ×
                  </button>
                )}
              </li>
            ))}
          </ul>
          <div className="mt-1.5 border-t border-border pt-1.5">
            <button
              onClick={() => setPopover('add')}
              className="w-full rounded px-2 py-1 text-left text-accent hover:bg-accent/10"
            >
              + Add new profile
            </button>
          </div>
        </div>
      )}

      {popover === 'add' && (
        <div className="absolute right-0 top-full z-50 mt-2 w-72 max-w-[90vw] rounded border border-border bg-panel p-3 text-xs shadow-lg">
          <label className="text-[10px] uppercase tracking-wider text-muted">
            New profile name
          </label>
          <input
            autoFocus
            value={addValue}
            onChange={(e) => setAddValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleAdd()
              if (e.key === 'Escape') setPopover(null)
            }}
            placeholder={`Profile ${build.profiles.length + 1}`}
            className="mt-1 w-full rounded border border-border bg-panel-2 px-2 py-1"
          />
          <div className="mt-2 flex gap-1.5">
            <button
              onClick={handleAdd}
              className="flex-1 rounded border border-accent/50 bg-accent/10 px-2 py-1 text-accent hover:bg-accent/20"
            >
              Create
            </button>
            <button
              onClick={() => setPopover(null)}
              className="flex-1 rounded border border-border bg-panel-2 px-2 py-1 text-muted hover:text-text"
            >
              Cancel
            </button>
          </div>
          <div className="mt-2 text-[10px] text-muted">
            New profile is seeded with the current state, then activated.
          </div>
        </div>
      )}

      {popover === 'rename' && (
        <div className="absolute right-0 top-full z-50 mt-2 w-72 max-w-[90vw] rounded border border-border bg-panel p-3 text-xs shadow-lg">
          <label className="text-[10px] uppercase tracking-wider text-muted">
            Rename profile
          </label>
          <input
            autoFocus
            value={renameValue}
            onChange={(e) => setRenameValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleRename()
              if (e.key === 'Escape') setPopover(null)
            }}
            className="mt-1 w-full rounded border border-border bg-panel-2 px-2 py-1"
          />
          <div className="mt-2 flex gap-1.5">
            <button
              onClick={handleRename}
              className="flex-1 rounded border border-accent/50 bg-accent/10 px-2 py-1 text-accent hover:bg-accent/20"
            >
              Save
            </button>
            <button
              onClick={() => setPopover(null)}
              className="flex-1 rounded border border-border bg-panel-2 px-2 py-1 text-muted hover:text-text"
            >
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  )
}
