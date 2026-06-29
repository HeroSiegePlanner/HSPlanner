import { afterEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

vi.mock('../utils/build/gistShare', () => {
  class GistShareError extends Error {
    kind: string
    constructor(kind: string, message: string) {
      super(message)
      this.kind = kind
    }
  }
  return {
    GistShareError,
    isGistSharingConfigured: vi.fn(() => true),
    uploadBuildToGist: vi.fn(async () => ({
      id: 'abc123',
      url: 'https://gist.github.com/u/abc123',
    })),
  }
})

import { ShareDialog } from './ShareButton'
import { isGistSharingConfigured, uploadBuildToGist } from '../utils/build/gistShare'

afterEach(() => {
  vi.clearAllMocks()
  vi.mocked(isGistSharingConfigured).mockReturnValue(true)
})

const noop = () => {}

describe('ShareDialog — Gist sharing', () => {
  it('uploads and shows the gist link', async () => {
    render(
      <ShareDialog
        code="CODE"
        meta={{ className: 'amazon', level: 50 }}
        status="idle"
        onClose={noop}
        onCopy={noop}
      />,
    )
    await userEvent.click(screen.getByRole('button', { name: /share via gist/i }))
    await waitFor(() =>
      expect(screen.getByDisplayValue('https://gist.github.com/u/abc123')).toBeTruthy(),
    )
    expect(vi.mocked(uploadBuildToGist)).toHaveBeenCalledWith('CODE', {
      className: 'amazon',
      level: 50,
    })
  })

  it('disables the button when sharing is not configured', () => {
    vi.mocked(isGistSharingConfigured).mockReturnValue(false)
    render(<ShareDialog code="CODE" status="idle" onClose={noop} onCopy={noop} />)
    expect(screen.getByRole('button', { name: /share via gist/i })).toBeDisabled()
    expect(screen.getByText(/not configured in this build/i)).toBeTruthy()
  })
})
