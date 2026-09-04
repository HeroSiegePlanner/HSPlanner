import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { EhpRows } from './EhpRows'
import type { DamageType, EhpResult } from '../utils/build/ehp'

const TYPES: DamageType[] = ['physical', 'fire', 'cold', 'lightning', 'poison', 'arcane']

function ehpOf(values: Partial<Record<DamageType, number>>): EhpResult {
  return {
    entries: TYPES.map((type) => ({ type, ehp: values[type] ?? 1000, multiplier: 1, layers: [] })),
    worst: 'physical',
  }
}

describe('<EhpRows>', () => {
  it('renders nothing when the engine has no entries', () => {
    const { container } = render(<EhpRows ehp={{ entries: [], worst: null }} />)
    expect(container).toBeEmptyDOMElement()
  })

  it('renders nothing while the engine result is missing', () => {
    const { container } = render(<EhpRows ehp={undefined} />)
    expect(container).toBeEmptyDOMElement()
  })

  it('collapses to a single "eHP" row when every type is equal', () => {
    render(<EhpRows ehp={ehpOf({})} />)
    expect(screen.getByText('eHP')).toBeInTheDocument()
    expect(screen.getByText('1,000')).toBeInTheDocument()
    expect(screen.queryByText('Fire eHP')).not.toBeInTheDocument()
  })

  it('shows "Physical eHP" + "Elemental eHP" when only physical differs', () => {
    render(<EhpRows ehp={ehpOf({ physical: 2000 })} />)
    expect(screen.getByText('Physical eHP')).toBeInTheDocument()
    expect(screen.getByText('2,000')).toBeInTheDocument()
    expect(screen.getByText('Elemental eHP')).toBeInTheDocument()
    expect(screen.getByText('1,000')).toBeInTheDocument()
  })

  it('lists every type when the elements differ', () => {
    render(<EhpRows ehp={ehpOf({ fire: 2000 })} />)
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
