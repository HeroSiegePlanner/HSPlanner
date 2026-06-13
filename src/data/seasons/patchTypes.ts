import { z } from 'zod'

// Canonical home of the tree-nodes.json node shape; treeStats.ts re-exports it.
export interface TreeNodeInfo {
  t: string
  n: string
  l: string[]
  g?: string[]
  note?: string
}

export interface ListPatch<T> {
  add?: T[]
  change?: Record<string, Partial<T>>
  remove?: string[]
}

export interface RecordPatch<T> {
  add?: Record<string, T>
  change?: Record<string, Partial<T>>
  remove?: string[]
}

export interface ScalarRecordPatch<T> {
  add?: Record<string, T>
  change?: Record<string, T>
  remove?: string[]
}

export type RawTreeNode = [number, number, number, number]
export type RawTreeEdge = [number, number, number, number]

export interface HeroSiegeTree {
  viewBox: string
  nodes: RawTreeNode[]
  edges: RawTreeEdge[]
}

export interface TreePatch {
  addNodes?: RawTreeNode[]
  changeNodes?: Record<string, [number, number, number]>
  removeNodes?: number[]
  addEdges?: [number, number][]
  removeEdges?: [number, number][]
}

export interface GameConfigPatch {
  change?: Record<string, unknown>
  stats?: ListPatch<Record<string, unknown>>
}

export const listPatchSchema = z
  .object({
    add: z.array(z.record(z.string(), z.unknown())).optional(),
    change: z.record(z.string(), z.record(z.string(), z.unknown())).optional(),
    remove: z.array(z.string()).optional(),
  })
  .strict()

export const recordPatchSchema = z
  .object({
    add: z.record(z.string(), z.record(z.string(), z.unknown())).optional(),
    change: z.record(z.string(), z.record(z.string(), z.unknown())).optional(),
    remove: z.array(z.string()).optional(),
  })
  .strict()

export const scalarRecordPatchSchema = z
  .object({
    add: z.record(z.string(), z.string()).optional(),
    change: z.record(z.string(), z.string()).optional(),
    remove: z.array(z.string()).optional(),
  })
  .strict()

export const treePatchSchema = z
  .object({
    addNodes: z.array(z.tuple([z.number(), z.number(), z.number(), z.number()])).optional(),
    changeNodes: z.record(z.string(), z.tuple([z.number(), z.number(), z.number()])).optional(),
    removeNodes: z.array(z.number()).optional(),
    addEdges: z.array(z.tuple([z.number(), z.number()])).optional(),
    removeEdges: z.array(z.tuple([z.number(), z.number()])).optional(),
  })
  .strict()

export const gameConfigPatchSchema = z
  .object({
    change: z.record(z.string(), z.unknown()).optional(),
    stats: listPatchSchema.optional(),
  })
  .strict()

export interface SeasonPatchSet {
  affixes?: ListPatch<Record<string, unknown>>
  crystals?: ListPatch<Record<string, unknown>>
  augments?: ListPatch<Record<string, unknown>>
  runewords?: ListPatch<Record<string, unknown>>
  sets?: ListPatch<Record<string, unknown>>
  items?: ListPatch<Record<string, unknown>>
  gems?: ListPatch<Record<string, unknown>>
  runes?: ListPatch<Record<string, unknown>>
  skills?: ListPatch<Record<string, unknown>>
  classes?: ListPatch<Record<string, unknown>>
  itemGrantedSkills?: ListPatch<Record<string, unknown>>
  treeNodes?: RecordPatch<Record<string, unknown>>
  nodeIcons?: ScalarRecordPatch<string>
  heroSiegeTree?: TreePatch
  gameConfig?: GameConfigPatch
  starScaling?: RecordPatch<Record<string, unknown>>
}
