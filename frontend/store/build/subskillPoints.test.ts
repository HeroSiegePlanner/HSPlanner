import { beforeEach, describe, expect, it } from 'vitest'
import { subskillKey, subskillPointsFor, useBuild } from '../build'

describe('subskill point pool', () => {
  beforeEach(() => {
    useBuild.setState({ level: 10, subskillRanks: {} })
  })

  it('gives each skill its own pool', () => {
    const total = subskillPointsFor(10)
    expect(total).toBe(2)

    useBuild.getState().setSubskillRank('fireball', 'node_a', total, 99)
    useBuild.getState().setSubskillRank('blizzard', 'node_a', total, 99)

    const ranks = useBuild.getState().subskillRanks
    expect(ranks[subskillKey('fireball', 'node_a')]).toBe(total)
    expect(ranks[subskillKey('blizzard', 'node_a')]).toBe(total)
  })

  it('clamps to the pool within a single skill', () => {
    const total = subskillPointsFor(10)

    useBuild.getState().setSubskillRank('fireball', 'node_a', total, 99)
    useBuild.getState().setSubskillRank('fireball', 'node_b', 1, 99)

    const ranks = useBuild.getState().subskillRanks
    expect(ranks[subskillKey('fireball', 'node_a')]).toBe(total)
    expect(ranks[subskillKey('fireball', 'node_b')]).toBeUndefined()
  })

  it('clamps a raise to the points left in that skill', () => {
    useBuild.setState({ level: 25 })
    const total = subskillPointsFor(25)
    expect(total).toBe(5)

    useBuild.getState().setSubskillRank('fireball', 'node_a', 3, 99)
    useBuild.getState().setSubskillRank('fireball', 'node_b', 4, 99)

    expect(
      useBuild.getState().subskillRanks[subskillKey('fireball', 'node_b')],
    ).toBe(2)
  })
})
