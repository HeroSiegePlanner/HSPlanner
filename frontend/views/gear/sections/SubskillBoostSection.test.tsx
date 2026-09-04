import { beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/react'
import { useBuild } from '../../../store/build'
import type { EquippedItem } from '../../../types'
import { SubskillBoostSection } from './SubskillBoostSection'

const ITEM: EquippedItem = {
  baseId: 'charm_unholy_overloaded_dice',
  affixes: [],
  socketCount: 0,
  socketed: [],
  socketTypes: [],
}

beforeEach(() => {
  useBuild.setState({ classId: 'amazon' })
})

describe('SubskillBoostSection', () => {
  it('offers only skills that own a subtree', () => {
    render(<SubskillBoostSection equipped={ITEM} onChange={vi.fn()} />)

    fireEvent.click(screen.getByText('Pick the rolled skill'))

    expect(screen.getByText('Noxious Strike')).toBeTruthy()
    expect(screen.queryByText('Master Poisoner')).toBeNull()
  })

  it('reports the picked skill to the caller', () => {
    const onChange = vi.fn()
    render(<SubskillBoostSection equipped={ITEM} onChange={onChange} />)

    fireEvent.click(screen.getByText('Pick the rolled skill'))
    fireEvent.click(screen.getByText('Noxious Strike'))

    expect(onChange).toHaveBeenCalledWith('noxious_strike')
  })
})
