import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { EhpSummary } from './EhpSummary'
import type { DefenseInsight } from '../utils/build/ehp'

function renderWith(statsCombined: Record<string, number>, insights: DefenseInsight[] = []) {
  return render(<EhpSummary stats={{}} statsCombined={statsCombined} insights={insights} />)
}

describe('<EhpSummary>', () => {
  it('shows a dash for avoidance without any avoidance stats', () => {
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

  it('renders the engine insights verbatim', () => {
    renderWith({ life: 1000 }, [
      { text: 'Cap fire res (10→75): +260% EHP vs fire', gainPct: 260 },
      { text: 'Cap cold res (20→75): immunity vs cold', gainPct: null },
    ])
    expect(screen.getByText(/Cap fire res \(10→75\)/)).toBeInTheDocument()
    expect(screen.getByText(/Cap cold res \(20→75\)/)).toBeInTheDocument()
  })

  it('renders no insight block when the engine has none', () => {
    renderWith({ life: 1000 })
    expect(screen.queryByText(/Cap /)).not.toBeInTheDocument()
  })
})
