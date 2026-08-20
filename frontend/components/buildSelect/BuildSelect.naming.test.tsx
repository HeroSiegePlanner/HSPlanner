import { describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import BuildSelect from './BuildSelect'
import { useBuild } from '../../store/build'
import { listSavedBuilds } from '../../utils/build/savedBuilds'
import { encodeBuildToShare } from '../../utils/build/shareBuild'

function renderSelect() {
  const onOpenBuild = vi.fn()
  render(<BuildSelect onOpenBuild={onOpenBuild} onClose={vi.fn()} canClose />)
  return onOpenBuild
}

describe('BuildSelect — New asks for a name', () => {
  it('creates a named build in the library and opens it', async () => {
    const before = listSavedBuilds().length
    const onOpenBuild = renderSelect()

    await userEvent.click(screen.getByRole('button', { name: /^new$/i }))
    await userEvent.type(
      screen.getByPlaceholderText(/lightning marksman/i),
      'My Fresh Build',
    )
    await userEvent.click(screen.getByRole('button', { name: /^create$/i }))

    await waitFor(() =>
      expect(listSavedBuilds().some((b) => b.name === 'My Fresh Build')).toBe(
        true,
      ),
    )
    expect(listSavedBuilds().length).toBe(before + 1)
    const rec = listSavedBuilds().find((b) => b.name === 'My Fresh Build')!
    expect(onOpenBuild).toHaveBeenCalledWith(rec.id)
  })
})

describe('BuildSelect — Import asks for a name on success', () => {
  it('shows a naming prompt after decode and saves under that name', async () => {
    const code = encodeBuildToShare(useBuild.getState().exportBuildSnapshot())
    const before = listSavedBuilds().length
    const onOpenBuild = renderSelect()

    await userEvent.click(screen.getByRole('button', { name: /import…/i }))
    await userEvent.click(screen.getByPlaceholderText(/paste shared build code/i))
    await userEvent.paste(code)
    await userEvent.click(
      screen.getByRole('button', { name: /import & open/i }),
    )

    const nameInput = await screen.findByDisplayValue(/^imported /i)
    await userEvent.clear(nameInput)
    await userEvent.type(nameInput, 'Renamed Import')
    await userEvent.click(screen.getByRole('button', { name: /save & open/i }))

    await waitFor(() =>
      expect(listSavedBuilds().some((b) => b.name === 'Renamed Import')).toBe(
        true,
      ),
    )
    expect(listSavedBuilds().length).toBe(before + 1)
    const rec = listSavedBuilds().find((b) => b.name === 'Renamed Import')!
    expect(onOpenBuild).toHaveBeenCalledWith(rec.id)
  })

  it('keeps the import overlay with an error for garbage input', async () => {
    const before = listSavedBuilds().length
    renderSelect()

    await userEvent.click(screen.getByRole('button', { name: /import…/i }))
    await userEvent.type(
      screen.getByPlaceholderText(/paste shared build code/i),
      'garbage',
    )
    await userEvent.click(
      screen.getByRole('button', { name: /import & open/i }),
    )

    await screen.findByText(
      /couldn't read a build code|invalid or corrupted build code/i,
    )
    expect(listSavedBuilds().length).toBe(before)
  })
})
