import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import SettingsModal from './SettingsModal'
import UpdateProvider from './UpdateProvider'
import { markChecked } from '../../utils/updatePrefs'
import { checkForUpdate } from '../../utils/version'
import type { UpdateInfo } from '../../utils/version'
import type * as VersionModule from '../../utils/version'

vi.mock('../../utils/installUpdate', () => ({
  inTauriRuntime: () => true,
  installUpdate: vi.fn(),
}))

vi.mock('../../utils/version', async (importOriginal) => {
  const actual = await importOriginal<typeof VersionModule>()
  return { ...actual, checkForUpdate: vi.fn() }
})

const checkMock = vi.mocked(checkForUpdate)

const upToDate: UpdateInfo = {
  current: '1.0.0',
  latest: '1.0.0',
  hasUpdate: false,
}

function renderSettings() {
  return render(
    <UpdateProvider>
      <SettingsModal onClose={() => {}} />
    </UpdateProvider>,
  )
}

beforeEach(() => {
  window.localStorage.clear()
  checkMock.mockReset()
})

describe('<SettingsModal> updates section', () => {
  it('shows automatic checking as on by default', () => {
    renderSettings()
    expect(
      screen.getByRole('checkbox', { name: /check for updates automatically/i }),
    ).toBeChecked()
  })

  it('persists the auto-check pref when turned off', async () => {
    const user = userEvent.setup()
    renderSettings()

    await user.click(
      screen.getByRole('checkbox', { name: /check for updates automatically/i }),
    )

    expect(window.localStorage.getItem('hsplanner.update.auto_check')).toBe('0')
  })

  it('offers no install-on-quit toggle', () => {
    renderSettings()
    expect(
      screen.queryByRole('checkbox', { name: /install updates on quit/i }),
    ).not.toBeInTheDocument()
  })

  it('runs a manual check on demand and reports the result', async () => {
    const user = userEvent.setup()
    checkMock.mockResolvedValue(upToDate)
    renderSettings()

    await user.click(screen.getByRole('button', { name: /check now/i }))

    expect(checkMock).toHaveBeenCalledTimes(1)
    expect(await screen.findByText(/up to date/i)).toBeInTheDocument()
  })

  it('checks even while the automatic cooldown is still running', async () => {
    const user = userEvent.setup()
    markChecked(Date.now())
    checkMock.mockResolvedValue(upToDate)
    renderSettings()

    await user.click(screen.getByRole('button', { name: /check now/i }))

    expect(checkMock).toHaveBeenCalledTimes(1)
  })

  it('says so when no check has ever run', () => {
    renderSettings()
    expect(screen.getByText(/never checked/i)).toBeInTheDocument()
  })

  it('reports how long ago the last check ran', () => {
    markChecked(Date.now() - 2 * 60 * 60 * 1000)
    renderSettings()
    expect(screen.getByText(/checked 2h ago/i)).toBeInTheDocument()
  })

  it('does not claim "never checked" once a check has run this session', async () => {
    const user = userEvent.setup()
    checkMock.mockResolvedValue({
      current: '1.0.0',
      latest: '9.9.9',
      hasUpdate: true,
      releaseName: 'HSPlanner 9.9.9',
    })
    renderSettings()

    await user.click(screen.getByRole('button', { name: /check now/i }))

    expect(
      await screen.findByRole('button', { name: /v9\.9\.9 available/i }),
    ).toBeInTheDocument()
    expect(screen.queryByText(/never checked/i)).not.toBeInTheDocument()
  })

  it('opens the update modal from settings', async () => {
    const user = userEvent.setup()
    checkMock.mockResolvedValue({
      current: '1.0.0',
      latest: '9.9.9',
      hasUpdate: true,
      body: '## New\n- thing',
    })
    renderSettings()

    await user.click(screen.getByRole('button', { name: /check now/i }))
    await user.click(
      await screen.findByRole('button', { name: /v9\.9\.9 available/i }),
    )

    expect(
      screen.getByRole('dialog', { name: /update available/i }),
    ).toBeInTheDocument()
  })
})
