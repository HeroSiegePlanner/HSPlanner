import { describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ImportOverlay } from './overlays'

describe('ImportOverlay', () => {
  it('awaits an async onImport and shows the returned error', async () => {
    let resolve: (v: string | null) => void = () => {}
    const onImport = vi.fn(
      () => new Promise<string | null>((r) => (resolve = r)),
    )
    render(<ImportOverlay onImport={onImport} onClose={() => {}} />)

    await userEvent.type(screen.getByRole('textbox'), 'some-code')
    await userEvent.click(screen.getByRole('button', { name: /import & open/i }))

    // While the promise is pending the button shows the busy label and is disabled.
    const busyBtn = screen.getByRole('button', { name: /importing/i })
    expect(busyBtn).toBeDisabled()

    resolve('Invalid or corrupted build code')
    await waitFor(() =>
      expect(screen.getByText(/invalid or corrupted build code/i)).toBeTruthy(),
    )
    expect(onImport).toHaveBeenCalledWith('some-code')
  })
})
