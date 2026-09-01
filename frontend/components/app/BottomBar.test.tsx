import { act, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import BottomBar from './BottomBar'
import UpdateProvider from './UpdateProvider'
import {
  BOOT_CHECK_DELAY_MS,
  markChecked,
  setAutoCheckEnabled,
} from '../../utils/updatePrefs'
import { UpdateCheckError, checkForUpdate } from '../../utils/version'
import type { UpdateInfo } from '../../utils/version'
import type * as VersionModule from '../../utils/version'

vi.mock('../../utils/installUpdate', () => ({
  inTauriRuntime: () => false,
  installUpdate: vi.fn(),
  installUpdateOnQuit: vi.fn(),
}))

vi.mock('../../utils/version', async (importOriginal) => {
  const actual = await importOriginal<typeof VersionModule>()
  return { ...actual, checkForUpdate: vi.fn() }
})

const checkMock = vi.mocked(checkForUpdate)

function updateInfo(latest: string, hasUpdate = true): UpdateInfo {
  return {
    current: '1.0.0',
    latest,
    hasUpdate,
    releaseName: `HSPlanner ${latest}`,
    releaseUrl: `https://example.test/r/${latest}`,
  }
}

// fake timers rule out testing-library's waitFor, so the boot timer and the
// promise it kicks off are both flushed here before asserting
function renderBar() {
  return render(
    <UpdateProvider>
      <BottomBar />
    </UpdateProvider>,
  )
}

async function bootAndSettle() {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(BOOT_CHECK_DELAY_MS)
  })
  await act(async () => {
    await Promise.resolve()
  })
}

beforeEach(() => {
  vi.useFakeTimers()
  window.localStorage.clear()
  checkMock.mockReset()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('<BottomBar> automatic update check', () => {
  it('surfaces a new release without anyone clicking Check', async () => {
    checkMock.mockResolvedValue(updateInfo('9.9.9'))
    renderBar()

    expect(checkMock).not.toHaveBeenCalled()
    await bootAndSettle()

    expect(
      screen.getByRole('button', { name: /v9\.9\.9 available/i }),
    ).toBeInTheDocument()
  })

  it('stays quiet when already up to date', async () => {
    checkMock.mockResolvedValue(updateInfo('1.0.0', false))
    renderBar()
    await bootAndSettle()

    expect(checkMock).toHaveBeenCalledTimes(1)
    expect(screen.queryByText(/up to date/i)).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: /^check$/i })).toBeInTheDocument()
  })

  it('swallows a failed automatic check instead of showing an error', async () => {
    checkMock.mockRejectedValue(new UpdateCheckError('Network error'))
    renderBar()
    await bootAndSettle()

    expect(screen.queryByText(/check failed/i)).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: /^check$/i })).toBeInTheDocument()
  })

  it('does not hit the network again inside the cooldown', async () => {
    markChecked(Date.now())
    checkMock.mockResolvedValue(updateInfo('9.9.9'))
    renderBar()
    await bootAndSettle()

    expect(checkMock).not.toHaveBeenCalled()
  })

  it('does not check at all when the pref is off', async () => {
    setAutoCheckEnabled(false)
    checkMock.mockResolvedValue(updateInfo('9.9.9'))
    renderBar()
    await bootAndSettle()

    expect(checkMock).not.toHaveBeenCalled()
  })

})
