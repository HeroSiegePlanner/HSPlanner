interface HeroAllocation {
  allocatedTreeNodes: ReadonlySet<number>
}

export function heroLevelFor(alloc: HeroAllocation): number {
  return alloc.allocatedTreeNodes.size
}
