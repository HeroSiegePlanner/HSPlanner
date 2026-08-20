import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { EhpSummary } from './EhpSummary'

function renderWith(statsCombined: Record<string, number>) {
  return render(<EhpSummary stats={{}} statsCombined={statsCombined} />)
}

describe('<EhpSummary>', () => {
  it('shows a dash for avoidance without life', () => {
    renderWith({})
    expect(screen.getByText(/Avoidance/)).toBeInTheDocument()
    expect(screen.getAllByText('—').length).toBeGreaterThan(0)
  })

  it('lists only non-zero avoidance entries', () => {
    renderWith({ life: 1000, block_chance: 25, dodge_chance: 10 })
    expect(screen.getByText(/block 25%/)).toBeInTheDocument()
    expect(screen.getByText(/dodge 10%/)).toBeInTheDocument()
    expect(screen.queryByText(/spell dodge/)).not.toBeInTheDocument()
  })

  it('rounds fractional avoidance values to one decimal place', () => {
    renderWith({ life: 1000, block_chance: 24.55 })
    expect(screen.getByText(/block 24\.6%/)).toBeInTheDocument()
  })

  it('shows a dash when all avoidance is zero', () => {
    renderWith({ life: 1000 })
    expect(screen.getByText(/Avoidance/)).toBeInTheDocument()
    expect(screen.getAllByText(/—/).length).toBeGreaterThan(0)
  })

  it('renders insights for uncapped resistances', () => {
    renderWith({
      life: 1000,
      fire_resistance: 10,
      cold_resistance: 20,
      lightning_resistance: 75,
      poison_resistance: 75,
      arcane_resistance: 75,
    })
    expect(screen.getByText(/Cap fire res \(10→75\)/)).toBeInTheDocument()
    expect(screen.getByText(/Cap cold res \(20→75\)/)).toBeInTheDocument()
  })

  it('renders no insight block when everything is capped', () => {
    renderWith({
      life: 1000,
      fire_resistance: 75,
      cold_resistance: 75,
      lightning_resistance: 75,
      poison_resistance: 75,
      arcane_resistance: 75,
    })
    expect(screen.queryByText(/Cap /)).not.toBeInTheDocument()
  })
})
