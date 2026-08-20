import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { items } from '@data'
import { useBuild } from '../../store/build'
import type { EquippedItem } from '../../types'
import { EquipmentDoll } from './EquipmentDoll'

const initialState = useBuild.getState()

function makeEquipped(): EquippedItem {
  return {
    baseId: items[0]!.id,
    affixes: [],
    socketCount: 0,
    socketed: [],
    socketTypes: [],
  }
}

describe('EquipmentDoll', () => {
  beforeEach(() => {
    useBuild.setState(initialState, true)
  })

  it('renders all 19 non-charm slots as empty buttons', () => {
    render(
      <EquipmentDoll activeSlot={null} offhandLocked={false} onSelect={() => {}} />,
    )

    const empty = screen.getAllByRole('button', { name: /: empty$/ })
    expect(empty).toHaveLength(19)
  })

  it('calls onSelect with the slot key when a slot is clicked', async () => {
    const onSelect = vi.fn()
    render(
      <EquipmentDoll activeSlot={null} offhandLocked={false} onSelect={onSelect} />,
    )

    await userEvent.click(screen.getByRole('button', { name: 'Helmet: empty' }))

    expect(onSelect).toHaveBeenCalledWith('helmet')
  })

  it('marks offhand as locked when offhandLocked is set', () => {
    render(
      <EquipmentDoll activeSlot={null} offhandLocked={true} onSelect={() => {}} />,
    )

    expect(
      screen.getByRole('button', { name: 'Offhand: locked' }),
    ).toBeInTheDocument()
  })

  it('toggles potion effects via the dot button', async () => {
    useBuild.setState({ inventory: { potion_1: makeEquipped() } })
    render(
      <EquipmentDoll activeSlot={null} offhandLocked={false} onSelect={() => {}} />,
    )

    await userEvent.click(
      screen.getByRole('button', { name: 'Potion 1 effects on' }),
    )

    expect(useBuild.getState().disabledPotions.potion_1).toBe(true)
    expect(
      screen.getByRole('button', { name: 'Potion 1 effects off' }),
    ).toBeInTheDocument()
  })
})
