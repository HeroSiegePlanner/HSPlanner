import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { BuildPreview } from './BuildPreview'
import type { SavedBuild } from '../../utils/build/savedBuilds'
import type { PreviewStats } from './usePreviewStats'

const previewMock = vi.fn<() => PreviewStats>()
vi.mock('./usePreviewStats', () => ({
  usePreviewStats: () => previewMock(),
}))

vi.mock('@data', () => ({
  activeSeasonId: 's10',
  getClass: () => undefined,
  getClassIcon: () => undefined,
  getMercClass: (id: string) =>
    id === 'merc_knight' ? { id, name: 'Knight' } : undefined,
  skills: [
    { id: 'sk_a', name: 'Lightning Surge' },
    { id: 'sk_b', name: 'Storm Cloud' },
    { id: 'sk_c', name: 'Static Shield' },
  ],
  gameConfig: { stats: [] },
  incarnationTree: { viewBox: '0 0 1 1', nodes: [], edges: [] },
  incarnationNodeInfo: {},
}))

const noop = () => {}

function makeBuild(season: string): SavedBuild {
  return {
    id: 'b1',
    name: 'Test build',
    classId: null,
    notes: '',
    createdAt: '2026-01-01T00:00:00.000Z',
    updatedAt: '2026-01-01T00:00:00.000Z',
    profiles: [{ id: 'p1', name: 'P1', code: 'CODE', updatedAt: '2026-01-01T00:00:00.000Z' }],
    activeProfileId: 'p1',
    folderId: null,
    favorite: false,
    tags: [],
    season,
  }
}

function renderPreview(season: string, preview: PreviewStats) {
  previewMock.mockReturnValue(preview)
  return render(
    <BuildPreview
      build={makeBuild(season)}
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

const SNAPSHOT_PREVIEW = {
  performance: null,
  snapshot: {
    activeSkillIds: [],
    skillRanks: {},
    allocatedTreeNodes: new Set<number>(),
    allocatedEtherNodes: new Set([1, 2, 3]),
    mercClassId: 'merc_knight',
  },
  loading: false,
  available: true,
} as unknown as PreviewStats

describe('<BuildPreview> metadata', () => {
  it('shows the build season in the header line', () => {
    renderPreview('s9', SNAPSHOT_PREVIEW)
    expect(screen.getByText('S9')).toBeInTheDocument()
    expect(screen.getByTitle('Season 9')).toBeInTheDocument()
  })

  it('shows ether node count and merc class from the snapshot', () => {
    renderPreview('s10', SNAPSHOT_PREVIEW)
    expect(screen.getByText('Ether')).toBeInTheDocument()
    expect(screen.getByText('3')).toBeInTheDocument()
    expect(screen.getByText('Merc')).toBeInTheDocument()
    expect(screen.getByText('Knight')).toBeInTheDocument()
  })

  it('shows placeholders when there is no snapshot', () => {
    renderPreview('s10', {
      performance: null,
      snapshot: null,
      loading: false,
      available: false,
    })
    expect(screen.getByText('Ether')).toBeInTheDocument()
    expect(screen.getByText('Merc')).toBeInTheDocument()
  })

  it('lists every active skill on its own row', () => {
    renderPreview('s10', {
      performance: null,
      snapshot: {
        activeSkillIds: ['sk_a', 'sk_b', 'sk_c'],
        skillRanks: {},
        allocatedTreeNodes: new Set<number>(),
        allocatedEtherNodes: new Set<number>(),
        mercClassId: null,
      },
      loading: false,
      available: true,
    } as unknown as PreviewStats)
    expect(screen.getByText('Lightning Surge')).toBeInTheDocument()
    expect(screen.getByText('Storm Cloud')).toBeInTheDocument()
    expect(screen.getByText('Static Shield')).toBeInTheDocument()
    expect(screen.getByText('Main Skills')).toBeInTheDocument()
    expect(screen.queryByText(/\+/)).not.toBeInTheDocument()
  })

  it('hides the main skill section when no skills are active', () => {
    renderPreview('s10', SNAPSHOT_PREVIEW)
    expect(screen.queryByText(/Main Skills?/)).not.toBeInTheDocument()
  })

  it('keeps the singular title for one active skill', () => {
    renderPreview('s10', {
      performance: null,
      snapshot: {
        activeSkillIds: ['sk_a'],
        skillRanks: {},
        allocatedTreeNodes: new Set<number>(),
        allocatedEtherNodes: new Set<number>(),
        mercClassId: null,
      },
      loading: false,
      available: true,
    } as unknown as PreviewStats)
    expect(screen.getByText('Main Skill')).toBeInTheDocument()
    expect(screen.getByText('Lightning Surge')).toBeInTheDocument()
  })
})
