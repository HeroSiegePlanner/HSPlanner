import { z } from 'zod'

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

export const gameConfigPatchSchema = z
  .object({
    change: z.record(z.string(), z.unknown()).optional(),
    stats: listPatchSchema.optional(),
  })
  .strict()

export interface IncarnationPatchNode {
  id: number
  x: number
  y: number
  r: number
  t: 'root' | 'small' | 'big'
  icon: string
}

export interface EtherPatchNode extends IncarnationPatchNode {
  key: string
}

export interface SpatialTreePatch<N> {
  viewBox?: string
  addNodes?: N[]
  changeNodes?: Record<string, Partial<Omit<N, 'id'>>>
  removeNodes?: number[]
  addEdges?: [number, number][]
  removeEdges?: [number, number][]
}

export type IncarnationTreePatch = SpatialTreePatch<IncarnationPatchNode>

export interface EtherTreePatch extends SpatialTreePatch<EtherPatchNode> {
  stats?: RecordPatch<Record<string, unknown>>
}

const incarnationNodeSchema = z
  .object({
    id: z.number(),
    x: z.number(),
    y: z.number(),
    r: z.number(),
    t: z.enum(['root', 'small', 'big']),
    icon: z.string(),
  })
  .strict()

const etherNodeSchema = incarnationNodeSchema
  .extend({
    key: z.string(),
  })
  .strict()

function spatialTreePatchShape<S extends z.ZodObject>(nodeSchema: S) {
  return {
    viewBox: z.string().optional(),
    addNodes: z.array(nodeSchema).optional(),
    changeNodes: z
      .record(z.string(), nodeSchema.omit({ id: true }).partial())
      .optional(),
    removeNodes: z.array(z.number()).optional(),
    addEdges: z.array(z.tuple([z.number(), z.number()])).optional(),
    removeEdges: z.array(z.tuple([z.number(), z.number()])).optional(),
  }
}

export const incarnationTreePatchSchema = z
  .object(spatialTreePatchShape(incarnationNodeSchema))
  .strict()

export const etherTreePatchSchema = z
  .object({
    ...spatialTreePatchShape(etherNodeSchema),
    stats: recordPatchSchema.optional(),
  })
  .strict()

export interface MercDataPatch {
  change?: Record<string, unknown>
  classes?: ListPatch<Record<string, unknown>>
}

export const mercDataPatchSchema = z
  .object({
    change: z.record(z.string(), z.unknown()).optional(),
    classes: listPatchSchema.optional(),
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
  incarnationNodes?: RecordPatch<Record<string, unknown>>
  incarnationTree?: IncarnationTreePatch
  gameConfig?: GameConfigPatch
  starScaling?: RecordPatch<Record<string, unknown>>
  etherTree?: EtherTreePatch
  mercenaries?: MercDataPatch
}
