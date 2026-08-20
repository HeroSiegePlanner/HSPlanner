import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { BuildPreview } from './BuildPreview'
import type { SavedBuild } from '../../utils/build/savedBuilds'

const build: SavedBuild = {
  id: 'b1',
  name: 'Fury of the Monsoon',
  classId: 'amazon',
  notes: '',
  createdAt: '2026-07-01T00:00:00.000Z',
  updatedAt: '2026-07-01T00:00:00.000Z',
  profiles: [
    { id: 'p1', name: 'Default', code: 'CODE', updatedAt: '2026-07-01T00:00:00.000Z' },
  ],
  activeProfileId: 'p1',
  folderId: null,
  favorite: false,
  tags: [],
  season: 's10',
}

const noop = () => {}

describe('BuildPreview share', () => {
  it('delegates the Share button to onShare with the build id', async () => {
    const onShare = vi.fn()
    const user = userEvent.setup()
    render(
      <BuildPreview
        build={build}
        meta={undefined}
        onOpen={noop}
        onShare={onShare}
        onSwitchProfile={noop}
        onAddProfile={noop}
        onRenameProfile={noop}
        onDuplicateProfile={noop}
        onRemoveProfile={noop}
      />,
    )
    await user.click(screen.getByRole('button', { name: /^share$/i }))
    expect(onShare).toHaveBeenCalledWith('b1')
  })
})
