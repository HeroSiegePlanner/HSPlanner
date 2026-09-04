import { beforeEach, describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import LoadoutSelect from './LoadoutSelect'
import { useBuild } from '../store/build'
import {
  emptyLoadoutSlots,
  initialLoadoutIndexes,
  LOADOUT_SLOT_COUNT,
} from '../utils/build/loadouts'
import type { EquippedItem } from '../types'

function item(baseId: string): EquippedItem {
  return {
    baseId,
    affixes: [],
    socketCount: 0,
    socketed: [],
    socketTypes: [],
    stars: 0,
    forgedMods: [],
  }
}

function reset() {
  useBuild.setState({
    loadoutSlots: emptyLoadoutSlots(),
    activeLoadouts: initialLoadoutIndexes(),
    inventory: {},
  })
}

const addBtn = () => screen.getByRole('button', { name: 'Add a stage' })
const renameBtn = () => screen.getByRole('button', { name: 'Rename this stage' })
const deleteBtn = () => screen.getByRole('button', { name: /^Delete |cannot be deleted$/ })
/** The select trigger — exact name, so it never collides with "Delete “X”". */
const trigger = (label: string) => screen.getByRole('button', { name: label })

async function addStage(user: ReturnType<typeof userEvent.setup>, name: string) {
  await user.click(addBtn())
  await user.type(screen.getByRole('textbox', { name: 'New stage name' }), name)
  await user.keyboard('{Enter}')
}

describe('LoadoutSelect', () => {
  beforeEach(reset)

  it('starts with a single unnamed stage as the current one', () => {
    render(<LoadoutSelect tab="gear" title="Progression" />)
    expect(screen.getByText('Progression')).toBeInTheDocument()
    expect(trigger('Stage 1')).toBeInTheDocument()
  })

  it('adds a named stage and makes it current', async () => {
    const user = userEvent.setup()
    render(<LoadoutSelect tab="gear" title="Progression" />)

    await addStage(user, 'Early')

    expect(useBuild.getState().activeLoadouts.gear).toBe(1)
    expect(useBuild.getState().loadoutSlots.gear[1]?.name).toBe('Early')
    expect(trigger('Early')).toBeInTheDocument()
  })

  it('does not create a stage without a name', async () => {
    const user = userEvent.setup()
    render(<LoadoutSelect tab="gear" title="Progression" />)

    await user.click(addBtn())
    await user.keyboard('{Enter}')

    expect(useBuild.getState().activeLoadouts.gear).toBe(0)
    expect(useBuild.getState().loadoutSlots.gear[1]?.name).toBeNull()
  })

  it('abandons a new stage on Escape', async () => {
    const user = userEvent.setup()
    render(<LoadoutSelect tab="gear" title="Progression" />)

    await user.click(addBtn())
    await user.type(screen.getByRole('textbox', { name: 'New stage name' }), 'Nope')
    await user.keyboard('{Escape}')

    expect(useBuild.getState().loadoutSlots.gear[1]?.name).toBeNull()
    expect(useBuild.getState().activeLoadouts.gear).toBe(0)
  })

  it('switches stage through the select, parking and restoring gear', async () => {
    const user = userEvent.setup()
    useBuild.setState({ inventory: { weapon: item('sword') } })
    render(<LoadoutSelect tab="gear" title="Progression" />)

    await addStage(user, 'Late')
    // The new stage starts with no gear; the old one kept the sword.
    expect(useBuild.getState().inventory).toEqual({})

    await user.click(trigger('Late'))
    await user.click(screen.getByRole('option', { name: /Stage 1/ }))

    expect(useBuild.getState().activeLoadouts.gear).toBe(0)
    expect(useBuild.getState().inventory).toEqual({ weapon: item('sword') })
  })

  it('lists every stage in the select', async () => {
    const user = userEvent.setup()
    render(<LoadoutSelect tab="gear" title="Progression" />)
    await addStage(user, 'Early')
    await addStage(user, 'Mid Game')

    await user.click(trigger('Mid Game'))

    expect(screen.getAllByRole('option')).toHaveLength(3)
    expect(screen.getByRole('option', { name: /Early/ })).toBeInTheDocument()
  })

  it('shows how many items each stage has', async () => {
    const user = userEvent.setup()
    useBuild.setState({ inventory: { weapon: item('sword'), helmet: item('cap') } })
    render(<LoadoutSelect tab="gear" title="Progression" />)

    await user.click(trigger('Stage 1'))

    expect(screen.getByRole('option', { name: /2 equipped/ })).toBeInTheDocument()
  })

  it('renames the current stage', async () => {
    const user = userEvent.setup()
    render(<LoadoutSelect tab="gear" title="Progression" />)

    await user.click(renameBtn())
    await user.type(screen.getByRole('textbox', { name: 'Stage name' }), 'Aspirational')
    await user.keyboard('{Enter}')

    expect(useBuild.getState().loadoutSlots.gear[0]?.name).toBe('Aspirational')
  })

  it('deletes the current stage and falls back to another', async () => {
    const user = userEvent.setup()
    useBuild.setState({ inventory: { weapon: item('sword') } })
    render(<LoadoutSelect tab="gear" title="Progression" />)
    await addStage(user, 'Late')

    await user.click(deleteBtn())

    expect(useBuild.getState().activeLoadouts.gear).toBe(0)
    expect(useBuild.getState().loadoutSlots.gear[1]).toEqual({ name: null, data: null })
    // Fell back onto the stage that still holds the sword.
    expect(useBuild.getState().inventory).toEqual({ weapon: item('sword') })
  })

  it('deletes a named stage that was never filled', async () => {
    const user = userEvent.setup()
    render(<LoadoutSelect tab="gear" title="Progression" />)
    await addStage(user, 'Empty Stage')
    expect(useBuild.getState().loadoutSlots.gear[1]?.name).toBe('Empty Stage')

    await user.click(deleteBtn())

    expect(useBuild.getState().loadoutSlots.gear[1]).toEqual({ name: null, data: null })
  })

  it('refuses to delete the last remaining stage', () => {
    render(<LoadoutSelect tab="gear" title="Progression" />)
    expect(
      screen.getByRole('button', { name: 'The last stage cannot be deleted' }),
    ).toBeDisabled()
  })

  it('stops adding once all slots are used', async () => {
    const user = userEvent.setup()
    render(<LoadoutSelect tab="gear" title="Progression" />)
    for (let i = 1; i < LOADOUT_SLOT_COUNT; i++) {
      await addStage(user, `Stage ${i + 1}`)
    }
    expect(
      screen.getByRole('button', { name: `All ${LOADOUT_SLOT_COUNT} stages used` }),
    ).toBeDisabled()
  })
})
