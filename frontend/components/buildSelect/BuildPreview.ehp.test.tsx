import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { BuildPreview } from './BuildPreview'
import type { SavedBuild } from '../../utils/build/savedBuilds'
import type { PreviewStats } from './usePreviewStats'
import type { DamageType } from '../../utils/build/ehp'

const TYPES: DamageType[] = ['physical', 'fire', 'cold', 'lightning', 'poison', 'arcane']

const previewMock = vi.fn<() => PreviewStats>()
vi.mock('./usePreviewStats', () => ({
  usePreviewStats: () => previewMock(),
}))

const noop = () => {}

function makeBuild(): SavedBuild {
  return {
    id: 'b1',
    name: 'Test build',
    classId: null,
    notes: '',
    createdAt: '2026-01-01T00:00:00.000Z',
    updatedAt: '2026-01-01T00:00:00.000Z',
    profiles: [
      { id: 'p1', name: 'P1', code: 'CODE', updatedAt: '2026-01-01T00:00:00.000Z' },
    ],
    activeProfileId: 'p1',
    folderId: null,
    favorite: false,
    tags: [],
    season: 's10',
  }
}

function renderPreview(preview: PreviewStats) {
  previewMock.mockReturnValue(preview)
  return render(
    <BuildPreview
      build={makeBuild()}
      meta={undefined}
      onOpen={noop}
      onShare={noop}
      onSwitchProfile={noop}
      onAddProfile={noop}
      onRenameProfile={noop}
      onDuplicateProfile={noop}
      onRemoveProfile={noop}
    />,
  )
}

describe('<BuildPreview> effective HP', () => {
  it('shows eHP rows computed from the preview performance', () => {
    renderPreview({
      performance: {
        stats: {},
        statsCombined: { life: 1000, physical_damage_reduction: 50 },
        ehp: {
          entries: TYPES.map((type) => ({
            type,
            ehp: type === 'physical' ? 2000 : 1000,
            multiplier: 1,
            layers: [],
          })),
          worst: 'fire',
        },
      } as unknown as PreviewStats['performance'],
      snapshot: null,
      loading: false,
      available: true,
    })
    expect(screen.getByText('Physical eHP')).toBeInTheDocument()
    expect(screen.getByText('2,000')).toBeInTheDocument()
    expect(screen.getByText('Elemental eHP')).toBeInTheDocument()
  })

  it('renders no eHP rows while the calc result is missing', () => {
    renderPreview({
      performance: null,
      snapshot: null,
      loading: true,
      available: true,
    })
    expect(screen.queryByText(/eHP/)).not.toBeInTheDocument()
  })
})
