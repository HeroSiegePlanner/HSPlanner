import { beforeEach, describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import LoadoutBar from './LoadoutBar'
import { useBuild } from '../store/build'
import {
  emptyLoadoutSlots,
  initialLoadoutIndexes,
  LOADOUT_SLOT_COUNT,
} from '../utils/build/loadouts'

function reset() {
  useBuild.setState({
    loadoutSlots: emptyLoadoutSlots(),
    activeLoadouts: initialLoadoutIndexes(),
    allocatedTreeNodes: new Set<number>(),
    treeSocketed: {},
    skillRanks: {},
    subskillRanks: {},
  })
}

const slotButton = (n: number) =>
  screen.getByRole('button', { name: new RegExp(`^Loadout ${n}\\b`) })
const copyBtn = () => screen.getByRole('button', { name: /^Copy this loadout|^Pick a slot/ })

describe('LoadoutBar', () => {
  beforeEach(reset)

  it('renders all 8 slots with slot 1 active', () => {
    render(<LoadoutBar tab="tree" scopeLabel="Tree nodes" />)
    for (let i = 1; i <= LOADOUT_SLOT_COUNT; i++) {
      expect(slotButton(i)).toBeInTheDocument()
    }
    expect(slotButton(1)).toHaveAttribute('aria-pressed', 'true')
    expect(slotButton(2)).toHaveAttribute('aria-pressed', 'false')
  })

  it('switches the active slot on click', async () => {
    const user = userEvent.setup()
    render(<LoadoutBar tab="tree" scopeLabel="Tree nodes" />)

    await user.click(slotButton(3))

    expect(useBuild.getState().activeLoadouts.tree).toBe(2)
    expect(slotButton(3)).toHaveAttribute('aria-pressed', 'true')
    expect(slotButton(1)).toHaveAttribute('aria-pressed', 'false')
  })

  it('parks live state in the previous slot and restores it on return', async () => {
    const user = userEvent.setup()
    useBuild.setState({ allocatedTreeNodes: new Set([1, 2]) })
    render(<LoadoutBar tab="tree" scopeLabel="Tree nodes" />)

    await user.click(slotButton(2))
    expect(useBuild.getState().allocatedTreeNodes.size).toBe(0)

    await user.click(slotButton(1))
    expect([...useBuild.getState().allocatedTreeNodes]).toEqual([1, 2])
  })

  it('only drives its own tab', async () => {
    const user = userEvent.setup()
    render(<LoadoutBar tab="skills" scopeLabel="Skill points" />)

    await user.click(slotButton(4))

    expect(useBuild.getState().activeLoadouts.skills).toBe(3)
    expect(useBuild.getState().activeLoadouts.tree).toBe(0)
  })

  it('renames the active slot and shows the name in place of a static label', async () => {
    const user = userEvent.setup()
    render(<LoadoutBar tab="tree" scopeLabel="Tree nodes" />)
    // No static "Loadout" word — the bar leads with the player's own name.
    expect(screen.queryByText('Loadout')).toBeNull()

    await user.click(screen.getByRole('button', { name: 'Rename this loadout' }))
    await user.type(screen.getByRole('textbox', { name: 'Loadout name' }), 'Boss')
    await user.keyboard('{Enter}')

    expect(useBuild.getState().loadoutSlots.tree[0]?.name).toBe('Boss')
    expect(screen.getByText('Boss')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Loadout 1 — Boss' })).toBeInTheDocument()
  })

  it('abandons a rename on Escape', async () => {
    const user = userEvent.setup()
    render(<LoadoutBar tab="tree" scopeLabel="Tree nodes" />)

    await user.click(screen.getByRole('button', { name: 'Rename this loadout' }))
    await user.type(screen.getByRole('textbox', { name: 'Loadout name' }), 'Nope')
    await user.keyboard('{Escape}')

    expect(useBuild.getState().loadoutSlots.tree[0]?.name).toBeNull()
  })

  it('copies into the slot the user picks', async () => {
    const user = userEvent.setup()
    useBuild.setState({ allocatedTreeNodes: new Set([7]) })
    render(<LoadoutBar tab="tree" scopeLabel="Tree nodes" />)

    await user.click(copyBtn())
    await user.click(screen.getByRole('button', { name: /^Copy into loadout 6/ }))

    expect(useBuild.getState().loadoutSlots.tree[5]?.data?.allocatedTreeNodes).toEqual(
      new Set([7]),
    )
    // Copy mode ends after one pick; clicking a slot switches again.
    await user.click(slotButton(3))
    expect(useBuild.getState().activeLoadouts.tree).toBe(2)
  })

  it('overwrites a slot that already holds a loadout', async () => {
    const user = userEvent.setup()
    useBuild.setState({ allocatedTreeNodes: new Set([1]) })
    render(<LoadoutBar tab="tree" scopeLabel="Tree nodes" />)
    // Park something in slot 2, come back, then overwrite it.
    await user.click(slotButton(2))
    useBuild.setState({ allocatedTreeNodes: new Set([2]) })
    await user.click(slotButton(1))
    expect(useBuild.getState().loadoutSlots.tree[1]?.data?.allocatedTreeNodes).toEqual(
      new Set([2]),
    )

    await user.click(copyBtn())
    await user.click(screen.getByRole('button', { name: /^Copy into loadout 2/ }))

    expect(useBuild.getState().loadoutSlots.tree[1]?.data?.allocatedTreeNodes).toEqual(
      new Set([1]),
    )
  })

  it('cancels copy mode when the button is clicked again', async () => {
    const user = userEvent.setup()
    render(<LoadoutBar tab="tree" scopeLabel="Tree nodes" />)

    await user.click(copyBtn())
    expect(screen.getByText('Copy to…')).toBeInTheDocument()

    await user.click(copyBtn())
    expect(screen.queryByText('Copy to…')).not.toBeInTheDocument()
    // Back to plain switching.
    await user.click(slotButton(4))
    expect(useBuild.getState().activeLoadouts.tree).toBe(3)
  })

  it('never targets the source slot itself', async () => {
    const user = userEvent.setup()
    render(<LoadoutBar tab="tree" scopeLabel="Tree nodes" />)
    await user.click(copyBtn())
    // Slot 1 is active, so it stays a plain slot rather than a copy target.
    expect(slotButton(1)).toHaveAttribute('aria-pressed', 'true')
    expect(screen.queryByRole('button', { name: /^Copy into loadout 1/ })).toBeNull()
  })

  it('clears the active loadout, resetting that tab live state', async () => {
    const user = userEvent.setup()
    useBuild.setState({ allocatedTreeNodes: new Set([1, 2, 3]) })
    render(<LoadoutBar tab="tree" scopeLabel="Tree nodes" />)

    await user.click(
      screen.getByRole('button', { name: 'Clear this loadout (Tree nodes)' }),
    )

    expect(useBuild.getState().allocatedTreeNodes.size).toBe(0)
  })
})
