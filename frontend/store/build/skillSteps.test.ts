import { beforeEach, describe, expect, it } from 'vitest'
import { useBuild } from './index'
import { subskillKey } from './helpers'

const CHARGE_KEY = subskillKey('charge', 'power_charge')

describe('skill rank steps', () => {
  beforeEach(() => {
    useBuild.getState().setClass('viking')
    useBuild.getState().setLevel(100)
    useBuild.getState().resetSkillRanks()
  })

  it('adds and removes a whole step at once', () => {
    useBuild.getState().incSkillRank('charge', 20, 5)
    expect(useBuild.getState().skillRanks.charge).toBe(5)

    useBuild.getState().decSkillRank('charge', 3)
    expect(useBuild.getState().skillRanks.charge).toBe(2)
  })

  it('clamps a step to the skill max rank', () => {
    useBuild.getState().incSkillRank('charge', 20, 999)
    expect(useBuild.getState().skillRanks.charge).toBe(20)
  })

  it('clamps a step to the points still available', () => {
    useBuild.getState().setLevel(1)
    useBuild.getState().incSkillRank('charge', 20, 999)
    const available = 1 * 1
    expect(useBuild.getState().skillRanks.charge).toBeLessThanOrEqual(available)
  })

  it('drops the skill entirely when the step removes every rank', () => {
    useBuild.getState().incSkillRank('charge', 20, 4)
    useBuild.getState().decSkillRank('charge', 999)
    expect(useBuild.getState().skillRanks.charge).toBeUndefined()
  })
})

describe('subskill rank steps', () => {
  beforeEach(() => {
    useBuild.getState().setClass('viking')
    useBuild.getState().setLevel(100)
    useBuild.getState().resetSubskillsFor('charge')
  })

  it('adds a whole step and clamps it to the subskill max rank', () => {
    useBuild.getState().incSubskillRank('charge', 'power_charge', 5, 999)
    expect(useBuild.getState().subskillRanks[CHARGE_KEY]).toBe(5)
  })

  it('drops the subskill when the step removes every rank', () => {
    useBuild.getState().incSubskillRank('charge', 'power_charge', 5, 3)
    useBuild.getState().decSubskillRank('charge', 'power_charge', 999)
    expect(useBuild.getState().subskillRanks[CHARGE_KEY]).toBeUndefined()
  })
})
