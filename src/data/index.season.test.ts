import { describe, expect, it } from 'vitest'
import {
  activeSeasonId,
  affixes,
  heroSiegeTree,
  nodeIcons,
  seasonDataErrors,
  treeNodeInfo,
} from './index'
import affixesJson from './affixes.json'
import heroSiegeTreeJson from './hero-siege-tree.json'
import treeNodesJson from './tree-nodes.json'
import nodeIconsJson from './node-icons.json'

// localStorage is empty under vitest, so the hub resolves the default season
// (s9) and every collection must be identical to its base JSON.
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
