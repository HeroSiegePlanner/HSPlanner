import { describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import BuildSelect from './BuildSelect'
import { useBuild } from '../../store/build'
import { listSavedBuilds } from '../../utils/build/savedBuilds'
import { encodeBuildToShare } from '../../utils/build/shareBuild'

function pasteText(text: string) {
  const evt = new Event('paste', { bubbles: true })
  Object.defineProperty(evt, 'clipboardData', {
    value: { getData: (type: string) => (type === 'text' ? text : '') },
  })
  window.dispatchEvent(evt)
}

function renderSelect() {
  return render(
    <BuildSelect onOpenBuild={vi.fn()} onClose={vi.fn()} canClose />,
  )
}

describe('BuildSelect — paste to import', () => {
  it('imports a pasted build code into the library as Unfiled', async () => {
    const code = encodeBuildToShare(useBuild.getState().exportBuildSnapshot())
    const before = listSavedBuilds().length
    renderSelect()

    pasteText(code)

    await waitFor(() => expect(listSavedBuilds().length).toBe(before + 1))
    const builds = listSavedBuilds()
    const added = builds[builds.length - 1]!
    expect(added.folderId).toBeNull()
    expect(added.name).toMatch(/^Imported /)
    expect((await screen.findAllByText(/imported/i)).length).toBeGreaterThan(0)
  })

  it('ignores pasted text that is not a build code', async () => {
    const before = listSavedBuilds().length
    renderSelect()

    pasteText('just some ordinary clipboard text')

    await new Promise((r) => setTimeout(r, 50))
    expect(listSavedBuilds().length).toBe(before)
  })
})
