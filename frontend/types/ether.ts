export type EtherNodeType = 'root' | 'small' | 'big'

export interface IncarnationNode {
  id: number
  x: number
  y: number
  r: number
  t: EtherNodeType
  icon: string
}

export interface IncarnationTree {
  viewBox: string
  nodes: IncarnationNode[]
  edges: [number, number][]
}

export interface EtherNode extends IncarnationNode {
  key: string
}

export interface EtherNodeStat {
  label: string
  value: string
  desc: string
}

export interface EtherTree {
  viewBox: string
  nodes: EtherNode[]
  edges: [number, number][]
  stats: Record<string, EtherNodeStat>
}
