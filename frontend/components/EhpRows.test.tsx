import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { EhpRows } from './EhpRows'

function renderWith(stats: Record<string, number>) {
  return render(<EhpRows stats={{}} statsCombined={stats} />)
}

describe('<EhpRows>', () => {
  it('renders nothing when the build has no life', () => {
    const { container } = renderWith({})
    expect(container).toBeEmptyDOMElement()
  })

  it('collapses to a single "eHP" row when every type is equal', () => {
    renderWith({ life: 1000 })
    expect(screen.getByText('eHP')).toBeInTheDocument()
    expect(screen.getByText('1,000')).toBeInTheDocument()
    expect(screen.queryByText('Fire eHP')).not.toBeInTheDocument()
  })

  it('shows "Physical eHP" + "Elemental eHP" when only physical differs', () => {
    renderWith({ life: 1000, physical_damage_reduction: 50 })
    expect(screen.getByText('Physical eHP')).toBeInTheDocument()
    expect(screen.getByText('2,000')).toBeInTheDocument()
    expect(screen.getByText('Elemental eHP')).toBeInTheDocument()
    expect(screen.getByText('1,000')).toBeInTheDocument()
  })

  it('lists every type when the elements differ', () => {
    renderWith({ life: 1000, fire_resistance: 50 })
    for (const label of [
      'Physical eHP',
      'Fire eHP',
      'Cold eHP',
      'Lightning eHP',
      'Poison eHP',
      'Arcane eHP',
    ]) {
      expect(screen.getByText(label)).toBeInTheDocument()
    }
    expect(screen.getByText('2,000')).toBeInTheDocument()
  })
})
