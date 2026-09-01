import { act, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { BuildSelectFooter } from './BuildSelectFooter'
import UpdateProvider from '../app/UpdateProvider'
import { BOOT_CHECK_DELAY_MS, markChecked } from '../../utils/updatePrefs'
import { checkForUpdate } from '../../utils/version'
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

const info: UpdateInfo = {
  current: '1.0.0',
  latest: '9.9.9',
  hasUpdate: true,
  releaseName: 'HSPlanner 9.9.9',
}

function renderFooter() {
  return render(
    <UpdateProvider>
      <BuildSelectFooter
        buildCount={2}
        folderCount={0}
        notice={null}
        autoOpen
        onToggleAutoOpen={() => {}}
      />
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

describe('<BuildSelectFooter> update status', () => {
  it('surfaces a release on the library screen without opening a build', async () => {
    checkMock.mockResolvedValue(info)
    renderFooter()
    await bootAndSettle()

    expect(
      screen.getByRole('button', { name: /v9\.9\.9 available/i }),
    ).toBeInTheDocument()
  })

  it('keeps the plain Check button when there is nothing new', async () => {
    checkMock.mockResolvedValue({ ...info, latest: '1.0.0', hasUpdate: false })
    renderFooter()
    await bootAndSettle()

    expect(screen.getByRole('button', { name: /^check$/i })).toBeInTheDocument()
    expect(screen.queryByText(/up to date/i)).not.toBeInTheDocument()
  })

  it('respects the cooldown shared with the planner bottom bar', async () => {
    markChecked(Date.now())
    checkMock.mockResolvedValue(info)
    renderFooter()
    await bootAndSettle()

    expect(checkMock).not.toHaveBeenCalled()
  })
})
