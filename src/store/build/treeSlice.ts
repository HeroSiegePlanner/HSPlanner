import type { StateCreator } from 'zustand'
import { ADJ, findPath, reachableFromAny, START_IDS } from '../../utils/tree/treeGraph'
import type { BuildStore } from './types'

type TreeSlice = Pick<
  BuildStore,
  | 'allocatedTreeNodes'
  | 'treeSocketed'
  | 'toggleTreeNode'
  | 'applySuggestedNodes'
  | 'resetTreeNodes'
  | 'setTreeSocketed'
>

export const createTreeSlice: StateCreator<
  BuildStore,
  [],
  [],
  TreeSlice
> = (set) => ({
  allocatedTreeNodes: new Set<number>(),
  treeSocketed: {},

  toggleTreeNode: (nodeId) =>
    set((s) => {
      const cur = s.allocatedTreeNodes
      if (cur.has(nodeId)) {
        const next = new Set(cur)
        next.delete(nodeId)
        const stillReachable = reachableFromAny(START_IDS, next)
        return { allocatedTreeNodes: stillReachable }
      }
      const sources = new Set<number>([...cur, ...START_IDS])
      const path = findPath(sources, nodeId)
      if (!path) return s
      const next = new Set(cur)
      for (const id of path) next.add(id)
      return { allocatedTreeNodes: next }
    }),

  applySuggestedNodes: (ids) =>
    set((s) => {
      const next = new Set(s.allocatedTreeNodes)
      for (const id of ids) next.add(id)
      for (const sid of START_IDS) {
        if (next.has(sid)) continue
        const nbrs = ADJ.get(sid)
        if (!nbrs) continue
        for (const nb of nbrs) {
          if (next.has(nb)) {
            next.add(sid)
            break
          }
        }
      }
      const reachable = reachableFromAny(START_IDS, next)
      return { allocatedTreeNodes: reachable }
    }),

  resetTreeNodes: () => set({ allocatedTreeNodes: new Set<number>(), treeSocketed: {} }),

  setTreeSocketed: (nodeId, content) =>
    set((s) => {
      const next = { ...s.treeSocketed }
      if (content == null) delete next[nodeId]
      else next[nodeId] = content
      return { treeSocketed: next }
    }),
})
