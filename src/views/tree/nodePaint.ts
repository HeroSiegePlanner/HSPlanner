import type { TreeNode } from './treeData'

export interface NodePaint {
  isAlloc: boolean
  isPreview: boolean
  isStart: boolean
  tier: TreeNode['tier']
}

export function nodeFill({ isAlloc, isPreview, isStart, tier }: NodePaint): string {
  if (isAlloc) return tier === 'keystone' ? '#e94f37' : '#c9a55a'
  if (isPreview) return '#5a4528'
  if (isStart) return '#3a3528'
  return '#1c1d24'
}

export function baseStrokeWidth(isStart: boolean, tier: TreeNode['tier']): number {
  if (isStart) return 2.5
  if (tier === 'minor') return 1
  return 2
}

export function nodeStroke({
  isAlloc,
  isPreview,
  isStart,
  tier,
}: NodePaint): string {
  if (isAlloc) return '#d4cfbf'
  if (isPreview) return '#e0b864'
  if (isStart) return '#c9a55a'
  if (tier === 'keystone') return '#8a6f3a'
  if (tier === 'notable') return '#5a5448'
  return '#3a3528'
}
