import { describe, expect, it } from 'vitest'
import {
  activeSeasonId,
  affixes,
  canStarForge,
  charmsAllowStarsForge,
  effectiveStars,
  heroSiegeTree,
  isCharmSlot,
  nodeIcons,
  patched,
  seasonDataErrors,
  treeNodeInfo,
} from './index'
import affixesJson from './affixes.json'
import heroSiegeTreeJson from './hero-siege-tree.json'
import treeNodesJson from './tree-nodes.json'
import nodeIconsJson from './node-icons.json'

describe('data hub season resolution (default season)', () => {
  it('resolves s9 with no errors', () => {
    expect(activeSeasonId).toBe('s9')
    expect(seasonDataErrors).toEqual([])
  })

  it('s9 collections equal base data', () => {
    expect(affixes).toEqual(affixesJson)
    expect(treeNodeInfo).toEqual(treeNodesJson)
    expect(nodeIcons).toEqual(nodeIconsJson)
    expect(heroSiegeTree.nodes).toEqual(heroSiegeTreeJson.nodes)
    expect(heroSiegeTree.edges).toEqual(heroSiegeTreeJson.edges)
  })
})

describe('charm stars/forge eligibility', () => {
  it('isCharmSlot matches only charm_* slots', () => {
    expect(isCharmSlot('charm_1')).toBe(true)
    expect(isCharmSlot('charm_30')).toBe(true)
    expect(isCharmSlot('weapon')).toBe(false)
    expect(isCharmSlot('relic')).toBe(false)
  })

  it('charmsAllowStarsForge is true for every season except s9', () => {
    expect(charmsAllowStarsForge('s9')).toBe(false)
    expect(charmsAllowStarsForge('s10')).toBe(true)
    expect(charmsAllowStarsForge('s11')).toBe(true)
  })

  it('canStarForge: gear always, charms only outside s9, other never', () => {
    expect(canStarForge('weapon', 's9')).toBe(true)
    expect(canStarForge('weapon', 's10')).toBe(true)
    expect(canStarForge('charm_1', 's9')).toBe(false)
    expect(canStarForge('charm_1', 's10')).toBe(true)
    expect(canStarForge('relic', 's10')).toBe(false)
  })

  it('effectiveStars: charm stars vanish in s9 but apply elsewhere', () => {
    expect(effectiveStars('charm_1', 's9', 3)).toBe(null)
    expect(effectiveStars('charm_1', 's10', 3)).toBe(3)
    expect(effectiveStars('weapon', 's9', 3)).toBe(3)
    expect(effectiveStars('weapon', 's10', 3)).toBe(3)
    expect(effectiveStars('relic_1', 's10', 3)).toBe(null)
    expect(effectiveStars('charm_1', 's9', null)).toBe(null)
    expect(effectiveStars('weapon', 's9', null)).toBe(null)
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
