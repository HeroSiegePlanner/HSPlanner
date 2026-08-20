import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { EhpBreakdown } from './EhpBreakdown'
import { ehpMatchesQuery } from './statKeys'

function renderWith(stats: Record<string, number>) {
  return render(<EhpBreakdown stats={{}} statsCombined={stats} />)
}

describe('<EhpBreakdown>', () => {
  it('shows an empty state without life', () => {
    renderWith({})
    expect(screen.getByText(/Add life and defenses/)).toBeInTheDocument()
  })

  it('lists life and per-type eHP with mitigation layers', () => {
    renderWith({ life: 1000, fire_resistance: 75 })
    expect(screen.getByText('Life')).toBeInTheDocument()
    expect(screen.getByText('Fire resistance')).toBeInTheDocument()
    expect(screen.getByText('75%')).toBeInTheDocument()
    expect(screen.getByText('4,000')).toBeInTheDocument()
  })

  it('flags the weakest type', () => {
    renderWith({ life: 1000, fire_resistance: 75 })
    expect(screen.getByText('Physical · weakest')).toBeInTheDocument()
  })

  it('colors each type header with its resistance palette color', () => {
    renderWith({ life: 1000, fire_resistance: 75 })
    expect(screen.getByText('Fire').className).toContain('text-stat-red')
    expect(screen.getByText('Cold').className).toContain('text-stat-blue')
    expect(screen.getByText('Lightning').className).toContain('text-stat-orange')
    expect(screen.getByText('Poison').className).toContain('text-stat-green')
    expect(screen.getByText('Arcane').className).toContain('text-stat-purple')
  })
})

describe('ehpMatchesQuery', () => {
  it('matches the panel name and abbreviation', () => {
    expect(ehpMatchesQuery('ehp')).toBe(true)
    expect(ehpMatchesQuery('effective')).toBe(true)
    expect(ehpMatchesQuery('hit pool')).toBe(true)
  })

  it('matches damage types and mitigation layer names', () => {
    expect(ehpMatchesQuery('fire')).toBe(true)
    expect(ehpMatchesQuery('arcane')).toBe(true)
    expect(ehpMatchesQuery('resist')).toBe(true)
    expect(ehpMatchesQuery('damage taken')).toBe(true)
    expect(ehpMatchesQuery('life')).toBe(true)
  })

  it('does not match unrelated or empty queries', () => {
    expect(ehpMatchesQuery('')).toBe(false)
    expect(ehpMatchesQuery('   ')).toBe(false)
    expect(ehpMatchesQuery('magic find')).toBe(false)
  })
})
