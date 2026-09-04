import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import { skills } from '@data'
import type { BuildPerformance } from '../../utils/build/buildPerformance'
import SubskillTooltip from './SubskillTooltip'

const skill = skills.find((s) => s.id === 'noxious_strike')!
const sub = skill.subskills!.find((s) => s.id === 'corrosive_force')!

function show(rank: number, boost: number) {
  render(
    <SubskillTooltip
      skill={skill}
      sub={sub}
      rank={rank}
      boost={boost}
      x={0}
      y={0}
      isKeystone={false}
      currentPerformance={{} as BuildPerformance}
      previewPerformance={null}
    />,
  )
}

describe('SubskillTooltip item rank boost', () => {
  it('shows the boosted rank with its breakdown', () => {
    show(3, 1)

    expect(screen.getByText('4')).toBeTruthy()
    expect(screen.getByText('(3+1)')).toBeTruthy()
    // 8% poison skill damage per rank, read at the boosted rank
    expect(screen.getByText('32%')).toBeTruthy()
  })

  it('leaves an unallocated node at zero', () => {
    show(0, 1)

    expect(screen.getByText('0')).toBeTruthy()
    expect(screen.queryByText(/\(0\+1\)/)).toBeNull()
  })
})
