import type { ZodIssue, ZodType } from 'zod'
import {
  gameConfigPatchSchema,
  listPatchSchema,
  recordPatchSchema,
  scalarRecordPatchSchema,
  treePatchSchema,
  type SeasonPatchSet,
} from './patchTypes'

const patchModules = import.meta.glob<{ default: unknown }>('./*/*.patch.json', {
  eager: true,
})

const FILE_TO_KEY: Record<string, { key: keyof SeasonPatchSet; schema: ZodType }> = {
  'affixes': { key: 'affixes', schema: listPatchSchema },
  'crystals': { key: 'crystals', schema: listPatchSchema },
  'augments': { key: 'augments', schema: listPatchSchema },
  'runewords': { key: 'runewords', schema: listPatchSchema },
  'sets': { key: 'sets', schema: listPatchSchema },
  'items': { key: 'items', schema: listPatchSchema },
  'gems': { key: 'gems', schema: listPatchSchema },
  'runes': { key: 'runes', schema: listPatchSchema },
  'skills': { key: 'skills', schema: listPatchSchema },
  'classes': { key: 'classes', schema: listPatchSchema },
  'item-granted-skills': { key: 'itemGrantedSkills', schema: listPatchSchema },
  'tree-nodes': { key: 'treeNodes', schema: recordPatchSchema },
  'node-icons': { key: 'nodeIcons', schema: scalarRecordPatchSchema },
  'hero-siege-tree': { key: 'heroSiegeTree', schema: treePatchSchema },
  'game-config': { key: 'gameConfig', schema: gameConfigPatchSchema },
  'star-scaling': { key: 'starScaling', schema: recordPatchSchema },
}

export interface SeasonPatchLoad {
  patches: SeasonPatchSet
  errors: string[]
}

export function buildSeasonPatchSet(
  seasonId: string,
  modules: Record<string, { default: unknown }>,
): SeasonPatchLoad {
  const patches: Record<string, unknown> = {}
  const errors: string[] = []
  for (const [path, mod] of Object.entries(modules)) {
    const m = path.match(/^\.\/([^/]+)\/([^/]+)\.patch\.json$/)
    if (!m) continue
    const [, dir, name] = m
    if (!dir || !name) continue
    if (dir !== seasonId) continue
    const entry = FILE_TO_KEY[name]
    if (!entry) {
      errors.push(`seasons/${dir}/${name}.patch.json: unknown collection`)
      continue
    }
    const parsed = entry.schema.safeParse(mod.default)
    if (!parsed.success) {
      errors.push(
        `seasons/${dir}/${name}.patch.json: ${parsed.error.issues
          .map((i: ZodIssue) => `${i.path.join('.')} ${i.message}`)
          .join('; ')}`,
      )
      continue
    }
    patches[entry.key] = parsed.data
  }
  return { patches: patches as SeasonPatchSet, errors }
}

export function loadSeasonPatchSet(seasonId: string): SeasonPatchLoad {
  return buildSeasonPatchSet(seasonId, patchModules)
}
