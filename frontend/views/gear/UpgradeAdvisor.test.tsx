import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { UpgradeAdvisor } from './UpgradeAdvisor'
import { scanForUpgrades } from './lib/upgradeAdvisor'
import { useBuildPerformanceDeps } from '../../hooks/useBuildPerformanceDeps'
import type { BuildPerformanceDeps } from '../../utils/build/buildPerformance'
import type { UpgradeScanResult } from './lib/upgradeAdvisor'

// eslint-disable-next-line @typescript-eslint/consistent-type-imports
type AdvisorModule = typeof import('./lib/upgradeAdvisor')

vi.mock('./lib/upgradeAdvisor', async (importOriginal) => {
  const original = await importOriginal<AdvisorModule>()
  return { ...original, scanForUpgrades: vi.fn() }
})
vi.mock('../../hooks/useBuildPerformanceDeps', () => ({
  useBuildPerformanceDeps: vi.fn(),
}))

const mockScan = vi.mocked(scanForUpgrades)
const mockUseDeps = vi.mocked(useBuildPerformanceDeps)

function makeDeps(overrides: Partial<BuildPerformanceDeps> = {}): BuildPerformanceDeps {
  return {
    classId: 'amazon',
    level: 50,
    allocatedAttrs: {},
    inventory: {},
    skillRanks: {},
    subskillRanks: {},
    activeAuraId: null,
    activeBuffs: {},
    customStats: [],
    allocatedTreeNodes: new Set(),
    allocatedIncarnationNodes: new Set(),
    treeSocketed: {},
    activeSkillIds: ['skill-1'],
    enemyConditions: {},
    playerConditions: {},
    skillProjectiles: {},
    enemyResistances: {},
    procToggles: {},
    killsPerSec: 0,
    ...overrides,
  }
}

const EMPTY_RESULT: UpgradeScanResult = { emptySlots: [], upgrades: [] }

const SUGGESTIONS: UpgradeScanResult = {
  emptySlots: [{ slot: 'helm', slotName: 'Helm' }],
  upgrades: [
    {
      slot: 'weapon',
      slotName: 'Weapon',
      currentBaseName: 'Short Sword',
      bestBaseId: 'base_c',
      bestBaseName: 'Grandfather',
      gainPct: 23.4,
    },
    {
      slot: 'ring',
      slotName: 'Ring',
      currentBaseName: 'Manald Heal',
      bestBaseId: 'base_d',
      bestBaseName: 'Nagelring',
      gainPct: 2.3,
    },
  ],
}

beforeEach(() => {
  mockScan.mockReset()
  mockUseDeps.mockReset()
  mockUseDeps.mockReturnValue(makeDeps())
})

