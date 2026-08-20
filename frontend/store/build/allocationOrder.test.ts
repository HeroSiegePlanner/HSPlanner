import { beforeEach, describe, expect, it } from 'vitest'
import { useBuild } from '../build'
import { ADJ, START_IDS } from '../../utils/tree/treeGraph'
import { ETHER_ADJ, ETHER_START_IDS } from '../../utils/tree/etherGraph'
import type { NodeAdjacency } from '../../utils/tree/treeGraph'

interface ClickPlan {
  start: number
  chain: number[]
  branch: number
}

function buildClickPlan(
  startIds: readonly number[],
  adj: NodeAdjacency,
): ClickPlan | null {
  for (const start of startIds) {
    const chain: number[] = []
    const used = new Set(startIds)
    let cur = start
    let branch: number | null = null
    for (let step = 0; step < 6; step++) {
      const valid = [...(adj.get(cur) ?? [])].filter((n) => !used.has(n))
      if (valid.length === 0) break
      const next = valid[valid.length - 1]!
      if (branch == null && valid.length >= 2) branch = valid[0]!
      chain.push(next)
      used.add(next)
      cur = next
    }
    if (chain.length >= 3 && branch != null && !chain.includes(branch)) {
      return { start, chain, branch }
    }
  }
  return null
}

beforeEach(() => {
  useBuild.setState({
    allocatedTreeNodes: new Set<number>(),
    allocatedEtherNodes: new Set<number>(),
    treeSocketed: {},
  })
})

describe('tree allocation keeps insertion order across edits', () => {
  it('deallocating a leaf preserves the remaining click order', () => {
    const plan = buildClickPlan(START_IDS, ADJ)
    expect(plan).not.toBeNull()
    const { start, chain, branch } = plan!
    const toggle = useBuild.getState().toggleTreeNode

    for (const id of chain) toggle(id)
    toggle(branch)
    const before = [...useBuild.getState().allocatedTreeNodes]
    expect(before).toEqual([start, ...chain, branch])

    const leaf = chain[chain.length - 1]!
    toggle(leaf)

    expect([...useBuild.getState().allocatedTreeNodes]).toEqual(
      before.filter((id) => id !== leaf),
    )
  })

  it('applySuggestedNodes keeps existing order and appends new nodes', () => {
    const plan = buildClickPlan(START_IDS, ADJ)
    expect(plan).not.toBeNull()
    const { start, chain, branch } = plan!
    const toggle = useBuild.getState().toggleTreeNode

    const [c1, c2, c3] = chain as [number, number, number]
    toggle(c1)
    toggle(c2)
    toggle(branch)

    const before = [...useBuild.getState().allocatedTreeNodes]
    expect(before).toEqual([start, c1, c2, branch])

    useBuild.getState().applySuggestedNodes([c3])

    const after = [...useBuild.getState().allocatedTreeNodes]
    expect(after.slice(0, before.length)).toEqual(before)
    const appended = after.slice(before.length)
    expect(appended).toContain(c3)
    for (const id of appended) {
      expect(id === c3 || START_IDS.includes(id)).toBe(true)
    }
  })
})

describe('ether allocation keeps insertion order across edits', () => {
  it('deallocating a leaf preserves the remaining click order', () => {
    const plan = buildClickPlan(ETHER_START_IDS, ETHER_ADJ)
    expect(plan).not.toBeNull()
    const { start, chain, branch } = plan!
    const toggle = useBuild.getState().toggleEtherNode

    for (const id of chain) toggle(id)
    toggle(branch)
    const before = [...useBuild.getState().allocatedEtherNodes]
    expect(before).toEqual([start, ...chain, branch])

    const leaf = chain[chain.length - 1]!
    toggle(leaf)

    expect([...useBuild.getState().allocatedEtherNodes]).toEqual(
      before.filter((id) => id !== leaf),
    )
  })
})
