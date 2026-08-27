import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { SkillEffectsBlock } from './SkillEffectsBlock'
import { skills } from '@data'
import type { Skill } from '../../types'

vi.mock('../../hooks/useSkillRankInfo', () => ({
  useSkillRankInfo: () => new Map(),
}))

function show(skill: Skill) {
  render(
    <SkillEffectsBlock
      skill={skill}
      currentRank={1}
      effRankMin={1}
      effRankMax={1}
      allClassSkills={[skill]}
      skillRanks={{ [skill.id]: 1 }}
      attributes={{}}
      rankBonuses={{}}
      onSynergyHover={() => {}}
    />,
  )
}

function bySkillId(id: string): Skill {
  const found = skills.find((s) => s.id === id)
  if (!found) throw new Error(`skill ${id} missing`)
  return found
}

function rowValue(label: string): string {
  return screen.getByText(label).parentElement?.textContent ?? ''
}

describe('SkillEffectsBlock hit model rows', () => {
  it('shows the interval and the resolved hit count', () => {
    show(bySkillId('blazing_trail'))
    expect(rowValue('Hit interval')).toContain('0.5')
    expect(rowValue('Hits per cast')).toContain('6')
  })

  it('shows "unknown" when the object has no extracted lifetime', () => {
    show(bySkillId('volcano'))
    expect(rowValue('Hits per cast')).toContain('unknown')
  })

  it('omits the rows for a skill without a hit model', () => {
    show(bySkillId('fireball'))
    expect(screen.queryByText('Hit interval')).toBeNull()
    expect(screen.queryByText('Hits per cast')).toBeNull()
  })
})
