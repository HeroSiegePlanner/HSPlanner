import { describe, expect, it } from 'vitest'
import {
  activeSeasonId,
  affixes,
  canStarForge,
  charmsAllowStarsForge,
  heroSiegeTree,
  isCharmSlot,
  nodeIcons,
  seasonDataErrors,
  treeNodeInfo,
} from './index'
import affixesJson from './affixes.json'
import heroSiegeTreeJson from './hero-siege-tree.json'
import treeNodesJson from './tree-nodes.json'
import nodeIconsJson from './node-icons.json'

// Empty localStorage under vitest resolves the default season (s9): collections equal base JSON.
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
})
