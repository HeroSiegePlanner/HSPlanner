import { describe, expect, it } from 'vitest'
import { SEASONS } from './registry'
import { loadSeasonPatchSet } from './load'
import {
  applyEtherTreePatch,
  applyGameConfigPatch,
  applyIncarnationTreePatch,
  applyListPatch,
  applyMercDataPatch,
  applyRecordMergePatch,
} from './resolve'
import type { RecordPatch, SeasonPatchSet } from './patchTypes'
import type { EtherTree, IncarnationTree, MercData } from '../../frontend/types'
import etherTreeJson from '../ether-tree.json'
import mercenariesJson from '../mercenaries.json'
import affixesJson from '../affixes.json'
import augmentsJson from '../augments.json'
import crystalsJson from '../crystals.json'
import gameConfigJson from '../game-config.json'
import incarnationNodesJson from '../incarnation-nodes.json'
import incarnationTreeJson from '../incarnation-tree.json'
import itemGrantedSkillsJson from '../item-granted-skills.json'
import runewordsJson from '../runewords.json'
import setsJson from '../sets.json'
import starScalingJson from '../star-scaling.json'

type Rec = Record<string, unknown>

const itemModules = import.meta.glob<{ default: Rec[] }>('../items/*.json', {
  eager: true,
})
const gemModules = import.meta.glob<{ default: Rec[] }>('../gems/*.json', {
  eager: true,
})
const runeModules = import.meta.glob<{ default: Rec[] }>('../runes/*.json', {
  eager: true,
})
const skillModules = import.meta.glob<{ default: Rec[] }>('../skills/*.json', {
  eager: true,
})
const classModules = import.meta.glob<{ default: Rec }>('../classes/*.json', {
  eager: true,
})

function collectFlat(modules: Record<string, { default: Rec[] }>): Rec[] {
  return Object.values(modules).flatMap((m) => m.default)
}

function collectScalar(modules: Record<string, { default: Rec }>): Rec[] {
  return Object.values(modules).map((m) => m.default)
}

const cases: ReadonlyArray<{
  name: string
  apply: (p: SeasonPatchSet) => { errors: string[] }
}> = [
  {
    name: 'affixes',
    apply: (p) => applyListPatch(affixesJson as Rec[], p.affixes, 'affixes'),
  },
  {
    name: 'crystals',
    apply: (p) => applyListPatch(crystalsJson as Rec[], p.crystals, 'crystals'),
  },
  {
    name: 'augments',
    apply: (p) => applyListPatch(augmentsJson as Rec[], p.augments, 'augments'),
  },
  {
    name: 'runewords',
    apply: (p) =>
      applyListPatch(runewordsJson as Rec[], p.runewords, 'runewords'),
  },
  {
    name: 'sets',
    apply: (p) => applyListPatch(setsJson as Rec[], p.sets, 'sets'),
  },
  {
    name: 'items',
    apply: (p) => applyListPatch(collectFlat(itemModules), p.items, 'items'),
  },
  {
    name: 'gems',
    apply: (p) => applyListPatch(collectFlat(gemModules), p.gems, 'gems'),
  },
  {
    name: 'runes',
    apply: (p) => applyListPatch(collectFlat(runeModules), p.runes, 'runes'),
  },
  {
    name: 'skills',
    apply: (p) => applyListPatch(collectFlat(skillModules), p.skills, 'skills'),
  },
  {
    name: 'classes',
    apply: (p) =>
      applyListPatch(collectScalar(classModules), p.classes, 'classes'),
  },
  {
    name: 'item-granted-skills',
    apply: (p) =>
      applyListPatch(
        itemGrantedSkillsJson as Rec[],
        p.itemGrantedSkills,
        'item-granted-skills',
        'name',
      ),
  },
  {
    name: 'incarnation-nodes',
    apply: (p) =>
      applyRecordMergePatch(
        incarnationNodesJson as Record<string, Rec>,
        p.incarnationNodes as unknown as RecordPatch<Rec> | undefined,
        'incarnation-nodes',
      ),
  },
  {
    name: 'incarnation-tree',
    apply: (p) =>
      applyIncarnationTreePatch(
        incarnationTreeJson as unknown as IncarnationTree,
        p.incarnationTree,
        'incarnation-tree',
      ),
  },
  {
    name: 'game-config',
    apply: (p) =>
      applyGameConfigPatch(gameConfigJson as Rec, p.gameConfig, 'game-config'),
  },
  {
    name: 'star-scaling',
    apply: (p) =>
      applyRecordMergePatch(
        starScalingJson as unknown as Record<string, Rec>,
        p.starScaling,
        'star-scaling',
      ),
  },
  {
    name: 'ether-tree',
    apply: (p) =>
      applyEtherTreePatch(
        etherTreeJson as unknown as EtherTree,
        p.etherTree,
        'ether-tree',
      ),
  },
  {
    name: 'mercenaries',
    apply: (p) =>
      applyMercDataPatch(
        mercenariesJson as unknown as MercData,
        p.mercenaries,
        'mercenaries',
      ),
  },
]

describe('season sweep', () => {
  for (const season of SEASONS) {
    it(`${season.id}: patches load and apply cleanly against base data`, () => {
      const load = loadSeasonPatchSet(season.id)
      expect(load.errors).toEqual([])
      for (const { name, apply } of cases) {
        const r = apply(load.patches)
        expect(r.errors, `${season.id}/${name}`).toEqual([])
      }
    })
  }
})

describe('incarnation node skill tags', () => {
  it('nodes carry g tags from game data', () => {
    const nodes = incarnationNodesJson as Record<string, { t: string; g?: string[] }>
    expect(nodes['443'].g).toEqual(['Ranged', 'Projectile'])
    expect(nodes['1951'].g).toEqual(['Area of Effect'])
    expect(nodes['0'].g).toBeUndefined()
    const tagged = Object.values(nodes).filter((n) => (n.g?.length ?? 0) > 0)
    expect(tagged.length).toBeGreaterThanOrEqual(480)
  })
})
