import { describe, expect, it } from 'vitest'
import {
  activeSeasonId,
  affixes,
  canStarForge,
  effectiveStars,
  incarnationNodeInfo,
  incarnationTree,
  isCharmSlot,
  patched,
  seasonDataErrors,
} from './index'
import { DEFAULT_SEASON_ID } from './seasons/registry'
import affixesJson from './affixes.json'
import incarnationNodesJson from './incarnation-nodes.json'
import incarnationTreeJson from './incarnation-tree.json'

describe('data hub season resolution (default season)', () => {
  it('resolves the default season with no errors', () => {
    expect(activeSeasonId).toBe(DEFAULT_SEASON_ID)
    expect(seasonDataErrors).toEqual([])
  })

  it('serves base data when the season ships no patches', () => {
    expect(affixes).toEqual(affixesJson)
    expect(incarnationNodeInfo).toEqual(incarnationNodesJson)
    expect(incarnationTree.nodes).toEqual(incarnationTreeJson.nodes)
    expect(incarnationTree.viewBox).toBe(incarnationTreeJson.viewBox)
  })
})

describe('charm stars/forge eligibility', () => {
  it('isCharmSlot matches only charm_* slots', () => {
    expect(isCharmSlot('charm_1')).toBe(true)
    expect(isCharmSlot('charm_30')).toBe(true)
    expect(isCharmSlot('weapon')).toBe(false)
    expect(isCharmSlot('relic')).toBe(false)
  })

  it('canStarForge: gear and charms yes, other slots never', () => {
    expect(canStarForge('weapon')).toBe(true)
    expect(canStarForge('charm_1')).toBe(true)
    expect(canStarForge('charm_30')).toBe(true)
    expect(canStarForge('relic')).toBe(false)
  })

  it('effectiveStars: stars apply on gear and charms, vanish elsewhere', () => {
    expect(effectiveStars('charm_1', 3)).toBe(3)
    expect(effectiveStars('weapon', 3)).toBe(3)
    expect(effectiveStars('relic_1', 3)).toBe(null)
    expect(effectiveStars('charm_1', null)).toBe(null)
    expect(effectiveStars('weapon', undefined)).toBe(null)
  })
})

describe('patched() all-or-nothing fallback', () => {
  it('keeps the base collection when the patch result carries errors', () => {
    const base = [{ id: 'a' }]
    expect(patched(base, { data: [{ id: 'b' }], errors: ['boom'] })).toBe(base)
  })

  it('returns the patched data when there are no errors', () => {
    const base = [{ id: 'a' }]
    const next = [{ id: 'b' }]
    expect(patched(base, { data: next, errors: [] })).toBe(next)
  })
})