describe('<UpgradeAdvisor>', () => {
  it('starts idle with the scan button and no engine call', () => {
    render(<UpgradeAdvisor onPickSlot={() => {}} />)
    expect(
      screen.getByRole('button', { name: /scan for upgrades/i }),
    ).toBeEnabled()
    expect(mockScan).not.toHaveBeenCalled()
  })

  it('disables scanning without a main skill', () => {
    mockUseDeps.mockReturnValue(makeDeps({ activeSkillIds: [] }))
    render(<UpgradeAdvisor onPickSlot={() => {}} />)
    expect(
      screen.getByRole('button', { name: /scan for upgrades/i }),
    ).toBeDisabled()
    expect(screen.getByText(/select a main skill first/i)).toBeInTheDocument()
  })

  it('shows progress while scanning', async () => {
    let resolveScan: (v: UpgradeScanResult) => void = () => {}
    mockScan.mockImplementation((_deps, onProgress) => {
      onProgress?.(3, 12)
      return new Promise((resolve) => {
        resolveScan = resolve
      })
    })
    render(<UpgradeAdvisor onPickSlot={() => {}} />)
    await userEvent.click(
      screen.getByRole('button', { name: /scan for upgrades/i }),
    )
    expect(await screen.findByText(/scanning 3\/12/i)).toBeInTheDocument()
    resolveScan(SUGGESTIONS)
    await waitFor(() => expect(screen.getByText('Helm')).toBeInTheDocument())
  })

  it('shows a plain scanning label before the first progress update', async () => {
    let resolveScan: (v: UpgradeScanResult) => void = () => {}
    mockScan.mockImplementation(() => {
      return new Promise((resolve) => {
        resolveScan = resolve
      })
    })
    render(<UpgradeAdvisor onPickSlot={() => {}} />)
    await userEvent.click(
      screen.getByRole('button', { name: /scan for upgrades/i }),
    )
    expect(await screen.findByText(/^scanning…$/i)).toBeInTheDocument()
    expect(screen.queryByText(/scanning 0\/0/i)).not.toBeInTheDocument()
    resolveScan(EMPTY_RESULT)
  })

  it('renders suggestions with gain and the empty-slot summary row', async () => {
    mockScan.mockResolvedValue(SUGGESTIONS)
    render(<UpgradeAdvisor onPickSlot={() => {}} />)
    await userEvent.click(
      screen.getByRole('button', { name: /scan for upgrades/i }),
    )
    expect(await screen.findByText('Empty slots (1)')).toBeInTheDocument()
    expect(screen.getByText('Helm')).toBeInTheDocument()
    expect(screen.getByText(/fill first/i)).toBeInTheDocument()
    expect(screen.getByText('Short Sword → Grandfather')).toBeInTheDocument()
    expect(screen.getByText('Manald Heal → Nagelring')).toBeInTheDocument()
    expect(screen.getByText('+23%')).toBeInTheDocument()
    expect(screen.getByText('+2.3%')).toBeInTheDocument()
  })

  it('shows the empty-slot summary label and routes clicks to the first empty slot', async () => {
    mockScan.mockResolvedValue(SUGGESTIONS)
    const onPick = vi.fn()
    render(<UpgradeAdvisor onPickSlot={onPick} />)
    await userEvent.click(
      screen.getByRole('button', { name: /scan for upgrades/i }),
    )
    expect(await screen.findByText('Empty slots (1)')).toBeInTheDocument()
    await userEvent.click(screen.getByText('Helm'))
    expect(onPick).toHaveBeenCalledWith('helm')
  })

  it('invokes onPickSlot when an upgrade row is clicked', async () => {
    mockScan.mockResolvedValue(SUGGESTIONS)
    const onPick = vi.fn()
    render(<UpgradeAdvisor onPickSlot={onPick} />)
    await userEvent.click(
      screen.getByRole('button', { name: /scan for upgrades/i }),
    )
    await userEvent.click(await screen.findByText('Weapon'))
    expect(onPick).toHaveBeenCalledWith('weapon')
  })

  it('truncates the empty-slot summary to 3 names with a +K more suffix', async () => {
    mockScan.mockResolvedValue({
      emptySlots: [
        { slot: 'helm', slotName: 'Helm' },
        { slot: 'boots', slotName: 'Boots' },
        { slot: 'gloves', slotName: 'Gloves' },
        { slot: 'belt', slotName: 'Belt' },
      ],
      upgrades: [],
    })
    render(<UpgradeAdvisor onPickSlot={() => {}} />)
    await userEvent.click(
      screen.getByRole('button', { name: /scan for upgrades/i }),
    )
    expect(await screen.findByText('Empty slots (4)')).toBeInTheDocument()
    expect(
      screen.getByText('Helm, Boots, Gloves, +1 more'),
    ).toBeInTheDocument()
  })

  it('shows the optimal message when scan finds nothing', async () => {
    mockScan.mockResolvedValue(EMPTY_RESULT)
    render(<UpgradeAdvisor onPickSlot={() => {}} />)
    await userEvent.click(
      screen.getByRole('button', { name: /scan for upgrades/i }),
    )
    expect(
      await screen.findByText(/no base upgrades found/i),
    ).toBeInTheDocument()
  })

  it('clears results when the build changes', async () => {
    mockScan.mockResolvedValue(SUGGESTIONS)
    const { rerender } = render(<UpgradeAdvisor onPickSlot={() => {}} />)
    await userEvent.click(
      screen.getByRole('button', { name: /scan for upgrades/i }),
    )
    await screen.findByText('Helm')
    mockUseDeps.mockReturnValue(makeDeps({ level: 51 }))
    rerender(<UpgradeAdvisor onPickSlot={() => {}} />)
    await waitFor(() =>
      expect(screen.queryByText('Helm')).not.toBeInTheDocument(),
    )
  })

  it('ignores results from a scan orphaned by a build change', async () => {
    let resolveScan: (v: UpgradeScanResult) => void = () => {}
    mockScan.mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveScan = resolve
        }),
    )
    const { rerender } = render(<UpgradeAdvisor onPickSlot={() => {}} />)
    await userEvent.click(
      screen.getByRole('button', { name: /scan for upgrades/i }),
    )
    mockUseDeps.mockReturnValue(makeDeps({ level: 51 }))
    rerender(<UpgradeAdvisor onPickSlot={() => {}} />)
    resolveScan(SUGGESTIONS)
    await waitFor(() =>
      expect(screen.getByRole('button', { name: /scan for upgrades/i })).toBeEnabled(),
    )
    expect(screen.queryByText('Helm')).not.toBeInTheDocument()
  })

  it('shows the error state when the scan rejects and allows retry', async () => {
    mockScan.mockRejectedValue(new Error('bridge down'))
    render(<UpgradeAdvisor onPickSlot={() => {}} />)
    await userEvent.click(
      screen.getByRole('button', { name: /scan for upgrades/i }),
    )
    expect(await screen.findByText(/scan failed/i)).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: /scan for upgrades/i }),
    ).toBeEnabled()
  })
})
