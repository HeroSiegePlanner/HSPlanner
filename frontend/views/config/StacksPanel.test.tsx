import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/react'
import { useBuild } from '../../store/build'
import type * as CalcBridge from '../../utils/calc/bridge'

const stats = vi.hoisted(() => ({ current: {} as Record<string, number> }))

vi.mock('../../utils/calc/bridge', async (importOriginal) => ({
  ...(await importOriginal<typeof CalcBridge>()),
  computeBuildPerformanceAsync: vi.fn(async () => ({ stats: stats.current })),
}))

import StacksPanel from './StacksPanel'

async function renderPanel(next: Record<string, number>) {
  stats.current = next
  const view = render(<StacksPanel />)
  await screen.findByText('Combat Stacks').catch(() => null)
  return view
}

beforeEach(() => {
  useBuild.setState({ stackCounts: {} })
})

afterEach(() => {
  useBuild.setState({ stackCounts: {} })
})

describe('StacksPanel', () => {
  it('renders nothing when the build has no stack cap', async () => {
    const { container } = await renderPanel({})

    expect(container).toBeEmptyDOMElement()
  })

  it('defaults an unconfigured stack to its cap', async () => {
    await renderPanel({ max_rage_stacks: 6 })

    expect(await screen.findByLabelText('Rage stacks')).toHaveValue(6)
    expect(screen.getByText('/ 6')).toBeInTheDocument()
  })

  it('clamps a stored count above the cap', async () => {
    useBuild.setState({ stackCounts: { rage: 99 } })

    await renderPanel({ max_rage_stacks: 4 })

    expect(await screen.findByLabelText('Rage stacks')).toHaveValue(4)
  })

  it('writes a typed count to the store', async () => {
    await renderPanel({ max_rage_stacks: 6 })

    fireEvent.change(await screen.findByLabelText('Rage stacks'), {
      target: { value: '2' },
    })

    expect(useBuild.getState().stackCounts.rage).toBe(2)
  })

  it('clamps a typed count above the cap', async () => {
    await renderPanel({ max_rage_stacks: 6 })

    fireEvent.change(await screen.findByLabelText('Rage stacks'), {
      target: { value: '99' },
    })

    expect(useBuild.getState().stackCounts.rage).toBe(6)
  })

  it('clearing the field falls back to the cap', async () => {
    useBuild.setState({ stackCounts: { rage: 2 } })

    await renderPanel({ max_rage_stacks: 6 })

    fireEvent.change(await screen.findByLabelText('Rage stacks'), {
      target: { value: '' },
    })

    expect(useBuild.getState().stackCounts.rage).toBeUndefined()
  })

  it('shows the sprite named after the stack key', async () => {
    const { container } = await renderPanel({ max_rage_stacks: 6 })

    await screen.findByLabelText('Rage stacks')
    expect(container.querySelector('img')).toBeInTheDocument()
  })

  it('lists the per-stack effects the build actually has', async () => {
    await renderPanel({ max_rage_stacks: 6, damage_per_rage_stack: 1 })

    expect(
      await screen.findByText(/\+5% Attack Speed/),
    ).toBeInTheDocument()
    expect(screen.getByText(/\+1% Enhanced Damage/)).toBeInTheDocument()
  })
})
