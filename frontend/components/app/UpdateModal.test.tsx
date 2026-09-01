import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import UpdateModal from './UpdateModal'
import type { UpdateInfo } from '../../utils/version'

const mockInfo: UpdateInfo = {
  current: '0.4.2',
  latest: '0.5.0',
  hasUpdate: true,
  releaseUrl: 'https://example.test/r/0.5.0',
  releaseName: 'HSPlanner 0.5.0',
  body: `## New
- Ascendancy trees for all 6 classes
- Build sharing via short URL

## Improved
- Tree renderer is ~3x faster

## Balance
- Eagle Eye notable: +35% projectile damage → +28%

## Fixes
- Allocated nodes were sometimes deallocated`,
  publishedAt: '2026-04-28T14:30:00Z',
  assetName: 'HSPlanner.dmg',
  assetSize: 12_400_000,
  assetSha: '3a8fabcdef1234567890abcdef1234567890abcdef1234d214',
  assetUrl: 'https://example.test/dl/HSPlanner.dmg',
}

afterEach(() => {
  vi.restoreAllMocks()
})

describe('<UpdateModal>', () => {
  it('renders header, version compare, and changelog sections', () => {
    render(<UpdateModal info={mockInfo} onClose={() => {}} />)

    expect(
      screen.getByRole('dialog', { name: /update available/i }),
    ).toBeInTheDocument()
    expect(screen.getByText('v0.4.2')).toBeInTheDocument()
    expect(screen.getByText('v0.5.0')).toBeInTheDocument()

    expect(screen.getByText('NEW')).toBeInTheDocument()
    expect(screen.getByText('IMPROVED')).toBeInTheDocument()
    expect(screen.getByText('BALANCE')).toBeInTheDocument()
    expect(screen.getByText('FIXES')).toBeInTheDocument()

    expect(
      screen.getByText(/Ascendancy trees for all 6 classes/),
    ).toBeInTheDocument()
    expect(screen.getByText(/Tree renderer is ~3x faster/)).toBeInTheDocument()
  })

  it('shows asset size, short SHA, and release date', () => {
    render(<UpdateModal info={mockInfo} onClose={() => {}} />)
    expect(screen.getByText(/12\.4 MB|11\.8 MB/)).toBeInTheDocument()
    expect(screen.getByText(/3a8f…d214/)).toBeInTheDocument()
    expect(screen.getByText(/APR 28, 2026/i)).toBeInTheDocument()
  })

  it('closes on the X button and on Escape', async () => {
    const onClose = vi.fn()
    render(<UpdateModal info={mockInfo} onClose={onClose} />)

    await userEvent.click(screen.getByRole('button', { name: /close/i }))
    expect(onClose).toHaveBeenCalledTimes(1)

    fireEvent.keyDown(window, { key: 'Escape' })
    expect(onClose).toHaveBeenCalledTimes(2)
  })

  it('Download & Install opens asset URL and closes', async () => {
    const onClose = vi.fn()
    const openSpy = vi
      .spyOn(window, 'open')
      .mockImplementation(() => null)
    render(<UpdateModal info={mockInfo} onClose={onClose} />)

    await userEvent.click(
      screen.getByRole('button', { name: /download & install/i }),
    )
    expect(openSpy).toHaveBeenCalledWith(
      'https://example.test/dl/HSPlanner.dmg',
      '_blank',
      'noopener,noreferrer',
    )
    expect(onClose).toHaveBeenCalled()
  })

  it('offers no skip or auto-install controls', () => {
    render(<UpdateModal info={mockInfo} onClose={() => {}} />)
    expect(
      screen.queryByRole('button', { name: /skip this version/i }),
    ).not.toBeInTheDocument()
    expect(screen.queryByRole('checkbox')).not.toBeInTheDocument()
  })

  it('falls back gracefully when there is no body', () => {
    render(
      <UpdateModal
        info={{ ...mockInfo, body: undefined }}
        onClose={() => {}}
      />,
    )
    expect(
      screen.getByText(/no release notes available/i),
    ).toBeInTheDocument()
  })

  it('escapes raw HTML in changelog items', () => {
    render(
      <UpdateModal
        info={{
          ...mockInfo,
          body: '## Fixes\n- <script>alert(1)</script> XSS attempt',
        }}
        onClose={() => {}}
      />,
    )
    expect(document.querySelector('script')).toBeNull()
    expect(screen.getByText(/XSS attempt/)).toBeInTheDocument()
  })
})
